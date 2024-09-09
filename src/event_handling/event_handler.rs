use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use chrono::Local;
use log::{info, error, debug};
use crate::models::{DockDoorEvent, DbInsert, DockDoor};
use crate::analysis::{AnalysisResult, context_analyzer, ContextAnalyzer};
use crate::errors::{DockManagerResult, DockManagerError};
use crate::alerting::alert_manager::{AlertManager, Alert, AlertType};
use crate::monitoring::{MonitoringItem, MonitoringQueue};
use crate::state_management::door_state_repository::DoorStateRepository;
use crate::services::db::DatabaseService;
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

/// The EventHandler is responsible for processing events in the dock door management system.
#[derive(Clone)]
pub struct EventHandler {
    /// A queue for receiving `DockDoorEvent`s.
    event_queue: Arc<Mutex<mpsc::Receiver<DockDoorEvent>>>,
    /// The state repository responsible for maintaining the state of dock doors.
    door_repository: Arc<DoorStateRepository>,
    /// The context analyzer used to analyze events and generate insights.
    context_analyzer: Arc<ContextAnalyzer>,
    /// The alert manager responsible for handling and sending alerts.
    alert_manager: Arc<AlertManager>,
    /// The queue for monitoring items.
    monitoring_queue: Arc<MonitoringQueue>,
    /// A channel sender for consolidated events.
    consolidated_event_sender: mpsc::Sender<ConsolidatedDockEvent>,
}

impl EventHandler {
    /// Creates a new `EventHandler`.
    ///
    /// # Arguments
    ///
    /// * `event_queue` - The receiver end of a channel to receive `DockDoorEvent`s.
    /// * `door_repository` - The `DoorStateRepository` to interact with for state updates.
    /// * `context_analyzer` - The `ContextAnalyzer` to use for event analysis.
    /// * `alert_manager` - The `AlertManager` to handle alerts.
    /// * `monitoring_queue` - The `MonitoringQueue` to add monitoring items.
    /// * `db_service` - The `DatabaseService` for database operations.
    ///
    /// # Returns
    ///
    /// A new `EventHandler` instance.
    pub fn new(
        event_queue: mpsc::Receiver<DockDoorEvent>,
        door_repository: Arc<DoorStateRepository>,
        context_analyzer: Arc<ContextAnalyzer>,
        alert_manager: Arc<AlertManager>,
        monitoring_queue: Arc<MonitoringQueue>,
        db_service: Arc<DatabaseService>,
    ) -> Self {
        let (consolidated_event_sender, consolidated_event_receiver) = mpsc::channel(1000);

        // Spawn a task to handle consolidated events
        tokio::spawn(Self::process_consolidated_events(consolidated_event_receiver, Arc::clone(&db_service)));

        Self {
            event_queue: Arc::new(Mutex::new(event_queue)),
            door_repository,
            context_analyzer,
            alert_manager,
            monitoring_queue,
            consolidated_event_sender,
        }
    }

    /// Processes consolidated events asynchronously.
    ///
    /// This function runs in a separate task and continuously receives consolidated events
    /// from a channel, inserting them into the database.
    ///
    /// # Arguments
    ///
    /// * `receiver` - The receiver end of the channel for consolidated events.
    /// * `db_service` - The database service for inserting events.
    async fn process_consolidated_events(
        mut receiver: mpsc::Receiver<ConsolidatedDockEvent>,
        db_service: Arc<DatabaseService>,
    ) {
        while let Some(event) = receiver.recv().await {
            if let Err(e) = db_service.insert_consolidated_event(&event).await {
                error!("Failed to insert consolidated event: {:?}", e);
            }
        }
    }

    /// Runs the event handler, continuously processing events from the queue
    /// until the queue is closed or an error occurs.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure of the run.
    pub async fn run(&self) -> DockManagerResult<()> {
        info!("EventHandler started");
        while let Some(event) = self.event_queue.lock().await.recv().await {
            if let Err(e) = self.process_event(event).await {
                error!("Error processing event: {:?}", e);
            }
        }
        info!("EventHandler stopped");
        Ok(())
    }

