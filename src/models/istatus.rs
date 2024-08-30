//! # Warehouse Management System (WMS) Data Structures

//! This module defines data structures that interact with and represent information from the Warehouse Management System (WMS). 
//! These structures facilitate the seamless integration and processing of WMS data within the IQX Dock Manager application.

use std::collections::HashSet;
use chrono::NaiveDateTime;
use derive_more::{Constructor, FromStr};
use serde::{Deserialize, Serialize};
use sqlx_oldapi::FromRow;

/// Represents the various loading statuses a shipment can have in the WMS.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum LoadingStatus {
    /// The dock is idle, not assigned to any shipment.
    Idle,
    /// The shipment is in the Customer Service Order (CSO) stage.
    CSO,
    /// The shipment is undergoing warehouse inspection.
    WhseInspection,
    /// The system is allocating an LGV (Laser Guided Vehicle) for the shipment.
    LgvAllocation,
    /// The shipment is currently being loaded.
    Loading,
    /// The loading process is temporarily suspended.
    Suspended,
    /// The loading is completed.
    Completed,
    /// The loaded shipment is awaiting departure.
    WaitingForExit,
    /// The shipment has been canceled.
    CancelledShipment,
    /// The shipment has been started with anticipation.
    StartedWithAnticipation,
}

/// Represents the status of a dock door as retrieved from the WMS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct WmsDoorStatus {
    /// The name of the dock door.
    pub dock_name: String,
    /// The shipment currently assigned to the dock door (if any).
    pub assigned_shipment: Option<String>,
    /// The percentage progress of the loading process.
    pub loading_progress_percent: Option<i32>,
    /// The current loading status of the dock door.
    pub loading_status: String,
    /// The status of the shipment in the WMS.
    pub wms_shipment_status: Option<String>,
    /// Any shipping fault code associated with the shipment.
    pub shipping_fault_code: Option<String>,
    /// The upper limit for the number of shipments allowed to be loading simultaneously.
    pub upper_ship_limit: i32,
    /// The current number of shipments in the loading state.
    pub shipments_loading: i32,
}

/// Represents an event related to a shipment in the WMS
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Eq, PartialEq, Hash)]
pub struct WmsEvent {
    /// The plant associated with the event
    #[sqlx(rename = "PLANT")]
    pub plant: String,
    /// The dock name associated with the event
    #[sqlx(rename = "DOCK_NAME")]
    pub dock_name: String,
    /// The ID of the shipment the event pertains to
    #[sqlx(rename = "SHIPMENT_ID")]
    pub shipment_id: String,
    /// The date and time the event occurred
    #[sqlx(rename = "LOG_DTTM")]
    pub log_dttm: Option<NaiveDateTime>,
    /// The source system that generated the event
    #[sqlx(rename = "MESSAGE_SOURCE")]
    pub message_source: String,
    /// The type of event
    #[sqlx(rename = "MESSAGE_TYPE")]
    pub message_type: String,
    /// An identifier for the event type
    #[sqlx(rename = "MESSAGE_TYPE_ID")]
    pub message_type_id: Option<String>,
    /// Additional notes or details about the event
    #[sqlx(rename = "MESSAGE_NOTES")]
    pub message_notes: Option<String>,
    /// A code indicating the result or outcome of the event
    #[sqlx(rename = "RESULT_CODE")]
    pub result_code: i32,
}

/// Represents a shipment that is or has been assigned to a dock door
#[derive(Debug, Clone, Serialize, Deserialize, Constructor, Default, Eq, PartialEq)]
pub struct AssignedShipment {
    /// The ID of the currently assigned shipment
    pub current_shipment: Option<String>,
    /// The date and time when the current shipment was assigned
    pub assignment_dttm: Option<NaiveDateTime>,
    /// The date and time when the current shipment was unassigned
    pub unassignment_dttm: Option<NaiveDateTime>,
    /// The ID of the previously assigned shipment
    pub previous_shipment: Option<String>,
    /// The date and time when the previous shipment was completed
    pub previous_completed_dttm: Option<NaiveDateTime>,
    /// A collection of WMS events related to the shipment
    pub events: HashSet<WmsEvent>,
}

impl AssignedShipment {
    /// Adds a WMS event to the shipment's event history
    pub fn add_event(&mut self, event: WmsEvent) {
        self.events.insert(event);
    }

    /// Clears the shipment's event history
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}