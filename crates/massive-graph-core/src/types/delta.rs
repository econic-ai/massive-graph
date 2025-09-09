use crate::{types::{field::FieldAddress, ParseError}, DeltaId, DocId};

/// Delta type definitions for the Massive Graph system - Empty Shell

/// Delta operation type - single byte on wire
#[repr(u8)]
pub enum DeltaOp {

    // Atomic Property-level operations
    /// Set property value
    Set = 0,
    /// Delete property
    Delete = 1,
    /// Increment numeric value
    Increment = 2,
    /// Multiply numeric value
    Multiply = 3,
    /// Modulus numeric value
    Modulus = 5,
    /// Power numeric value
    Power = 6,

    // Collection-level operations
    /// Append to collection
    Append = 8,
    /// Splice array/string
    Splice = 9,
    /// Insert into collection
    Insert = 10 ,
    /// Remove from collection
    Remove = 11,
    /// Clear collection
    Clear = 12,
    /// Update slice of data
    SliceUpdate = 13,
    /// Reshape data structure
    Reshape = 14,
    
    // Document-level operations
    /// Create new schema
    CreateSchema = 16,
    /// Create new document
    CreateDocument = 17,
    /// Create document snapshot
    CreateSnapshot = 18,
    /// Delete document
    DeleteDocument = 19,

    // Relationships-level operations

    /// Add child to document
    AddChild = 24,
    /// Remove child from document
    RemoveChild = 25,
    /// Set parent of document
    SetParent = 26,
    
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

    // Delta Operations

    /// Apply group of deltas (atomically)
    Deltas = 64,
}

/// Raw delta bytes in wire format
#[repr(C)]
pub struct DeltaRaw {
    /// The raw bytes of the delta
    pub bytes: [u8]
}

/// Delta operation containing document changes
pub struct Delta {
    // 32 byte header
    /// Document identifier (16 bytes)
    pub doc_id: DocId,
    /// Delta identifier (8 bytes)
    pub delta_id: DeltaId,
    /// Timestamp when delta was created (8 bytes)
    pub timestamp: u64,

    // 6 bytes minimum
    /// Length of the payload data
    pub payload_len: u32,
    /// Type of delta operation (1 byte)
    pub delta_op: DeltaOp,
    /// Field address with parameters (1-3 bytes + params)
    pub field_address: FieldAddress,
    /// Pointer to payload data and its length
    pub payload: (*const u8, usize),
}

/// Parse from wire bytes using same varint/TLV encoding as SchemaRegistry
/// SAFETY: Caller ensures bytes remain valid for Delta's lifetime
impl Delta {
    /// Parse delta from wire format bytes
    pub fn from_wire_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        // Single upfront check for minimum required bytes

        if bytes.len() < 38 {
            return Err(ParseError::InsufficientData { 
                expected: 38, 
                actual: bytes.len() 
            });
        }
        
        // Now we can use unsafe for speed, knowing we have enough bytes
        unsafe {
            let ptr = bytes.as_ptr();
            
            // Direct memory casts - no bounds checks needed
            let doc_id = *(ptr as *const DocId);
            let delta_id = *(ptr.add(16) as *const DeltaId);
            let timestamp = *(ptr.add(24) as *const u64);
            let delta_op = match *ptr.add(32) {
                0..=64 => std::mem::transmute(*ptr.add(32)),
                op => return Err(ParseError::InvalidOperation(op)),
            };
            
            // Parse varints (these still need bounds checking)
            let mut offset = 33;
            
            // Use unchecked slicing for varints since we validate length
            let (schema_version, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
            offset += consumed;
            
            let (field_index, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
            offset += consumed;
            
            // Params count
            if offset >= bytes.len() {
                return Err(ParseError::InsufficientData { expected: offset + 1, actual: bytes.len() });
            }
            let params_count = *ptr.add(offset);
            offset += 1;
            
            // Mark params section
            let params_start = offset;
            let params_ptr = ptr.add(offset);
            
            // Fast skip over TLV params
            for _ in 0..params_count {
                if offset >= bytes.len() {
                    return Err(ParseError::InsufficientData { expected: offset + 1, actual: bytes.len() });
                }
                offset += 1; // Skip type byte
                
                let (length, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
                offset += consumed + length as usize;
                
                if offset > bytes.len() {
                    return Err(ParseError::InsufficientData { expected: offset, actual: bytes.len() });
                }
            }
            
            let params_len = offset - params_start;
            
            // Payload length
            let (payload_len, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
            offset += consumed;
            
            // Final check for payload
            if bytes.len() < offset + payload_len as usize {
                return Err(ParseError::InsufficientData { 
                    expected: offset + payload_len as usize, 
                    actual: bytes.len() 
                });
            }
            
            let payload_ptr = ptr.add(offset);
            
            Ok(Delta {
                doc_id,
                delta_id,
                delta_op,
                timestamp,
                field_address: FieldAddress {
                    schema_version: schema_version as u16,
                    field_index,
                    params_raw: (params_ptr, params_len),
                },
                payload_len,
                payload: (payload_ptr, payload_len as usize),
            })
        }
    }

}

/// Fast varint decoder with minimal checking
#[inline(always)]
unsafe fn decode_varint_unchecked(ptr: *const u8, max_len: usize) -> Result<(u32, usize), ParseError> {
    if max_len == 0 {
        return Err(ParseError::InsufficientData { expected: 1, actual: 0 });
    }
    
    let first = *ptr;
    if first < 128 {
        Ok((first as u32, 1))
    } else if first < 192 && max_len >= 2 {
        Ok((((first & 0x3F) as u32) << 8 | *ptr.add(1) as u32, 2))
    } else if max_len >= 3 {
        Ok((((first & 0x3F) as u32) << 16 | (*ptr.add(1) as u32) << 8 | *ptr.add(2) as u32, 3))
    } else {
        Err(ParseError::InsufficientData { expected: 3, actual: max_len })
    }
}