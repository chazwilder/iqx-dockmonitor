use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::config::Settings;
use crate::services::plc::PlcService;
use crate::state_management::DockDoorStateManager;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::event_handling::EventHandler;
use crate::services::db::DatabaseService;
use crate::models::{DbInsert, WmsEvent};

/// The central controller for managing dock doors and their interactions with PLCs, the WMS, and the database
pub struct DockDoorController {
    /// The application settings
    pub settings: Arc<Settings>,
    /// The service for interacting with PLCs
    pub plc_service: Arc<PlcService>,
    /// The state manager for tracking dock door states
    pub state_manager: Arc<DockDoorStateManager>,
    /// The event handler for processing dock door events
    pub event_handler: Arc<EventHandler>,
    /// The service for interacting with the database
    pub db_service: Arc<Mutex<DatabaseService>>,
}

impl DockDoorController {
    /// Creates a new `DockDoorController`
    ///
    /// Initializes the controller with the provided settings, services, and state manager
    /// Logs an informational message upon creation
    ///
    /// # Arguments
    ///
    /// * `settings`: The application settings
    /// * `plc_service`: The `PlcService` for PLC communication
    /// * `state_manager`: The `DockDoorStateManager` for managing door states
    /// * `event_handler`: The `EventHandler` for processing events
    /// * `db_service`: The `DatabaseService` for database interactions
    pub fn new(
        settings: Settings,
        plc_service: PlcService,
        state_manager: Arc<DockDoorStateManager>,
        event_handler: Arc<EventHandler>,
        db_service: DatabaseService
    ) -> Self {
        info!("Initializing Dock Door Controller");
        Self {
            settings: Arc::new(settings),
            plc_service: Arc::new(plc_service),
            state_manager,
            event_handler,
            db_service: Arc::new(Mutex::new(db_service)),
        }
    }

    /// Executes a single polling cycle, updating sensor data and processing events
    ///
    /// 1. Polls sensors using the `plc_service`
    /// 2. Updates the state manager with the new sensor values, which may generate events
    /// 3. Sends the generated events to the `event_handler` for processing, which may result in database insert events
    /// 4. Inserts the database events into the database using the `db_service`
    /// 5. Logs informational messages about the process
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the polling cycle completes successfully
    /// * `Err(DockManagerError)` if any errors occur during polling, state updates, event handling, or database insertion
    pub async fn run_polling_cycle(&self) -> DockManagerResult<()> {
        let start = std::time::Instant::now();
        info!("Starting PLC value polling...");
        let plc_values = self.plc_service.poll_sensors(&self.settings).await?;
        info!("PLC value polling completed in {:?}", start.elapsed());

        let update_start = std::time::Instant::now();
        info!("Starting sensor update...");
        let events = self.state_manager.update_sensors(plc_values).await?;
        info!("Sensor update completed in {:?}", update_start.elapsed());

        let event_start = std::time::Instant::now();
        info!("Processing {} events...", events.len());
        let mut db_events = Vec::new();
        for event in events {
            let new_db_events = self.event_handler.send_event(event).await?;
            db_events.extend(new_db_events);
        }
        info!("Event processing completed in {:?}", event_start.elapsed());

        if !db_events.is_empty() {
            let db_start = std::time::Instant::now();
            info!("Inserting {} DB events", db_events.len());
            self.db_service.lock().await.insert_dock_door_events(db_events).await?;
            info!("DB insertion completed in {:?}", db_start.elapsed());
        } else {
            info!("No DB events to insert");
        }

        info!("Full polling cycle completed in {:?}", start.elapsed());
        Ok(())
    }

    /// Updates WMS events for doors with assigned shipments
    ///
    /// 1. Acquires a read lock on the `state_manager`'s doors
    /// 2. Filters doors that have an assigned shipment
    /// 3. For each such door, fetches WMS events concurrently using the `db_service`
    /// 4. Collects all fetched WMS events and processes them using the `state_manager`, which may generate database insert events
    /// 5. Inserts the database events into the database
    /// 6. Logs informational messages about the process
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the WMS event update completes successfully
    /// * `Err(DockManagerError)` if any errors occur during fetching WMS events, processing them, or inserting into the database
    pub async fn update_wms_events(&self) -> DockManagerResult<()> {
        let db_service = Arc::clone(&self.db_service);
        let doors = self.state_manager.doors.read().await;

        let futures: Vec<_> = doors.values()
            .filter(|dock_door| dock_door.assigned_shipment.current_shipment.is_some())
            .map(|door| {
                let shipment_id = door.assigned_shipment.current_shipment.clone().unwrap();
                let door_name = door.dock_name.clone();
                let plant_id = door.plant_id.clone();
                let db_service = Arc::clone(&db_service);
                tokio::spawn(async move {
                    let db = db_service.lock().await;
                    db.fetch_wms_events(&plant_id, &shipment_id, &door_name).await
                })
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        let mut all_wms_events = Vec::new();

        for result in results {
            match result {
                Ok(Ok(events)) => all_wms_events.extend(events),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(DockManagerError::TaskJoinError(e.to_string())),
            }
        }

        let new_db_events = self.state_manager.process_wms_events(all_wms_events).await?;
        self.db_service.lock().await.insert_dock_door_events(new_db_events).await?;

        Ok(())
    }

    /// Updates the WMS door status for all plants
    ///
    /// 1. Iterates through all configured plants
    /// 2. Fetches WMS data (door statuses) for the current plant using the `db_service`
    /// 3. Updates the `state_manager` with the fetched WMS data, which may generate events
    /// 4. Sends the generated events to the `event_handler` for processing, which may result in database insert events
    /// 5. Inserts the database events into the database
    /// 6. Logs informational messages about the process
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the WMS door status update completes successfully
    /// * `Err(DockManagerError)` if any errors occur during fetching WMS data, updating the state manager, event handling, or database insertion
    pub async fn update_wms_door_status(&self) -> DockManagerResult<()> {
        let mut all_events = Vec::new();

        for plant in &self.settings.plants {
            let plant_id = &plant.plant_id;
            let wms_data = self.db_service.lock().await.fetch_wms_data(plant_id).await?;
            let events = self.state_manager.update_from_wms(wms_data).await?;
            all_events.extend(events);
        }

        let mut db_events = Vec::new();
        for event in all_events {
            let new_db_events = self.event_handler.send_event(event).await?;
            db_events.extend(new_db_events);
        }

        if !db_events.is_empty() {
            info!("Inserting {} WMS door status DB events", db_events.len());
            self.db_service.lock().await.insert_dock_door_events(db_events).await?;
        }

        Ok(())
    }

    /// Handles a WMS event and creates a `DbInsert` for it
    ///
    /// This method extracts the user ID (if applicable) from the WMS event's message notes
    /// and constructs a `DbInsert` object representing the event for database insertion
    ///
    /// # Arguments
    ///
    /// * `event`: The `WmsEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(DbInsert)`: The `DbInsert` object representing the WMS event
    pub async fn handle_wms_event(&self, event: WmsEvent) -> DockManagerResult<DbInsert> {
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
        Ok(DbInsert {
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
        })
    }
}