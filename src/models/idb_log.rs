//! # Database Insertion Structure

//! This module defines the `DbInsert` struct, which represents a record to be inserted into the database. It encapsulates 
//! various fields that capture information about events, their timestamps, associated entities (like plants, doors, and shipments), 
//! success status, notes, and other relevant details. 
//! The `DbInsert` struct facilitates the structured storage of log entries and other events within the database for further analysis and reporting

#![allow(non_snake_case)]

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx_oldapi::FromRow;
use crate::analysis::LogEntry;
use crate::models::WmsEvent;

/// Represents a record to be inserted into the database, typically derived from a `LogEntry`
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct DbInsert {
    /// The date and time the event occurred
    pub LOG_DTTM: NaiveDateTime,
    /// The plant associated with the event
    pub PLANT: String,
    /// The name of the dock door related to the event
    pub DOOR_NAME: String,
    /// The ID of the shipment, if applicable to the event
    pub SHIPMENT_ID: Option<String>,
    /// The type of event
    pub EVENT_TYPE: String,
    /// Indicates whether the event was successful (1) or not (0)
    pub SUCCESS: i32,
    /// Additional notes or details about the event
    pub NOTES: String,
    /// The ID of the user who triggered or is associated with the event
    pub ID_USER: Option<String>,
    /// The severity level of the event
    pub SEVERITY: i32,
    /// The previous state or value before the event occurred
    pub PREVIOUS_STATE: Option<String>,
    /// The date and time of the previous state
    pub PREVIOUS_STATE_DTTM: Option<NaiveDateTime>,
}

impl DbInsert {
    /// Creates a `DbInsert` instance from a `LogEntry`
    /// 
    /// This method extracts relevant information from the `LogEntry` and constructs 
    /// a `DbInsert` object suitable for database insertion
    ///
    /// # Arguments
    /// * `log_entry`: The `LogEntry` from which to create the `DbInsert`
    /// 
    /// # Returns
    /// * A new `DbInsert` instance populated with data from the `LogEntry`
    pub fn from_log_entry(log_entry: &LogEntry) -> Self {
        match log_entry {
            LogEntry::DockingTime { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ManualModeActivated { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::LoadingStatusChange { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ShipmentUnassigned { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ShipmentAssigned { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::NewShipmentPreviousTrailerPresent { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ManualInterventionStarted { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ManualInterventionSuccess { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ManualInterventionFailure { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::TrailerStateChange { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::SuspendedDoor { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::LongLoadingStart { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::TrailerHostage { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::ShipmentStartedLoadNotReady { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::TrailerUndocked { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } |
            LogEntry::TrailerPatternIssue { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm } => {
                DbInsert {
                    LOG_DTTM: *log_dttm,
                    PLANT: plant.clone(),
                    DOOR_NAME: door_name.clone(),
                    SHIPMENT_ID: shipment_id.clone(),
                    EVENT_TYPE: event_type.clone(),
                    SUCCESS: if *success { 1 } else { 0 },
                    NOTES: notes.clone(),
                    ID_USER: None,
                    SEVERITY: *severity,
                    PREVIOUS_STATE: previous_state.clone(),
                    PREVIOUS_STATE_DTTM: *previous_state_dttm,
                }
            },
            LogEntry::WmsEvent { log_dttm, plant, door_name, shipment_id, event_type, success, notes, severity, previous_state, previous_state_dttm, .. } => {
                DbInsert {
                    LOG_DTTM: *log_dttm,
                    PLANT: plant.clone(),
                    DOOR_NAME: door_name.clone(),
                    SHIPMENT_ID: shipment_id.clone(),
                    EVENT_TYPE: event_type.clone(),
                    SUCCESS: if *success { 1 } else { 0 },
                    NOTES: notes.clone(),
                    ID_USER: None,
                    SEVERITY: *severity,
                    PREVIOUS_STATE: previous_state.clone(),
                    PREVIOUS_STATE_DTTM: *previous_state_dttm,
                }
            }
        }
    }

    pub fn get_plant_id(&self) -> &str {
        &self.PLANT
    }
}

impl TryFrom<WmsEvent> for DbInsert {
    type Error = String;

    fn try_from(event: WmsEvent) -> Result<Self, Self::Error> {
        Ok(DbInsert {
            LOG_DTTM: event.log_dttm.unwrap_or_else(|| chrono::Local::now().naive_local()),
            PLANT: event.plant,
            DOOR_NAME: event.dock_name,
            SHIPMENT_ID: Some(event.shipment_id),
            EVENT_TYPE: event.message_type,
            SUCCESS: if event.result_code == 0 { 1 } else { 0 },
            NOTES: event.message_notes.unwrap_or_default(),
            ID_USER: None,
            SEVERITY: event.result_code,
            PREVIOUS_STATE: None,
            PREVIOUS_STATE_DTTM: None,
        })
    }
}