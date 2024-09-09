use log::info;
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult};
use crate::models::{DockDoor, DockDoorEvent, WmsEventWrapper, DbInsert};

pub struct WmsEventsRule;

impl AnalysisRule for WmsEventsRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        info!("WmsEventsRule for: {:?}", event);
        match event {
            DockDoorEvent::WmsEvent(e) => vec![create_wms_db_insert(door, e)],
            DockDoorEvent::ShipmentStarted(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::ShipmentSuspended(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::ShipmentCancelled(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::ShipmentResumed(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::PriorityUpdated(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::LoadPlanSaved(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::ShipmentForcedClosed(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::LoadQuantityAdjusted(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::DriverCheckedIn(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::TrailerRejected(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::LgvStartLoading(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::FirstDrop(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::ShipmentCheckout(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::TrailerPatternProcessed(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::AppointmentUpdated(e) => vec![create_wms_db_insert(door, &e.base_event)],
            DockDoorEvent::TripProcessed(e) => vec![create_wms_db_insert(door, &e.base_event)],
            _ => vec![],
        }
    }
}

fn create_wms_db_insert(door: &DockDoor, base_event: &WmsEventWrapper) -> AnalysisResult {
    let user_id = if ["STARTED_SHIPMENT", "SUSPENDED_SHIPMENT", "RESUMED_SHIPMENT",
        "UPDATED_PRIORITY", "CANCELLED_SHIPMENT", "SDM_LOAD_PLAN",
        "LOAD_QTY_ADJUSTED", "SDM_CHECK_IN", "SDM_TRAILER_REJECTION"]
        .contains(&base_event.event_type.as_str()) {
        base_event.message_notes
            .as_ref()
            .and_then(|notes| notes.split('-').next())
            .map(|user| user.trim().to_string())
    } else {
        None
    };

    AnalysisResult::DbInsert(DbInsert {
        LOG_DTTM: base_event.timestamp,
        PLANT: door.plant_id.clone(),
        DOOR_NAME: door.dock_name.clone(),
        SHIPMENT_ID: Some(base_event.shipment_id.clone()),
        EVENT_TYPE: base_event.event_type.clone(),
        SUCCESS: if base_event.result_code == 0 { 1 } else { 0 },
        NOTES: base_event.message_notes.clone().unwrap_or_default(),
        ID_USER: user_id,
        SEVERITY: base_event.result_code,
        PREVIOUS_STATE: None,
        PREVIOUS_STATE_DTTM: None
    })
}