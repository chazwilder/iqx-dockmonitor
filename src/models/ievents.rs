//! # Dock Door Event Definitions

//! This module defines the `DockDoorEvent` enum and its associated structs, which represent the various events that can occur 
//! within the dock door management system. These events capture changes in sensor states, door states, loading statuses, 
//! trailer states, and shipment assignments, enabling the system to track and respond to these changes effectively.


use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use crate::models::istates::DoorState;
use crate::models::istatus::{LoadingStatus};
use crate::models::{DbInsert, TrailerState, WmsEvent};

/// Represents the different types of events that can occur at a dock door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DockDoorEvent {
    /// Event indicating that a dock has been assigned to a shipment
    DockAssigned(DockAssignedEvent),
    /// Event indicating that a dock has been unassigned from a shipment
    DockUnassigned(DockUnassignedEvent),
    /// Event indicating that a trailer has docked at a door
    TrailerDocked(TrailerDockedEvent),
    /// Event indicating that a trailer has departed from a door
    TrailerDeparted(TrailerDepartedEvent),
    /// Event indicating that the loading process has started at a door
    LoadingStarted(LoadingStartedEvent),
    /// Event indicating that the loading process has been completed at a door
    LoadingCompleted(LoadingCompletedEvent),
    /// Event indicating that the state of a sensor has changed
    SensorStateChanged(SensorStateChangedEvent),
    /// Event indicating that the state of a door has changed
    DoorStateChanged(DoorStateChangedEvent),
    /// Event indicating that the loading status of a door has changed
    LoadingStatusChanged(LoadingStatusChangedEvent),
    /// Event indicating that the state of a trailer has changed
    TrailerStateChanged(TrailerStateChangedEvent),
    /// Event indicating that a shipment has been assigned to a door
    ShipmentAssigned(ShipmentAssignedEvent),
    /// Event indicating that a shipment has been unassigned from a door
    ShipmentUnassigned(ShipmentUnassignedEvent),
    /// Event originating from the Warehouse Management System (WMS)
    WmsEvent(WmsEventWrapper),
}

impl DockDoorEvent {
    /// Retrieves the name of the dock associated with the event
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
        }
    }

    /// Creates a `DockDoorEvent` from a `WmsEvent`
    pub fn from_wms_event(wms_event: WmsEvent) -> Self {
        DockDoorEvent::WmsEvent(WmsEventWrapper {
            dock_name: wms_event.dock_name,
            shipment_id: wms_event.shipment_id,
            event_type: wms_event.message_type,
            timestamp: wms_event.log_dttm.unwrap_or_else(|| chrono::Local::now().naive_local()),
            message_source: wms_event.message_source,
            message_notes: wms_event.message_notes,
            result_code: wms_event.result_code,
        })
    }

    /// Creates a `DockDoorEvent` from a `DbInsert`
    pub fn from_db_insert(db_insert: &DbInsert) -> Self {
        DockDoorEvent::WmsEvent(WmsEventWrapper {
            dock_name: db_insert.DOOR_NAME.clone(),
            shipment_id: db_insert.SHIPMENT_ID.clone().unwrap_or_default(),
            event_type: db_insert.EVENT_TYPE.clone(),
            timestamp: db_insert.LOG_DTTM,
            message_source: "DB_INSERT".to_string(),
            message_notes: Some(db_insert.NOTES.clone()),
            result_code: db_insert.SUCCESS,
        })
    }
}

/// A wrapper for WMS events, providing additional context for the Dock Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmsEventWrapper {
    /// The name of the dock associated with the event
    pub dock_name: String,
    /// The ID of the shipment related to the event
    pub shipment_id: String,
    /// The type of WMS event
    pub event_type: String,
    /// The timestamp of the event
    pub timestamp: NaiveDateTime,
    /// The source of the WMS event
    pub message_source: String,
    /// Additional notes or details about the event
    pub message_notes: Option<String>,
    /// The result code of the event
    pub result_code: i32,
}

// ... (other code and docstrings)

