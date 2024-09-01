//! # Dock Door State Management

//! This module manages the state of all dock doors in the system. It receives updates from PLC sensors and the WMS, 
//! processes them using an `EventHandler` and a `ContextAnalyzer`, and maintains an in-memory representation of each door's state. 
//! It also handles the persistence of events to the database.

use crate::models::WmsEvent;
use tokio::sync::{RwLock, mpsc, oneshot};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
use tracing::{debug, error, info, warn};
use crate::config::Settings;
use crate::analysis::ContextAnalyzer;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::event_handling::event_handler::EventHandler;
use crate::models::{DbInsert, DockDoor, DockDoorEvent, DoorState, PlcVal, SensorStateChangedEvent, WmsDoorStatus};

/// The maximum size of the event queue
const MAX_QUEUE_SIZE: usize = 1000;

/// Manages the state of all dock doors in the system
#[derive(Clone)]
pub struct DockDoorStateManager {
    /// A thread-safe map of dock doors, keyed by their names
    pub doors: Arc<RwLock<HashMap<String, DockDoor>>>,
    /// A thread-safe collection of database events to be inserted
    pub db_events: Arc<RwLock<Vec<DbInsert>>>,
    /// A channel for sending commands to the state manager
    command_sender: mpsc::Sender<StateManagerCommand>,
    /// A channel for sending events to the event handler
    event_sender: mpsc::Sender<DockDoorEvent>,
    /// A signal to trigger the shutdown of the state manager
    shutdown_signal: Arc<tokio::sync::Notify>,
}


/// Represents commands that can be sent to the `DockDoorStateManager`
#[derive(Debug)]
pub enum StateManagerCommand {
    /// Updates sensor values and returns generated events
    UpdateSensors(Vec<PlcVal>, oneshot::Sender<DockManagerResult<Vec<DockDoorEvent>>>),
    /// Updates door states from WMS data and returns generated events
    UpdateFromWms(Vec<WmsDoorStatus>, oneshot::Sender<DockManagerResult<Vec<DockDoorEvent>>>),
    /// Retrieves the state of a specific door
    GetDoorState(String, oneshot::Sender<DockManagerResult<DoorState>>),
    /// Handles WMS events and returns database insert events
    HandleWmsEvents(Vec<WmsEvent>, oneshot::Sender<DockManagerResult<Vec<DbInsert>>>),
    /// Retrieves and clears the batched database events
    GetAndClearDbBatch(oneshot::Sender<Vec<DbInsert>>),
    /// Evaluates and updates the states of all doors
    EvaluateDoorStates(oneshot::Sender<DockManagerResult<()>>),
}

impl DockDoorStateManager {
    /// Creates a new `DockDoorStateManager` and an associated `EventHandler`
    ///
    /// Initializes the state manager with dock doors from the settings, sets up communication channels,
    /// and spawns a task to run the state manager's event loop
    ///
    /// # Arguments
    ///
    /// * `settings`: The application settings containing door configurations
    /// * `context_analyzer`: The analyzer used to process events and generate insights
    ///
    /// # Returns
    ///
    /// A tuple containing the `DockDoorStateManager` and its associated `EventHandler`
    pub fn new(settings: &Settings, context_analyzer: ContextAnalyzer) -> (Self, EventHandler) {
        let (command_sender, command_receiver) = channel(100);
        let (event_sender, event_receiver) = channel(MAX_QUEUE_SIZE);

        let mut doors = HashMap::new();
        for plant in &settings.plants {
            for dock in &plant.dock_doors.dock_door_config {
                let door = DockDoor::new(
                    plant.plant_id.clone(),
                    dock.dock_name.clone(),
                    dock.dock_ip.clone(),
                    plant,
                );
                doors.insert(dock.dock_name.clone(), door);
            }
        }
        let shutdown_signal = Arc::new(tokio::sync::Notify::new());

        let manager = Self {
            doors: Arc::new(RwLock::new(doors)),
            db_events: Arc::new(RwLock::new(Vec::new())),
            command_sender: command_sender.clone(),
            event_sender: event_sender.clone(),
            shutdown_signal
        };

        let event_handler = EventHandler::new(
            event_receiver,
            Arc::new(manager.clone()),
            Arc::new(context_analyzer),
        );

        tokio::spawn(manager.clone().run(command_receiver));
        info!("Initializing State Manager");
        (manager, event_handler)
    }

