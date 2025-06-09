//! Graph node implementation

use crate::core::types::{NodeId, Properties, Label};
use std::collections::HashMap;

/// Graph node
pub struct Node {
    /// Unique node identifier
    pub id: NodeId,
    /// Node label
    pub label: Option<Label>,
    /// Node properties
    pub properties: Properties,
}

impl Node {
    /// Create a new node
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            label: None,
            properties: HashMap::new(),
        }
    }
}
