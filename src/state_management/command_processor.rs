use tokio::sync::mpsc;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::models::{PlcVal, WmsDoorStatus, DockDoorEvent, DbInsert, WmsEvent, DoorState};
use crate::state_management::door_state_repository::DoorStateRepository;
use crate::state_management::sensor_data_processor::SensorDataProcessor;
use crate::state_management::wms_data_processor::WmsDataProcessor;
use crate::state_management::database_event_manager::DatabaseEventManager;
use crate::state_management::event_dispatcher::EventDispatcher;
use tokio::sync::oneshot;
use std::sync::Arc;
use tracing::error;

/// Represents the different commands that can be processed by the CommandProcessor
#[derive(Debug)]
pub enum StateManagerCommand {
    UpdateSensors(Vec<PlcVal>, oneshot::Sender<Result<Vec<DockDoorEvent>, DockManagerError>>),
    UpdateFromWms(Vec<WmsDoorStatus>, oneshot::Sender<Result<Vec<DockDoorEvent>, DockManagerError>>),
    GetDoorState(String, String, oneshot::Sender<Result<DoorState, DockManagerError>>),
    HandleWmsEvents(Vec<WmsEvent>, oneshot::Sender<Result<Vec<DbInsert>, DockManagerError>>),
    GetAndClearDbBatch(oneshot::Sender<Vec<DbInsert>>),
    EvaluateDoorStates(oneshot::Sender<Result<(), DockManagerError>>),
}

/// Processes commands for the dock monitoring system
pub struct CommandProcessor {
    command_receiver: mpsc::Receiver<StateManagerCommand>,
    door_repository: Arc<DoorStateRepository>,
    sensor_processor: Arc<SensorDataProcessor>,
    wms_processor: Arc<WmsDataProcessor>,
    db_event_manager: Arc<DatabaseEventManager>,
    event_dispatcher: Arc<EventDispatcher>,
}

impl CommandProcessor {
    /// Creates a new CommandProcessor
    ///
    /// # Arguments
    ///
    /// * `command_receiver` - The receiver end of the command channel
    /// * `door_repository` - The repository for managing door states
    /// * `sensor_processor` - The processor for sensor data
    /// * `wms_processor` - The processor for WMS data
    /// * `db_event_manager` - The manager for database events
    /// * `event_dispatcher` - The dispatcher for dock door events
    ///
    /// # Returns
    ///
    /// A new instance of CommandProcessor
    pub fn new(
        command_receiver: mpsc::Receiver<StateManagerCommand>,
        door_repository: Arc<DoorStateRepository>,
        sensor_processor: Arc<SensorDataProcessor>,
        wms_processor: Arc<WmsDataProcessor>,
        db_event_manager: Arc<DatabaseEventManager>,
        event_dispatcher: Arc<EventDispatcher>,
    ) -> Self {
        Self {
            command_receiver,
            door_repository,
            sensor_processor,
            wms_processor,
            db_event_manager,
            event_dispatcher,
        }
    }

    /// Runs the command processing loop
    ///
    /// This method continuously receives commands and processes them until the channel is closed
    pub async fn run(&mut self) -> DockManagerResult<()> {
        while let Some(command) = self.command_receiver.recv().await {
            if let Err(e) = self.process_command(command).await {
                error!("Error processing command: {:?}", e);
                return Err(e);
            }
        }
        Ok(())
    }

