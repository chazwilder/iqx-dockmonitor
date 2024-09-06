use std::sync::Arc;
use chrono::{Local, Duration, NaiveDateTime};
use tokio::time::interval;
use tracing::{info, warn, error};
use crate::alerting::alert_manager::{AlertManager, Alert, AlertType};
use crate::config::Settings;
use crate::models::LoadingStatus;
use crate::state_management::DockDoorStateManager;
use crate::utils::format_duration;
use super::monitoring_queue::{MonitoringQueue, MonitoringItem};

/// Represents a worker that monitors and processes items from a monitoring queue
#[derive(Clone)]
pub struct MonitoringWorker {
    queue: Arc<MonitoringQueue>,
    state_manager: Arc<DockDoorStateManager>,
    alert_manager: Arc<AlertManager>,
    settings: Arc<Settings>,
}

impl MonitoringWorker {
    /// Creates a new MonitoringWorker instance
    ///
    /// # Arguments
    ///
    /// * `queue` - The monitoring queue to process items from
    /// * `state_manager` - The state manager to retrieve dock door information
    /// * `alert_manager` - The alert manager to send alerts
    /// * `settings` - The application settings
    ///
    /// # Returns
    ///
    /// A new MonitoringWorker instance
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

    /// Runs the monitoring worker, continuously processing items from the queue
    pub async fn run(&self) {
        let monitoring_check_interval = self.settings.monitoring.check_interval;
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

    /// Processes a single monitoring item
    ///
    /// # Arguments
    ///
    /// * `item` - The monitoring item to process
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the item should be requeued
    async fn process_item(&self, item: MonitoringItem) -> bool {
        match item {
            MonitoringItem::SuspendedShipment { door_name, shipment_id, suspended_at, user } => {
                self.process_suspended_shipment(door_name, shipment_id, suspended_at, user).await
            },
            MonitoringItem::TrailerDockedNotStarted { door_name, docked_at } => {
                self.process_trailer_docked_not_started(door_name, docked_at).await
            },
            MonitoringItem::ShipmentStartedLoadNotReady { door_name, shipment_id, started_at } => {
                self.process_shipment_started_load_not_ready(door_name, shipment_id, started_at).await
            },
        }
    }

    /// Processes a suspended shipment monitoring item
    ///
    /// # Arguments
    ///
    /// * `door_name` - The name of the door
    /// * `shipment_id` - The ID of the shipment
    /// * `suspended_at` - The timestamp when the shipment was suspended
    /// * `user` - The user who suspended the shipment
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the item should be requeued
    async fn process_suspended_shipment(&self, door_name: String, shipment_id: String, suspended_at: NaiveDateTime, user: String) -> bool {
        info!("Processing SuspendedShipment for door: {}, shipment: {}", door_name, shipment_id);
        let door = self.state_manager.get_door(&door_name).await;
        if let Some(door) = door {
            if door.loading_status == LoadingStatus::Suspended {
                let duration = Local::now().naive_local().signed_duration_since(suspended_at);
                let alert_threshold = Duration::seconds(self.settings.monitoring.suspended_shipment.alert_threshold as i64);
                let repeat_interval = Duration::seconds(self.settings.monitoring.suspended_shipment.repeat_interval as i64);

                info!("Door {} is suspended for {:?}. Alert threshold: {:?}, Repeat interval: {:?}",
                    door_name, duration, alert_threshold, repeat_interval);

                if duration >= alert_threshold && self.should_alert(duration, repeat_interval) {
                    info!("Sending alert for suspended door {}", door_name);
                    let alert = Alert::new(AlertType::SuspendedDoor, door_name.clone())
                        .shipment_id(shipment_id.clone())
                        .duration(duration)
                        .add_info("user".to_string(), user.clone())
                        .build();
                    if let Err(e) = self.alert_manager.handle_alert(alert).await {
                        error!("Failed to handle SuspendedDoor alert: {:?}", e);
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
    }

    /// Processes a trailer docked not started monitoring item
    ///
    /// # Arguments
    ///
    /// * `door_name` - The name of the door
    /// * `docked_at` - The timestamp when the trailer was docked
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the item should be requeued
    async fn process_trailer_docked_not_started(&self, door_name: String, docked_at: NaiveDateTime) -> bool {
        info!("Processing TrailerDockedNotStarted for door: {}", door_name);
        let door = self.state_manager.get_door(&door_name).await;
        if let Some(door) = door {
            info!("Door state: {:?}, Loading status: {:?}", door.trailer_state, door.loading_status);
            let loading_started = matches!(door.loading_status,
                LoadingStatus::Loading |
                LoadingStatus::Suspended |
                LoadingStatus::Completed |
                LoadingStatus::WaitingForExit |
                LoadingStatus::CancelledShipment |
                LoadingStatus::Idle |
                LoadingStatus::StartedWithAnticipation
            );

            let door_check = door.sensors.get("TRAILER_AT_DOOR").unwrap().get_sensor_data().current_value.unwrap();

            if door_check == 0 {
                info!("Trailer is not at door {} sensor value = {}", door_name, door_check);
                return false // Remove from queue
            }

            if loading_started {
                info!("Loading is started or progressed for door {}", door_name);
                false // Remove from queue
            } else {
                let duration = Local::now().naive_local().signed_duration_since(docked_at);
                let alert_threshold = Duration::seconds(self.settings.monitoring.trailer_docked_not_started.alert_threshold as i64);
                let repeat_interval = Duration::seconds(self.settings.monitoring.trailer_docked_not_started.repeat_interval as i64);

                if duration >= alert_threshold && self.should_alert(duration, repeat_interval) {
                    info!("Sending alert for trailer docked not started {}", door_name);
                    let alert = Alert::new(AlertType::TrailerDockedNotStarted, door_name.clone())
                        .duration(duration)
                        .build();
                    if let Err(e) = self.alert_manager.handle_alert(alert).await {
                        error!("Failed to handle TrailerDockedNotStarted alert: {:?}", e);
                    }
                }
                true // Keep in queue for future checks
            }
        } else {
            warn!("Door {} not found in state manager", door_name);
            false // Remove from queue
        }
    }

    /// Processes a shipment started load not ready monitoring item
    ///
    /// # Arguments
    ///
    /// * `door_name` - The name of the door
    /// * `shipment_id` - The ID of the shipment
    /// * `started_at` - The timestamp when the shipment started loading
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the item should be requeued
    async fn process_shipment_started_load_not_ready(&self, door_name: String, shipment_id: String, started_at: NaiveDateTime) -> bool {
        info!("Processing ShipmentStartedLoadNotReady for door: {}, shipment: {}", door_name, shipment_id);
        let door = self.state_manager.get_door(&door_name).await;
        if let Some(door) = door {
            if !door.check_loading_readiness() {
                let duration = Local::now().naive_local().signed_duration_since(started_at);
                let alert_threshold = Duration::seconds(self.settings.monitoring.shipment_started_load_not_ready.alert_threshold as i64);
                let repeat_interval = Duration::seconds(self.settings.monitoring.shipment_started_load_not_ready.repeat_interval as i64);

                info!("Door {} has shipment started load not ready for {:?}. Alert threshold: {:?}, Repeat interval: {:?}",
                    door_name, duration, alert_threshold, repeat_interval);

                if duration >= alert_threshold && self.should_alert(duration, repeat_interval) {
                    info!("Sending alert for shipment started load not ready {}", door_name);
                    let alert = Alert::new(AlertType::ShipmentStartedLoadNotReady, door_name.clone())
                        .shipment_id(shipment_id.clone())
                        .add_info("reason".to_string(), format!("Dock still not ready after {}", format_duration(&duration)))
                        .build();
                    if let Err(e) = self.alert_manager.handle_alert(alert).await {
                        error!("Failed to handle ShipmentStartedLoadNotReady alert: {:?}", e);
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
    }

    /// Determines if an alert should be sent based on the duration and repeat interval
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration since the event occurred
    /// * `repeat_interval` - The interval at which alerts should be repeated
    ///
    /// # Returns
    ///
    /// A boolean indicating whether an alert should be sent
    fn should_alert(&self, duration: Duration, repeat_interval: Duration) -> bool {
        let intervals_passed = duration.num_seconds() / repeat_interval.num_seconds();
        let last_alert_time = intervals_passed * repeat_interval.num_seconds();
        duration.num_seconds() - last_alert_time < 60 // Alert within the first minute after an interval
    }
}