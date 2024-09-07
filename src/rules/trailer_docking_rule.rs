use crate::models::{DockDoor, DockDoorEvent, DoorState, TrailerState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry, AlertType};
use chrono::Local;
use tracing::{info, debug};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for the TrailerDockingRule
#[derive(Debug, Deserialize, Serialize)]
pub struct TrailerDockingRuleConfig {
    /// The loading status that is considered invalid for docking
    pub invalid_loading_status: String,
    /// The WMS shipment status that is considered invalid for docking
    pub invalid_wms_shipment_status: String,
    /// Sensors to monitor during the docking process
    pub sensors_to_monitor: Vec<SensorConfig>,
}

/// Configuration for a sensor to monitor
#[derive(Debug, Deserialize, Serialize)]
pub struct SensorConfig {
    /// The name of the sensor
    pub name: String,
    /// The value that indicates a successful state for this sensor
    pub success_value: u8,
}

/// Rule for analyzing trailer docking events
pub struct TrailerDockingRule {
    config: TrailerDockingRuleConfig,
}

impl TrailerDockingRule {
    /// Creates a new TrailerDockingRule with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - JSON configuration for the rule
    ///
    /// # Returns
    ///
    /// A new instance of TrailerDockingRule
    pub fn new(config: Value) -> Self {
        let parsed_config: TrailerDockingRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse TrailerDockingRule configuration");
        TrailerDockingRule { config: parsed_config }
    }

    /// Checks if the docking is successful based on loading status, WMS shipment status, and sensor values
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the docking is successful
    fn is_docking_successful(&self, dock_door: &DockDoor) -> bool {
        let loading_status_condition = self.check_loading_status(dock_door);
        let wms_status_condition = self.check_wms_status(dock_door);
        let shipment_condition = dock_door.assigned_shipment.current_shipment.is_some();
        let sensor_condition = self.check_sensors(dock_door);

        debug!(
            "DockDoor: {} - Docking conditions: loading_status={}, wms_status={}, shipment={}, sensors={}",
            dock_door.dock_name, loading_status_condition, wms_status_condition, shipment_condition, sensor_condition
        );

        loading_status_condition && wms_status_condition && shipment_condition && sensor_condition
    }

    /// Checks if the loading status is valid for successful docking
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the loading status is valid
    fn check_loading_status(&self, dock_door: &DockDoor) -> bool {
        dock_door.loading_status.to_string() != self.config.invalid_loading_status
    }

    /// Checks if the WMS shipment status is valid for successful docking
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the WMS shipment status is valid
    fn check_wms_status(&self, dock_door: &DockDoor) -> bool {
        dock_door.wms_shipment_status
            .as_ref()
            .map(|status| status != &self.config.invalid_wms_shipment_status)
            .unwrap_or(false)
    }

    /// Checks if all monitored sensors are in their success state
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A boolean indicating whether all sensors are in their success state
    fn check_sensors(&self, dock_door: &DockDoor) -> bool {
        self.config.sensors_to_monitor.iter().all(|sensor| {
            dock_door.sensors.get(&sensor.name)
                .and_then(|s| s.get_sensor_data().current_value)
                .map(|value| value == sensor.success_value)
                .unwrap_or(false)
        })
    }

    /// Gets the reason for a docking failure
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A string describing the reason for the docking failure
    fn get_failure_reason(&self, dock_door: &DockDoor) -> String {
        let mut reasons = Vec::new();

        if !self.check_loading_status(dock_door) {
            reasons.push("Invalid loading status");
        }
        if !self.check_wms_status(dock_door) {
            reasons.push("Invalid WMS shipment status");
        }
        if dock_door.assigned_shipment.current_shipment.is_none() {
            reasons.push("No shipment assigned");
        }
        if !self.check_sensors(dock_door) {
            reasons.push("Sensor conditions not met");
        }

        if reasons.is_empty() {
            "Unknown docking failure".to_string()
        } else {
            reasons.join(", ")
        }
    }
}

impl AnalysisRule for TrailerDockingRule {
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
        let mut results = Vec::new();
        match event {
            DockDoorEvent::TrailerStateChanged(e) => {
                if e.new_state == TrailerState::Docked && e.old_state == TrailerState::Undocked {
                    let is_successful = self.is_docking_successful(dock_door);
                    let failure_reason = if !is_successful {
                        Some(self.get_failure_reason(dock_door))
                    } else {
                        None
                    };

                    results.push(AnalysisResult::Alert(AlertType::TrailerDocked {
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        timestamp: e.timestamp,
                        success: is_successful,
                        failure_reason,
                    }));

                    let log_entry = LogEntry::DockingTime {
                        log_dttm: Local::now().naive_local(),
                        plant: dock_door.plant_id.clone(),
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        event_type: "TRAILER_DOCKING".to_string(),
                        success: is_successful,
                        notes: if is_successful {
                            "Trailer docked successfully".to_string()
                        } else {
                            format!("Trailer docking failed: {}", self.get_failure_reason(dock_door))
                        },
                        severity: if is_successful { 0 } else { 2 },
                        previous_state: Some(format!("{:?}", e.old_state)),
                        previous_state_dttm: Some(e.timestamp),
                    };

                    info!("TrailerDockingRule: Generated docking log entry: {:?}", log_entry);
                    results.push(AnalysisResult::Log(log_entry));
                }
            },
            DockDoorEvent::DoorStateChanged(e) => {
                if e.new_state == DoorState::DoorReady && self.is_docking_successful(dock_door) {
                    let log_entry = LogEntry::DockingTime {
                        log_dttm: Local::now().naive_local(),
                        plant: dock_door.plant_id.clone(),
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        event_type: "DOCK_READY".to_string(),
                        success: true,
                        notes: "Dock ready, docking process completed successfully".to_string(),
                        severity: 0,
                        previous_state: Some(format!("{:?}", e.old_state)),
                        previous_state_dttm: Some(e.timestamp),
                    };
                    info!("TrailerDockingRule: Generated successful docking log entry: {:?}", log_entry);
                    results.push(AnalysisResult::Log(log_entry));

                    results.push(AnalysisResult::Alert(AlertType::DockReady {
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        timestamp: e.timestamp,
                    }));
                }
            },
            _ => {},
        }
        results
    }
}