use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use chrono::{Duration, Local, NaiveDateTime};
use reqwest::Client;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{info, error};
use crate::config::AlertThresholds;
use crate::utils::format_duration;

/// Configuration for alert thresholds and repeat intervals
pub struct AlertConfig {
    pub suspended_door: AlertThresholds,
    pub trailer_pattern: AlertThresholds,
    pub long_loading_start: AlertThresholds,
    pub shipment_started_load_not_ready: AlertThresholds,
    pub trailer_hostage: AlertThresholds,
    pub trailer_docked: AlertThresholds,
    pub dock_ready: AlertThresholds,
    pub trailer_undocked: AlertThresholds,
}

/// Threshold configuration for a specific alert type
pub struct AlertThreshold {
    pub repeat_interval: u64,
}

/// Represents different types of alerts that can be generated by the system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertType {
    SuspendedDoor,
    TrailerPatternIssue,
    LongLoadingStart,
    ShipmentStartedLoadNotReady,
    ManualModeAlert,
    ManualInterventionTimeout,
    NewShipmentPreviousTrailerPresent,
    TrailerHostage,
    TrailerDockedNotStarted,
    TrailerDocked,
    DockReady,
    TrailerUndocked,
}

/// Represents an alert with all its associated information
#[derive(Debug, Clone)]
pub struct Alert {
    alert_type: AlertType,
    door_name: String,
    shipment_id: Option<String>,
    duration: Option<Duration>,
    additional_info: HashMap<String, String>,
}

/// Builder for creating Alert instances
pub struct AlertBuilder {
    alert: Alert,
}

impl Alert {
    /// Creates a new AlertBuilder instance
    ///
    /// # Arguments
    ///
    /// * `alert_type` - The type of the alert
    /// * `door_name` - The name of the door associated with the alert
    ///
    /// # Returns
    ///
    /// An AlertBuilder instance
    pub fn new(alert_type: AlertType, door_name: String) -> AlertBuilder {
        AlertBuilder {
            alert: Alert {
                alert_type,
                door_name,
                shipment_id: None,
                duration: None,
                additional_info: HashMap::new(),
            }
        }
    }
}

impl AlertBuilder {
    /// Sets the shipment ID for the alert
    pub fn shipment_id(mut self, shipment_id: String) -> Self {
        self.alert.shipment_id = Some(shipment_id);
        self
    }

    /// Sets the duration for the alert
    pub fn duration(mut self, duration: Duration) -> Self {
        self.alert.duration = Some(duration);
        self
    }

    /// Adds additional information to the alert
    pub fn add_info(mut self, key: String, value: String) -> Self {
        self.alert.additional_info.insert(key, value);
        self
    }

    /// Builds and returns the Alert instance
    pub fn build(self) -> Alert {
        self.alert
    }
}

impl fmt::Display for Alert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base_msg = match self.alert_type {
            AlertType::SuspendedDoor => format!("🚨 SUSPENDED DOOR ALERT: Door {} has been suspended", self.door_name),
            AlertType::TrailerPatternIssue => format!("⚠️ TRAILER PATTERN ISSUE: Door {}", self.door_name),
            AlertType::LongLoadingStart => format!("⏳ LONG LOADING START: Door {}", self.door_name),
            AlertType::ShipmentStartedLoadNotReady => format!("🛑 SHIPMENT STARTED LOAD NOT READY: Door {}", self.door_name),
            AlertType::ManualModeAlert => format!("🔧 MANUAL MODE ALERT: Door {}", self.door_name),
            AlertType::ManualInterventionTimeout => format!("⏰ MANUAL INTERVENTION TIMEOUT: Door {}", self.door_name),
            AlertType::NewShipmentPreviousTrailerPresent => format!("🚛 NEW SHIPMENT, PREVIOUS TRAILER PRESENT: Door {}", self.door_name),
            AlertType::TrailerHostage => format!("🚨 TRAILER HOSTAGE ALERT: Door {}", self.door_name),
            AlertType::TrailerDockedNotStarted => format!("⏳ TRAILER DOCKED NOT STARTED: Door {}", self.door_name),
            AlertType::TrailerDocked => {
                let success = self.additional_info.get("success").unwrap().parse::<bool>().unwrap();

                if success {
                    format!(
                        "🚛 TRAILER DOCKED: Door {}",
                        self.door_name
                    )
                } else {
                    format!(
                        "⚠️ TRAILER DOCKING FAILED: Door {}",
                        self.door_name
                    )
                }
            },
            AlertType::DockReady => format!("✅ DOCK READY: Door {}", self.door_name),
            AlertType::TrailerUndocked => format!("🚚 TRAILER UNDOCKED: Door {}", self.door_name),
        };

        let mut full_msg = base_msg;
        if let Some(shipment_id) = &self.shipment_id {
            full_msg.push_str(&format!(" - Shipment ID: {}", shipment_id));
        }
        if let Some(duration) = self.duration {
            full_msg.push_str(&format!(" - Duration: {}", format_duration(&duration)));
        }
        for (key, value) in &self.additional_info {
            if key.contains("timestamp") {
                if let Ok(val) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f") {
                    full_msg.push_str(&format!(" - {}: {}", key, val.format("%Y-%m-%d %H:%M:%S")));
                } else {
                    full_msg.push_str(&format!(" - {}: {}", key, value));
                }
            } else {
                full_msg.push_str(&format!(" - {}: {}", key, value));
            }
        }

        write!(f, "{}", full_msg)
    }
}

