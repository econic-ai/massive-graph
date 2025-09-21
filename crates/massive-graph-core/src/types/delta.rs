use std::{cell::OnceCell, sync::atomic::{AtomicU16, Ordering}};

use crate::{types::{field::FieldAddress, storage::WireFormat, ParseError}, DeltaId, DocId, UserId};

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


/// Incoming delta bytes in wire format
#[repr(C)]
pub struct DeltaIncoming<'a> {
    /// The raw bytes of the unvalidated delta
    pub bytes: &'a [u8],
    /// The client-provided header
    pub client_header: OnceCell<&'a DeltaClientHeader>,
    /// The delta operation
    pub delta_payload: OnceCell<&'a DeltaPayload<'a>>,
}

/// The client-provided header
#[repr(C, align(8))]  // Cache-line aligned for performance
pub struct DeltaClientHeader {
    /// The content hash for delta short-window identification, deduplication and validation
    pub content_hash: u64,          // 8 bytes
    // // The client-provided sequence number for delta deduplication
    // pub client_sequence: u64,
    // 8 bytes

}

/// The delta operation
#[repr(C)]
pub struct DeltaPayload<'a> {
    /// delta size
    pub payload_len: u32,          // 4 bytes
    /// The delta operation type
    pub delta_op: DeltaOp,        // 1 byte
    /// The field address with parameters
    pub field_address: FieldAddress, // Variable length        
    /// The payload data
    pub payload_value: &'a [u8], // variable length
}

/// Server generated delta for ordering and validation guarantees
#[repr(C, align(128))]  // Cache-line aligned for performance
pub struct DeltaSecureHeader {
    /// The user id
    pub user_id: UserId,                // 32 bytes
    /// The document identifier
    pub doc_id: DocId,                  // 16 bytes
    /// The delta identifier
    pub delta_id: DeltaId,              // 8 bytes
    /// The previous delta identifier
    pub previous_delta_id: DeltaId,     // 8 bytes
    /// DElta sequence number (document bound)
    pub delta_sequence_number: u64,            // 8 bytes
    /// When delta was created
    pub timestamp: u64,                 // 8 bytes
    /// The delta signature - BLAKE3 MAC
    pub signature: [u8; 32],            // 32 bytes
    /// padding
    pub padding: [u8; 16],              // 16 bytes
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// Bitflags for delta validation/state tracking.
pub struct DeltaFlags(pub u16);

impl DeltaFlags {
    /// No flags set.
    pub const EMPTY: DeltaFlags = DeltaFlags(0);
    /// Delta has been validated.
    pub const VALID: DeltaFlags = DeltaFlags(0b0000_0001);
    /// Signature (MAC) verified.
    pub const SIGNED: DeltaFlags = DeltaFlags(0b0000_0010);
    /// Delta was deduplicated.
    pub const DEDUPED: DeltaFlags = DeltaFlags(0b0000_0100);
    /// Delta was processed.
    pub const PROCESSED: DeltaFlags = DeltaFlags(0b0000_1000);    
    /// Delta was processed.
    pub const RETRIED: DeltaFlags = DeltaFlags(0b0001_0000);
    /// Delta was processed.
    pub const FLAG1: DeltaFlags = DeltaFlags(0b0010_0000);    
    /// Delta was processed.
    pub const FLAG2: DeltaFlags = DeltaFlags(0b0100_0000);        
    /// Delta was processed.
    pub const FLAG3: DeltaFlags = DeltaFlags(0b1000_0000);            

    /// Get raw bits.
    pub const fn bits(self) -> u16 { self.0 }
    /// Construct from raw bits.
    pub const fn from_bits(bits: u16) -> Self { DeltaFlags(bits) }
    /// Check whether all bits in mask are set.
    pub const fn contains(self, mask: DeltaFlags) -> bool { (self.bits() & mask.bits()) == mask.bits() }
}

/// Mutable tracking state for delta flags; uses atomics for optional cross-thread updates.
pub struct DeltaTracking {
    /// Mutable tracking flags (thread-safe when shared across threads).
    flags: AtomicU16,
}
impl DeltaTracking {
    /// Create with no flags set.
    pub fn new() -> Self { Self { flags: AtomicU16::new(DeltaFlags::EMPTY.bits()) } }
    /// Read current flags.
    pub fn get(&self) -> DeltaFlags { DeltaFlags::from_bits(self.flags.load(Ordering::Relaxed)) }
    /// Overwrite flags.
    pub fn set(&self, v: DeltaFlags) { self.flags.store(v.bits(), Ordering::Release) }
    /// Set bits by mask.
    pub fn set_bits(&self, mask: DeltaFlags) { self.flags.fetch_or(mask.bits(), Ordering::AcqRel); }
    /// Clear bits by mask.
    pub fn clear_bits(&self, mask: DeltaFlags) { self.flags.fetch_and(!mask.bits(), Ordering::AcqRel); }
}

/// Delta Ref
pub struct Delta<'a> {
    /// The raw bytes
    pub bytes: &'a [u8],
}