/// Represents an event where a dock has been assigned to a shipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockAssignedEvent {
    /// The name of the dock that has been assigned.
    pub dock_name: String,
    /// The ID of the shipment assigned to the dock.
    pub shipment_id: String,
    /// The timestamp when the dock was assigned.
    pub timestamp: NaiveDateTime,
}

/// Represents an event where a dock has been unassigned from a shipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockUnassignedEvent {
    /// The name of the dock that has been unassigned
    pub dock_name: String,
    /// The ID of the shipment that was unassigned from the dock
    pub shipment_id: String,
    /// The timestamp when the dock was unassigned
    pub timestamp: NaiveDateTime,
}

/// Represents an event where a trailer has docked at a door.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerDockedEvent {
    /// The name of the dock where the trailer docked
    pub dock_name: String,
    /// The ID of the shipment associated with the docked trailer
    pub shipment_id: String,
    /// The timestamp when the trailer docked
    pub timestamp: NaiveDateTime,
}

/// Represents an event where the loading process has started at a door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingStartedEvent {
    /// The name of the dock where loading started
    pub dock_name: String,
    /// The ID of the shipment being loaded
    pub shipment_id: String,
    /// The timestamp when loading started
    pub timestamp: NaiveDateTime,
}

/// Represents an event where the loading process has been completed at a door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingCompletedEvent {
    /// The name of the dock where loading was completed
    pub dock_name: String,
    /// The ID of the shipment that was loaded
    pub shipment_id: String,
    /// The timestamp when loading was completed
    pub timestamp: NaiveDateTime,
}

/// Represents an event where a trailer has departed from a door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerDepartedEvent {
    /// The name of the dock from which the trailer departed
    pub dock_name: String,
    /// The ID of the shipment associated with the departed trailer
    pub shipment_id: String,
    /// The timestamp when the trailer departed
    pub timestamp: NaiveDateTime,
}

/// Represents an event where a shipment has been assigned to a door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentAssignedEvent {
    /// The name of the dock to which the shipment is assigned
    pub dock_name: String,
    /// The ID of the assigned shipment
    pub shipment_id: String,
    /// The timestamp when the shipment was assigned
    pub timestamp: NaiveDateTime,
    /// The ID of the previously assigned shipment (if any)
    pub previous_shipment: Option<String>,
}

/// Represents an event where a shipment has been unassigned from a door
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentUnassignedEvent {
    /// The name of the dock from which the shipment is unassigned
    pub dock_name: String,
    /// The ID of the unassigned shipment
    pub shipment_id: String,
    /// The timestamp when the shipment was unassigned
    pub timestamp: NaiveDateTime,
}

/// Represents an event where the state of a sensor has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorStateChangedEvent {
    /// The name of the dock where the sensor is located
    pub dock_name: String,
    /// The name of the sensor that changed state
    pub sensor_name: String,
    /// The old value of the sensor
    pub old_value: Option<u8>,
    /// The new value of the sensor
    pub new_value: Option<u8>,
    /// The timestamp when the sensor state changed
    pub timestamp: NaiveDateTime,
}

/// Represents an event where the state of a door has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorStateChangedEvent {
    /// The name of the door whose state changed
    pub dock_name: String,
    /// The previous state of the door
    pub old_state: DoorState,
    /// The new state of the door
    pub new_state: DoorState,
    /// The timestamp when the door state changed
    pub timestamp: NaiveDateTime,
}

/// Represents an event where the loading status of a door has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingStatusChangedEvent {
    /// The name of the dock where the loading status changed
    pub dock_name: String,
    /// The previous loading status
    pub old_status: LoadingStatus,
    /// The new loading status
    pub new_status: LoadingStatus,
    /// The timestamp when the loading status changed
    pub timestamp: NaiveDateTime,
}

/// Represents an event where the state of a trailer has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerStateChangedEvent {
    /// The name of the dock associated with the trailer
    pub dock_name: String,
    /// The previous state of the trailer
    pub old_state: TrailerState,
    /// The new state of the trailer
    pub new_state: TrailerState,
    /// The timestamp when the trailer state changed
    pub timestamp: NaiveDateTime,
}