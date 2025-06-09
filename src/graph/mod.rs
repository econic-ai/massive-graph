//! Graph data structures and operations
//! 
//! This module contains all graph-related functionality including nodes, edges,
//! change tracking, indexing, and conflict resolution.

pub mod node;
pub mod edge;
pub mod delta;
pub mod index;
pub mod merge;

// Re-export main graph types
pub use node::Node;
pub use edge::Edge;
pub use delta::Delta;

// Add a main graph module (renamed from graph.rs)
mod graph_ops;
pub use graph_ops::*; 