    /// Processes a single dock door event.
    ///
    /// This method retrieves the associated dock door, analyzes the event using the `ContextAnalyzer`,
    /// handles any resulting state transitions, alerts, or logs, updates the door's state,
    /// and inserts any generated `DbInsert` events into the database.
    ///
    /// # Arguments
    ///
    /// * `event` - The `DockDoorEvent` to be processed.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DbInsert>)` - A vector of `DbInsert` events generated during processing.
    /// * `Err(DockManagerError)` - If there's an error processing the event or interacting with the database or state manager.
    pub async fn process_event(&self, event: DockDoorEvent) -> DockManagerResult<Vec<DbInsert>> {
        debug!("Processing event: {:?}", event);

        let door_name = event.get_dock_name();
        let plant_id = event.get_plant_id();

        let mut door = self.door_repository.get_door_state(plant_id, door_name).await
            .ok_or_else(|| DockManagerError::DoorNotFound(door_name.to_string()))?;

        let analysis_results = self.context_analyzer.analyze(&door, &event).await;
        info!("Analysis results: {:?}", analysis_results);

        let mut db_events = Vec::new();
        for result in analysis_results {
            match result {
                AnalysisResult::StateTransition(new_state) => {
                    door.door_state = new_state;
                },
                AnalysisResult::Log(log_entry) => {
                    info!("EVENT HANDLER: Processing event log entry for log: {:?}", log_entry);
                    let db_insert = DbInsert::from_log_entry(&log_entry);
                    db_events.push(db_insert);
                },
                AnalysisResult::Alert(alert_type) => {
                    info!("EVENT HANDLER: Processing event alert for alert: {:?}", alert_type);
                    let alert = self.create_alert(alert_type.clone(), &door);
                    match self.alert_manager.handle_alert(alert.clone()).await {
                        Ok(_) => info!("Alert handled successfully: {:?}", alert),
                        Err(e) => error!("Failed to handle alert: {:?}. Error: {:?}", alert, e),
                    }
                    self.add_to_monitoring_queue(alert_type, &door).await;
                },
                AnalysisResult::DbInsert(db_insert) => {
                    db_events.push(db_insert);
                },
                AnalysisResult::ConsolidatedEvent(consolidated_event) => {
                    if let DockDoorEvent::LgvStartLoading(_) = event {
                        if let Err(e) = self.consolidated_event_sender.send(consolidated_event).await {
                            error!("Failed to send consolidated event: {:?}", e);
                        }
                    }
                }
            }
        }

        door.handle_event(&event)?;

        if !db_events.is_empty() {
            self.insert_db_events(&db_events).await?;
        }

        Ok(db_events)
    }

    /// Inserts a batch of database events.
    ///
    /// # Arguments
    ///
    /// * `db_events` - A slice of `DbInsert` events to be inserted.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure of the insertion.
    async fn insert_db_events(&self, db_events: &[DbInsert]) -> DockManagerResult<()> {
        for event in db_events {
            let plant_id = event.get_plant_id();
            self.door_repository.insert_db_event(plant_id, event.clone()).await?;
        }
        Ok(())
    }

