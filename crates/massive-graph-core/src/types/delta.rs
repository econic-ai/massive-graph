/// Delta type definitions for the Massive Graph system - Empty Shell

/// User's delta in wire format - as received from client
pub struct Delta<'a> {
    wire_bytes: &'a [u8],  // Complete user delta in wire format
}

impl<'a> Delta<'a> {
    /// Extract document ID without parsing (bytes 0-15)
    pub fn doc_id(&self) -> [u8; 16] {
        let mut id = [0u8; 16];
        id.copy_from_slice(&self.wire_bytes[0..16]);
        id
    }
    
    /// Extract schema version (bytes 16-17)
    pub fn schema_version(&self) -> u16 {
        u16::from_le_bytes([self.wire_bytes[16], self.wire_bytes[17]])
    }
    
    /// Get the complete wire bytes for storage
    pub fn as_bytes(&self) -> &[u8] {
        self.wire_bytes
    }
}

/// Delta after being stored with header in chunk
pub struct StoredDelta<'a> {
    chunk_bytes: &'a [u8],     // Points to header + delta in chunk
    // header_len removed - always 16
}

impl<'a> StoredDelta<'a> {
    /// Get header bytes (first 16)
    pub fn header_bytes(&self) -> &[u8] {
        &self.chunk_bytes[0..16]
    }
    
    /// Get user delta bytes (after first 16)
    pub fn delta_bytes(&self) -> &[u8] {
        &self.chunk_bytes[16..]
    }
    
    /// Get complete bytes for propagation
    pub fn as_wire_bytes(&self) -> &[u8] {
        self.chunk_bytes
    }
}


/// Delta operation type - single byte on wire
#[repr(u8)]
pub enum DeltaOp {
    // Property-level operations
    /// Set property value
    Set = 0,
    /// Delete property
    Delete = 1,
    /// Increment numeric value
    Increment = 2,
    /// Append to collection
    Append = 3,
    /// Splice array/string
    Splice = 4,
    /// Insert into collection
    Insert = 5,
    /// Remove from collection
    Remove = 6,
    /// Clear collection
    Clear = 7,
    /// Update slice of data
    SliceUpdate = 8,
    /// Reshape data structure
    Reshape = 9,
    
    // Document-level operations
    /// Create new schema
    CreateSchema = 16,
    /// Create new document
    CreateDocument = 17,
    /// Create document snapshot
    CreateSnapshot = 18,
    /// Delete document
    DeleteDocument = 19,
    /// Add field to document
    AddField = 20,
    /// Remove field from document
    RemoveField = 21,
    /// Add child to document
    AddChild = 22,
    /// Remove child from document
    RemoveChild = 23,
    /// Set parent of document
    SetParent = 24,
    
    // Collaboration operations
    /// Prepend to collection
    Prepend = 32,
    /// Insert at specific position
    InsertAt = 33,
    /// Insert where condition matches
    InsertWhere = 34,
    /// Replace at specific position
    ReplaceAt = 35,
    /// Replace where condition matches
    ReplaceWhere = 36,
    /// Delete at specific position
    DeleteAt = 37,
    /// Delete where condition matches
    DeleteWhere = 38,
    
    // Stream operations
    /// Append to stream
    StreamAppend = 48,
    /// Mark position in stream
    StreamMarkAt = 49,

    // Delta
    /// Delta operations
    Deltas = 64,
}