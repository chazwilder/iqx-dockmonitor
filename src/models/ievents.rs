use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use crate::models::istates::DoorState;
use crate::models::istatus::LoadingStatus;
use crate::models::{DbInsert, TrailerState, WmsEvent};

/// Represents the different types of events that can occur at a dock door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DockDoorEvent {
    DockAssigned(DockAssignedEvent),
    DockUnassigned(DockUnassignedEvent),
    TrailerDocked(TrailerDockedEvent),
    TrailerDeparted(TrailerDepartedEvent),
    LoadingStarted(LoadingStartedEvent),
    LoadingCompleted(LoadingCompletedEvent),
    SensorStateChanged(SensorStateChangedEvent),
    DoorStateChanged(DoorStateChangedEvent),
    LoadingStatusChanged(LoadingStatusChangedEvent),
    TrailerStateChanged(TrailerStateChangedEvent),
    ShipmentAssigned(ShipmentAssignedEvent),
    ShipmentUnassigned(ShipmentUnassignedEvent),
    WmsEvent(WmsEventWrapper),
    ShipmentStarted(ShipmentStartedEvent),
    ShipmentSuspended(ShipmentSuspendedEvent),
    ShipmentCancelled(ShipmentCancelledEvent),
    ShipmentResumed(ShipmentResumedEvent),
    PriorityUpdated(PriorityUpdatedEvent),
    LoadPlanSaved(LoadPlanSavedEvent),
    ShipmentForcedClosed(ShipmentForcedClosedEvent),
    LoadQuantityAdjusted(LoadQuantityAdjustedEvent),
    DriverCheckedIn(DriverCheckedInEvent),
    TrailerRejected(TrailerRejectedEvent),
    LgvStartLoading(LgvStartLoadingEvent),
    FirstDrop(FirstDropEvent),
    ShipmentCheckout(ShipmentCheckoutEvent),
    TrailerPatternProcessed(TrailerPatternProcessedEvent),
    AppointmentUpdated(AppointmentUpdatedEvent),
    TripProcessed(TripProcessedEvent),
    UnknownWmsEvent(UnknownWmsEventEvent),
}

impl DockDoorEvent {
    pub fn get_dock_name(&self) -> &str {
        match self {
            DockDoorEvent::DockAssigned(e) => &e.dock_name,
            DockDoorEvent::DockUnassigned(e) => &e.dock_name,
            DockDoorEvent::TrailerDocked(e) => &e.dock_name,
            DockDoorEvent::LoadingStarted(e) => &e.dock_name,
            DockDoorEvent::LoadingCompleted(e) => &e.dock_name,
            DockDoorEvent::TrailerDeparted(e) => &e.dock_name,
            DockDoorEvent::ShipmentAssigned(e) => &e.dock_name,
            DockDoorEvent::ShipmentUnassigned(e) => &e.dock_name,
            DockDoorEvent::SensorStateChanged(e) => &e.dock_name,
            DockDoorEvent::DoorStateChanged(e) => &e.dock_name,
            DockDoorEvent::LoadingStatusChanged(e) => &e.dock_name,
            DockDoorEvent::TrailerStateChanged(e) => &e.dock_name,
            DockDoorEvent::WmsEvent(e) => &e.dock_name,
            DockDoorEvent::ShipmentStarted(e) => &e.base_event.dock_name,
            DockDoorEvent::ShipmentSuspended(e) => &e.base_event.dock_name,
            DockDoorEvent::ShipmentCancelled(e) => &e.base_event.dock_name,
            DockDoorEvent::ShipmentResumed(e) => &e.base_event.dock_name,
            DockDoorEvent::PriorityUpdated(e) => &e.base_event.dock_name,
            DockDoorEvent::LoadPlanSaved(e) => &e.base_event.dock_name,
            DockDoorEvent::ShipmentForcedClosed(e) => &e.base_event.dock_name,
            DockDoorEvent::LoadQuantityAdjusted(e) => &e.base_event.dock_name,
            DockDoorEvent::DriverCheckedIn(e) => &e.base_event.dock_name,
            DockDoorEvent::TrailerRejected(e) => &e.base_event.dock_name,
            DockDoorEvent::LgvStartLoading(e) => &e.base_event.dock_name,
            DockDoorEvent::FirstDrop(e) => &e.base_event.dock_name,
            DockDoorEvent::ShipmentCheckout(e) => &e.base_event.dock_name,
            DockDoorEvent::TrailerPatternProcessed(e) => &e.base_event.dock_name,
            DockDoorEvent::AppointmentUpdated(e) => &e.base_event.dock_name,
            DockDoorEvent::TripProcessed(e) => &e.base_event.dock_name,
            DockDoorEvent::UnknownWmsEvent(e) => &e.base_event.dock_name,
        }
    }

