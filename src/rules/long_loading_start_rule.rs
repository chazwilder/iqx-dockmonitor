use std::collections::HashMap;
use chrono::{NaiveDateTime, Local, Duration, Utc, TimeZone};
use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType};
use crate::models::{DockDoor, DockDoorEvent, LoadingStatus};

/// Configuration for the LongLoadingStartRule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongLoadingStartRuleConfig {
    /// The time threshold (in seconds) after which an alert should be triggered
    pub alert_threshold: u64,
    /// The interval (in seconds) at which repeat alerts should be sent
    pub repeat_interval: u64,
}

/// Rule for detecting and alerting on long loading start times
pub struct LongLoadingStartRule {
    /// The parsed configuration for this rule
    config: LongLoadingStartRuleConfig,
    /// A map to track the last alert time for each door
    last_alert_time: HashMap<String, NaiveDateTime>,
}

impl LongLoadingStartRule {
    /// Creates a new LongLoadingStartRule with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The JSON configuration containing the rule parameters
    ///
    /// # Returns
    ///
    /// A new instance of LongLoadingStartRule
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: LongLoadingStartRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse LongLoadingStartRule configuration");
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
}

impl AnalysisRule for LongLoadingStartRule {
    /// Applies the LongLoadingStartRule to the given dock door and event
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

        match event {
            DockDoorEvent::LoadingStatusChanged(e) if e.new_status == LoadingStatus::Loading => {
                let loading_duration = Local::now().signed_duration_since(Utc.from_utc_datetime(&e.timestamp));
                if loading_duration > Duration::seconds(self.config.alert_threshold as i64) {
                    // Check if the loading progress is still 0%
                    if dock_door.wms_shipment_status == Some("Started".to_string()) &&
                        dock_door.loading_status == LoadingStatus::Loading {
                        if self.should_send_alert(&dock_door.dock_name) {
                            results.push(AnalysisResult::Alert(AlertType::LongLoadingStart {
                                door_name: dock_door.dock_name.clone(),
                                shipment_id: dock_door.assigned_shipment.current_shipment.clone().unwrap_or_default(),
                                duration: loading_duration,
                            }));
                        }
                    }
                }
            },
            DockDoorEvent::WmsEvent(e) if e.event_type == "STARTED_SHIPMENT" => {
                let loading_duration = Local::now().signed_duration_since(Utc.from_utc_datetime(&e.timestamp));
                if loading_duration > Duration::seconds(self.config.alert_threshold as i64) {
                    if dock_door.loading_status == LoadingStatus::Loading {
                        if self.should_send_alert(&dock_door.dock_name) {
                            results.push(AnalysisResult::Alert(AlertType::LongLoadingStart {
                                door_name: dock_door.dock_name.clone(),
                                shipment_id: e.shipment_id.clone(),
                                duration: loading_duration,
                            }));
                        }
                    }
                }
            },
            _ => {}
        }

        results
    }
}