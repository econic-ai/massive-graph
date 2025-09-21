/// Document type definitions for the Massive Graph system
/// 
/// This module contains document-related type definitions aligned with the architectural documentation.

// use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64};
// use crate::{types::Delta, UserId};

// use std::sync::atomic::{AtomicPtr};

use crate::{types::{storage::{ WireFormat }}, UserId};

use super::{DocId};

/// Document type identifiers
#[repr(u8)]
#[derive(Debug)]
pub enum DocumentType {
    // Root document type
    /// The default document type, used in every document
    Tree = 0,
    /// A Graph structure
    Graph = 1,
    /// State document (DAG Graph)
    StateGraph = 2,
    /// Schema
    Schema = 3,
    
    // Binary documents
    /// Raw binary data
    Binary = 16,
    /// Binary with image metadata
    Image = 17,
    /// Binary with video metadata
    Video = 18,
    /// Binary with audio metadata
    Audio = 19,
    
    // Flat Data structures
    /// Matrix value type
    Matrix = 34,
    /// Tensor value type
    Tensor = 33,
    /// Collection of rows
    Table = 35,

    // Text Structured documents
    /// Stored as Map
    JSON = 48,
    /// Stored with schema
    XML = 49,
    /// Stored as Map
    YAML = 50,

    // Text Semistructured documents (BTree piecetables)
    /// String with MD structure
    Markdown = 64,
    /// Simple string
    PlainText = 65,
    /// String with language hint
    Code = 66,
    
    // Append-only streams (linked lists)
    /// Text stream
    TextStream = 80,
    /// Binary stream
    BinaryStream = 81,
    /// Delta stream
    DeltaStream = 82,
    /// Document stream
    DocumentStream = 83,
    /// Event stream
    EventStream = 84,
}

// Document metadata structure
/// Persistent document header (immutable, stored in chunks)
/// Keeps reference to raw bytes but parses all values upfront
#[derive(Debug)]
#[repr(C, align(64))]
pub struct DocumentHeader<'a> {
    /// Raw bytes backing this header (zero-copy borrow)
    raw_bytes: &'a [u8],
    /// Document identifier
    doc_id: DocId,              // 16 bytes
    /// Document type
    doc_type: DocumentType,     // 1 byte
    /// Owner user ID
    owner_id: UserId,           // 16 bytes
    /// Creation timestamp
    created_at: u64,            // 8 bytes
    /// padding
    padding: [u8; 25],          // 25 bytes

}

impl<'a> DocumentHeader<'a> {

    /// Create a new document header
    pub fn new(doc_id: DocId, doc_type: DocumentType, owner_id: UserId, created_at: u64) -> Self {
        Self {
            raw_bytes: &[],
            doc_id,
            doc_type,
            owner_id,
            created_at,
            padding: [0; 25]
        }
    }

    /// Document ID
    pub fn doc_id(&self) -> &DocId {
        &self.doc_id
    }
    
    /// Document type
    pub fn doc_type(&self) -> &DocumentType {
        &self.doc_type
    }
    
    /// Owner ID
    pub fn owner_id(&self) -> &UserId {
        &self.owner_id
    }
    
    /// Created at timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }
    
    /// Get the raw bytes backing this header
    pub fn raw_bytes(&self) -> &'a [u8] {
        self.raw_bytes
    }
}

impl Default for DocumentHeader<'_> {
    fn default() -> Self {
        let doc_id = DocId::default();
        let doc_type = DocumentType::Tree;
        let owner_id = UserId::default();
        let created_at = 0;
        let padding = [0; 25];

        DocumentHeader::new(doc_id, doc_type, owner_id, created_at)
    }
}

