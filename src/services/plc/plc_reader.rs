use tokio::task;
use std::time::Duration;
use crate::errors::DockManagerError;

/// # PlcReader
///
/// A struct responsible for reading data from PLC (Programmable Logic Controller) tags.
/// It handles the actual communication with the PLC, including timeout management and error handling.
///
/// ## Fields
///
/// * `timeout_ms`: The timeout for PLC read operations in milliseconds.
///
/// ## Usage
///
/// The `PlcReader` is typically instantiated within the `PlcService` and used to read sensor values
/// from PLCs in the dock door management system.
///
/// ## Example
///
/// ```rust
/// let reader = PlcReader::new(5000);
/// let tag = PlcTagFactory::create_tag("192.168.1.100", "Tag1", 5000)?;
/// let value = reader.read_tag(tag).await?;
/// ```
pub struct PlcReader {
    pub timeout_ms: u64,
}

impl PlcReader {
    /// Creates a new instance of `PlcReader`.
    ///
    /// # Arguments
    ///
    /// * `timeout_ms`: The timeout for PLC read operations in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a new `PlcReader` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// let reader = PlcReader::new(5000);
    /// ```
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    /// Reads a value from a PLC tag.
    ///
    /// This method attempts to read an 8-bit unsigned integer value from the given PLC tag.
    /// It uses Tokio's `spawn_blocking` to perform the blocking PLC read operation in a separate thread,
    /// and implements a timeout to prevent indefinite blocking.
    ///
    /// # Arguments
    ///
    /// * `tag`: A `RawTag` instance representing the PLC tag to read from.
    ///
    /// # Returns
    ///
    /// Returns a `Result<u8, DockManagerError>`:
    /// - `Ok(u8)`: The successfully read 8-bit unsigned integer value.
    /// - `Err(DockManagerError)`: An error if the read operation fails or times out.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The PLC read operation fails.
    /// - The spawned task panics.
    /// - The operation times out.
    ///
    /// # Usage
    ///
    /// This method is typically called in the `read_sensor` method of `PlcService`:
    ///
    /// ```rust
    /// let value = reader.read_tag(tag).await?;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// let reader = PlcReader::new(5000);
    /// let tag = PlcTagFactory::create_tag("192.168.1.100", "Tag1", 5000)?;
    /// let value = reader.read_tag(tag).await?;
    /// println!("Read value: {}", value);
    /// ```
    pub async fn read_tag(&self, tag: plctag::RawTag) -> Result<u8, DockManagerError> {
        let timeout = Duration::from_millis(self.timeout_ms);
        let timeout_ms = self.timeout_ms;

        tokio::time::timeout(timeout, task::spawn_blocking(move || {
            tag.read(timeout_ms as u32);
            tag.get_u8(0)
        }))
            .await
            .map_err(|_| DockManagerError::PlcError("PLC read operation timed out".to_string()))?
            .map_err(|e| DockManagerError::PlcError(format!("Task join error: {}", e)))?
            .map_err(|e| DockManagerError::PlcError(format!("Failed to read tag value: {}", e)))
    }
}