    pub fn get_plant_id(&self) -> &str {
        match self {
            DockDoorEvent::DockAssigned(e) => &e.plant_id,
            DockDoorEvent::DockUnassigned(e) => &e.plant_id,
            DockDoorEvent::TrailerDocked(e) => &e.plant_id,
            DockDoorEvent::TrailerDeparted(e) => &e.plant_id,
            DockDoorEvent::LoadingStarted(e) => &e.plant_id,
            DockDoorEvent::LoadingCompleted(e) => &e.plant_id,
            DockDoorEvent::SensorStateChanged(e) => &e.plant_id,
            DockDoorEvent::DoorStateChanged(e) => &e.plant_id,
            DockDoorEvent::LoadingStatusChanged(e) => &e.plant_id,
            DockDoorEvent::TrailerStateChanged(e) => &e.plant_id,
            DockDoorEvent::ShipmentAssigned(e) => &e.plant_id,
            DockDoorEvent::ShipmentUnassigned(e) => &e.plant_id,
            DockDoorEvent::WmsEvent(e) => &e.plant_id,
            DockDoorEvent::ShipmentStarted(e) => &e.base_event.plant_id,
            DockDoorEvent::ShipmentSuspended(e) => &e.base_event.plant_id,
            DockDoorEvent::ShipmentCancelled(e) => &e.base_event.plant_id,
            DockDoorEvent::ShipmentResumed(e) => &e.base_event.plant_id,
            DockDoorEvent::PriorityUpdated(e) => &e.base_event.plant_id,
            DockDoorEvent::LoadPlanSaved(e) => &e.base_event.plant_id,
            DockDoorEvent::ShipmentForcedClosed(e) => &e.base_event.plant_id,
            DockDoorEvent::LoadQuantityAdjusted(e) => &e.base_event.plant_id,
            DockDoorEvent::DriverCheckedIn(e) => &e.base_event.plant_id,
            DockDoorEvent::TrailerRejected(e) => &e.base_event.plant_id,
            DockDoorEvent::LgvStartLoading(e) => &e.base_event.plant_id,
            DockDoorEvent::FirstDrop(e) => &e.base_event.plant_id,
            DockDoorEvent::ShipmentCheckout(e) => &e.base_event.plant_id,
            DockDoorEvent::TrailerPatternProcessed(e) => &e.base_event.plant_id,
            DockDoorEvent::AppointmentUpdated(e) => &e.base_event.plant_id,
            DockDoorEvent::TripProcessed(e) => &e.base_event.plant_id,
            DockDoorEvent::UnknownWmsEvent(e) => &e.base_event.plant_id,
        }
    }

    pub fn from_wms_event(wms_event: WmsEvent) -> Self {
        let base_event = WmsEventWrapper {
            plant_id: wms_event.plant.clone(),
            dock_name: wms_event.dock_name.clone(),
            shipment_id: wms_event.shipment_id.clone(),
            event_type: wms_event.message_type.clone(),
            timestamp: wms_event.log_dttm.unwrap_or_else(|| chrono::Local::now().naive_local()),
            message_source: wms_event.message_source.clone(),
            message_notes: wms_event.message_notes.clone(),
            result_code: wms_event.result_code,
        };

        match wms_event.message_type.as_str() {
            "STARTED_SHIPMENT" => DockDoorEvent::ShipmentStarted(ShipmentStartedEvent { base_event }),
            "SUSPENDED_SHIPMENT" => DockDoorEvent::ShipmentSuspended(ShipmentSuspendedEvent { base_event }),
            "CANCELLED_SHIPMENT" => DockDoorEvent::ShipmentCancelled(ShipmentCancelledEvent { base_event }),
            "RESUMED_SHIPMENT" => DockDoorEvent::ShipmentResumed(ShipmentResumedEvent { base_event }),
            "UPDATED_PRIORITY" => DockDoorEvent::PriorityUpdated(PriorityUpdatedEvent { base_event }),
            "SDM_LOAD_PLAN" => DockDoorEvent::LoadPlanSaved(LoadPlanSavedEvent { base_event }),
            "SHIPMENT_FORCED_CLOSED" => DockDoorEvent::ShipmentForcedClosed(ShipmentForcedClosedEvent { base_event }),
            "LOAD_QTY_ADJUSTED" => DockDoorEvent::LoadQuantityAdjusted(LoadQuantityAdjustedEvent { base_event }),
            "SDM_CHECK_IN" => DockDoorEvent::DriverCheckedIn(DriverCheckedInEvent { base_event }),
            "SDM_TRAILER_REJECTION" => DockDoorEvent::TrailerRejected(TrailerRejectedEvent { base_event }),
            "DOCK_ASSIGNMENT" => DockDoorEvent::DockAssigned(DockAssignedEvent::from(base_event)),
            "LGV_START_LOADING" => DockDoorEvent::LgvStartLoading(LgvStartLoadingEvent { base_event }),
            "FIRST_DROP" => DockDoorEvent::FirstDrop(FirstDropEvent { base_event }),
            "COMPLETED_LOAD" => DockDoorEvent::LoadingCompleted(LoadingCompletedEvent::from(base_event)),
            "CHECKOUT" => DockDoorEvent::ShipmentCheckout(ShipmentCheckoutEvent { base_event }),
            "TRK_PTRN" => DockDoorEvent::TrailerPatternProcessed(TrailerPatternProcessedEvent { base_event }),
            "APPT_UPDATE" => DockDoorEvent::AppointmentUpdated(AppointmentUpdatedEvent { base_event }),
            "PROCTRIP" => DockDoorEvent::TripProcessed(TripProcessedEvent { base_event }),
            _ => DockDoorEvent::UnknownWmsEvent(UnknownWmsEventEvent { base_event }),
        }
    }

