pub mod plcvalue;
pub mod idoor;
pub mod istatus;
pub mod isensor;
pub mod ievents;
pub mod istates;
pub mod idb_log;
pub mod consolidated_dock_event;

pub use idoor::*;
pub use istatus::*;
pub use isensor::*;
pub use ievents::*;
pub use istates::*;
pub use idb_log::*;

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
pub use plcvalue::*;

pub fn local_now() -> NaiveDateTime {
    Local::now().naive_local()
}

#[derive(Debug, sqlx_oldapi::FromRow, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct TrailerPatternData {
    pub id_shipment: i32,
    pub shipmentnumber: String,
    pub id_delivery: i32,
    pub dock_door: String,
    pub load_pattern_position: i32,
    pub send_trl_ptrn_alert: i32,
    pub expected_pallet_count: i32,
}