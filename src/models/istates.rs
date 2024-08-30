//! # Dock Manager State Enums

//! This module defines several enums representing various states within the IQX Dock Manager system.
//! These enums enable structured and type-safe representation of different states related to doors, trailers, and other components, enhancing code clarity and maintainability.

use serde::{Deserialize, Serialize};
use derive_more::FromStr;

/// Represents the different states a dock door can be in.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum DoorState {
    /// The door is not assigned to any shipment.
    Unassigned,
    /// The door has been assigned to a shipment but the driver has not yet checked in.
    Assigned,
    /// The driver has checked in at the door.
    DriverCheckedIn,
    /// The trailer is approaching the door.
    TrailerApproaching,
    /// The trailer is in the process of docking.
    TrailerDocking,
    /// The trailer is fully docked at the door.
    TrailerDocked,
    /// The door is ready for loading/unloading.
    DoorReady,
    /// The loading/unloading process is in progress.
    Loading,
    /// The loading/unloading process is complete.
    LoadingCompleted,
    /// The shipment is complete and the trailer is waiting to exit.
    WaitingForExit,
}

/// Represents the two possible states of a trailer: docked or undocked.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum TrailerState {
    /// The trailer is docked at a door.
    Docked,
    /// The trailer is not docked at any door.
    Undocked,
}

/// Represents whether manual mode is enabled or disabled for a dock door.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum ManualMode {
    /// Manual mode is enabled, allowing manual control of the door.
    Enabled,
    /// Manual mode is disabled, the door operates automatically.
    Disabled,
}

/// Represents the state of the dock lock: engaged or disengaged.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum DockLockState {
    /// The dock lock is engaged, securing the trailer to the dock.
    Engaged,
    /// The dock lock is disengaged, allowing the trailer to move.
    Disengaged,
}

/// Represents the position of the door: open or closed.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum DoorPosition {
    /// The door is open.
    Open,
    /// The door is closed.
    Closed,
}

/// Represents the position of the leveler: stored or extended.
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum LevelerPosition {
    /// The leveler is stored, not in use.
    Stored,
    /// The leveler is extended, bridging the gap between the dock and the trailer.
    Extended,
}

/// Represents the fault state of a component: no fault or fault present
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, FromStr)]
pub enum FaultState {
    /// No fault is detected.
    NoFault,
    /// A fault is present.
    FaultPresent,
}