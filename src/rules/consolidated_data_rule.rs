use std::sync::Arc;
use dashmap::DashMap;
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult};
use crate::models::{DockDoor, DockDoorEvent};
use crate::errors::DockManagerError;
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

/// `ConsolidatedDataRule` is responsible for generating consolidated events
/// when a shipment is started in the Warehouse Management System (WMS).
/// It aggregates various pieces of information about a dock door and shipment
/// into a single `ConsolidatedDockEvent`.
pub struct ConsolidatedDataRule {
    /// Stores the consolidated events, keyed by a tuple of (plant_id, door_name, shipment_id)
    consolidated_events: Arc<DashMap<(String, String, i32), ConsolidatedDockEvent>>,
}

impl ConsolidatedDataRule {
    /// Creates a new instance of `ConsolidatedDataRule`.
    pub fn new() -> Self {
        Self {
            consolidated_events: Arc::new(DashMap::new()),
        }
    }

    /// Updates or creates a consolidated event based on the provided dock door and event.
    fn update_consolidated_event(&self, door: &DockDoor, event: &DockDoorEvent) -> Result<Option<ConsolidatedDockEvent>, DockManagerError> {
        match event {
            DockDoorEvent::WmsEvent(e) if e.event_type == "STARTED_SHIPMENT" => {
                let shipment_id = e.shipment_id.parse::<i32>().unwrap_or(0);
                let key = (door.plant_id.clone(), door.dock_name.clone(), shipment_id);

                let mut entry = self.consolidated_events.entry(key.clone()).or_insert_with(|| ConsolidatedDockEvent {
                    plant: door.plant_id.clone(),
                    door_name: door.dock_name.clone(),
                    shipment_id,
                    docking_time_minutes: None,
                    inspection_time_minutes: None,
                    enqueued_time_minutes: None,
                    shipment_assigned: door.assigned_shipment.assignment_dttm,
                    dock_assignment: None, // This might be available in door.assigned_shipment if implemented
                    trailer_docking: door.docking_time,
                    started_shipment: Some(e.timestamp),
                    lgv_start_loading: None, // This will be updated later
                    dock_ready: door.last_dock_ready_time,
                    is_preload: door.is_preload,
                });

                self.calculate_durations(&mut entry);

                log::info!("Created consolidated event for shipment {}: {:?}", shipment_id, entry);

                Ok(Some(entry.clone()))
            },
            _ => Ok(None),
        }
    }

    /// Calculates the duration-based fields of a `ConsolidatedDockEvent`.
    fn calculate_durations(&self, event: &mut ConsolidatedDockEvent) {
        if let (Some(shipment_assigned), Some(trailer_docking)) = (event.shipment_assigned, event.trailer_docking) {
            event.docking_time_minutes = Some((trailer_docking - shipment_assigned).num_minutes() as i32);
        }

        if let (Some(trailer_docking), Some(started_shipment)) = (event.trailer_docking, event.started_shipment) {
            event.inspection_time_minutes = Some((started_shipment - trailer_docking).num_minutes() as i32);
        }

        // Note: We can't calculate enqueued_time_minutes here as we don't have lgv_start_loading yet
    }
}

impl AnalysisRule for ConsolidatedDataRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match self.update_consolidated_event(door, event) {
            Ok(Some(consolidated_event)) => {
                vec![AnalysisResult::ConsolidatedEvent(consolidated_event)]
            },
            Ok(None) => vec![],
            Err(e) => {
                log::error!("Error updating consolidated event: {:?}", e);
                vec![]
            }
        }
    }
}