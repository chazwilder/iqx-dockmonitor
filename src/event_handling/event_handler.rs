use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use chrono::Local;
use tracing::{info, error, debug};
use crate::models::{DockDoorEvent, DbInsert, DockDoor};
use crate::analysis::{AnalysisResult, context_analyzer, ContextAnalyzer};
use crate::errors::{DockManagerResult, DockManagerError};
use crate::alerting::alert_manager::{AlertManager, Alert, AlertType};
use crate::monitoring::{MonitoringItem, MonitoringQueue};
use crate::state_management::door_state_repository::DoorStateRepository;
use crate::services::db::DatabaseService;
use std::collections::HashMap;
use crate::alerting::alert_manager;
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

#[derive(Clone)]
pub struct EventHandler {
    event_queue: Arc<Mutex<mpsc::Receiver<DockDoorEvent>>>,
    door_repository: Arc<DoorStateRepository>,
    context_analyzer: Arc<ContextAnalyzer>,
    alert_manager: Arc<AlertManager>,
    monitoring_queue: Arc<MonitoringQueue>,
    db_service: Arc<DatabaseService>,
    consolidated_events: Arc<Mutex<HashMap<(String, String, i32), ConsolidatedDockEvent>>>,
}

impl EventHandler {
    pub fn new(
        event_queue: mpsc::Receiver<DockDoorEvent>,
        door_repository: Arc<DoorStateRepository>,
        context_analyzer: Arc<ContextAnalyzer>,
        alert_manager: Arc<AlertManager>,
        monitoring_queue: Arc<MonitoringQueue>,
        db_service: Arc<DatabaseService>,
    ) -> Self {
        Self {
            event_queue: Arc::new(Mutex::new(event_queue)),
            door_repository,
            context_analyzer,
            alert_manager,
            monitoring_queue,
            db_service,
            consolidated_events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

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

    pub async fn process_event(&self, event: DockDoorEvent) -> DockManagerResult<Vec<DbInsert>> {
        debug!("Processing event: {:?}", event);

        let door_name = event.get_dock_name();
        let plant_id = event.get_plant_id();

        let mut door = self.door_repository.get_door_state(plant_id, door_name)
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
                }
            }
        }

        door.handle_event(&event)?;
        self.door_repository.update_door(plant_id, door)?;

        if !db_events.is_empty() {
            self.insert_db_events(&db_events).await?;
        }

        self.update_consolidated_event(&event).await?;

