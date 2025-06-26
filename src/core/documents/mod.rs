/// Document type implementations for the unified document architecture.
/// 
/// This module provides specialized implementations for different document types
/// while maintaining the unified document model where everything is a document
/// with properties and children.
/// 
/// ## Document Types
/// 
/// - **Root**: Top-level container documents for organizational boundaries
/// - **Binary**: Large binary data storage with streaming capabilities
/// - **Text**: Simple text content with basic properties  
/// - **TextFile**: Collaborative text editing with line-based architecture
/// - **Graph**: Container for nodes and edges in graph structures
/// 
/// ## Design Principles
/// 
/// All document types use the same underlying `Document` structure but provide
/// specialized factory methods, validation, and helper functions for their
/// specific use cases.

pub mod root;
/// Binary data storage
pub mod binary;
/// Text content with basic properties
pub mod text;
/// Collaborative text editing with line-based architecture
pub mod textfile;
/// Container for nodes and edges in graph structures
pub mod graph;

pub use root::*;
pub use binary::*;
pub use text::*;
pub use textfile::*;
pub use graph::*; 