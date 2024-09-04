use std::sync::Arc;
use chrono::{Local, Duration};
use tokio::time::interval;
use tracing::{info, warn, error};
use crate::alerting::alert_manager::AlertManager;
use crate::analysis::context_analyzer::AlertType;
use crate::config::Settings;
use crate::state_management::DockDoorStateManager;
use super::monitoring_queue::{MonitoringQueue, MonitoringItem};

#[derive(Clone)]
pub struct MonitoringWorker {
    queue: Arc<MonitoringQueue>,
    state_manager: Arc<DockDoorStateManager>,
    alert_manager: Arc<AlertManager>,
    settings: Arc<Settings>,
}

impl MonitoringWorker {
    pub fn new(
        queue: Arc<MonitoringQueue>,
        state_manager: Arc<DockDoorStateManager>,
        alert_manager: Arc<AlertManager>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            queue,
            state_manager,
            alert_manager,
            settings
        }
    }

    pub async fn run(&self) {
        let mut interval = interval(tokio::time::Duration::from_secs(60));  // Check every minute

        loop {
            interval.tick().await;
            info!("Starting Monitoring Worker Loop...");

            let queue_size = self.queue.len().await;
            info!("Current Monitoring Queue size: {}", queue_size);

            let mut items_to_requeue = Vec::new();

            while let Some(item) = self.queue.get().await {
                info!("Processing Monitoring Item: {:#?}", item);
                if self.process_item(item.clone()).await {
                    items_to_requeue.push(item);
                }
            }

            for item in items_to_requeue {
                self.queue.add(item).await;
            }

            info!("Monitoring Worker Loop Completed");
        }
    }

    async fn process_item(&self, item: MonitoringItem) -> bool {
        match item {
            MonitoringItem::SuspendedShipment { door_name, shipment_id, suspended_at } => {
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if door.loading_status == crate::models::LoadingStatus::Suspended {
                        let duration = Local::now().naive_local().signed_duration_since(suspended_at);
                        if duration >= Duration::seconds(self.settings.monitoring.suspended_shipment.alert_threshold as i64) {
                            if let Err(e) = self.alert_manager.handle_alert(AlertType::SuspendedDoor {
                                door_name: door_name.clone(),
                                duration,
                                shipment_id: Some(shipment_id.clone()),
                            }).await {
                                error!("Failed to handle SuspendedDoor alert: {:?}", e);
                            }
                            true // Keep in queue for future checks
                        } else {
                            true // Not yet reached threshold, keep in queue
                        }
                    } else {
                        info!("Door {} is no longer suspended", door_name);
                        false // Remove from queue
                    }
                } else {
                    warn!("Door {} not found in state manager", door_name);
                    false // Remove from queue
                }
            },
            MonitoringItem::TrailerDockedNotStarted { door_name, docked_at } => {
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if door.trailer_state == crate::models::TrailerState::Docked &&
                        door.loading_status != crate::models::LoadingStatus::Loading {
                        let duration = Local::now().naive_local().signed_duration_since(docked_at);
                        if duration >= Duration::seconds(self.settings.monitoring.trailer_docked_not_started.alert_threshold as i64) {
                            if let Err(e) = self.alert_manager.handle_alert(AlertType::TrailerDockedNotStarted {
                                door_name: door_name.clone(),
                                duration,
                            }).await {
                                error!("Failed to handle TrailerDockedNotStarted alert: {:?}", e);
                            }
                            true // Keep in queue for future checks
                        } else {
                            true // Not yet reached threshold, keep in queue
                        }
                    } else {
                        info!("Door {} trailer state or loading status has changed", door_name);
                        false // Remove from queue
                    }
                } else {
                    warn!("Door {} not found in state manager", door_name);
                    false // Remove from queue
                }
            },
            MonitoringItem::ShipmentStartedLoadNotReady { door_name, shipment_id, started_at } => {
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if !door.check_loading_readiness() {
                        let duration = Local::now().naive_local().signed_duration_since(started_at);
                        if duration >= Duration::seconds(self.settings.monitoring.shipment_started_load_not_ready.alert_threshold as i64) {
                            if let Err(e) = self.alert_manager.handle_alert(AlertType::ShipmentStartedLoadNotReady {
                                door_name: door_name.clone(),
                                shipment_id: shipment_id.clone(),
                                reason: format!("Dock still not ready after {}", self.format_duration(&duration)),
                            }).await {
                                error!("Failed to handle ShipmentStartedLoadNotReady alert: {:?}", e);
                            }
                            true // Keep in queue for future checks
                        } else {
                            true // Not yet reached threshold, keep in queue
                        }
                    } else {
                        info!("Door {} is now ready for loading", door_name);
                        false // Remove from queue
                    }
                } else {
                    warn!("Door {} not found in state manager", door_name);
                    false // Remove from queue
                }
            },
        }
    }

    fn format_duration(&self, duration: &Duration) -> String {
        let total_seconds = duration.num_seconds();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}