        Ok(db_events)
    }

    async fn update_consolidated_event(&self, event: &DockDoorEvent) -> DockManagerResult<()> {
        let key = (
            event.get_plant_id().to_string(),
            event.get_dock_name().to_string(),
            event.get_shipment_id().unwrap_or_default().parse::<i32>().unwrap_or(0)
        );

        let mut consolidated_events = self.consolidated_events.lock().await;
        let mut should_remove = false;
        let mut should_insert = false;

        if let Some(consolidated_event) = consolidated_events.get_mut(&key) {
            match event {
                DockDoorEvent::ShipmentAssigned(e) => {
                    consolidated_event.shipment_assigned = Some(e.timestamp);
                },
                DockDoorEvent::DockAssigned(e) => {
                    consolidated_event.dock_assignment = Some(e.timestamp);
                },
                DockDoorEvent::TrailerDocked(e) => {
                    consolidated_event.trailer_docking = Some(e.timestamp);
                },
                DockDoorEvent::LoadingStarted(e) => {
                    consolidated_event.started_shipment = Some(e.timestamp);
                },
                DockDoorEvent::WmsEvent(e) if e.event_type == "STARTED_SHIPMENT" => {
                    consolidated_event.started_shipment = Some(e.timestamp);
                },
                DockDoorEvent::WmsEvent(e) if e.event_type == "LGV_START_LOADING" => {
                    consolidated_event.lgv_start_loading = Some(e.timestamp);
                    should_insert = true;
                    should_remove = true;
                },
                DockDoorEvent::DoorStateChanged(e) if e.new_state == crate::models::DoorState::DoorReady => {
                    consolidated_event.dock_ready = Some(e.timestamp);
                },
                _ => {}
            }

            self.calculate_durations(consolidated_event);
        } else {
            let mut new_event = ConsolidatedDockEvent {
                plant: event.get_plant_id().to_string(),
                door_name: event.get_dock_name().to_string(),
                shipment_id: key.2,
                docking_time_minutes: None,
                inspection_time_minutes: None,
                enqueued_time_minutes: None,
                shipment_assigned: None,
                dock_assignment: None,
                trailer_docking: None,
                started_shipment: None,
                lgv_start_loading: None,
                dock_ready: None,
                is_preload: false,
            };

            match event {
                DockDoorEvent::ShipmentAssigned(e) => {
                    new_event.shipment_assigned = Some(e.timestamp);
                },
                DockDoorEvent::DockAssigned(e) => {
                    new_event.dock_assignment = Some(e.timestamp);
                },
                DockDoorEvent::TrailerDocked(e) => {
                    new_event.trailer_docking = Some(e.timestamp);
                },
                DockDoorEvent::LoadingStarted(e) => {
                    new_event.started_shipment = Some(e.timestamp);
                },
                DockDoorEvent::WmsEvent(e) if e.event_type == "STARTED_SHIPMENT" => {
                    new_event.started_shipment = Some(e.timestamp);
                },
                DockDoorEvent::WmsEvent(e) if e.event_type == "LGV_START_LOADING" => {
                    new_event.lgv_start_loading = Some(e.timestamp);
                    should_insert = true;
                },
                DockDoorEvent::DoorStateChanged(e) if e.new_state == crate::models::DoorState::DoorReady => {
                    new_event.dock_ready = Some(e.timestamp);
                },
                _ => {}
            }

            self.calculate_durations(&mut new_event);
            consolidated_events.insert(key.clone(), new_event);
        }

        if should_insert {
            if let Some(event_to_insert) = consolidated_events.get(&key) {
                self.insert_consolidated_event(event_to_insert).await?;
            }
        }

        if should_remove {
            consolidated_events.remove(&key);
        }

        Ok(())
    }

    fn calculate_durations(&self, event: &mut ConsolidatedDockEvent) {
        if let (Some(dock_assignment), Some(trailer_docking)) = (event.dock_assignment, event.trailer_docking) {
            event.docking_time_minutes = Some((trailer_docking - dock_assignment).num_minutes() as i32);
        }

        if let (Some(trailer_docking), Some(started_shipment)) = (event.trailer_docking, event.started_shipment) {
            event.inspection_time_minutes = Some((started_shipment - trailer_docking).num_minutes() as i32);
        }

        if let (Some(started_shipment), Some(lgv_start_loading)) = (event.started_shipment, event.lgv_start_loading) {
            event.enqueued_time_minutes = Some((lgv_start_loading - started_shipment).num_minutes() as i32);
        }
    }

    async fn insert_consolidated_event(&self, event: &ConsolidatedDockEvent) -> DockManagerResult<()> {
        self.db_service.insert_consolidated_event(event).await
    }

    async fn insert_db_events(&self, db_events: &[DbInsert]) -> DockManagerResult<()> {
        for event in db_events {
            let plant_id = event.get_plant_id();
            self.door_repository.insert_db_event(plant_id, event.clone()).await?;
        }
        Ok(())
    }

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
                Alert::new(alert_manager::AlertType::ManualModeAlert, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .build()
            },
            context_analyzer::AlertType::TrailerPatternIssue { door_name, shipment_id, .. } => {
                Alert::new(AlertType::TrailerPatternIssue, door_name)
                    .shipment_id(shipment_id.unwrap_or_default())
                    .build()
            },
            context_analyzer::AlertType::TrailerDockedNotStarted { door_name, duration } => {
                Alert::new(alert_manager::AlertType::TrailerDockedNotStarted, door_name)
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

    async fn add_to_monitoring_queue(&self, alert_type: context_analyzer::AlertType, door: &DockDoor) {
        match alert_type {
            context_analyzer::AlertType::SuspendedDoor { door_name, shipment_id, user, .. } => {
                self.monitoring_queue.add(MonitoringItem::SuspendedShipment {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    shipment_id: shipment_id.unwrap_or_default(),
                    suspended_at: Local::now().naive_local(),
                    user,
                }).await;
            },
            context_analyzer::AlertType::TrailerDocked { door_name, timestamp, success: true, .. } => {
                self.monitoring_queue.add(MonitoringItem::TrailerDockedNotStarted {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    docked_at: timestamp,
                }).await;
            },
            context_analyzer::AlertType::ShipmentStartedLoadNotReady { door_name, shipment_id, .. } => {
                self.monitoring_queue.add(MonitoringItem::ShipmentStartedLoadNotReady {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    shipment_id,
                    started_at: Local::now().naive_local(),
                }).await;
            },
            context_analyzer::AlertType::DockReady { door_name, timestamp, .. } => {
                self.monitoring_queue.add(MonitoringItem::TrailerDockedNotStarted {
                    plant_id: door.plant_id.clone(),
                    door_name,
                    docked_at: timestamp,
                }).await;
            },
            _ => {}
        }
    }
}