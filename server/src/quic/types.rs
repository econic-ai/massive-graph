//! Core types for QUIC service

use massive_graph_core::types::{DocId, UserId};
use std::time::Duration;
use crate::constants::{LANES_PER_CONNECTION, DELTA_HEADER_SIZE};

/// Lane identifier (0..11)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LaneId(pub u8);

impl LaneId {
    /// Create lane ID from document ID hash
    pub fn from_doc_id(doc_id: &DocId) -> Self {
        // Simple hash for lane selection
        let hash = doc_id.as_bytes().iter().fold(0u64, |acc, &b| {
            acc.wrapping_mul(31).wrapping_add(b as u64)
        });
        LaneId((hash % LANES_PER_CONNECTION as u64) as u8)
    }
}

/// Shard identifier for doc-level sharding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardId(pub u16);

impl ShardId {
    /// Compute shard from document ID
    pub fn from_doc_id(doc_id: &DocId, shard_count: u16) -> Self {
        let hash = doc_hash_u64(doc_id);
        ShardId((hash % shard_count as u64) as u16)
    }
}

/// Parsed delta header for routing decisions
#[derive(Debug, Clone)]
pub struct DeltaHeaderMeta {
    pub doc_id: DocId,
    pub total_size: u32,
    pub magic: u32,
    pub delta_type: u16,
}

impl DeltaHeaderMeta {
    /// Parse header from bytes (assumes network byte order)
    pub fn parse(header_bytes: &[u8; DELTA_HEADER_SIZE]) -> Result<Self, String> {
        if header_bytes.len() < DELTA_HEADER_SIZE {
            return Err("Invalid header size".to_string());
        }
        
        // Extract fields (simplified - you'd use proper parsing)
        let magic = u32::from_be_bytes([header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3]]);
        let delta_type = u16::from_be_bytes([header_bytes[8], header_bytes[9]]);
        
        // Doc ID at offset 12, 16 bytes
        let mut doc_id_bytes = [0u8; 16];
        doc_id_bytes.copy_from_slice(&header_bytes[12..28]);
        let doc_id = DocId::from_bytes(doc_id_bytes);
        
        // Total size at offset 40, 4 bytes
        let total_size = u32::from_be_bytes([header_bytes[40], header_bytes[41], header_bytes[42], header_bytes[43]]);
        
        Ok(DeltaHeaderMeta {
            doc_id,
            total_size,
            magic,
            delta_type,
        })
    }
}

/// Compute a stable 64-bit hash from DocId for routing
pub fn doc_hash_u64(doc_id: &DocId) -> u64 {
    doc_id
        .as_bytes()
        .iter()
        .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

/// Connection metadata
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub user_id: UserId,
    pub connection_id: String,
    pub established_at: std::time::Instant,
}

/// Timeouts for stream operations
#[derive(Debug, Clone)]
pub struct Timeouts {
    pub header_read: Duration,
    pub payload_read: Duration,
    pub stream_idle: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            header_read: Duration::from_secs(5),
            payload_read: Duration::from_secs(30),
            stream_idle: Duration::from_secs(300),
        }
    }
}
