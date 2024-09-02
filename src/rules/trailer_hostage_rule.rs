use std::collections::HashMap;
use chrono::{NaiveDateTime, Local, Duration, Utc, TimeZone};
use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType};
use crate::models::{DockDoor, DockDoorEvent, LoadingStatus, TrailerState, ManualMode};

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
    last_alert_time: HashMap<String, NaiveDateTime>,
}

impl TrailerHostageRule {
    /// Creates a new TrailerHostageRule with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The JSON configuration containing the rule parameters
    ///
    /// # Returns
    ///
    /// A new instance of TrailerHostageRule
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: TrailerHostageRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse TrailerHostageRule configuration");
        Self {
            config: parsed_config,
            last_alert_time: HashMap::new(),
        }
    }

    /// Checks if an alert should be sent based on the last alert time and repeat interval
    ///
    /// # Arguments
    ///
    /// * `door_name` - The name of the dock door
    ///
    /// # Returns
    ///
    /// A boolean indicating whether an alert should be sent
    fn should_send_alert(&self, door_name: &str) -> bool {
        let now = Local::now().naive_local();
        let last_alert = self.last_alert_time.get(door_name);

        match last_alert {
            Some(time) if now.signed_duration_since(*time) < Duration::seconds(self.config.repeat_interval as i64) => false,
            _ => {
                self.last_alert_time.clone().insert(door_name.to_string(), now);
                true
            }
        }
    }

    /// Determines if a trailer hostage situation is occurring
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A boolean indicating whether a trailer hostage situation is occurring
    fn is_hostage_situation(&self, dock_door: &DockDoor) -> bool {
        (dock_door.loading_status == LoadingStatus::Completed ||
            dock_door.loading_status == LoadingStatus::WaitingForExit) &&
            dock_door.trailer_state == TrailerState::Docked &&
            dock_door.manual_mode == ManualMode::Enabled
    }
}

impl AnalysisRule for TrailerHostageRule {
    /// Applies the TrailerHostageRule to the given dock door and event
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to which the rule is being applied
    /// * `event` - The DockDoorEvent being processed
    ///
    /// # Returns
    ///
    /// A vector of AnalysisResult, which may contain alerts if the rule conditions are met
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let mut results = Vec::new();

        // Check for hostage situation on relevant events
        match event {
            DockDoorEvent::SensorStateChanged(e) => {
                if let DockDoorEvent::SensorStateChanged(_) = event {
                    if e.sensor_name != "RH_MANUAL_MODE" {
                        return vec![];
                    }
                }

                if self.is_hostage_situation(dock_door) {
                    let duration = dock_door.trailer_state_changed
                        .map(|t| Local::now().signed_duration_since(Utc.from_utc_datetime(&t)))
                        .unwrap_or_else(|| Duration::seconds(0));

                    if duration > Duration::seconds(self.config.alert_threshold as i64) &&
                        self.should_send_alert(&dock_door.dock_name) {
                        results.push(AnalysisResult::Alert(AlertType::TrailerHostage {
                            door_name: dock_door.dock_name.clone(),
                            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                            duration,
                        }));
                    }
                }
            },
            _ => {}
        }

        results
    }
}