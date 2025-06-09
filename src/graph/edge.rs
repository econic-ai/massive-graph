//! Graph edge implementation

use crate::core::types::{EdgeId, NodeId, Properties, Label};
use std::collections::HashMap;

/// Graph edge
pub struct Edge {
    /// Unique edge identifier
    pub id: EdgeId,
    /// Source node
    pub from: NodeId,
    /// Target node
    pub to: NodeId,
    /// Edge label
    pub label: Option<Label>,
    /// Edge properties
    pub properties: Properties,
}

impl Edge {
    /// Create a new edge
    pub fn new(id: EdgeId, from: NodeId, to: NodeId) -> Self {
        Self {
            id,
            from,
            to,
            label: None,
            properties: HashMap::new(),
        }
    }
}
