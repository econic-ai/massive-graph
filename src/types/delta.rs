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
    Set = 0,
    Delete = 1,
    Increment = 2,
    Append = 3,
    Splice = 4,
    Insert = 5,
    Remove = 6,
    Clear = 7,
    SliceUpdate = 8,
    Reshape = 9,
    
    // Document-level operations
    CreateSchema = 16,    
    CreateDocument = 17,
    CreateSnapshot = 18,
    DeleteDocument = 19,
    AddField = 20,
    RemoveField = 21,
    AddChild = 22,
    RemoveChild = 23,
    SetParent = 24,
    
    // Collaboration operations
    Prepend = 32,
    InsertAt = 33,
    InsertWhere = 34,
    ReplaceAt = 35,
    ReplaceWhere = 36,
    DeleteAt = 37,
    DeleteWhere = 38,
    
    // Stream operations
    StreamAppend = 48,
    StreamMarkAt = 49,

    // Delta
    Deltas = 64,
}