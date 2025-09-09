//! Zero-copy payload types for WebRTC data channels

use serde::{Serialize, Deserialize};

/// Zero-copy payload for efficient data transmission
pub struct Payload {
    /// Pointer to the data
    pub ptr: *const u8,
    /// Length of the data
    pub len: usize,
}

impl Payload {
    /// Create a new payload from a byte slice
    /// SAFETY: The caller must ensure the slice remains valid for the lifetime of the Payload
    pub unsafe fn from_slice(data: &[u8]) -> Self {
        Self {
            ptr: data.as_ptr(),
            len: data.len(),
        }
    }
    
    /// Create a payload from owned data
    pub fn from_vec(data: Vec<u8>) -> (Self, Vec<u8>) {
        let payload = Self {
            ptr: data.as_ptr(),
            len: data.len(),
        };
        (payload, data)
    }
    
    /// Get the payload as a slice
    /// SAFETY: The caller must ensure the underlying data is still valid
    pub unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr, self.len)
    }
    
    /// Get the size of the payload
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if the payload is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// Payload is Send but not Sync (can be moved between threads but not shared)
unsafe impl Send for Payload {}

/// Message types for the ping-pong test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestMessage {
    /// Ping message with timestamp
    Ping {
        /// Timestamp when ping was sent
        timestamp: u64,
        /// Test payload
        payload: String,
    },
    /// Pong response with original timestamp
    Pong {
        /// Original timestamp from ping
        timestamp: u64,
        /// Response payload
        payload: String,
    },
}

impl TestMessage {
    /// Convert to payload for transmission
    pub fn to_payload(&self) -> Result<(Payload, Vec<u8>), bincode::error::EncodeError> {
        let bytes = bincode::serde::encode_to_vec(self, bincode::config::standard())?;
        Ok(Payload::from_vec(bytes))
    }
    
    /// Parse from payload
    /// SAFETY: The caller must ensure the payload's data is still valid
    pub unsafe fn from_payload(payload: &Payload) -> Result<Self, bincode::error::DecodeError> {
        let slice = payload.as_slice();
        let (msg, _) = bincode::serde::decode_from_slice(slice, bincode::config::standard())?;
        Ok(msg)
    }
}