    /// Creates an `Alert` from an `AlertType` and `DockDoor`.
    ///
    /// This method constructs the appropriate `Alert` based on the given `AlertType`,
    /// populating it with relevant information from the `DockDoor`.
    ///
    /// # Arguments
    ///
    /// * `alert_type` - The `AlertType` to convert into an `Alert`.
    /// * `door` - The `DockDoor` associated with the alert.
    ///
    /// # Returns
    ///
    /// An `Alert` instance constructed from the given `AlertType` and `DockDoor`.
    fn create_alert(&self, alert_type: context_analyzer::AlertType, door: &DockDoor) -> Alert {
        match alert_type {
            context_analyzer::AlertType::SuspendedDoor { door_name, duration, shipment_id, user } => {
                Alert::new(AlertType::SuspendedDoor, door_name)
                    .duration(duration)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .add_info("user".to_string(), user)
                    .build()
            },
            context_analyzer::AlertType::TrailerDocked { door_name, shipment_id, timestamp, success, failure_reason } => {
                Alert::new(AlertType::TrailerDocked, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .add_info("timestamp".to_string(), timestamp.to_string())
                    .add_info("success".to_string(), success.to_string())
                    .add_info("failure_reason".to_string(), failure_reason.unwrap_or_default())
                    .build()
            },
            context_analyzer::AlertType::ShipmentStartedLoadNotReady { door_name, shipment_id, reason } => {
                Alert::new(AlertType::ShipmentStartedLoadNotReady, door_name)
                    .shipment_id(shipment_id)
                    .add_info("reason".to_string(), reason)
                    .build()
            },
            context_analyzer::AlertType::DockReady { door_name, shipment_id, timestamp } => {
                Alert::new(AlertType::DockReady, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .add_info("timestamp".to_string(), timestamp.to_string())
                    .build()
            },
            context_analyzer::AlertType::ManualModeAlert { door_name, shipment_id } => {
                Alert::new(AlertType::ManualModeAlert, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .build()
            },
            context_analyzer::AlertType::TrailerPatternIssue { door_name, shipment_id, .. } => {
                Alert::new(AlertType::TrailerPatternIssue, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .build()
            },
            context_analyzer::AlertType::TrailerDockedNotStarted { door_name, duration } => {
                Alert::new(AlertType::TrailerDockedNotStarted, door_name)
                    .add_info("has had a trailer docked without inspection or starting in wms for".to_string(), duration.to_string())
                    .build()
            },
            context_analyzer::AlertType::TrailerHostage { door_name, shipment_id, duration } => {
                Alert::new(AlertType::TrailerHostage, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .add_info("Trailer has been held hostage for ".to_string(), duration.to_string())
                    .build()
            },
            _ => Alert::new(AlertType::ManualModeAlert, door.dock_name.clone()).build(),
        }
    }

    /// Adds a monitoring item to the monitoring queue based on the alert type.
    ///
    /// This method creates and adds appropriate `MonitoringItem`s to the monitoring queue
    /// based on the given `AlertType`.
    ///
    /// # Arguments
    ///
    /// * `alert_type` - The `AlertType` to convert into a monitoring item.
    /// * `door` - The `DockDoor` associated with the alert.
    async fn add_to_monitoring_queue(&self, alert_type: context_analyzer::AlertType, door: &DockDoor) {
        match alert_type {
            context_analyzer::AlertType::SuspendedDoor { door_name, shipment_id, user, .. } => {
                self.monitoring_queue.add(MonitoringItem::SuspendedShipment {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    shipment_id: shipment_id.unwrap_or_default(),
                    suspended_at: Local::now().naive_local(),
                    user,
                });
            },
            context_analyzer::AlertType::TrailerDocked { door_name, timestamp, success: true, .. } => {
                self.monitoring_queue.add(MonitoringItem::TrailerDockedNotStarted {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    docked_at: timestamp,
                });
            },
            context_analyzer::AlertType::ShipmentStartedLoadNotReady { door_name, shipment_id, .. } => {
                self.monitoring_queue.add(MonitoringItem::ShipmentStartedLoadNotReady {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    shipment_id,
                    started_at: Local::now().naive_local(),
                });
            },
            context_analyzer::AlertType::DockReady { door_name, timestamp, .. } => {
                self.monitoring_queue.add(MonitoringItem::TrailerDockedNotStarted {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    docked_at: timestamp,
                });
            },
            context_analyzer::AlertType::TrailerHostage { door_name, shipment_id, .. } => {
                self.monitoring_queue.add(MonitoringItem::TrailerHostage {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    shipment_id,
                    detected_at: Local::now().naive_local(),
                });
            },
            _ => {}
        }
    }
}