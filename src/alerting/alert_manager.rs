use crate::analysis::AlertType;
use crate::config::Settings;
use crate::errors::DockManagerResult;
use chrono::{NaiveDateTime, Local, Duration};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, warn};

#[derive(Debug, Clone)]
pub enum Alert {
    SuspendedDoor {
        door_name: String,
        duration: Duration,
        shipment_id: Option<String>,
    },
    TrailerPatternIssue {
        door_name: String,
        issue: String,
        severity: i32,
        shipment_id: Option<String>,
    },
    LongLoadingStart {
        door_name: String,
        shipment_id: String,
        duration: Duration,
    },
    ShipmentStartedLoadNotReady {
        door_name: String,
        shipment_id: String,
        reason: String,
    },
    ManualModeAlert {
        door_name: String,
        shipment_id: Option<String>,
    },
    ManualInterventionTimeout {
        door_name: String,
        shipment_id: String,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    },
    NewShipmentPreviousTrailerPresent {
        dock_name: String,
        new_shipment: String,
        previous_shipment: Option<String>,
    },
    TrailerHostage {
        door_name: String,
        shipment_id: Option<String>,
        duration: Duration,
    },
}

pub struct AlertManager {
    settings: Arc<Settings>,
    client: Client,
    alert_cooldown: Arc<Mutex<std::collections::HashMap<String, NaiveDateTime>>>,
}

impl AlertManager {
    pub fn new(settings: Arc<Settings>) -> Self {
        info!("Initializing Alert Manager");
        let client = Client::new();
        let alert_cooldown = Arc::new(Mutex::new(std::collections::HashMap::new()));
        Self { settings, client, alert_cooldown }
    }

    pub async fn handle_alert(&self, alert_type: AlertType) -> DockManagerResult<()> {
        let alert = self.convert_alert_type(alert_type);
        let (cooldown_key, repeat_interval) = match &alert {
            Alert::SuspendedDoor { door_name, .. } => (
                format!("suspended_door_{}", door_name),
                self.settings.alerts.suspended_door.repeat_interval,
            ),
            Alert::TrailerPatternIssue { door_name, .. } => (
                format!("trailer_pattern_{}", door_name),
                self.settings.alerts.trailer_pattern.repeat_interval,
            ),
            Alert::LongLoadingStart { door_name, .. } => (
                format!("long_loading_start_{}", door_name),
                self.settings.alerts.long_loading_start.repeat_interval,
            ),
            Alert::ShipmentStartedLoadNotReady { door_name, .. } => (
                format!("shipment_started_load_not_ready_{}", door_name),
                self.settings.alerts.shipment_started_load_not_ready.repeat_interval,
            ),
            Alert::TrailerHostage { door_name, .. } => (
                format!("trailer_hostage_{}", door_name),
                self.settings.alerts.trailer_hostage.repeat_interval,
            ),
            // Add other cases as needed
            _ => (
                "default".to_string(),
                self.settings.alerts.suspended_door.repeat_interval, // Use a default interval
            ),
        };

        let should_alert = self.check_cooldown(&cooldown_key, repeat_interval).await;

        if should_alert {
            self.send_alert(alert).await?;
            self.update_cooldown(cooldown_key).await;
        }

        Ok(())
    }

    async fn process_alert(&self, alert: Alert) -> DockManagerResult<()> {
        let alert_key = self.get_alert_key(&alert);
        let repeat_interval = self.get_repeat_interval(&alert);
        let should_send = self.check_cooldown(&alert_key, repeat_interval).await;

        if should_send {
            self.send_alert(alert).await?;
            self.update_cooldown(alert_key).await;
        } else {
            warn!("Alert suppressed due to cooldown: {:?}", alert);
        }

        Ok(())
    }

    fn get_repeat_interval(&self, alert: &Alert) -> u64 {
        match alert {
            Alert::SuspendedDoor { .. } => self.settings.alerts.suspended_door.repeat_interval,
            Alert::TrailerPatternIssue { .. } => self.settings.alerts.trailer_pattern.repeat_interval,
            Alert::LongLoadingStart { .. } => self.settings.alerts.long_loading_start.repeat_interval,
            Alert::ShipmentStartedLoadNotReady { .. } => self.settings.alerts.shipment_started_load_not_ready.repeat_interval,
            Alert::TrailerHostage { .. } => self.settings.alerts.trailer_hostage.repeat_interval,
            // Add other cases as needed
            _ => self.settings.alerts.suspended_door.repeat_interval, // Use a default interval
        }
    }

    async fn send_alert(&self, alert: Alert) -> DockManagerResult<()> {
        let webhook_url = &self.settings.plants[0].alert_webhook_url;
        let alert_message = self.format_alert_message(&alert);

        let response = self.client.post(webhook_url)
            .json(&json!({
                "text": alert_message
            }))
            .send()
            .await
            .map_err(|e| crate::errors::DockManagerError::ConnectionError(e.to_string()))?;

        if response.status().is_success() {
            info!("Alert sent successfully: {:?}", alert);
        } else {
            error!("Failed to send alert: {:?}", alert);
        }

        Ok(())
    }

