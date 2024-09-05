//! # Event Handling

//! This module handles the processing of events in the dock door management system. It receives events from a queue, analyzes them using a `ContextAnalyzer`,
//! updates the state of dock doors through a `DockDoorStateManager`, and persists relevant events to the database.

use crate::models::{DockDoorEvent, DbInsert, DockDoor};
use crate::state_management::DockDoorStateManager;
use crate::analysis::{AlertType, AnalysisResult, ContextAnalyzer};
use crate::errors::{DockManagerResult, DockManagerError};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use chrono::Local;
use tracing::{info, error, debug};
use crate::alerting::alert_manager::AlertManager;
use crate::monitoring::{MonitoringItem, MonitoringQueue};

/// The `EventHandler` is responsible for processing events in the dock door management system.
pub struct EventHandler {
    /// A queue for receiving `DockDoorEvent`s.
    event_queue: Mutex<mpsc::Receiver<DockDoorEvent>>,
    /// The state manager responsible for maintaining the state of dock doors.
    state_manager: Arc<DockDoorStateManager>,
    /// The context analyzer used to analyze events and generate insights.
    context_analyzer: Arc<ContextAnalyzer>,
    alert_manager: Arc<AlertManager>,
    monitoring_queue: Arc<MonitoringQueue>,
}

impl EventHandler {
    /// Creates a new `EventHandler`.
    ///
    /// # Arguments
    /// * `event_queue`: The receiver end of a channel to receive `DockDoorEvent`s.
    /// * `state_manager`: The `DockDoorStateManager` to interact with for state updates.
    /// * `context_analyzer`: The `ContextAnalyzer` to use for event analysis.
    pub fn new(
        event_queue: mpsc::Receiver<DockDoorEvent>,
        state_manager: Arc<DockDoorStateManager>,
        context_analyzer: Arc<ContextAnalyzer>,
        alert_manager: Arc<AlertManager>,
        monitoring_queue: Arc<MonitoringQueue>,
    ) -> Self {
        Self {
            event_queue: Mutex::new(event_queue),
            state_manager,
            context_analyzer,
            alert_manager,
            monitoring_queue
        }
    }

    /// Runs the event handler, continuously processing events from the queue
    /// until the queue is closed or an error occurs
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

    /// Processes an event and potentially generates database insert events
    ///
    /// This method is primarily used for testing purposes
    ///
    /// # Arguments
    ///
    /// * `event`: The `DockDoorEvent` to process.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` containing a vector of `DbInsert` events if successful,
    /// or an error if the event processing fails.
    pub async fn send_event(&self, event: DockDoorEvent) -> DockManagerResult<Vec<DbInsert>> {
        self.process_event(event).await
    }

    /// Processes a single dock door event
    ///
    /// This method retrieves the associated dock door, analyzes the event using the `ContextAnalyzer`,
    /// handles any resulting state transitions, alerts, or logs, updates the door's state,
    /// and inserts any generated `DbInsert` events into the database
    ///
    /// # Arguments
    ///
    /// * `event`: The `DockDoorEvent` to be processed
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DbInsert>)`: A vector of `DbInsert` events generated during processing
    /// * `Err(DockManagerError)`: If there's an error processing the event or interacting with the database or state manager
    pub async fn process_event(&self, event: DockDoorEvent) -> DockManagerResult<Vec<DbInsert>> {
        debug!("Processing event: {:?}", event);

        let door_name = event.get_dock_name();
        let mut door = self.state_manager.get_door(door_name).await
            .ok_or_else(|| DockManagerError::DoorNotFound(door_name.to_string()))?;

        let analysis_results = self.context_analyzer.analyze(&door, &event).await;

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
                AnalysisResult::Alert(alert) => {
                    info!("EVENT HANDLER: Processing event alert for alert: {:?}", alert);
                    match self.alert_manager.handle_alert(alert.clone()).await {
                        Ok(_) => info!("Alert handled successfully: {:?}", alert),
                        Err(e) => error!("Failed to handle alert: {:?}. Error: {:?}", alert, e),
                    }
                    self.add_to_monitoring_queue(alert, &door).await;
                },
                AnalysisResult::DbInsert(db_insert) => {
                    db_events.push(db_insert);
                }
            }
        }

        door.handle_event(&event)?;
        self.state_manager.update_door(door).await?;

        if !db_events.is_empty() {
            self.insert_db_events(&db_events).await?;
        }

        Ok(db_events)
    }

    /// Inserts a batch of database events
    ///
    /// This method iterates through the provided `DbInsert` events and attempts to insert each one into the database
    /// using the state manager's `insert_db_event` method
    ///
    /// # Arguments
    ///
    /// * `db_events`: A slice of `DbInsert` events to be inserted
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all events were inserted successfully
    /// * The first encountered `DockManagerError` if any insertion fails
    pub async fn insert_db_events(&self, db_events: &[DbInsert]) -> DockManagerResult<()> {
        for event in db_events {
            self.state_manager.insert_db_event(event.clone()).await?;
        }
        Ok(())
    }

    async fn add_to_monitoring_queue(&self, alert: AlertType, _door: &DockDoor) {
        match alert {
            AlertType::SuspendedDoor { door_name, duration: _, shipment_id } => {
                self.monitoring_queue.add(MonitoringItem::SuspendedShipment {
                    door_name,
                    shipment_id: shipment_id.unwrap_or_default(),
                    suspended_at: Local::now().naive_local(),
                }).await;
            },
            AlertType::TrailerDocked { door_name, shipment_id: _, timestamp, success: true, failure_reason: _ } => {
                self.monitoring_queue.add(MonitoringItem::TrailerDockedNotStarted {
                    door_name,
                    docked_at: timestamp,
                }).await;
            },
            AlertType::ShipmentStartedLoadNotReady { door_name, shipment_id, reason: _ } => {
                self.monitoring_queue.add(MonitoringItem::ShipmentStartedLoadNotReady {
                    door_name,
                    shipment_id,
                    started_at: Local::now().naive_local(),
                }).await;
            },
            AlertType::DockReady { door_name, timestamp, .. } => {
                self.monitoring_queue.add(MonitoringItem::TrailerDockedNotStarted {
                    door_name,
                    docked_at: timestamp,
                }).await;
            },
            _ => {}
        }
    }
}