/// Error types specific to alert operations
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("Failed to send alert: {0}")]
    SendFailure(String),
    #[error("Invalid alert configuration: {0}")]
    InvalidConfig(String),
}

type AlertResult<T> = Result<T, AlertError>;

const DEFAULT_REPEAT_INTERVAL: u64 = 300; // 5 minutes

/// Manages the creation and sending of alerts
pub struct AlertManager {
    settings: Arc<AlertConfig>,
    client: Client,
    alert_cooldown: Arc<Mutex<HashMap<String, NaiveDateTime>>>,
    monitored_alert_types: HashSet<AlertType>,
    webhook_url: String,
}

impl AlertManager {
    /// Creates a new AlertManager instance
    ///
    /// # Arguments
    ///
    /// * `settings` - Alert configuration settings
    /// * `webhook_url` - URL for sending alert webhooks
    ///
    /// # Returns
    ///
    /// A new AlertManager instance
    pub fn new(settings: Arc<AlertConfig>, webhook_url: String) -> Self {
        info!("Initializing Alert Manager");
        let client = Client::new();
        let alert_cooldown = Arc::new(Mutex::new(HashMap::new()));
        let mut monitored_alert_types = HashSet::new();
        monitored_alert_types.insert(AlertType::SuspendedDoor);
        monitored_alert_types.insert(AlertType::TrailerDockedNotStarted);
        monitored_alert_types.insert(AlertType::ShipmentStartedLoadNotReady);

        Self {
            settings,
            client,
            alert_cooldown,
            monitored_alert_types,
            webhook_url,
        }
    }

    /// Handles an incoming alert
    ///
    /// # Arguments
    ///
    /// * `alert` - The alert to handle
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    pub async fn handle_alert(&self, alert: Alert) -> AlertResult<()> {
        info!("Handling Alert: {:#?}", alert);

        let (cooldown_key, repeat_interval) = match alert.alert_type {
            AlertType::SuspendedDoor => (
                format!("suspended_door_{}", alert.door_name),
                self.settings.suspended_door.repeat_interval,
            ),
            AlertType::TrailerPatternIssue => (
                format!("trailer_pattern_{}", alert.door_name),
                self.settings.trailer_pattern.repeat_interval,
            ),
            AlertType::LongLoadingStart => (
                format!("long_loading_start_{}", alert.door_name),
                self.settings.long_loading_start.repeat_interval,
            ),
            AlertType::ShipmentStartedLoadNotReady => (
                format!("shipment_started_load_not_ready_{}", alert.door_name),
                self.settings.shipment_started_load_not_ready.repeat_interval,
            ),
            AlertType::TrailerHostage => (
                format!("trailer_hostage_{}", alert.door_name),
                self.settings.trailer_hostage.repeat_interval,
            ),
            AlertType::TrailerDocked => (
                format!("trailer_docked_{}", alert.door_name),
                self.settings.trailer_docked.repeat_interval,
            ),
            AlertType::DockReady => (
                format!("dock_ready_{}", alert.door_name),
                self.settings.dock_ready.repeat_interval,
            ),
            AlertType::TrailerUndocked => (
                format!("trailer_undocked_{}", alert.door_name),
                self.settings.trailer_undocked.repeat_interval,
            ),
            _ => (
                format!("default_{}", alert.door_name),
                DEFAULT_REPEAT_INTERVAL,
            ),
        };

        if self.monitored_alert_types.contains(&alert.alert_type) {
            info!("Sending monitored alert: {:?}", alert.alert_type);
            self.send_alert(&alert).await?;
        } else if self.check_cooldown(&cooldown_key, repeat_interval).await {
            info!("Sending alert: {:?}", alert);
            self.send_alert(&alert).await?;
            self.update_cooldown(cooldown_key).await;
        }

        Ok(())
    }

    /// Sends an alert
    ///
    /// # Arguments
    ///
    /// * `alert` - The alert to send
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    async fn send_alert(&self, alert: &Alert) -> AlertResult<()> {
        let alert_message = alert.to_string();

        let response = self.client.post(&self.webhook_url)
            .json(&json!({
                "text": alert_message
            }))
            .send()
            .await
            .map_err(|e| AlertError::SendFailure(e.to_string()))?;

        if response.status().is_success() {
            info!("Alert sent successfully: {:?}", alert);
            Ok(())
        } else {
            error!("Failed to send alert: {:?}", alert);
            Err(AlertError::SendFailure("Failed to send alert".to_string()))
        }
    }

    /// Checks if an alert should be sent based on the cooldown period
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the alert
    /// * `repeat_interval` - The minimum time between alerts
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the alert should be sent
    async fn check_cooldown(&self, key: &str, repeat_interval: u64) -> bool {
        let cooldown_map = self.alert_cooldown.lock().await;
        if let Some(last_sent) = cooldown_map.get(key) {
            let now = Local::now().naive_local();
            now.signed_duration_since(*last_sent) > Duration::seconds(repeat_interval as i64)
        } else {
            true
        }
    }

    /// Updates the cooldown time for an alert
    ///
    /// # Arguments
    ///
    /// * `alert_key` - The key identifying the alert
    async fn update_cooldown(&self, alert_key: String) {
        let mut cooldown_map = self.alert_cooldown.lock().await;
        cooldown_map.insert(alert_key, Local::now().naive_local());
    }
}
