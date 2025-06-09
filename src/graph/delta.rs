//! Delta operations for real-time synchronization

use crate::core::types::{NodeId, EdgeId, Timestamp, Version};
use serde::{Deserialize, Serialize};

/// Delta operation representing a change to the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Unique delta identifier
    pub id: String,
    /// Timestamp when delta was created
    pub timestamp: Timestamp,
    /// Version number
    pub version: Version,
    /// The actual operation
    pub operation: Operation,
}

/// Graph operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Create a new node
    CreateNode {
        id: NodeId,
    },
    /// Update node properties
    UpdateNode {
        id: NodeId,
    },
    /// Delete a node
    DeleteNode {
        id: NodeId,
    },
    /// Create a new edge
    CreateEdge {
        id: EdgeId,
        from: NodeId,
        to: NodeId,
    },
    /// Update edge properties
    UpdateEdge {
        id: EdgeId,
    },
    /// Delete an edge
    DeleteEdge {
        id: EdgeId,
    },
}

impl Delta {
    /// Create a new delta
    pub fn new(operation: Operation) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Timestamp::now(),
            version: Version::initial(),
            operation,
        }
    }
}
