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
pub mod schema;
/// Stream types
pub mod stream;
/// User types
pub mod user;
/// Field types
pub mod field;
/// Storage types (native only - contains platform-specific code)
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

/// Connection identifier for WebRTC connections
pub type ConnectionId = ids::ID16;

// Re-export commonly used types for convenience
pub use ids::{ID8, ID16, ID32};
// pub use document::{Document, DocumentType, DocumentIndexes, DocumentState};
pub use value::{Value, ValueType};
pub use delta::{Delta, DeltaOp};
pub use schema::{ImmutableSchema, CachedSchemaVersion, SchemaRegistry};
pub use field::{FieldDescriptor, FieldAddress, ParamGroup, FieldParams, ArrayParam, ArrayParamType};
pub use error::ParseError;