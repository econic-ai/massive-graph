/// Document type definitions for the Massive Graph system
/// 
/// This module contains document-related type definitions aligned with the architectural documentation.

// use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64};
// use crate::{types::Delta, UserId};

use std::sync::atomic::{AtomicPtr};

use crate::{types::{storage::ChunkRef, stream::{DeltaNode, VersionNode, DeltaStream, VersionStream}}, types::ImmutableSchema, UserId};

use super::{DocId, VersionId};


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
#[repr(C)]
pub struct DocumentHeader {
    /// 62base encoded 16 byte identifier
    document_id: DocId,             // 62base encoded 16 byte identifier
    /// Document type for operation validation
    doc_type: DocumentType,         // Document type for operation validation
    /// Reference to immutable header in chunk memory
    first_delta: *const ChunkRef,   // Reference to immutable header in chunk memory
    /// User ID of the owner
    owner_id: UserId,               // User ID of the owner
    /// Unix timestamp of creation
    created_at: u64,                // Unix timestamp of creation    
}

/// Immutable version snapshot (created on the side, stored in chunks)
#[repr(align(64))]  // Cache-line aligned for performance
/// Immutable document version snapshot (fields are read via accessors to avoid warnings).
pub struct DocumentVersion<'a> {
    version_id: VersionId,
    schema_ptr: *const ImmutableSchema,
    wire_version: &'a [u8],
    delta_ref: *const ChunkRef,
    delta_sequence: u64,
}

impl<'a> DocumentVersion<'a> {
    /// Version identifier.
    pub fn id(&self) -> VersionId { self.version_id }
    /// Schema pointer.
    pub fn schema(&self) -> *const ImmutableSchema { self.schema_ptr }
    /// Version bytes.
    pub fn bytes(&self) -> &'a [u8] { self.wire_version }
    /// Delta reference.
    pub fn delta_ref(&self) -> *const ChunkRef { self.delta_ref }
    /// Delta sequence number.
    pub fn delta_seq(&self) -> u64 { self.delta_sequence }
}

/// Queue state - cache-line aligned, only writers touch this
#[repr(align(64))]
/// Tracking of stream cursors for a document (ephemeral; persisted separately for recovery).
pub struct DocumentCursors<'a> {
    /// Cursor for the delta stream traversal.
    pub delta_cursor: AtomicPtr<DeltaNode<'a>>,
    /// Cursor for the version stream traversal.
    pub version_cursor: AtomicPtr<VersionNode<'a>>,
}

/// Runtime document
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct DocumentView<'a> {
    /// Immutable document header from chunk storage.
    header: &'a DocumentHeader,
    /// Heads for per-document streams.
    delta_head: *mut DeltaNode<'a>,
    /// Heads for per-document streams.
    version_head: *mut VersionNode<'a>,
    /// Cursors for resumable traversal.
    cursors: DocumentCursors<'a>,
    /// Current version pointer (optional; may be set by version application pipeline).
    current_version: AtomicPtr<DocumentVersion<'a>>,
}

impl<'a> DocumentView<'a> {
    /// Create a new runtime view into a document's immutable header and stream heads.
    pub fn new(header: &'a DocumentHeader, delta_head: *mut DeltaNode<'a>, version_head: *mut VersionNode<'a>) -> Self {
        Self {
            header,
            delta_head,
            version_head,
            cursors: DocumentCursors { delta_cursor: AtomicPtr::new(delta_head), version_cursor: AtomicPtr::new(version_head) },
            current_version: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Header accessor.
    pub fn header(&self) -> &'a DocumentHeader { self.header }

    /// Build next delta batch into an existing vector for capacity reuse; updates cursor.
    pub fn build_next_delta_batch_into(&self, stream: &DeltaStream<'a>, max_scan: usize, out: &mut Vec<*mut DeltaNode<'a>>) {
        let start = self.cursors.delta_cursor.load(core::sync::atomic::Ordering::Acquire);
        let next = stream.build_next_batch_into(start, max_scan, out);
        self.cursors.delta_cursor.store(next, core::sync::atomic::Ordering::Release);
    }

    /// Build next version batch into an existing vector for capacity reuse; updates cursor.
    pub fn build_next_version_batch_into(&self, stream: &VersionStream<'a>, max_scan: usize, out: &mut Vec<*mut VersionNode<'a>>) {
        let start = self.cursors.version_cursor.load(core::sync::atomic::Ordering::Acquire);
        let next = stream.build_next_batch_into(start, max_scan, out);
        self.cursors.version_cursor.store(next, core::sync::atomic::Ordering::Release);
    }

    /// Get current delta cursor pointer.
    pub fn delta_cursor(&self) -> *mut DeltaNode<'a> { self.cursors.delta_cursor.load(core::sync::atomic::Ordering::Acquire) }
    /// Get current version cursor pointer.
    pub fn version_cursor(&self) -> *mut VersionNode<'a> { self.cursors.version_cursor.load(core::sync::atomic::Ordering::Acquire) }
}