    /// The main event loop of the state manager
    ///
    /// This asynchronous function continuously receives commands from the `command_receiver` channel
    /// and processes them using the `handle_command` method
    /// It also handles shutdown signals and performs cleanup when the loop terminates
    ///
    /// # Arguments
    ///
    /// * `command_receiver`: The channel for receiving `StateManagerCommand`s
    async fn run(self, mut command_receiver: Receiver<StateManagerCommand>) {
        loop {
            tokio::select! {
                Some(command) = command_receiver.recv() => {
                    debug!("Received Command: {:?}", command);
                    self.handle_command(command).await;
                }
                 _ = self.shutdown_signal.notified() => {
                    info!("Shutdown signal received, stopping DockDoorStateManager");
                    break;
                }
                else => {
                    info!("All receivers closed, shutting down DockDoorStateManager");
                    break;
                }
            }
        }
        self.cleanup().await;
    }

    /// Triggers the shutdown of the state manager
    pub async fn shutdown(&self) {
        self.shutdown_signal.notify_one();
    }

    /// Performs cleanup operations before the state manager shuts down
    ///
    /// This method flushes any remaining database events, closes PLC connections,
    /// cancels ongoing operations, logs the final state of doors, and releases resources
    async fn cleanup(&self) {
        info!("Performing cleanup for DockDoorStateManager");

        // Flush any remaining DB events
        let db_events = self.get_and_clear_db_batch().await;
        if !db_events.is_empty() {
            info!("Flushing {} remaining DB events", db_events.len());
            for event in db_events {
                if let Err(e) = self.insert_db_event(event).await {
                    error!("Error flushing DB events during cleanup: {:?}", e);
                }
            }
        }

        let doors = self.doors.read().await;
        // Close PLC connections TODO
        // for door in doors.values() {
        //     if let Err(e) = self.close_plc_connection(&door.dock_ip).await {
        //         error!("Error closing PLC connection for {}: {:?}", door.dock_name, e);
        //     }
        // }

        // Cancel any ongoing operations TODO
        // self.cancel_ongoing_operations().await;

        // Log final state
        for (name, door) in doors.iter() {
            info!("Final state of {}: {:?}", name, door.door_state);
        }

        // Release any held resources TODO
        // self.release_resources().await;

        // Notify other components of shutdown TODO
        // self.notify_shutdown().await;

        info!("Cleanup completed for DockDoorStateManager");
    }

    /// Returns a clone of the command sender for sending commands to the state manager
    pub fn get_command_sender(&self) -> mpsc::Sender<StateManagerCommand> {
        self.command_sender.clone()
    }

        /// Handles incoming commands to the state manager
    ///
    /// This method matches the command type and executes the corresponding internal method
    /// It also sends the result of the operation back through the provided `response_sender`
    ///
    /// # Arguments
    ///
    /// * `command`: The `StateManagerCommand` to be handled
    async fn handle_command(&self, command: StateManagerCommand) {
        match command {
            StateManagerCommand::UpdateSensors(sensor_values, response_sender) => {
                let result = self.internal_update_sensors(sensor_values).await;
                if let Err(e) = response_sender.send(result) {
                    error!("Failed to send UpdateSensors response: {:?}", e);
                }
            }
            StateManagerCommand::UpdateFromWms(wms_data, response_sender) => {
                let result = self.internal_update_from_wms(wms_data).await;
                if let Err(e) = response_sender.send(result) {
                    error!("Failed to send UpdateFromWms response: {:?}", e);
                }
            }
            StateManagerCommand::GetDoorState(door_name, response_sender) => {
                let result = self.internal_get_door_state(&door_name).await;
                if let Err(e) = response_sender.send(result) {
                    error!("Failed to send GetDoorState response: {:?}", e);
                }
            }
            StateManagerCommand::HandleWmsEvents(wms_events, response_sender) => {
                let result = self.handle_wms_events(wms_events).await;
                if let Err(e) = response_sender.send(result) {
                    error!("Failed to send HandleWmsEvents response: {:?}", e);
                }
            }
            StateManagerCommand::GetAndClearDbBatch(response_sender) => {
                let db_events = self.get_and_clear_db_batch().await;
                if let Err(e) = response_sender.send(db_events) {
                    error!("Failed to send GetAndClearDbBatch response: {:?}", e);
                }
            }
            StateManagerCommand::EvaluateDoorStates(response_sender) => {
                let result = self.evaluate_door_states().await;
                if let Err(e) = response_sender.send(result) {
                    error!("Failed to send EvaluateDoorStates response: {:?}", e);
                }
            }
        }
    }

