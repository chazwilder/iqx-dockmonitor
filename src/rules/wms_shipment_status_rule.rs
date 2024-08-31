use crate::models::{DockDoor, DockDoorEvent};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry};

/// An analysis rule that handles events related to WMS shipment status and loading status changes
pub struct WmsShipmentStatus;

impl AnalysisRule for WmsShipmentStatus {
    /// Applies the rule to a dock door event, generating log entries for shipment assignment, unassignment, and loading status changes
    ///
    /// The method matches the event type and creates corresponding log entries:
    /// - `DockDoorEvent::ShipmentAssigned`: Creates a `LogEntry::ShipmentAssigned` entry
    /// - `DockDoorEvent::ShipmentUnassigned`: Creates a `LogEntry::ShipmentUnassigned` entry
    /// - `DockDoorEvent::LoadingStatusChanged`: Creates a `LogEntry::LoadingStatusChange` entry
    /// For other event types, it returns an empty vector
    ///
    /// # Arguments
    ///
    /// * `door`: A reference to the `DockDoor` object the event is associated with
    /// * `event`: A reference to the `DockDoorEvent` to be analyzed
    ///
    /// # Returns
    ///
    /// A vector containing a `LogEntry` wrapped in an `AnalysisResult` if the event is relevant, otherwise an empty vector
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::ShipmentAssigned(e) => {
                vec![
                    AnalysisResult::Log(LogEntry::ShipmentAssigned {
                        log_dttm: e.timestamp,
                        plant: door.plant_id.clone(),
                        door_name: e.dock_name.clone(),
                        shipment_id: Some(e.shipment_id.clone()),
                        event_type: "SHIPMENT_ASSIGNED".to_string(),
                        success: true,
                        notes: format!("New shipment assigned: {}", e.shipment_id),
                        severity: 0,
                        previous_state: e.previous_shipment.clone(),
                        previous_state_dttm: None,
                    }),
                ]
            },
            DockDoorEvent::ShipmentUnassigned(e) => {
                vec![
                    AnalysisResult::Log(LogEntry::ShipmentUnassigned {
                        log_dttm: e.timestamp,
                        plant: door.plant_id.clone(),
                        door_name: e.dock_name.clone(),
                        shipment_id: Some(e.shipment_id.clone()),
                        event_type: "SHIPMENT_UNASSIGNED".to_string(),
                        success: true,
                        notes: format!("Shipment unassigned: {}", e.shipment_id),
                        severity: 0,
                        previous_state: None,
                        previous_state_dttm: None,
                    }),
                ]
            },
            DockDoorEvent::LoadingStatusChanged(e) => {
                vec![
                    AnalysisResult::Log(LogEntry::LoadingStatusChange {
                        log_dttm: e.timestamp,
                        plant: door.plant_id.clone(),
                        door_name: e.dock_name.clone(),
                        shipment_id: door.assigned_shipment.current_shipment.clone(),
                        event_type: "LOADING_STATUS_CHANGED".to_string(),
                        success: true,
                        notes: format!("Loading status changed from {:?} to {:?}", e.old_status, e.new_status),
                        severity: 0,
                        previous_state: Some(format!("{:?}", e.old_status)),
                        previous_state_dttm: None,
                    }),
                ]
            },
            _ => vec![],
        }
    }
}