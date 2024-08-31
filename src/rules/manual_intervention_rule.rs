use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{Local, NaiveDateTime};
use derive_more::Constructor;
use crate::models::{DockDoor, DockDoorEvent};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use serde::{Deserialize, Serialize};

/// Configuration for the `ManualInterventionRule`
#[derive(Debug, Deserialize, Serialize, Constructor)]
pub struct ManualInterventionRuleConfig {
    /// The interval (in checks) at which to evaluate monitored doors for manual intervention timeouts
    pub check_interval: u32,
    /// The maximum number of checks allowed before a manual intervention timeout alert is triggered
    pub max_checks: u32,
}

/// Type alias for the monitoring data structure used by the `ManualInterventionRule`
type MonitoringData = Arc<Mutex<HashMap<String, (NaiveDateTime, String, u32)>>>;

/// An analysis rule that monitors and handles manual interventions on dock doors
pub struct ManualInterventionRule {
    /// The configuration for this rule
    config: ManualInterventionRuleConfig,
    /// Stores data about ongoing manual interventions for each dock door
    monitoring: MonitoringData,
}

impl ManualInterventionRule {
    /// Creates a new `ManualInterventionRule` with the given configuration
    pub fn new(config: ManualInterventionRuleConfig) -> Self {
        ManualInterventionRule {
            config,
            monitoring: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts monitoring a dock door for manual intervention
    ///
    /// Adds an entry to the `monitoring` map with the dock name, current timestamp, shipment ID, and an initial check count of 0
    fn start_monitoring(&self, dock_name: String, shipment_id: String) {
        let mut monitoring = self.monitoring.lock().unwrap();
        monitoring.insert(dock_name, (Local::now().naive_local(), shipment_id, 0));
    }

    /// Stops monitoring a dock door for manual intervention
    ///
    /// Removes the entry for the given dock name from the `monitoring` map and returns the associated data if found
    fn stop_monitoring(&self, dock_name: &str) -> Option<(NaiveDateTime, String, u32)> {
        let mut monitoring = self.monitoring.lock().unwrap();
        monitoring.remove(dock_name)
    }

    /// Periodically checks the status of monitored doors for manual intervention timeouts
    ///
    /// Iterates through the `monitoring` map and checks if the "RH_DOCK_READY" sensor is active for each door
    /// If so, it generates a success log entry and stops monitoring the door
    /// If the maximum number of checks is reached, it generates a timeout alert and a failure log entry, then stops monitoring
    ///
    /// # Arguments
    ///
    /// * `dock_doors`: A reference to the map of all `DockDoor` objects
    ///
    /// # Returns
    ///
    /// A vector of `AnalysisResult` containing log entries or alerts generated during the check
    pub async fn check_monitored_doors(&self, dock_doors: &HashMap<String, DockDoor>) -> Vec<AnalysisResult> {
        let mut results = Vec::new();
        let now = Local::now().naive_local();

        let mut monitoring = self.monitoring.lock().unwrap();
        monitoring.retain(|dock_name, (start_time, shipment_id, check_count)| {
            let new_check_count = *check_count + 1;
            if let Some(door) = dock_doors.get(dock_name) {
                if door.sensors.get("RH_DOCK_READY").map_or(false, |s| s.get_sensor_data().current_value == Some(1)) {
                    results.push(AnalysisResult::Log(LogEntry::ManualInterventionSuccess {
                        log_dttm: now,
                        plant: door.plant_id.clone(),
                        door_name: dock_name.clone(),
                        shipment_id: Some(shipment_id.clone()),
                        event_type: "MANUAL_INTERVENTION_SUCCESS".to_string(),
                        success: true,
                        notes: format!("Dock ready after manual intervention, duration: {:?}", now.signed_duration_since(*start_time)),
                        severity: 0,
                        previous_state: None,
                        previous_state_dttm: None,
                    }));
                    return false;
                } else if new_check_count >= self.config.max_checks {
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
                        notes: "Dock not ready after manual intervention timeout".to_string(),
                        severity: 2, // Assuming higher severity for failure
                        previous_state: None,
                        previous_state_dttm: None,
                    }));
                    return false;
                }
            }
            *check_count = new_check_count;
            true
        });

        results
    }
}

impl AnalysisRule for ManualInterventionRule {
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "RH_MANUAL_MODE" => {
                if e.new_value == Some(1) {
                    // Manual mode engaged
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
                } else if e.new_value == Some(0) {
                    // Manual mode disengaged
                    if let Some((start_time, _shipment_id, _)) = self.stop_monitoring(&e.dock_name) {
                        vec![AnalysisResult::Log(LogEntry::ManualInterventionSuccess {
                            log_dttm: e.timestamp,
                            plant: dock_door.plant_id.clone(),
                            door_name: e.dock_name.clone(),
                            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                            event_type: "MANUAL_INTERVENTION_SUCCESS".to_string(),
                            success: true,
                            notes: format!("Manual intervention completed successfully, duration: {:?}", e.timestamp.signed_duration_since(start_time)),
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
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "RH_DOCK_READY" && e.new_value == Some(1) => {
                if let Some((start_time, _shipment_id, _)) = self.stop_monitoring(&e.dock_name) {
                    vec![AnalysisResult::Log(LogEntry::ManualInterventionSuccess {
                        log_dttm: e.timestamp,
                        plant: dock_door.plant_id.clone(),
                        door_name: e.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        event_type: "MANUAL_INTERVENTION_SUCCESS".to_string(),
                        success: true,
                        notes: format!("Dock ready after manual intervention, duration: {:?}", e.timestamp.signed_duration_since(start_time)),
                        severity: 0,
                        previous_state: None,
                        previous_state_dttm: None,
                    })]
                } else {
                    vec![]
                }
            },
            _ => vec![],
        }
    }
}