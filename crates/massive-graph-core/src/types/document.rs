/// Document type definitions for the Massive Graph system
/// 
/// This module contains document-related type definitions aligned with the architectural documentation.

use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64};
use super::{DocId, VersionId};
use super::schemas::{Schema, SchemaFamilyId, PropertyId, PatternEntry};
use super::stream::{AppendOnlyStream, Node};

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

/// Document metadata structure
/// Persistent document header (immutable, stored in chunks)
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct DocumentHeader {
    document_id: DocId,             // 62base encoded 16 byte identifier
    doc_type: DocumentType,         // Document type for operation validation
    schema_family: SchemaFamilyId,  // Schema family for property resolution
    deltas: AppendOnlyStream,       // Stream of all deltas since creation
    versions: AppendOnlyStream,     // Stream of wire format snapshots
    created_at: u64,                // Unix timestamp of creation
}

/// Immutable version snapshot (created on the side, stored in chunks)
#[repr(align(64))]  // Cache-line aligned for performance
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct DocumentVersion {
    version_id: VersionId,                  // Unique version identifier
    schema_ptr: *const Schema,              // Schema for this version
    schema_version: u16,                    // Schema version number
    wire_version: *const u8,                // Direct pointer to wire format in chunk
    wire_size: u32,                         // Size of wire format
    delta_ref: *const Node,             // Delta this version represents
    delta_sequence: u64,                    // Sequence number of delta
}

/// Queue state - cache-line aligned, only writers touch this
#[repr(align(64))]
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct DocumentState {
    last_delta: AtomicPtr<Node>,    // Last delta in stream
    pending_count: AtomicU32,           // Number of unapplied deltas
    is_processing: AtomicBool,          // Currently generating snapshot       
    last_updated: AtomicU64,            // Last update timestamp
}

/// Document metadata for future extensions
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct DocumentIndexes {
    // Indexes for the document
    cached_property_patterns: [Option<PatternEntry>; 8],  // Hot path pattern cache
    cached_property_ids: [PropertyId; 8],            // Hot path property ID cache        
}


/// Runtime document
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct Document {
    header: *const DocumentHeader,              // Reference to immutable header in chunk memory
    current_version: AtomicPtr<DocumentVersion>,// Current version - single atomic pointer for lock-free reads
    state: DocumentState,                       // Queue state - only writers touch this
    indexes: DocumentIndexes,                   // Indexes for the document
    
}