    /// Updates sensor values for all doors and returns generated events
    ///
    /// This method sends an `UpdateSensors` command to the state manager's command channel
    /// and awaits the response containing the result of the operation and any generated events
    ///
    /// # Arguments
    ///
    /// * `sensor_values`: A vector of `PlcVal` representing the updated sensor values
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DockDoorEvent>)`: The events generated due to sensor updates
    /// * `Err(DockManagerError)`: If there's an error sending or receiving the command or processing the sensor updates
    pub async fn update_sensors(&self, sensor_values: Vec<PlcVal>) -> DockManagerResult<Vec<DockDoorEvent>> {
        let (tx, rx) = oneshot::channel();
        self.command_sender.send(StateManagerCommand::UpdateSensors(sensor_values, tx)).await?;
        rx.await?
    }

    /// Updates the state of dock doors based on WMS data and returns generated events
    ///
    /// This method sends an `UpdateFromWms` command to the state manager's command channel
    /// and awaits the response containing the result of the operation and any generated events
    ///
    /// # Arguments
    ///
    /// * `wms_data`: A vector of `WmsDoorStatus` representing the updated WMS data for the doors
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DockDoorEvent>)`: The events generated due to the WMS data update
    /// * `Err(DockManagerError)`: If there's an error sending or receiving the command or processing the WMS data
    pub async fn update_from_wms(&self, wms_data: Vec<WmsDoorStatus>) -> DockManagerResult<Vec<DockDoorEvent>> {
        let (tx, rx) = oneshot::channel();
        self.command_sender.send(StateManagerCommand::UpdateFromWms(wms_data, tx)).await?;
        rx.await?
    }

    /// Retrieves the current state of a specific door
    ///
    /// This method sends a `GetDoorState` command to the state manager's command channel
    /// and awaits the response containing the result of the operation (the door's state or an error)
    ///
    /// # Arguments
    ///
    /// * `door_name`: The name of the door whose state is to be retrieved
    ///
    /// # Returns
    ///
    /// * `Ok(DoorState)`: The current state of the specified door
    /// * `Err(DockManagerError)`: If there's an error sending or receiving the command, or if the door is not found
    pub async fn get_door_state(&self, door_name: &str) -> DockManagerResult<DoorState> {
        let (tx, rx) = oneshot::channel();
        self.command_sender.send(StateManagerCommand::GetDoorState(door_name.to_string(), tx)).await?;
        rx.await?
    }

    /// Processes WMS events and generates corresponding database insert events
    ///
    /// This method sends a `HandleWmsEvents` command to the state manager's command channel
    /// and awaits the response containing the result of the operation and the generated `DbInsert` events
    ///
    /// # Arguments
    ///
    /// * `wms_events`: A vector of `WmsEvent` representing the WMS events to be processed
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DbInsert>)`: The `DbInsert` events generated from the WMS events
    /// * `Err(DockManagerError)`: If there's an error sending or receiving the command or handling the WMS events
    pub async fn process_wms_events(&self, wms_events: Vec<WmsEvent>) -> DockManagerResult<Vec<DbInsert>> {
        let (tx, rx) = oneshot::channel();
        self.command_sender.send(StateManagerCommand::HandleWmsEvents(wms_events, tx)).await?;
        rx.await?
    }

    /// Internally updates the sensor values for all doors and generates events
    ///
    /// This method is called in response to the `UpdateSensors` command
    /// It iterates through the provided sensor values, updates the corresponding sensors in the `doors` map
    /// and generates `SensorStateChangedEvent` if the sensor value has changed
    /// It logs informational and debug messages and returns the generated events or an error
    ///
    /// # Arguments
    ///
    /// * `sensor_values`: A vector of `PlcVal` representing the updated sensor values
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DockDoorEvent>)`: The events generated due to sensor updates
    /// * `Err(DockManagerError)`: If there's an error updating a sensor
    async fn internal_update_sensors(&self, sensor_values: Vec<PlcVal>) -> DockManagerResult<Vec<DockDoorEvent>> {
        let mut doors = self.doors.write().await;
        let mut events = Vec::new();
        for sensor_value in sensor_values {
            debug!("Processing sensor value: {:?}", sensor_value);
            if let Some(door) = doors.get_mut(&sensor_value.door_name) {
                    if let Ok(sensor) = door.update_sensor(&sensor_value.sensor_name, Some(sensor_value.value)) {
                    if sensor.old_value.is_none() {
                        continue;
                    }
                    if sensor.changed {
                        info!("DockDoor: {} - Sensor state changed for sensor {} from {:?} to {:?}", door.dock_name, sensor_value.sensor_name, sensor.old_value, sensor.new_value);
                        let event = DockDoorEvent::SensorStateChanged(SensorStateChangedEvent {
                            dock_name: door.dock_name.clone(),
                            sensor_name: sensor_value.sensor_name.clone(),
                            old_value: door.sensors.get(&sensor_value.sensor_name).and_then(|s| s.get_sensor_data().previous_value),
                            new_value: Some(sensor_value.value),
                            timestamp: chrono::Local::now().naive_local(),
                        });
                        events.push(event);
                    } else {
                        debug!("No change in sensor state for door {}, sensor {}", door.dock_name, sensor_value.sensor_name);
                    }
                } else {
                    error!("Failed to update sensor for door {}, sensor {}", door.dock_name, sensor_value.sensor_name);
                }
            } else {
                warn!("Door not found: {}", sensor_value.door_name);
            }
        }
        Ok(events)
    }

    /// Internally updates door states based on WMS data and generates events
    ///
    /// This method is called in response to the `UpdateFromWms` command
    /// It iterates through the provided WMS data, updates the corresponding doors in the `doors` map
    /// and collects any events generated by the `update_from_wms` method of each door
    ///
    /// # Arguments
    ///
    /// * `wms_data`: A vector of `WmsDoorStatus` representing the updated WMS data for the doors
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DockDoorEvent>)`: The events generated due to the WMS data update
    async fn internal_update_from_wms(&self, wms_data: Vec<WmsDoorStatus>) -> DockManagerResult<Vec<DockDoorEvent>> {
        let mut new_events = Vec::new();
        let mut doors = self.doors.write().await;

        for wms_status in wms_data {
            if let Some(door) = doors.get_mut(&wms_status.dock_name) {
                let events = door.update_from_wms(&wms_status)?;
                new_events.extend(events);
            }
        }

        Ok(new_events)
    }

    /// Internally retrieves the state of a specific door
    ///
    /// This method is called in response to the `GetDoorState` command
    /// It looks up the door in the `doors` map and returns its state or an error if the door is not found
    ///
    /// # Arguments
    ///
    /// * `door_name`: The name of the door whose state is to be retrieved
    ///
    /// # Returns:
    ///
    /// * `Ok(DoorState)`: The current state of the specified door
    /// * `Err(DockManagerError::DoorNotFound)`: If the door is not found
    async fn internal_get_door_state(&self, door_name: &str) -> DockManagerResult<DoorState> {
        let doors = self.doors.read().await;
        doors.get(door_name)
            .map(|door| door.door_state)
            .ok_or_else(|| DockManagerError::DoorNotFound(door_name.to_string()))
    }

    /// Retrieves and clears the batched database events
    ///
    /// This method acquires a write lock on the `db_events` and returns 
    /// all accumulated events while clearing the internal buffer
    ///
    /// # Returns
    /// * A vector of `DbInsert` representing the batched database events
    pub async fn get_and_clear_db_batch(&self) -> Vec<DbInsert> {
        let mut db_events = self.db_events.write().await;
        std::mem::take(&mut *db_events)
    }

    /// Handles WMS events and converts them into database insert events
    ///
    /// This method processes a vector of `WmsEvent`
    /// It extracts relevant information from each event, including the user ID if applicable
    /// and constructs `DbInsert` objects for database insertion
    ///
    /// # Arguments
    /// * `wms_events`: A vector of `WmsEvent` to be processed
    ///
    /// # Returns
    /// * `Ok(Vec<DbInsert>)`: The `DbInsert` events generated from the WMS events
    pub async fn handle_wms_events(&self, wms_events: Vec<WmsEvent>) -> DockManagerResult<Vec<DbInsert>> {
        let mut db_events = Vec::new();
        for event in wms_events {
                let user_id = if ["STARTED_SHIPMENT", "SUSPENDED_SHIPMENT", "RESUMED_SHIPMENT",
                    "UPDATED_PRIORITY", "CANCELLED_SHIPMENT", "SDM_LOAD_PLAN",
                    "LOAD_QTY_ADJUSTED", "SDM_CHECK_IN", "SDM_TRAILER_REJECTION"]
                    .contains(&event.message_type.as_str()) {
                    event.message_notes
                        .as_ref()
                        .and_then(|notes| notes.split('-').next())
                        .map(|user| user.trim().to_string())
                } else {
                    None
                };
                let db_insert = DbInsert {
                    LOG_DTTM: event.log_dttm.unwrap_or_else(|| chrono::Local::now().naive_local()),
                    PLANT: event.plant.clone(),
                    DOOR_NAME: event.dock_name,
                    SHIPMENT_ID: Some(event.shipment_id),
                    EVENT_TYPE: event.message_type,
                    SUCCESS: if event.result_code == 0 { 1 } else { 0 },
                    NOTES: event.message_notes.unwrap_or_default(),
                    ID_USER: user_id,
                    SEVERITY: event.result_code,
                    PREVIOUS_STATE: None,
                    PREVIOUS_STATE_DTTM: None
                };
            db_events.push(db_insert);
        }
        Ok(db_events)
    }

    /// Triggers the evaluation and update of door states
    ///
    /// This method sends an `EvaluateDoorStates` command to the state manager's command channel
    /// and awaits the response containing the result of the operation
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If the evaluation and update process is successful
    /// * `Err(DockManagerError)`: If there's an error sending or receiving the command or evaluating the door states
    pub async fn evaluate_and_update_door_states(&self) -> DockManagerResult<()> {
        let (tx, rx) = oneshot::channel();
        self.command_sender.send(StateManagerCommand::EvaluateDoorStates(tx)).await?;
        rx.await?
    }

    /// Evaluates and updates the states of all doors (implementation placeholder)
    ///
    /// This method is currently a placeholder and needs to be implemented with the actual logic
    /// to evaluate and update the door states based on sensor data, WMS information, and any other relevant factors
    ///
    /// # Returns
    ///
    /// * `Ok(())`: indicating successful evaluation (even if no updates were made)
    async fn evaluate_door_states(&self) -> DockManagerResult<()> {
        // Implement logic to evaluate and update door states
        Ok(())
    }

    /// Retrieves a specific door by its name
    ///
    /// # Arguments
    /// * `door_name`: The name of the door to retrieve
    ///
    /// # Returns
    /// * `Some(DockDoor)`: If the door with the given name is found
    /// * `None`: If no door with the given name exists
    pub async fn get_door(&self, door_name: &str) -> Option<DockDoor> {
        let doors = self.doors.read().await;
        doors.get(door_name).cloned()
    }

    /// Updates the state of a specific door
    ///
    /// # Arguments
    ///
    /// * `door`: The updated `DockDoor` object
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If the door was updated successfully
    /// * `Err(DockManagerError)`: If there's an error acquiring the write lock or updating the door
    pub async fn update_door(&self, door: DockDoor) -> DockManagerResult<()> {
        let mut doors = self.doors.write().await;
        doors.insert(door.dock_name.clone(), door);
        Ok(())
    }

    /// Retrieves all dock doors managed by the state manager
    ///
    /// This method acquires a read lock on the `doors` map and returns a vector of cloned `DockDoor` objects,
    /// representing the current state of all managed doors
    ///
    /// # Returns
    ///
    /// A vector of `DockDoor` objects
    pub async fn get_all_doors(&self) -> Vec<DockDoor> {
        let doors = self.doors.read().await;
        doors.values().cloned().collect()
    }

    /// Inserts a database event into the batch for later processing
    ///
    /// This method acquires a write lock on the `db_events` vector and appends the provided `DbInsert` event to it
    ///
    /// # Arguments
    ///
    /// * `event`: The `DbInsert` event to be inserted into the batch
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If the event was inserted successfully
    /// * `Err(DockManagerError)`: If there's an error acquiring the write lock
    pub async fn insert_db_event(&self, event: DbInsert) -> DockManagerResult<()> {
        let mut db_events = self.db_events.write().await;
        db_events.push(event);
        Ok(())
    }

    /// Sends a `DockDoorEvent` to the event handler for processing
    ///
    /// This method attempts to send the provided event to the `event_sender` channel
    /// If the send operation fails, it returns an `EventProcessingError`
    ///
    /// # Arguments
    ///
    /// * `event`: The `DockDoorEvent` to be sent
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If the event was sent successfully
    /// * `Err(DockManagerError::EventProcessingError)`: If there's an error sending the event
    pub async fn send_event(&self, event: DockDoorEvent) -> DockManagerResult<()> {
        self.event_sender.send(event).await.map_err(|e| DockManagerError::EventProcessingError(e.to_string()))
    }
}