use plctag::builder::*;
use plctag::RawTag;
use crate::errors::DockManagerError;

/// # PlcTagFactory
///
/// A factory struct for creating PLC (Programmable Logic Controller) tags.
/// This struct provides a static method to create `RawTag` instances, which are used
/// for communication with PLCs in the dock door management system.
///
/// ## Usage
///
/// The `PlcTagFactory` is typically used within the `PlcService`, specifically in the `read_sensor` method,
/// to create tags for each sensor that needs to be read.
///
/// ## Example
///
/// ```rust
/// let tag = PlcTagFactory::create_tag("192.168.1.100", "Tag1", 5000)?;
/// ```
pub struct PlcTagFactory;

impl PlcTagFactory {
    /// Creates a new PLC tag for communication with a specific sensor on a PLC.
    ///
    /// This method constructs a `RawTag` using the provided parameters and the libplctag library.
    /// It sets up the communication path and parameters required to interact with a specific PLC tag.
    ///
    /// # Arguments
    ///
    /// * `door_ip`: A string slice containing the IP address of the PLC.
    /// * `plc_tag_address`: A string slice representing the address of the tag in the PLC.
    /// * `timeout_ms`: The timeout for PLC operations in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a `Result<RawTag, DockManagerError>`:
    /// - `Ok(RawTag)`: A successfully created `RawTag` instance.
    /// - `Err(DockManagerError)`: An error if tag creation fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The PLC path building fails.
    /// - The `RawTag` creation fails.
    ///
    /// # Usage
    ///
    /// This method is typically called in the `read_sensor` method of `PlcService`:
    ///
    /// ```rust
    /// let tag = PlcTagFactory::create_tag(door_ip, plc_tag_address, reader.timeout_ms)?;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// let tag = PlcTagFactory::create_tag("192.168.1.100", "Tag1", 5000)?;
    /// // Use the tag for PLC communication
    /// ```
    ///
    /// # TODO
    ///
    /// - Consider adding support for different PLC types beyond MicroLogix.
    /// - Implement a caching mechanism for frequently used tags to improve performance.
    /// - Add validation for the `plc_tag_address` format to catch configuration errors early.
    ///
    /// # Safety
    ///
    /// This method creates tags that directly interact with industrial control systems.
    /// Ensure that proper security measures are in place to prevent unauthorized access or manipulation.
    ///
    /// # Performance Considerations
    ///
    /// Tag creation can be a relatively expensive operation. If the same tags are used frequently,
    /// consider implementing a caching mechanism to reuse tag instances where possible.
    pub fn create_tag(
        door_ip: &str,
        plc_tag_address: &str,
        timeout_ms: u64
    ) -> Result<RawTag, DockManagerError> {
        let path = PathBuilder::default()
            .protocol(Protocol::EIP)
            .gateway(door_ip)
            .plc(PlcKind::MicroLogix)
            .name(plc_tag_address)
            .element_size(1)
            .element_count(1)
            .path("0")
            .read_cache_ms(0)
            .build()
            .map_err(|e| DockManagerError::PlcError(format!("Failed to build PLC path: {:?}", e)))?;

        RawTag::new(path, timeout_ms as u32)
            .map_err(|e| DockManagerError::PlcError(format!("Failed to create PLC tag: {:?}", e)))
    }
}