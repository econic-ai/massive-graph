//! Delta-related type definitions

use crate::core::types::{ID8, ID16};
use bytes::Bytes;

/// Operation types for granular document modifications.
/// Each operation is designed for minimal overhead and atomic application.
/// 
/// Bit layout for access control:
/// - 0x00-0x7F (0-127): User operations (bit 7 = 0)
/// - 0x80-0xFF (128-255): Privileged operations (bit 7 = 1)
///   - 0x80-0x9F: Admin operations (bits 7,5 = 1,0)  
///   - 0xA0-0xBF: Statistical operations (bits 7,5 = 1,1)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpType {
    // User operations (0x00-0x7F) - bit 7 = 0
    
    // Property operations
    /// Set a property value on a document
    PropertySet = 0,
    /// Delete a property from a document
    PropertyDelete = 1,
    
    // String property operations
    /// Insert text at a specific position in a string property
    StringInsert = 2,
    /// Remove text from a specific range in a string property
    StringRemove = 3,
    
    // Numeric property operations
    /// Increment a numeric property value (integers, floats, booleans)
    PropertyIncrement = 4,
    /// Decrement a numeric property value (integers, floats, booleans)
    PropertyDecrement = 5,
    
    // Children operations
    /// Add a child relationship to a document
    ChildAdd = 10,
    /// Remove a child relationship from a document
    ChildRemove = 11,
    /// Replace all children with a new set
    ChildReplace = 12,
    /// Move a child to a new position
    ChildMove = 13,
    /// Copy a child to a new document
    ChildCopy = 14,
    
    // Stream operations
    /// Append binary data to stream
    StreamAppendBinary = 15,
    /// Append text data to stream
    StreamAppendText = 16,
    
    // Document lifecycle
    /// Create a new document
    DocumentCreate = 20,
    /// Delete an existing document
    DocumentDelete = 21,
    /// Move document to new parent
    DocumentMove = 22,
    /// Copy document with new ID
    DocumentCopy = 23,
    
    // Privileged operations (0x80-0xFF) - bit 7 = 1
    
    // Admin operations (0x80-0x9F) - bits 7,5 = 1,0
    /// Rebuild indexes for performance optimization
    Reindex = 0x80,
    /// Truncate old data/logs for storage management  
    Truncate = 0x81,
    /// Compact storage and optimize layout
    Compact = 0x82,
    /// Backup data to external storage
    Backup = 0x83,
    /// Restore data from backup
    Restore = 0x84,
    
    // Statistical/ML operations (0xA0-0xBF) - bits 7,5 = 1,1  
    /// Calculate and update ML weights
    CalculateWeight = 0xA0,
    /// Update statistical model parameters
    UpdateStatistics = 0xA1,
    /// Rebuild statistical indexes
    RebuildStatistics = 0xA2,
    /// Synchronize distributed model parameters
    SyncModel = 0xA3,
}

/// Zero-copy operation parsed from binary stream.
/// References data without allocation.
pub struct Operation<'a> {
    /// Type of operation to perform
    pub op_type: OpType,
    /// Target document ID for this operation
    pub target_id: ID16,
    /// Sequence number for operation ordering
    pub sequence: u64,
    /// Raw payload data for the operation
    pub payload: &'a [u8],
}

/// Delta containing multiple operations for atomic application.
/// Designed for efficient network transmission and storage.
pub struct Delta {
    /// Delta metadata
    pub header: DeltaHeader,
    
    /// Serialized operations
    pub data: Bytes,
}

/// Delta header with metadata about the operation batch.
/// Fixed 32-byte structure for efficient network transmission and cache alignment.
#[repr(C, align(32))]
#[derive(Debug, Clone)]
pub struct DeltaHeader {
    /// Unique delta identifier
    pub id: ID8,                            // 8 bytes
    
    /// Creation timestamp (nanoseconds since epoch)
    pub timestamp: u64,                     // 8 bytes
    
    /// Executor/author of these operations
    pub executor_id: ID16,                  // 16 bytes
    
    /// Total size of operation data in bytes
    pub data_size: u32,                     // 4 bytes
    
    /// Number of operations in this delta
    pub op_count: u16,                      // 2 bytes
    
    /// Processing status flags
    pub status: DeltaStatus,                // 1 byte
    
    /// Reserved for future use and alignment
    _padding: u8,                           // 1 byte padding = 32 total
}

/// Processing status for delta operations
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeltaStatus {
    /// Waiting to be processed
    Pending = 0,
    /// Currently being validated
    Validating = 1,
    /// Currently being applied
    Applying = 2,
    /// Successfully applied
    Applied = 3,
    /// Validation failed
    Rejected = 4,
    /// Application failed
    Failed = 5,
}

/// Address of delta data within chunked heap storage
#[derive(Clone, Copy, Debug)]
pub struct ChunkAddress {
    /// Which chunk contains this delta
    pub chunk_id: usize,
    /// Offset within the chunk
    pub offset: usize,
    /// Total length of delta data (header + operations)
    pub length: usize,
}

impl DeltaHeader {
    /// Create a new delta header with proper initialization
    pub fn new(
        id: ID8,
        timestamp: u64,
        executor_id: ID16,
        data_size: u32,
        op_count: u16,
        status: DeltaStatus,
    ) -> Self {
        Self {
            id,
            timestamp,
            executor_id,
            data_size,
            op_count,
            status,
            _padding: 0,
        }
    }

    /// Convert header to bytes for storage
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const DeltaHeader as *const u8,
                std::mem::size_of::<DeltaHeader>()
            )
        }
    }
} 