use log::info;
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry};
use crate::models::{DockDoor, DockDoorEvent, WmsEventWrapper};

pub struct WmsEventsRule;

impl AnalysisRule for WmsEventsRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        info!("WmsEventsRule for: {:?}", event);
        match event {
            DockDoorEvent::WmsEvent(e) => vec![create_wms_log_entry(door, e)],
            DockDoorEvent::ShipmentStarted(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::ShipmentSuspended(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::ShipmentCancelled(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::ShipmentResumed(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::PriorityUpdated(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::LoadPlanSaved(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::ShipmentForcedClosed(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::LoadQuantityAdjusted(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::DriverCheckedIn(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::TrailerRejected(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::LgvStartLoading(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::FirstDrop(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::ShipmentCheckout(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::TrailerPatternProcessed(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::AppointmentUpdated(e) => vec![create_wms_log_entry(door, &e.base_event)],
            DockDoorEvent::TripProcessed(e) => vec![create_wms_log_entry(door, &e.base_event)],
            _ => vec![],
        }
    }
}

fn create_wms_log_entry(door: &DockDoor, base_event: &WmsEventWrapper) -> AnalysisResult {
    AnalysisResult::Log(LogEntry::WmsEvent {
        log_dttm: base_event.timestamp,
        plant: door.plant_id.clone(),
        door_name: door.dock_name.clone(),
        shipment_id: Some(base_event.shipment_id.clone()),
        event_type: base_event.event_type.clone(),
        success: base_event.result_code == 0,
        notes: base_event.message_notes.clone().unwrap_or_default(),
        severity: if base_event.result_code == 0 { 0 } else { 1 },
        previous_state: None,
        previous_state_dttm: None,
        message_source: base_event.message_source.clone(),
        result_code: base_event.result_code,
    })
}