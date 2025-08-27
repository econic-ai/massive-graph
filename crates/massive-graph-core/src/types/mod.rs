/// Type definitions for the Massive Graph system
/// 
/// This module contains all type definitions organized by category.

/// Identifier types
pub mod ids;
/// Document-related types
pub mod document;
/// Value-related types  
pub mod value;
/// System-wide error types
pub mod error;
/// Parsing error types
pub mod parse;
/// Delta operation types
pub mod delta;
/// Schema types
pub mod schemas;
/// Stream types
pub mod stream;
/// Storage types (native only - contains platform-specific code)
#[cfg(not(target_arch = "wasm32"))]
pub mod storage;

/// Friendlier type aliases for the Ids
pub type DocId = ids::ID16;

/// Delta identifier
pub type DeltaId = ids::ID8;

/// Version identifier
pub type VersionId = ids::ID8;

/// Stream identifier
pub type StreamId = ids::ID16;

/// User identifier
pub type UserId = ids::ID32;

// Re-export commonly used types for convenience
pub use ids::{ID8, ID16, ID32};
pub use document::{Document, DocumentType, DocumentIndexes, DocumentState};
pub use value::{Value, ValueType};
pub use parse::ParseError;
pub use delta::{StoredDelta, DeltaOp};
pub use schemas::{SchemaVersion, SchemaFamilyId, PropertyId};