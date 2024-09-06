/// # Dock Manager Errors
/// This module defines the `DockManagerError` enum, which encapsulates all potential errors that can occur within the IQX Dock Manager application.
/// The enum variants provide specific error types for different components and operations, facilitating clear error handling and reporting throughout the application.


use thiserror::Error;
use sqlx_oldapi::Error as SqlxError;
use std::io;
use plctag::Status as PlcTagStatus;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;

#[derive(Error, Debug)]
pub enum DockManagerError {
    /// Represents errors originating from database interactions.
    #[error("Database error: {0}")]
    DatabaseError(#[from] SqlxError),

    /// Represents errors related to establishing or maintaining connections (e.g., to PLCs or databases).
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Represents errors occurring during communication with PLCs.
    #[error("PLC communication error: {0}")]
    PlcError(String),

    /// Represents errors specifically related to PLC tags.
    #[error("PLC Tag error: {0}")]
    PlcTagError(PlcTagStatus),

    /// Represents errors arising from misconfigurations or invalid settings.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Represents errors occurring within the state management component.
    #[error("State management error: {0}")]
    StateError(String),

    /// Represents standard input/output errors.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// Represents errors encountered during sensor polling operations.
    #[error("Sensor polling error: {0}")]
    SensorPollingError(String),

    /// Represents errors that happen while processing events.
    #[error("Event processing error: {0}")]
    EventProcessingError(String),

    /// Represents errors during the initialization of the logging system.
    #[error("Logging initialization error: {0}")]
    LoggingError(String),

    /// Represents errors that occur during serialization or deserialization of data.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Represents errors when waiting for tasks to complete.
    #[error("Task join error: {0}")]
    TaskJoinError(String),

    /// Represents errors specifically related to reading sensor data.
    #[error("Sensor read error: {0}")]
    SensorReadError(String),

    /// Represents an error when a requested door is not found.
    #[error("Door not found: {0}")]
    DoorNotFound(String),

    /// Represents errors when sending data over a channel.
    #[error("Channel send error: {0}")]
    ChannelSendError(String),

    /// Represents errors when receiving data from a channel.
    #[error("Channel receive error: {0}")]
    ChannelRecvError(String),

    #[error("Plant not found: {0}")]
    PlantNotFound(String),
}

impl<T> From<SendError<T>> for DockManagerError {
    fn from(err: SendError<T>) -> Self {
        DockManagerError::ChannelSendError(err.to_string())
    }
}

impl From<RecvError> for DockManagerError {
    fn from(err: RecvError) -> Self {
        DockManagerError::ChannelRecvError(err.to_string())
    }
}


impl From<config::ConfigError> for DockManagerError {
    fn from(err: config::ConfigError) -> Self {
        DockManagerError::ConfigError(err.to_string())
    }
}

pub type DockManagerResult<T> = Result<T, DockManagerError>;

impl From<String> for DockManagerError {
    fn from(err: String) -> Self {
        DockManagerError::PlcError(err)
    }
}

impl From<PlcTagStatus> for DockManagerError {
    fn from(status: PlcTagStatus) -> Self {
        DockManagerError::PlcTagError(status)
    }
}