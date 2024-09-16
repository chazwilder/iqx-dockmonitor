use std::sync::Arc;
use bb8::{Pool, ManageConnection};
use crate::errors::DockManagerError;
use plctag::RawTag;

pub struct PlcConnection {
    tag: Arc<RawTag>,
}

impl PlcConnection {
    pub fn new(ip: &str, tag_address: &str, timeout_ms: u32) -> Result<Self, DockManagerError> {
        let tag = Arc::new(RawTag::new(
            format!("protocol=ab_eip&gateway={}&path=1,0&plc=ControlLogix&elem_size=1&elem_count=1&name={}", ip, tag_address),
            timeout_ms,
        ).map_err(|e| DockManagerError::PlcError(format!("Failed to create PLC tag: {:?}", e)))?);

        Ok(Self { tag })
    }

    pub fn read(&self) -> Result<u8, DockManagerError> {
        self.tag.read(0); // Use a constant timeout of 0 milliseconds
        self.tag.get_u8(0).map_err(|e| DockManagerError::PlcError(format!("Failed to read PLC tag: {:?}", e)))
    }
}

pub struct PlcConnectionManager {
    ip: String,
    tag_address: String,
    timeout_ms: u32,
}

impl PlcConnectionManager {
    pub fn new(ip: String, tag_address: String, timeout_ms: u32) -> Self {
        Self { ip, tag_address, timeout_ms }
    }
}

#[async_trait::async_trait]
impl ManageConnection for PlcConnectionManager {
    type Connection = PlcConnection;
    type Error = DockManagerError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        PlcConnection::new(&self.ip, &self.tag_address, self.timeout_ms)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Perform a simple read operation to check if the connection is still valid
        conn.read().map(|_| ())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

pub type PlcConnectionPool = Pool<PlcConnectionManager>;