impl<'a> WireFormat<'a> for DocumentHeader<'a> {
    fn from_bytes(raw_bytes: &'a [u8]) -> Self {
        // Parse all values from the wire format using direct offsets
        // Skip wire version at offset 0 (2 bytes)
        let doc_id = unsafe {
            let ptr = raw_bytes[2..].as_ptr() as *const DocId;
            *ptr
        };
        
        let doc_type = unsafe {
            let byte = *raw_bytes.get(18).unwrap();
            std::mem::transmute(byte)
        };
        
        let owner_id = unsafe {
            let ptr = raw_bytes[19..].as_ptr() as *const UserId;
            *ptr
        };
        
        let created_at = unsafe {
            let ptr = raw_bytes[35..].as_ptr() as *const u64;
            *ptr
        };
        
        Self {
            raw_bytes,
            doc_id,
            doc_type,
            owner_id,
            created_at,
            padding: [0; 25],
        }

    }

    fn to_bytes(&self) -> &[u8] {
        self.raw_bytes
    }

}

// /// Immutable version snapshot (created on the side, stored in chunks)
// #[repr(align(64))]  // Cache-line aligned for performance
// /// Immutable document version snapshot (fields are read via accessors to avoid warnings).
// pub struct DocumentVersion<'a> {
//     wire_version: &'a [u8],
//     version_id: VersionId,
//     schema_ptr: *const ImmutableSchema,
//     delta_ref: *const ChunkRef<DeltaStreamChunk>,
//     delta_sequence: u64,
// }

// impl<'a> DocumentVersion<'a> {
//     /// Version identifier.
//     pub fn id(&self) -> VersionId { self.version_id }
//     /// Schema pointer.
//     pub fn schema(&self) -> *const ImmutableSchema { self.schema_ptr }
//     /// Version bytes.
//     pub fn bytes(&self) -> &'a [u8] { self.wire_version }
//     /// Delta reference.
//     pub fn delta_ref(&self) -> *const ChunkRef<DeltaStreamChunk> { self.delta_ref }
//     /// Delta sequence number.
//     pub fn delta_seq(&self) -> u64 { self.delta_sequence }
// }

// impl<'a> WireFormat<'a> for DocumentVersion<'a> {
//     fn from_bytes(bytes: &'a [u8]) -> Self {
//         // TODO: Implement actual parsing logic
//         Self {
//             wire_version: bytes,
//             version_id: VersionId::random(),
//             schema_ptr: std::ptr::null(),
//             delta_ref: std::ptr::null(),
//             delta_sequence: 0,
//         }
//     }

//     fn to_bytes(&self) -> &[u8] {
//         self.wire_version
//     }
// }



// /// Runtime document
// #[allow(dead_code)] // POC: Fields will be used in future implementation
// pub struct DocumentRef {
//     /// A chunk reference to the original document header
//     header: ChunkRef<DocumentHeaderChunk>,

//     /// A chunk reference to the current version
//     current_version: ChunkRef<DocumentVersionChunk>,
    
//     /// A chunk reference to the delta stream
//     delta_stream: ChunkRef<DeltaStreamChunk>,
    
//     /// A chunk reference to the version stream
//     version_stream: ChunkRef<VersionStreamChunk>,

// }

// impl DocumentRef {
//     /// Create a new runtime view into a document's immutable header and stream heads.
//     // pub fn new(header: &'a DocumentHeader<'a>, delta_head: *mut DeltaNode<'a>, version_head: *mut VersionNode<'a>) -> Self {
//     //     Self {
//     //         header,
//     //         delta_head,
//     //         version_head,
//     //         cursors: DocumentCursors { delta_cursor: AtomicPtr::new(delta_head), version_cursor: AtomicPtr::new(version_head) },
//     //         current_version: AtomicPtr::new(core::ptr::null_mut()),
//     //     }
//     // }

//     /// Header accessor.
//     // pub fn header(&self) -> &'a DocumentHeader<'a> { self.header }




//     /// Convert this view to bytes for storage/compat shims (temporary stub).
//     pub fn to_bytes(&self) -> Vec<u8> {
//         Vec::new()
//     }
// }

