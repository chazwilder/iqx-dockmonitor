use std::collections::HashMap;
use std::sync::Mutex;
use chrono::{NaiveDateTime, Local, Duration};
use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use crate::models::{DockDoor, DockDoorEvent, LoadingStatus, TrailerState, ManualMode};
use tracing::{debug};

/// Configuration for the TrailerHostageRule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerHostageRuleConfig {
    /// The time threshold (in seconds) after which an alert should be triggered
    pub alert_threshold: u64,
    /// The interval (in seconds) at which repeat alerts should be sent
    pub repeat_interval: u64,
}

/// Rule for detecting and alerting on trailer hostage situations
pub struct TrailerHostageRule {
    /// The parsed configuration for this rule
    config: TrailerHostageRuleConfig,
    /// A map to track the last alert time for each door
    last_alert_time: Mutex<HashMap<String, NaiveDateTime>>,
}

impl TrailerHostageRule {
    /// Creates a new TrailerHostageRule with the given configuration
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: TrailerHostageRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse TrailerHostageRule configuration");
        Self {
            config: parsed_config,
            last_alert_time: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if an alert should be sent based on the last alert time and repeat interval
    fn should_send_alert(&self, door_name: &str) -> bool {
        let now = Local::now().naive_local();
        let mut last_alert_time = self.last_alert_time.lock().unwrap();
        let last_alert = last_alert_time.get(door_name);

        match last_alert {
            Some(time) if now.signed_duration_since(*time) < Duration::seconds(self.config.repeat_interval as i64) => false,
            _ => {
                last_alert_time.insert(door_name.to_string(), now);
                true
            }
        }
    }

    /// Determines if a trailer hostage situation is occurring
    fn is_hostage_situation(&self, dock_door: &DockDoor) -> bool {
        (dock_door.loading_status == LoadingStatus::Completed ||
            dock_door.loading_status == LoadingStatus::WaitingForExit) &&
            dock_door.trailer_state == TrailerState::Docked &&
            dock_door.manual_mode == ManualMode::Enabled
    }

    /// Generates alert and log entry for a trailer hostage situation
    fn generate_hostage_results(&self, dock_door: &DockDoor, duration: Duration) -> Vec<AnalysisResult> {
        let mut results = Vec::new();

        results.push(AnalysisResult::Alert(AlertType::TrailerHostage {
            door_name: dock_door.dock_name.clone(),
            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
            duration,
        }));

        let log_entry = LogEntry::TrailerHostage {
            log_dttm: Local::now().naive_local(),
            plant: dock_door.plant_id.clone(),
            door_name: dock_door.dock_name.clone(),
            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
            event_type: "TRAILER_HOSTAGE".to_string(),
            success: false,
            notes: format!("Trailer hostage situation detected. Duration: {:?}", duration),
            severity: 2,
            previous_state: None,
            previous_state_dttm: None,
        };
        results.push(AnalysisResult::Log(log_entry));

        results
    }
}

impl AnalysisRule for TrailerHostageRule {
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "RH_MANUAL_MODE" => {
                if self.is_hostage_situation(dock_door) {
                    let duration = dock_door.trailer_state_changed
                        .map(|t| Local::now().naive_local().signed_duration_since(t))
                        .unwrap_or_else(|| Duration::seconds(0));

                    if duration > Duration::seconds(self.config.alert_threshold as i64) &&
                        self.should_send_alert(&dock_door.dock_name) {
                        debug!("Trailer hostage situation detected for door: {}", dock_door.dock_name);
                        self.generate_hostage_results(dock_door, duration)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            },
            DockDoorEvent::LoadingStatusChanged(_) | DockDoorEvent::TrailerStateChanged(_) => {
                // Check for hostage situation on these events as well
                if self.is_hostage_situation(dock_door) {
                    let duration = dock_door.trailer_state_changed
                        .map(|t| Local::now().naive_local().signed_duration_since(t))
                        .unwrap_or_else(|| Duration::seconds(0));

                    if duration > Duration::seconds(self.config.alert_threshold as i64) &&
                        self.should_send_alert(&dock_door.dock_name) {
                        debug!("Trailer hostage situation detected for door: {}", dock_door.dock_name);
                        self.generate_hostage_results(dock_door, duration)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            },
            _ => Vec::new()
        }
    }
}