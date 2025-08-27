/// Document type definitions for the Massive Graph system
/// 
/// This module contains document-related type definitions aligned with the architectural documentation.

use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64};
use crate::types::{DocId, VersionId};
use super::schemas::{Schema, SchemaFamilyId, PropertyId, PatternEntry};
use super::stream::{AppendOnlyStream, Node};
use arc_swap::ArcSwap;
use im::OrdMap;


/// Generic versioned tree that can be specialized for different uses
pub struct VersionedTree<K: Ord + Clone, V: Clone> {
    root: ArcSwap<OrdMap<K, V>>,
}

/// Specialize for different use cases via type aliases
pub type DocumentTree = VersionedTree<Path, Value>;
pub type SchemaTree = VersionedTree<PropertyPath, EncodingId>;
pub type PieceTable<T> = VersionedTree<Range<usize>, Piece<T>>;

/// Document type identifiers
#[repr(u8)]
#[derive(Debug)]
pub enum DocumentType {
    // Root document type
    Tree = 0,         // The default document type, used in every document
    Graph = 1,        // A Graph structure
    StateGraph = 2,   // State document (DAG Graph)
    
    // Binary documents
    Binary = 16,      // Raw binary data
    Image = 17,       // Binary with image metadata
    Video = 18,       // Binary with video metadata
    Audio = 19,       // Binary with audio metadata
    
    // Flat Data structures
    Matrix = 34,      // Matrix value type
    Tensor = 33,      // Tensor value type
    Table = 35,       // Collection of rows

    // Text Structured documents
    JSON = 48,        // Stored as Map
    XML = 49,         // Stored with schema
    YAML = 50,        // Stored as Map

    // Text Semistructured documents (BTree piecetables)
    Markdown = 64,    // String with MD structure
    PlainText = 65,   // Simple string
    Code = 66,        // String with language hint
    
    // Append-only streams (linked lists)
    TextStream = 80,      // (text)
    BinaryStream = 81,    // (binary)
    DeltaStream = 82,     // (delta)
    DocumentStream = 83,  // (document)
    EventStream = 84,     // (events)
}

/// Document metadata structure
/// Persistent document header (immutable, stored in chunks)
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
pub struct DocumentState {
    last_delta: AtomicPtr<Node>,    // Last delta in stream
    pending_count: AtomicU32,           // Number of unapplied deltas
    is_processing: AtomicBool,          // Currently generating snapshot       
    last_updated: AtomicU64,            // Last update timestamp
}

/// Document metadata for future extensions
pub struct DocumentIndexes {
    // Indexes for the document
    cached_property_patterns: [Option<PatternEntry>; 8],  // Hot path pattern cache
    cached_property_ids: [PropertyId; 8],            // Hot path property ID cache        
}


/// Runtime document
pub struct Document {
    header: *const DocumentHeader,              // Reference to immutable header in chunk memory
    current_version: AtomicPtr<DocumentVersion>,// Current version - single atomic pointer for lock-free reads
    state: DocumentState,                       // Queue state - only writers touch this
    indexes: DocumentIndexes,                   // Indexes for the document

    root: DocumentTree,                         // Document data

    
}

