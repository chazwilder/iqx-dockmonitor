use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use plctag::builder::*;
use plctag::RawTag;
use crate::errors::DockManagerError;

/// # PlcTagFactory
///
/// A factory struct for creating and caching PLC (Programmable Logic Controller) tags.
/// This struct provides methods to create and retrieve `RawTag` instances, which are used
/// for communication with PLCs in the dock door management system.
pub struct PlcTagFactory {
    tag_cache: Arc<Mutex<HashMap<String, Arc<RawTag>>>>,
}

impl PlcTagFactory {
    /// Creates a new `PlcTagFactory` instance with an empty tag cache.
    ///
    /// # Returns
    ///
    /// A new `PlcTagFactory` instance.
    pub fn new() -> Self {
        Self {
            tag_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieves an existing tag from the cache or creates a new one if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `door_ip` - A string slice containing the IP address of the PLC.
    /// * `plc_tag_address` - A string slice representing the address of the tag in the PLC.
    /// * `timeout_ms` - The timeout for PLC operations in milliseconds.
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - `Ok(RawTag)`: The retrieved or newly created `RawTag` instance.
    /// - `Err(DockManagerError)`: An error if tag creation fails.
    pub async fn get_or_create_tag(&self, door_ip: &str, plc_tag_address: &str, timeout_ms: u64) -> Result<Arc<RawTag>, DockManagerError> {
        let cache_key = format!("{}:{}", door_ip, plc_tag_address);
        let mut cache = self.tag_cache.lock().await;

        if let Some(tag) = cache.get(&cache_key) {
            Ok(Arc::clone(tag))
        } else {
            let tag = Arc::new(Self::create_tag(door_ip, plc_tag_address, timeout_ms)?);
            cache.insert(cache_key, Arc::clone(&tag));
            Ok(tag)
        }
    }

    /// Creates a new PLC tag for communication with a specific sensor on a PLC.
    ///
    /// # Arguments
    ///
    /// * `door_ip` - A string slice containing the IP address of the PLC.
    /// * `plc_tag_address` - A string slice representing the address of the tag in the PLC.
    /// * `timeout_ms` - The timeout for PLC operations in milliseconds.
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - `Ok(RawTag)`: A successfully created `RawTag` instance.
    /// - `Err(DockManagerError)`: An error if tag creation fails.
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