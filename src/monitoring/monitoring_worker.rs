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
        let monitoring_check_interval = self.settings.monitoring.check_interval.clone();
        let mut interval = interval(tokio::time::Duration::from_secs(monitoring_check_interval));

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
                info!("Processing SuspendedShipment for door: {}, shipment: {}", door_name, shipment_id);
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if door.loading_status == crate::models::LoadingStatus::Suspended {
                        let duration = Local::now().naive_local().signed_duration_since(suspended_at);
                        let alert_threshold = Duration::seconds(self.settings.monitoring.suspended_shipment.alert_threshold as i64);
                        let repeat_interval = Duration::seconds(self.settings.monitoring.suspended_shipment.repeat_interval as i64);

                        info!("Door {} is suspended for {:?}. Alert threshold: {:?}, Repeat interval: {:?}",
                          door_name, duration, alert_threshold, repeat_interval);

                        if duration >= alert_threshold {
                            let intervals_passed = duration.num_seconds() / repeat_interval.num_seconds();
                            let last_alert_time = intervals_passed * repeat_interval.num_seconds();
                            let should_alert = duration.num_seconds() - last_alert_time < 60; // Alert within the first minute after an interval

                            info!("Intervals passed: {}, Last alert time: {}s, Current duration: {}s, Should alert: {}",
                              intervals_passed, last_alert_time, duration.num_seconds(), should_alert);

                            if should_alert {
                                info!("Sending alert for suspended door {}", door_name);
                                if let Err(e) = self.alert_manager.handle_alert(AlertType::SuspendedDoor {
                                    door_name: door_name.clone(),
                                    duration,
                                    shipment_id: Some(shipment_id.clone()),
                                }).await {
                                    error!("Failed to handle SuspendedDoor alert: {:?}", e);
                                }
                            }
                        }
                        true // Keep in queue for future checks
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
                info!("Processing TrailerDockedNotStarted for door: {}", door_name);
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    info!("Door state: {:?}, Loading status: {:?}", door.trailer_state, door.loading_status);
                    if door.loading_status == crate::models::LoadingStatus::Loading {
                        info!("Loading has started for door {}", door_name);
                        false // Remove from queue
                    } else {
                        let duration = Local::now().naive_local().signed_duration_since(docked_at);
                        let alert_threshold = Duration::seconds(self.settings.monitoring.trailer_docked_not_started.alert_threshold as i64);
                        let repeat_interval = Duration::seconds(self.settings.monitoring.trailer_docked_not_started.repeat_interval as i64);

                        if duration >= alert_threshold {
                            let intervals_passed = duration.num_seconds() / repeat_interval.num_seconds();
                            let last_alert_time = intervals_passed * repeat_interval.num_seconds();
                            let should_alert = duration.num_seconds() - last_alert_time < 60;

                            if should_alert {
                                info!("Sending alert for trailer docked not started {}", door_name);
                                if let Err(e) = self.alert_manager.handle_alert(AlertType::TrailerDockedNotStarted {
                                    door_name: door_name.clone(),
                                    duration,
                                }).await {
                                    error!("Failed to handle TrailerDockedNotStarted alert: {:?}", e);
                                }
                            }
                        }
                        true // Keep in queue for future checks
                    }
                } else {
                    warn!("Door {} not found in state manager", door_name);
                    false // Remove from queue
                }
            },
            MonitoringItem::ShipmentStartedLoadNotReady { door_name, shipment_id, started_at } => {
                info!("Processing ShipmentStartedLoadNotReady for door: {}, shipment: {}", door_name, shipment_id);
                let door = self.state_manager.get_door(&door_name).await;
                if let Some(door) = door {
                    if !door.check_loading_readiness() {
                        let duration = Local::now().naive_local().signed_duration_since(started_at);
                        let alert_threshold = Duration::seconds(self.settings.monitoring.shipment_started_load_not_ready.alert_threshold as i64);
                        let repeat_interval = Duration::seconds(self.settings.monitoring.shipment_started_load_not_ready.repeat_interval as i64);

                        info!("Door {} has shipment started load not ready for {:?}. Alert threshold: {:?}, Repeat interval: {:?}",
                  door_name, duration, alert_threshold, repeat_interval);

                        if duration >= alert_threshold {
                            let intervals_passed = duration.num_seconds() / repeat_interval.num_seconds();
                            let last_alert_time = intervals_passed * repeat_interval.num_seconds();
                            let should_alert = duration.num_seconds() - last_alert_time < 60;

                            info!("Intervals passed: {}, Last alert time: {}s, Current duration: {}s, Should alert: {}",
                      intervals_passed, last_alert_time, duration.num_seconds(), should_alert);

                            if should_alert {
                                info!("Sending alert for shipment started load not ready {}", door_name);
                                if let Err(e) = self.alert_manager.handle_alert(AlertType::ShipmentStartedLoadNotReady {
                                    door_name: door_name.clone(),
                                    shipment_id: shipment_id.clone(),
                                    reason: format!("Dock still not ready after {}", self.format_duration(&duration)),
                                }).await {
                                    error!("Failed to handle ShipmentStartedLoadNotReady alert: {:?}", e);
                                }
                            }
                        }
                        true // Keep in queue for future checks
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