impl<'a> WireFormat<'a> for Delta<'a> {

    fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn to_bytes(&self) -> &[u8] {
        self.bytes
    }
}

/// Delta operation containing document changes
pub struct DeltaRef<'a> {

    /// raw bytes
    bytes: &'a [u8],

    /// The mutable flags
    state: DeltaTracking,

    /// The secured header
    server_header: &'a DeltaSecureHeader,

    /// The delta Client header
    client_header: &'a DeltaClientHeader,

    /// The field address
    /// The delta payload
    payload: &'a DeltaPayload<'a>,

}

impl<'a> DeltaRef<'a> {
    // Read-only accessors for borrowed data
    /// Get raw bytes.
    pub fn bytes(&self) -> &'a [u8] { self.bytes }

    /// Get secured header.
    pub fn server_header(&self) -> &'a DeltaSecureHeader { self.server_header }
    /// Get client header.
    pub fn client(&self) -> &'a DeltaClientHeader { self.client_header }
    /// Get payload view.
    pub fn payload(&self) -> &'a DeltaPayload<'a> { self.payload }

    // Mutators operate only on state via &self
    /// Current flags.
    pub fn flags(&self) -> DeltaFlags { self.state.get() }
    /// Overwrite flags.
    pub fn set_flags(&self, f: DeltaFlags) { self.state.set(f) }
    /// Mark the delta as validated.
    pub fn mark_as_valid(&self) { self.state.set_bits(DeltaFlags::VALID) }
}

/// Parse from wire bytes using same varint/TLV encoding as SchemaRegistry
/// SAFETY: Caller ensures bytes remain valid for Delta's lifetime
// impl Delta<'a> {
    // /// Parse delta from wire format bytes
    // pub fn from_wire_bytes(bytes: &'a [u8]) -> Result<Self, ParseError> {
    //     // Single upfront check for minimum required bytes

    //     if bytes.len() < 38 {
    //         return Err(ParseError::InsufficientData { 
    //             expected: 38, 
    //             actual: bytes.len() 
    //         });
    //     }
        
    //     // Now we can use unsafe for speed, knowing we have enough bytes
    //     unsafe {
    //         let ptr = bytes.as_ptr();
            
    //         // Direct memory casts - no bounds checks needed
    //         let doc_id = *(ptr as *const DocId);
    //         let delta_id = *(ptr.add(16) as *const DeltaId);
    //         let timestamp = *(ptr.add(24) as *const u64);
    //         let delta_op = match *ptr.add(32) {
    //             0..=64 => std::mem::transmute(*ptr.add(32)),
    //             op => return Err(ParseError::InvalidOperation(op)),
    //         };
            
    //         // Parse varints (these still need bounds checking)
    //         let mut offset = 33;
            
    //         // Use unchecked slicing for varints since we validate length
    //         let (schema_version, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
    //         offset += consumed;
            
    //         let (field_index, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
    //         offset += consumed;
            
    //         // Params count
    //         if offset >= bytes.len() {
    //             return Err(ParseError::InsufficientData { expected: offset + 1, actual: bytes.len() });
    //         }
    //         let params_count = *ptr.add(offset);
    //         offset += 1;
            
    //         // Mark params section
    //         let params_start = offset;
    //         let params_ptr = ptr.add(offset);
            
    //         // Fast skip over TLV params
    //         for _ in 0..params_count {
    //             if offset >= bytes.len() {
    //                 return Err(ParseError::InsufficientData { expected: offset + 1, actual: bytes.len() });
    //             }
    //             offset += 1; // Skip type byte
                
    //             let (length, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
    //             offset += consumed + length as usize;
                
    //             if offset > bytes.len() {
    //                 return Err(ParseError::InsufficientData { expected: offset, actual: bytes.len() });
    //             }
    //         }
            
    //         let params_len = offset - params_start;
            
    //         // Payload length
    //         let (payload_len, consumed) = decode_varint_unchecked(ptr.add(offset), bytes.len() - offset)?;
    //         offset += consumed;
            
    //         // Final check for payload
    //         if bytes.len() < offset + payload_len as usize {
    //             return Err(ParseError::InsufficientData { 
    //                 expected: offset + payload_len as usize, 
    //                 actual: bytes.len() 
    //             });
    //         }
            
    //         let payload_ptr = ptr.add(offset);
            
    //         Ok(Delta {
    //             doc_id,
    //             delta_id,
    //             delta_op,
    //             timestamp,
    //             field_address: FieldAddress {
    //                 schema_version: schema_version as u16,
    //                 field_index,
    //                 params_raw: (params_ptr, params_len),
    //             },
    //             payload_len,
    //             payload: (payload_ptr, payload_len as usize),
    //         })
    //     }
    // }

// }

/// Fast varint decoder with minimal checking
#[inline(always)]
#[allow(dead_code)]
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