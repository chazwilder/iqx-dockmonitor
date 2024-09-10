use chrono::NaiveDateTime;
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult};
use crate::models::{DockDoor, DockDoorEvent, FirstDropEvent};
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

/// `ConsolidatedDataRule` is responsible for generating consolidated events
/// when a shipment is started in the Warehouse Management System (WMS).
/// It aggregates various pieces of information about a dock door and shipment
/// into a single `ConsolidatedDockEvent`.
pub struct ConsolidatedDataRule;

impl ConsolidatedDataRule {
    /// Creates a new instance of `ConsolidatedDataRule`.
    pub fn new() -> Self {
        Self {}
    }

    fn build_consolidated(&self, door: &DockDoor, event: &FirstDropEvent) -> Vec<AnalysisResult> {
        let (docking_time_minutes,inspection_time_minutes,enqueued_time_minutes) = self.calculate_times(door, event);
        vec![AnalysisResult::ConsolidatedEvent(ConsolidatedDockEvent{
            plant: door.plant_id.to_string(),
            door_name: door.dock_name.to_string(),
            shipment_id: door.assigned_shipment.current_shipment.as_ref()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0),
            docking_time_minutes,
            inspection_time_minutes,
            enqueued_time_minutes,
            shipment_assigned: door.assigned_shipment.assignment_dttm,
            dock_assignment: door.consolidated.dock_assignment,
            trailer_docking: door.consolidated.docking_time,
            started_shipment: door.consolidated.shipment_started_dttm,
            lgv_start_loading: Some(event.base_event.timestamp),
            dock_ready: door.consolidated.last_dock_ready_time,
            is_preload: door.consolidated.is_preload,
        })]
    }

    fn calculate_times(&self, door: &DockDoor, event: &FirstDropEvent) -> (Option<i32>, Option<i32>, Option<i32>) {
        let mut docking_time_minutes: Option<i32> = None;
        let mut inspection_time_minutes: Option<i32> = None;
        let mut enqueued_time_minutes: Option<i32> = None;

        if let (Some(dock_assignment), Some(trailer_docking)) = (door.consolidated.dock_assignment.or(door.assigned_shipment.assignment_dttm), door.consolidated.docking_time) {
            docking_time_minutes = Some(ConsolidatedDataRule::calculate_duration_minutes(dock_assignment, trailer_docking));
        }

        if let (Some(trailer_docking), Some(started_shipment)) = (door.consolidated.docking_time.or(door.consolidated.last_dock_ready_time), door.consolidated.shipment_started_dttm) {
            inspection_time_minutes = Some(ConsolidatedDataRule::calculate_duration_minutes(trailer_docking, started_shipment));
        }

        if let (Some(started_shipment), lgv_start_loading) = (door.consolidated.shipment_started_dttm, event.base_event.timestamp) {
            enqueued_time_minutes = Some(ConsolidatedDataRule::calculate_duration_minutes(started_shipment, lgv_start_loading));
        }
        (docking_time_minutes, inspection_time_minutes, enqueued_time_minutes)
    }

    fn calculate_duration_minutes(start: NaiveDateTime, end: NaiveDateTime) -> i32 {
        (end - start).num_minutes() as i32
    }
}

impl AnalysisRule for ConsolidatedDataRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::FirstDrop(e) => {
                self.build_consolidated(door, e)
            },
            _ => Vec::new()
        }
    }
}