    pub fn from_db_insert(db_insert: &DbInsert) -> Self {
        DockDoorEvent::WmsEvent(WmsEventWrapper {
            plant_id: db_insert.PLANT.clone(),
            dock_name: db_insert.DOOR_NAME.clone(),
            shipment_id: db_insert.SHIPMENT_ID.clone().unwrap_or_default(),
            event_type: db_insert.EVENT_TYPE.clone(),
            timestamp: db_insert.LOG_DTTM,
            message_source: "DB_INSERT".to_string(),
            message_notes: Some(db_insert.NOTES.clone()),
            result_code: db_insert.SUCCESS,
        })
    }

    pub fn get_shipment_id(&self) -> Option<String> {
        match self {
            DockDoorEvent::ShipmentAssigned(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::ShipmentUnassigned(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::DockAssigned(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::DockUnassigned(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::TrailerDocked(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::TrailerDeparted(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::LoadingStarted(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::LoadingCompleted(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::LoadingStatusChanged(_) => None,
            DockDoorEvent::WmsEvent(e) => Some(e.shipment_id.clone()),
            DockDoorEvent::SensorStateChanged(_) => None,
            DockDoorEvent::DoorStateChanged(_) => None,
            DockDoorEvent::TrailerStateChanged(_) => None,
            DockDoorEvent::ShipmentStarted(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::ShipmentSuspended(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::ShipmentCancelled(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::ShipmentResumed(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::PriorityUpdated(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::LoadPlanSaved(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::ShipmentForcedClosed(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::LoadQuantityAdjusted(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::DriverCheckedIn(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::TrailerRejected(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::LgvStartLoading(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::FirstDrop(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::ShipmentCheckout(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::TrailerPatternProcessed(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::AppointmentUpdated(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::TripProcessed(e) => Some(e.base_event.shipment_id.clone()),
            DockDoorEvent::UnknownWmsEvent(e) => Some(e.base_event.shipment_id.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmsEventWrapper {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub event_type: String,
    pub timestamp: NaiveDateTime,
    pub message_source: String,
    pub message_notes: Option<String>,
    pub result_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockAssignedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

impl From<WmsEventWrapper> for DockAssignedEvent {
    fn from(wrapper: WmsEventWrapper) -> Self {
        Self {
            plant_id: wrapper.plant_id,
            dock_name: wrapper.dock_name,
            shipment_id: wrapper.shipment_id,
            timestamp: wrapper.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockUnassignedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerDockedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerDepartedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingStartedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingCompletedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

impl From<WmsEventWrapper> for LoadingCompletedEvent {
    fn from(wrapper: WmsEventWrapper) -> Self {
        Self {
            plant_id: wrapper.plant_id,
            dock_name: wrapper.dock_name,
            shipment_id: wrapper.shipment_id,
            timestamp: wrapper.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorStateChangedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub sensor_name: String,
    pub old_value: Option<u8>,
    pub new_value: Option<u8>,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorStateChangedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub old_state: DoorState,
    pub new_state: DoorState,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingStatusChangedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub old_status: LoadingStatus,
    pub new_status: LoadingStatus,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerStateChangedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub old_state: TrailerState,
    pub new_state: TrailerState,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentAssignedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
    pub previous_shipment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentUnassignedEvent {
    pub plant_id: String,
    pub dock_name: String,
    pub shipment_id: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentStartedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentSuspendedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentCancelledEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentResumedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityUpdatedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPlanSavedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentForcedClosedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadQuantityAdjustedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverCheckedInEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerRejectedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LgvStartLoadingEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstDropEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentCheckoutEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerPatternProcessedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppointmentUpdatedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripProcessedEvent {
    pub base_event: WmsEventWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownWmsEventEvent {
    pub base_event: WmsEventWrapper,
}