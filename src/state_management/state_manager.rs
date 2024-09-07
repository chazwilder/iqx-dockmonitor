use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::Receiver;
use log::{info, error};
use crate::config::Settings;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::models::{DockDoorEvent, PlcVal, WmsDoorStatus, DbInsert, WmsEvent, DockDoor};
use crate::state_management::door_state_repository::DoorStateRepository;
use crate::state_management::command_processor::CommandProcessor;
use crate::state_management::sensor_data_processor::SensorDataProcessor;
use crate::state_management::wms_data_processor::WmsDataProcessor;
use crate::state_management::database_event_manager::DatabaseEventManager;
use crate::state_management::event_dispatcher::EventDispatcher;
use crate::state_management::state_manager_lifecycle::StateManagerLifecycle;
use crate::services::db::DatabaseService;

/// Manages the overall state of the dock door monitoring system.
#[derive(Clone)]
pub struct DockDoorStateManager {
    door_repository: Arc<DoorStateRepository>,
    command_processor: Arc<Mutex<CommandProcessor>>,
    sensor_processor: Arc<SensorDataProcessor>,
    wms_processor: Arc<WmsDataProcessor>,
    db_event_manager: Arc<DatabaseEventManager>,
    event_dispatcher: Arc<EventDispatcher>,
    lifecycle: Arc<StateManagerLifecycle>,
}

impl DockDoorStateManager {
    /// Creates a new `DockDoorStateManager`.
    ///
    /// # Arguments
    ///
    /// * `settings` - The application settings.
    /// * `db_service` - A reference to the database service.
    ///
    /// # Returns
    ///
    /// A new instance of `DockDoorStateManager`.
    pub fn new(settings: &Settings, db_service: Arc<DatabaseService>) -> (Self, Receiver<DockDoorEvent>) {
        let door_repository = Arc::new(DoorStateRepository::new());
        door_repository.initialize_from_settings(settings)
            .expect("Failed to initialize doors from settings");
        let (_command_sender, command_receiver) = mpsc::channel(100);
        let (event_sender, event_receiver) = mpsc::channel(1000);

        let db_event_manager = Arc::new(DatabaseEventManager::new(settings.batch_size,
            Arc::clone(&db_service)
        ));

        let event_dispatcher = Arc::new(EventDispatcher::new(event_sender));

        let sensor_processor = Arc::new(SensorDataProcessor::new(Arc::clone(&door_repository)));
        let wms_processor = Arc::new(WmsDataProcessor::new(Arc::clone(&door_repository)));

        let command_processor = Arc::new(Mutex::new(CommandProcessor::new(
            command_receiver,
            Arc::clone(&door_repository),
            Arc::clone(&sensor_processor),
            Arc::clone(&wms_processor),
            Arc::clone(&db_event_manager),
            Arc::clone(&event_dispatcher),
        )));

        let lifecycle = Arc::new(StateManagerLifecycle::new(Arc::clone(&db_event_manager)));

        (Self {
            door_repository,
            command_processor,
            sensor_processor,
            wms_processor,
            db_event_manager,
            event_dispatcher,
            lifecycle,
        }, event_receiver)
    }

    /// Runs the main loop of the state manager.
    ///
    /// This method processes commands and handles shutdown when signaled.
    pub async fn run(&self) {
        info!("Starting DockDoorStateManager");

        let command_processor = Arc::clone(&self.command_processor);
        let command_processor_handle = tokio::spawn(async move {
            if let Err(e) = command_processor.lock().await.run().await {
                error!("Error in command processor: {:?}", e);
            }
        });

        loop {
            tokio::select! {
                _ = self.lifecycle.wait_for_shutdown() => {
                    info!("Shutdown signal received, stopping DockDoorStateManager");
                    break;
                }
                result = command_processor_handle => {
                    match result {
                        Ok(_) => info!("Command processor finished successfully"),
                        Err(e) => error!("Command processor task panicked: {:?}", e),
                    }
                    break;
                }
            }
        }

        self.shutdown().await;
    }

    /// Handles the shutdown process for the state manager.
    async fn shutdown(&self) {
        info!("Initiating shutdown process");

        // Perform cleanup
        if let Err(e) = self.lifecycle.cleanup().await {
            error!("Error during cleanup: {:?}", e);
        }

        info!("DockDoorStateManager shutdown complete");
    }

    /// Updates sensor values and generates corresponding events.
    ///
    /// # Arguments
    ///
    /// * `sensor_values` - A vector of `PlcVal` representing the sensor updates.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` containing a vector of generated `DockDoorEvent`s.
    pub async fn update_sensors(&self, sensor_values: Vec<PlcVal>) -> DockManagerResult<Vec<DockDoorEvent>> {
        self.sensor_processor.process_sensor_updates(sensor_values).await
    }

    /// Updates the state based on WMS data and generates corresponding events.
    ///
    /// # Arguments
    ///
    /// * `wms_data` - A vector of `WmsDoorStatus` representing the WMS updates.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` containing a vector of generated `DockDoorEvent`s.
    pub async fn update_from_wms(&self, wms_data: Vec<WmsDoorStatus>) -> DockManagerResult<Vec<DockDoorEvent>> {
        self.wms_processor.process_wms_updates(wms_data).await
    }

    /// Processes WMS events and generates database insert events.
    ///
    /// # Arguments
    ///
    /// * `wms_events` - A vector of `WmsEvent`s to process.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` containing a vector of generated `DbInsert` events.
    pub async fn process_wms_events(&self, wms_events: Vec<WmsEvent>) -> DockManagerResult<Vec<DbInsert>> {
        Ok(wms_events
            .into_iter()
            .filter_map(|event| DbInsert::try_from(event).ok())
            .collect())
    }

    /// Dispatches a single event.
    ///
    /// # Arguments
    ///
    /// * `event` - The `DockDoorEvent` to dispatch.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure.
    pub async fn dispatch_event(&self, event: DockDoorEvent) -> DockManagerResult<()> {
        self.event_dispatcher.dispatch_event(event).await
    }

    /// Dispatches multiple events.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of `DockDoorEvent`s to dispatch.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure.
    pub async fn dispatch_events(&self, events: Vec<DockDoorEvent>) -> DockManagerResult<()> {
        self.event_dispatcher.dispatch_events(events).await
    }

    /// Triggers the shutdown process for the state manager.
    pub fn trigger_shutdown(&self) {
        self.lifecycle.trigger_shutdown();
    }

    /// Adds a database event to be processed.
    ///
    /// # Arguments
    ///
    /// * `event` - The `DbInsert` event to add.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure.
    pub async fn add_db_event(&self, event: DbInsert) -> DockManagerResult<()> {
        self.db_event_manager.add_event(event).await
    }

    /// Flushes all pending database events.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure.
    pub async fn flush_db_events(&self) -> DockManagerResult<()> {
        self.db_event_manager.flush_events().await
    }

    pub async fn get_door(&self, plant_id: &str, door_name: &str) -> Option<DockDoor> {
        self.door_repository.get_door_state(plant_id, door_name)
    }

    pub async fn update_door(&self, plant_id: &str, door: DockDoor) -> Result<(), DockManagerError> {
        self.door_repository.update_door(plant_id, door)
    }

    pub async fn insert_db_event(&self, event: DbInsert) -> Result<(), DockManagerError> {
        self.db_event_manager.add_event(event).await
    }

    pub fn get_door_repository(&self) -> Arc<DoorStateRepository> {
        Arc::clone(&self.door_repository)
    }

}