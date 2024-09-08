use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{Duration, Local, NaiveDateTime};
use derive_more::Constructor;
use crate::models::{DockDoor, DockDoorEvent, ManualMode};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for the `ManualInterventionRule`
#[derive(Debug, Deserialize, Serialize, Constructor)]
pub struct ManualInterventionRuleConfig {
    /// The interval (in seconds) at which to check monitored doors
    pub check_interval: u32,
    /// Maximum duration (in seconds) allowed for manual intervention before it's considered a failure
    pub max_duration: u64,
}

/// Type alias for the monitoring data structure used by the `ManualInterventionRule`
type MonitoringData = Arc<Mutex<HashMap<String, (NaiveDateTime, String)>>>;

/// An analysis rule that monitors and handles manual interventions on dock doors
pub struct ManualInterventionRule {
    /// The configuration for this rule
    config: ManualInterventionRuleConfig,
    /// Stores data about ongoing manual interventions for each dock door
    monitoring: MonitoringData,
}

impl ManualInterventionRule {
    /// Creates a new `ManualInterventionRule` with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - A JSON Value containing the rule configuration
    ///
    /// # Returns
    ///
    /// A new instance of `ManualInterventionRule`
    pub fn new(config: Value) -> Self {
        let parsed_config: ManualInterventionRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse ManualInterventionRule configuration");
        ManualInterventionRule {
            config: parsed_config,
            monitoring: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts monitoring a dock door for manual intervention
    ///
    /// # Arguments
    ///
    /// * `dock_name` - The name of the dock door to monitor
    /// * `shipment_id` - The ID of the shipment associated with the manual intervention
    fn start_monitoring(&self, dock_name: String, shipment_id: String) {
        let mut monitoring = self.monitoring.lock().unwrap();
        monitoring.insert(dock_name, (Local::now().naive_local(), shipment_id));
    }

    /// Stops monitoring a dock door for manual intervention
    ///
    /// # Arguments
    ///
    /// * `dock_name` - The name of the dock door to stop monitoring
    ///
    /// # Returns
    ///
    /// An Option containing the start time and shipment ID if the door was being monitored, None otherwise
    fn stop_monitoring(&self, dock_name: &str) -> Option<(NaiveDateTime, String)> {
        let mut monitoring = self.monitoring.lock().unwrap();
        monitoring.remove(dock_name)
    }

    /// Checks the status of all monitored doors for manual intervention timeouts
    ///
    /// # Arguments
    ///
    /// * `dock_doors` - A reference to the map of all `DockDoor` objects
    ///
    /// # Returns
    ///
    /// A vector of `AnalysisResult` containing logs and alerts generated during the check
    pub async fn check_monitored_doors(&self, dock_doors: &HashMap<String, DockDoor>) -> Vec<AnalysisResult> {
        let mut results = Vec::new();
        let now = Local::now().naive_local();

        let mut monitoring = self.monitoring.lock().unwrap();
        monitoring.retain(|dock_name, (start_time, shipment_id)| {
            if let Some(door) = dock_doors.get(dock_name) {
                if door.assigned_shipment.current_shipment.is_some() {
                    let duration = now.signed_duration_since(*start_time);
                    if door.manual_mode == ManualMode::Disabled {
                        results.push(AnalysisResult::Log(LogEntry::ManualInterventionSuccess {
                            log_dttm: now,
                            plant: door.plant_id.clone(),
                            door_name: dock_name.clone(),
                            shipment_id: Some(shipment_id.clone()),
                            event_type: "MANUAL_INTERVENTION_SUCCESS".to_string(),
                            success: true,
                            notes: format!("Manual intervention completed, duration: {:?}", duration),
                            severity: 0,
                            previous_state: None,
                            previous_state_dttm: None,
                        }));
                        false
                    } else if duration > Duration::seconds(self.config.max_duration as i64) {
                        results.push(AnalysisResult::Alert(AlertType::ManualInterventionTimeout {
                            dock_name: dock_name.clone(),
                            shipment_id: shipment_id.clone(),
                            start_time: *start_time,
                            end_time: now,
                        }));
                        results.push(AnalysisResult::Log(LogEntry::ManualInterventionFailure {
                            log_dttm: now,
                            plant: door.plant_id.clone(),
                            door_name: dock_name.clone(),
                            shipment_id: Some(shipment_id.clone()),
                            event_type: "MANUAL_INTERVENTION_FAILURE".to_string(),
                            success: false,
                            notes: format!("Manual intervention timeout after {:?}", duration),
                            severity: 2,
                            previous_state: None,
                            previous_state_dttm: None,
                        }));
                        false
                    } else {
                        true
                    }
                } else {
                    false // Remove from monitoring if there's no assigned shipment
                }
            } else {
                false
            }
        });

        results
    }
}

impl AnalysisRule for ManualInterventionRule {
    /// Applies the rule to a dock door event, generating appropriate analysis results
    ///
    /// This method analyzes the given event and generates relevant logs and alerts
    /// based on manual intervention events.
    ///
    /// # Arguments
    ///
    /// * `dock_door` - A reference to the `DockDoor` associated with the event
    /// * `event` - A reference to the `DockDoorEvent` to analyze
    ///
    /// # Returns
    ///
    /// A vector of `AnalysisResult` containing logs and alerts generated based on the event
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "RH_MANUAL_MODE" => {
                if e.new_value == Some(1) && e.old_value == Some(0) && dock_door.assigned_shipment.current_shipment.is_some() {
                    self.start_monitoring(e.dock_name.clone(), dock_door.assigned_shipment.current_shipment.clone().unwrap_or_default());
                    vec![AnalysisResult::Log(LogEntry::ManualInterventionStarted {
                        log_dttm: e.timestamp,
                        plant: dock_door.plant_id.clone(),
                        door_name: e.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        event_type: "MANUAL_INTERVENTION_STARTED".to_string(),
                        success: true,
                        notes: "Manual mode engaged".to_string(),
                        severity: 0,
                        previous_state: None,
                        previous_state_dttm: None,
                    })]
                } else if e.new_value == Some(0) && e.old_value == Some(1) {
                    if let Some((start_time, _shipment_id)) = self.stop_monitoring(&e.dock_name) {
                        vec![AnalysisResult::Log(LogEntry::ManualInterventionSuccess {
                            log_dttm: e.timestamp,
                            plant: dock_door.plant_id.clone(),
                            door_name: e.dock_name.clone(),
                            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                            event_type: "MANUAL_INTERVENTION_SUCCESS".to_string(),
                            success: true,
                            notes: format!("Manual intervention completed, duration: {:?}", e.timestamp.signed_duration_since(start_time)),
                            severity: 0,
                            previous_state: None,
                            previous_state_dttm: None,
                        })]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            },
            _ => vec![],
        }
    }
}