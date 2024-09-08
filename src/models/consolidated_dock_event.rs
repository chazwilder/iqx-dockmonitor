use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx_oldapi::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ConsolidatedDockEvent {
    pub plant: String,
    pub door_name: String,
    pub shipment_id: i32,
    pub docking_time_minutes: Option<i32>,
    pub inspection_time_minutes: Option<i32>,
    pub enqueued_time_minutes: Option<i32>,
    pub shipment_assigned: Option<NaiveDateTime>,
    pub dock_assignment: Option<NaiveDateTime>,
    pub trailer_docking: Option<NaiveDateTime>,
    pub started_shipment: Option<NaiveDateTime>,
    pub lgv_start_loading: Option<NaiveDateTime>,
    pub dock_ready: Option<NaiveDateTime>,
    pub is_preload: Option<bool>,
}