    fn format_alert_message(&self, alert: &Alert) -> String {
        match alert {
            Alert::SuspendedDoor { door_name, duration, shipment_id } => {
                format!("🚨 SUSPENDED DOOR ALERT: Door {} has been suspended for {}. Shipment ID: {}",
                        door_name, self.format_duration(duration), shipment_id.as_deref().unwrap_or("N/A"))
            },
            Alert::TrailerPatternIssue { door_name, issue, severity, shipment_id } => {
                format!("⚠️ TRAILER PATTERN ISSUE: Door {} - {}. Severity: {}. Shipment ID: {}",
                        door_name, issue, severity, shipment_id.as_deref().unwrap_or("N/A"))
            },
            Alert::LongLoadingStart { door_name, shipment_id, duration } => {
                format!("⏳ LONG LOADING START: Door {} - Shipment {} has been in loading state for {} with no progress",
                        door_name, shipment_id, self.format_duration(duration))
            },
            Alert::ShipmentStartedLoadNotReady { door_name, shipment_id, reason } => {
                format!("🛑 SHIPMENT STARTED LOAD NOT READY: Door {} - Shipment {} - Reason: {}",
                        door_name, shipment_id, reason)
            },
            Alert::ManualModeAlert { door_name, shipment_id } => {
                format!("🔧 MANUAL MODE ALERT: Door {} is in manual mode. Shipment ID: {}",
                        door_name, shipment_id.as_deref().unwrap_or("N/A"))
            },
            Alert::ManualInterventionTimeout { door_name, shipment_id, start_time, end_time } => {
                let duration = end_time.signed_duration_since(*start_time);
                format!("⏰ MANUAL INTERVENTION TIMEOUT: Door {} - Shipment {}. Duration: {}",
                        door_name, shipment_id, self.format_duration(&duration))
            },
            Alert::NewShipmentPreviousTrailerPresent { dock_name, new_shipment, previous_shipment } => {
                format!("🚛 NEW SHIPMENT, PREVIOUS TRAILER PRESENT: Door {} - New Shipment: {}, Previous Shipment: {}",
                        dock_name, new_shipment, previous_shipment.as_deref().unwrap_or("N/A"))
            },
            Alert::TrailerHostage { door_name, shipment_id, duration } => {
                format!("🚨 TRAILER HOSTAGE ALERT: Door {} has a trailer hostage for {}. Shipment ID: {}",
                        door_name, self.format_duration(duration), shipment_id.as_deref().unwrap_or("N/A"))
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

    fn convert_alert_type(&self, alert_type: AlertType) -> Alert {
        match alert_type {
            AlertType::LongDockingTime(duration) => Alert::TrailerPatternIssue {
                door_name: "Unknown".to_string(), // You might need to pass more context to get the door name
                issue: format!("Long docking time: {}", self.format_duration(&duration)),
                severity: 1,
                shipment_id: None,
            },
            AlertType::ManualIntervention => Alert::ManualModeAlert {
                door_name: "Unknown".to_string(),
                shipment_id: None,
            },
            AlertType::TrailerHostage => Alert::TrailerPatternIssue {
                door_name: "Unknown".to_string(),
                issue: "Trailer hostage situation detected".to_string(),
                severity: 2,
                shipment_id: None,
            },
            AlertType::UnsafeDeparture => Alert::TrailerPatternIssue {
                door_name: "Unknown".to_string(),
                issue: "Unsafe departure detected".to_string(),
                severity: 3,
                shipment_id: None,
            },
            AlertType::ManualModeAlert => Alert::ManualModeAlert {
                door_name: "Unknown".to_string(),
                shipment_id: None,
            },
            AlertType::NewShipmentPreviousTrailerPresent { dock_name, new_shipment, previous_shipment, timestamp: _ } => {
                Alert::NewShipmentPreviousTrailerPresent {
                    dock_name,
                    new_shipment,
                    previous_shipment,
                }
            },
            AlertType::ManualInterventionTimeout { dock_name, shipment_id, start_time, end_time } => {
                Alert::ManualInterventionTimeout {
                    door_name: dock_name,
                    shipment_id,
                    start_time,
                    end_time,
                }
            },
        }
    }

    fn get_alert_key(&self, alert: &Alert) -> String {
        match alert {
            Alert::SuspendedDoor { door_name, .. } => format!("suspended_door_{}", door_name),
            Alert::TrailerPatternIssue { door_name, issue, .. } => format!("trailer_pattern_{}_{}", door_name, issue),
            Alert::LongLoadingStart { door_name, shipment_id, .. } => format!("long_loading_{}_{}", door_name, shipment_id),
            Alert::ShipmentStartedLoadNotReady { door_name, shipment_id, .. } => format!("load_not_ready_{}_{}", door_name, shipment_id),
            Alert::ManualModeAlert { door_name, .. } => format!("manual_mode_{}", door_name),
            Alert::ManualInterventionTimeout { door_name, shipment_id, .. } => format!("manual_intervention_timeout_{}_{}", door_name, shipment_id),
            Alert::NewShipmentPreviousTrailerPresent { dock_name, new_shipment, .. } => format!("new_shipment_previous_trailer_{}_{}", dock_name, new_shipment),
            Alert::TrailerHostage { door_name, .. } => format!("trailer_hostage_{}", door_name),
        }
    }

    async fn check_cooldown(&self, key: &str, repeat_interval: u64) -> bool {
        let cooldown_map = self.alert_cooldown.lock().await;
        if let Some(last_sent) = cooldown_map.get(key) {
            let now = Local::now().naive_local();
            now.signed_duration_since(*last_sent) > Duration::seconds(repeat_interval as i64)
        } else {
            true
        }
    }

    async fn update_cooldown(&self, alert_key: String) {
        let mut cooldown_map = self.alert_cooldown.lock().await;
        cooldown_map.insert(alert_key, Local::now().naive_local());
    }
}
