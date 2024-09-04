use std::sync::Arc;
use chrono::Local;
use tokio::time::{interval, Duration};
use tracing::info;
use crate::alerting::alert_manager::AlertManager;
use crate::analysis::context_analyzer::AlertType;
use crate::state_management::DockDoorStateManager;
use super::monitoring_queue::{MonitoringQueue, MonitoringItem};

pub struct MonitoringWorker {
    queue: Arc<MonitoringQueue>,
    state_manager: Arc<DockDoorStateManager>,
    alert_manager: Arc<AlertManager>,
}

impl MonitoringWorker {
    pub fn new(
        queue: Arc<MonitoringQueue>,
        state_manager: Arc<DockDoorStateManager>,
        alert_manager: Arc<AlertManager>,
    ) -> Self {
        Self {
            queue,
            state_manager,
            alert_manager,
        }
    }

    pub async fn run(&self) {
        let mut interval = interval(Duration::from_secs(60));  // Check every minute

        loop {
            info!("Starting Monitoring Working Loop...");
            interval.tick().await;

            while let Some(item) = self.queue.get().await {
                info!("Processing Monitoring Item: {:#?}", item.clone());
                self.process_item(item).await;
            }
            info!("Monitoring Working Loop Completed");
        }
    }

    async fn process_item(&self, item: MonitoringItem) {
        match item.clone() {
            MonitoringItem::SuspendedShipment { door_name, shipment_id, suspended_at } => {
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if door.loading_status == crate::models::LoadingStatus::Suspended {
                        let duration = Local::now().naive_local().signed_duration_since(suspended_at);
                        self.alert_manager.handle_alert(AlertType::SuspendedDoor {
                            door_name,
                            duration,
                            shipment_id: Some(shipment_id),
                        }).await.ok();
                        self.queue.add(item).await;  // Re-add to queue for future checking
                    }
                }
            },
            MonitoringItem::TrailerDockedNotStarted { door_name, docked_at } => {
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if door.trailer_state == crate::models::TrailerState::Docked &&
                        door.loading_status != crate::models::LoadingStatus::Loading { // need to fix to != Loading
                        let duration = Local::now().naive_local().signed_duration_since(docked_at);
                        self.alert_manager.handle_alert(AlertType::TrailerDockedNotStarted {
                            door_name,
                            duration,
                        }).await.ok();
                        self.queue.add(item).await;  // Re-add to queue for future checking
                    }
                }
            },
            MonitoringItem::ShipmentStartedLoadNotReady { door_name, shipment_id, started_at } => {
                let _ = started_at;
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if !door.check_loading_readiness() {
                        self.alert_manager.handle_alert(AlertType::ShipmentStartedLoadNotReady {
                            door_name,
                            shipment_id,
                            reason: "Dock still not ready".to_string(),
                        }).await.ok();
                        self.queue.add(item).await;
                    }
                }
            },
        }
    }
}
