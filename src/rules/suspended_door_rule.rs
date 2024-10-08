    use std::collections::HashMap;
    use std::sync::Mutex;
    use chrono::{NaiveDateTime, Local, Duration};
    use serde::{Deserialize, Serialize};
    use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
    use crate::models::{DockDoor, DockDoorEvent, LoadingStatus};
    use log::{debug, info};

    /// Configuration for the SuspendedDoorRule
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SuspendedDoorRuleConfig {
        /// Time threshold (in seconds) after which to generate an alert
        pub alert_threshold: u64,
        /// Interval (in seconds) between repeated alerts
        pub repeat_interval: u64,
    }

    /// Rule for detecting and alerting on suspended doors
    pub struct SuspendedDoorRule {
        config: SuspendedDoorRuleConfig,
        last_alert_time: Mutex<HashMap<String, NaiveDateTime>>,
    }

    impl SuspendedDoorRule {
        /// Creates a new SuspendedDoorRule with the given configuration
        ///
        /// # Arguments
        ///
        /// * `config` - JSON configuration for the rule
        ///
        /// # Returns
        ///
        /// A new instance of SuspendedDoorRule
        pub fn new(config: serde_json::Value) -> Self {
            let parsed_config: SuspendedDoorRuleConfig = serde_json::from_value(config)
                .expect("Failed to parse SuspendedDoorRule configuration");
            Self {
                config: parsed_config,
                last_alert_time: Mutex::new(HashMap::new()),
            }
        }

        /// Determines if an alert should be sent based on the last alert time
        ///
        /// # Arguments
        ///
        /// * `door_name` - The name of the door
        ///
        /// # Returns
        ///
        /// A boolean indicating whether an alert should be sent
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

        /// Formats a duration into a human-readable string
        ///
        /// # Arguments
        ///
        /// * `duration` - The duration to format
        ///
        /// # Returns
        ///
        /// A formatted duration string
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

        /// Generates alert and log entry for a suspended door
        ///
        /// # Arguments
        ///
        /// * `dock_door` - The DockDoor that is suspended
        /// * `duration` - The duration of the suspension
        /// * `user` - The user who suspended the door
        /// * `timestamp` - The timestamp of the suspension
        ///
        /// # Returns
        ///
        /// A vector of AnalysisResult items
        fn generate_suspended_door_results(&self, dock_door: &DockDoor, duration: Duration, user: String, timestamp: NaiveDateTime) -> Vec<AnalysisResult> {
            let mut results = Vec::new();

            results.push(AnalysisResult::Alert(AlertType::SuspendedDoor {
                door_name: dock_door.dock_name.clone(),
                duration,
                shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                user: user.clone(),
            }));

            let log_entry = LogEntry::SuspendedDoor {
                log_dttm: Local::now().naive_local(),
                plant: dock_door.plant_id.clone(),
                door_name: dock_door.dock_name.clone(),
                shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                event_type: "SUSPENDED_DOOR".to_string(),
                success: false,
                notes: format!("Door suspended for {} by user {}", self.format_duration(&duration), user),
                severity: 2,
                previous_state: None,
                previous_state_dttm: Some(timestamp),
            };
            results.push(AnalysisResult::Log(log_entry));

            results
        }
    }

    impl AnalysisRule for SuspendedDoorRule {
        /// Applies the rule to a dock door event, generating appropriate analysis results
        ///
        /// # Arguments
        ///
        /// * `dock_door` - The DockDoor associated with the event
        /// * `event` - The DockDoorEvent to analyze
        ///
        /// # Returns
        ///
        /// A vector of AnalysisResult items generated by applying the rule
        fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
            info!("SuspendedDoorRule applying to event: {:?}", event);
            match event {
                DockDoorEvent::ShipmentSuspended(e) => {
                    if dock_door.loading_status.loading_status != LoadingStatus::Suspended {
                        debug!("Ignoring ShipmentSuspended event as door is not currently suspended");
                        return Vec::new();
                    }

                    if self.should_send_alert(&dock_door.dock_name) {
                        let duration = e.base_event.timestamp.signed_duration_since(
                            dock_door.assigned_shipment.assignment_dttm.unwrap_or(e.base_event.timestamp)
                        );
                        let user = e.base_event.message_notes
                            .as_ref()
                            .and_then(|notes| notes.split('-').next())
                            .map(|user| user.trim().to_string())
                            .unwrap_or_else(|| "Unknown".to_string());

                        debug!("Extracted user name for suspended door alert: {}", user);

                        self.generate_suspended_door_results(dock_door, duration, user, e.base_event.timestamp)
                    } else {
                        Vec::new()
                    }
                },
                _ => Vec::new()
            }
        }
    }