    /// Processes a single command
    ///
    /// # Arguments
    ///
    /// * `command` - The command to process
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the command processing
    async fn process_command(&self, command: StateManagerCommand) -> DockManagerResult<()> {
        match command {
            StateManagerCommand::UpdateSensors(sensor_values, response_sender) => {
                let result = self.handle_update_sensors(sensor_values).await;
                response_sender.send(result).map_err(|_| DockManagerError::ChannelSendError("Failed to send UpdateSensors response".to_string()))?;
            },
            StateManagerCommand::UpdateFromWms(wms_data, response_sender) => {
                let result = self.handle_update_from_wms(wms_data).await;
                response_sender.send(result).map_err(|_| DockManagerError::ChannelSendError("Failed to send UpdateFromWms response".to_string()))?;
            },
            StateManagerCommand::GetDoorState(plant_id, door_name, response_sender) => {
                let result = self.handle_get_door_state(&plant_id, &door_name).await;
                response_sender.send(result).map_err(|_| DockManagerError::ChannelSendError("Failed to send GetDoorState response".to_string()))?;
            },
            StateManagerCommand::HandleWmsEvents(wms_events, response_sender) => {
                let result = self.handle_wms_events(wms_events).await;
                response_sender.send(result).map_err(|_| DockManagerError::ChannelSendError("Failed to send HandleWmsEvents response".to_string()))?;
            },
            StateManagerCommand::GetAndClearDbBatch(response_sender) => {
                let result = self.db_event_manager.get_and_clear_events().await;
                response_sender.send(result).map_err(|_| DockManagerError::ChannelSendError("Failed to send GetAndClearDbBatch response".to_string()))?;
            },
            StateManagerCommand::EvaluateDoorStates(response_sender) => {
                let result = self.handle_evaluate_door_states().await;
                response_sender.send(result).map_err(|_| DockManagerError::ChannelSendError("Failed to send EvaluateDoorStates response".to_string()))?;
            },
        }
        Ok(())
    }

    /// Handles the UpdateSensors command
    ///
    /// # Arguments
    ///
    /// * `sensor_values` - A vector of PlcVal representing sensor updates
    ///
    /// # Returns
    ///
    /// A Result containing a vector of DockDoorEvents or a DockManagerError
    async fn handle_update_sensors(&self, sensor_values: Vec<PlcVal>) -> Result<Vec<DockDoorEvent>, DockManagerError> {
        let events = self.sensor_processor.process_sensor_updates(sensor_values).await?;
        for event in &events {
            if let Err(e) = self.event_dispatcher.dispatch_event(event.clone()).await {
                error!("Error dispatching event: {:?}", e);
            }
        }
        Ok(events)
    }

    /// Handles the UpdateFromWms command
    ///
    /// # Arguments
    ///
    /// * `wms_data` - A vector of WmsDoorStatus representing WMS updates
    ///
    /// # Returns
    ///
    /// A Result containing a vector of DockDoorEvents or a DockManagerError
    async fn handle_update_from_wms(&self, wms_data: Vec<WmsDoorStatus>) -> Result<Vec<DockDoorEvent>, DockManagerError> {
        let events = self.wms_processor.process_wms_updates(wms_data).await?;
        for event in &events {
            if let Err(e) = self.event_dispatcher.dispatch_event(event.clone()).await {
                error!("Error dispatching event: {:?}", e);
            }
        }
        Ok(events)
    }

    /// Handles the GetDoorState command
    ///
    /// # Arguments
    ///
    /// * `door_name` - The name of the door to get the state for
    ///
    /// # Returns
    ///
    /// A Result containing the DoorState or a DockManagerError
    async fn handle_get_door_state(&self, plant_id: &str, door_name: &str) -> Result<DoorState, DockManagerError> {
        self.door_repository.get_door_state(plant_id, door_name)
            .ok_or_else(|| DockManagerError::DoorNotFound(format!("Plant: {}, Door: {}", plant_id, door_name)))
            .map(|door| door.door_state)
    }

    /// Handles the HandleWmsEvents command
    ///
    /// # Arguments
    ///
    /// * `wms_events` - A vector of WmsEvent to be processed
    ///
    /// # Returns
    ///
    /// A Result containing a vector of DbInsert or a DockManagerError
    async fn handle_wms_events(&self, _wms_events: Vec<WmsEvent>) -> Result<Vec<DbInsert>, DockManagerError> {
        // Implement WMS event handling logic
        // This is a placeholder and should be implemented based on your specific requirements
        todo!("Implement WMS event handling")
    }

    /// Handles the EvaluateDoorStates command
    ///
    /// # Returns
    ///
    /// A Result indicating success or a DockManagerError
    async fn handle_evaluate_door_states(&self) -> Result<(), DockManagerError> {
        // Implement logic to evaluate and update door states
        // This is a placeholder and should be implemented based on your specific requirements
        Ok(())
    }
}