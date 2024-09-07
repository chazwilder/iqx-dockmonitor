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
pub use plcvalue::*;

pub fn local_now() -> NaiveDateTime {
    Local::now().naive_local()
}