/// Graph document implementation for graph data structures.
/// 
/// Graph documents serve as containers for graph structures, holding collections
/// of nodes and edges as child documents. They use the unified document model
/// where everything is a document, making graphs just another document type
/// with specific organizational patterns.
/// 
/// ## Architecture
/// 
/// Graph documents act as root containers that:
/// - Store metadata about the graph (name, type, layout)
/// - Reference child documents that are nodes and edges
/// - Provide organizational structure for graph traversal
/// - Enable efficient type-based filtering of children
/// 
/// ## Structure
/// 
/// ```
/// Graph Document {
///     doc_type: DocumentType::Graph,
///     properties: {
///         "name" -> "Knowledge Graph",
///         "graph_type" -> "directed",
///         "layout_algorithm" -> "force_directed",
///         "node_count" -> 150,
///         "edge_count" -> 300,
///     },
///     children: [node1_id, node2_id, edge1_id, edge2_id, ...]
/// }
/// 
/// Node Document {
///     doc_type: DocumentType::Node,
///     properties: {
///         "label" -> "Person",
///         "name" -> "Alice",
///         "x" -> 100.0,
///         "y" -> 200.0,
///         "z" -> 0.0,
///         "weight" -> 0.8,
///     },
///     children: [] // Nodes typically have no children
/// }
/// 
/// Edge Document {
///     doc_type: DocumentType::Edge,
///     properties: {
///         "label" -> "knows",
///         "weight" -> 0.7,
///         "directed" -> true,
///     },
///     children: [source_node_id, target_node_id] // Edge connects these nodes
/// }
/// ```
/// 
/// ## Example Usage
/// 
/// ```rust
/// // Create a new graph
/// let graph = GraphDocument::new("Social Network", "directed");
/// 
/// // Add nodes to the graph
/// let alice_id = NodeDocument::create_person_node("Alice", 100.0, 200.0);
/// let bob_id = NodeDocument::create_person_node("Bob", 300.0, 150.0);
/// GraphDocument::add_child(&mut graph, alice_id);
/// GraphDocument::add_child(&mut graph, bob_id);
/// 
/// // Add edge between nodes
/// let edge_id = EdgeDocument::create_relationship_edge("knows", 0.8, true);
/// EdgeDocument::connect_nodes(&mut edge, alice_id, bob_id);
/// GraphDocument::add_child(&mut graph, edge_id);
/// ```

use crate::core::types::document::{Value, AdaptiveMap};

/// Builder for creating Graph documents that serve as containers for graph structures.
pub struct GraphDocument;

impl GraphDocument {
    /// Create a new Graph document.
    /// 
    /// This creates a graph container with metadata properties and an empty
    /// children list ready for nodes and edges to be added.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Display name for the graph
    /// * `graph_type` - Type of graph ("directed", "undirected", "mixed")
    /// 
    /// # Returns
    /// 
    /// Properties map that can be used to create a Document with DocumentType::Graph
    pub fn new(name: &str, graph_type: &str) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Set core graph metadata
        properties.insert("name".to_string(), Value::String(name.to_string()));
        properties.insert("graph_type".to_string(), Value::String(graph_type.to_string()));
        properties.insert("node_count".to_string(), Value::U32(0));
        properties.insert("edge_count".to_string(), Value::U32(0));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        // Set default layout properties
        properties.insert("layout_algorithm".to_string(), Value::String("force_directed".to_string()));
        properties.insert("auto_layout".to_string(), Value::Boolean(true));
        
        properties
    }
    
    /// Create a new empty graph with default settings.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Display name for the graph
    /// 
    /// # Returns
    /// 
    /// Properties map for a directed graph with default settings
    pub fn new_simple(name: &str) -> AdaptiveMap<String, Value> {
        Self::new(name, "directed")
    }
    
    /// Set the layout algorithm for the graph.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to graph properties
    /// * `algorithm` - Layout algorithm ("force_directed", "hierarchical", "circular", "grid")
    pub fn set_layout_algorithm(properties: &mut AdaptiveMap<String, Value>, algorithm: &str) {
        properties.insert("layout_algorithm".to_string(), Value::String(algorithm.to_string()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Enable or disable automatic layout calculation.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to graph properties
    /// * `enabled` - Whether automatic layout should be enabled
    pub fn set_auto_layout(properties: &mut AdaptiveMap<String, Value>, enabled: bool) {
        properties.insert("auto_layout".to_string(), Value::Boolean(enabled));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Increment the node count when a node is added.
    /// 
    /// This should be called whenever a node document is added as a child.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to graph properties
    pub fn increment_node_count(properties: &mut AdaptiveMap<String, Value>) {
        if let Some(Value::U32(ref mut count)) = properties.get_mut("node_count") {
            *count += 1;
        }
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Increment the edge count when an edge is added.
    /// 
    /// This should be called whenever an edge document is added as a child.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to graph properties
    pub fn increment_edge_count(properties: &mut AdaptiveMap<String, Value>) {
        if let Some(Value::U32(ref mut count)) = properties.get_mut("edge_count") {
            *count += 1;
        }
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Decrement the node count when a node is removed.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to graph properties
    pub fn decrement_node_count(properties: &mut AdaptiveMap<String, Value>) {
        if let Some(Value::U32(ref mut count)) = properties.get_mut("node_count") {
            if *count > 0 {
                *count -= 1;
            }
        }
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Decrement the edge count when an edge is removed.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to graph properties
    pub fn decrement_edge_count(properties: &mut AdaptiveMap<String, Value>) {
        if let Some(Value::U32(ref mut count)) = properties.get_mut("edge_count") {
            if *count > 0 {
                *count -= 1;
            }
        }
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Get metadata about the graph.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Graph document properties
    /// 
    /// # Returns
    /// 
    /// GraphMetadata struct with name, type, counts, etc.
    pub fn get_metadata(properties: &AdaptiveMap<String, Value>) -> GraphMetadata {
        GraphMetadata {
            name: properties.get("name")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            graph_type: properties.get("graph_type")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            node_count: properties.get("node_count")
                .and_then(|v| if let Value::U32(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            edge_count: properties.get("edge_count")
                .and_then(|v| if let Value::U32(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            layout_algorithm: properties.get("layout_algorithm")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            auto_layout: properties.get("auto_layout")
                .and_then(|v| if let Value::Boolean(b) = v { Some(*b) } else { None })
                .unwrap_or(true),
            created_at: properties.get("created_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            modified_at: properties.get("modified_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
        }
    }
    
    /// Validate that a document has the correct structure for a graph.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties to validate
    /// 
    /// # Returns
    /// 
    /// True if the document is a valid graph document structure
    pub fn validate(properties: &AdaptiveMap<String, Value>) -> bool {
        let has_name = properties.get("name")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_graph_type = properties.get("graph_type")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_node_count = properties.get("node_count")
            .map(|v| matches!(v, Value::U32(_)))
            .unwrap_or(false);
            
        let has_edge_count = properties.get("edge_count")
            .map(|v| matches!(v, Value::U32(_)))
            .unwrap_or(false);
        
        has_name && has_graph_type && has_node_count && has_edge_count
    }
    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Builder for creating Node documents within graphs.
pub struct NodeDocument;

impl NodeDocument {
    /// Create a new Node document with basic properties.
    /// 
    /// # Arguments
    /// 
    /// * `label` - Type or category of the node (e.g., "Person", "Company")
    /// * `name` - Display name for the node
    /// * `x` - X coordinate for positioning
    /// * `y` - Y coordinate for positioning
    /// * `z` - Z coordinate for positioning (optional, default 0.0)
    /// 
    /// # Returns
    /// 
    /// Properties map that can be used to create a Document with DocumentType::Node
    pub fn new(label: &str, name: &str, x: f64, y: f64, z: Option<f64>) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Set core node properties
        properties.insert("label".to_string(), Value::String(label.to_string()));
        properties.insert("name".to_string(), Value::String(name.to_string()));
        properties.insert("x".to_string(), Value::F64(x));
        properties.insert("y".to_string(), Value::F64(y));
        properties.insert("z".to_string(), Value::F64(z.unwrap_or(0.0)));
        properties.insert("weight".to_string(), Value::F64(1.0));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        
        properties
    }
    
    /// Create a person node with standard properties.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Person's name
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// 
    /// # Returns
    /// 
    /// Properties map for a person node
    pub fn create_person_node(name: &str, x: f64, y: f64) -> AdaptiveMap<String, Value> {
        Self::new("Person", name, x, y, None)
    }
    
    /// Create a company node with standard properties.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Company name
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// 
    /// # Returns
    /// 
    /// Properties map for a company node
    pub fn create_company_node(name: &str, x: f64, y: f64) -> AdaptiveMap<String, Value> {
        Self::new("Company", name, x, y, None)
    }
    
    /// Set the position of a node.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to node properties
    /// * `x` - New X coordinate
    /// * `y` - New Y coordinate
    /// * `z` - New Z coordinate (optional)
    pub fn set_position(properties: &mut AdaptiveMap<String, Value>, x: f64, y: f64, z: Option<f64>) {
        properties.insert("x".to_string(), Value::F64(x));
        properties.insert("y".to_string(), Value::F64(y));
        if let Some(z_coord) = z {
            properties.insert("z".to_string(), Value::F64(z_coord));
        }
    }
    
    /// Set the weight of a node.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to node properties
    /// * `weight` - New weight value (typically 0.0 to 1.0)
    pub fn set_weight(properties: &mut AdaptiveMap<String, Value>, weight: f64) {
        properties.insert("weight".to_string(), Value::F64(weight));
    }
    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Builder for creating Edge documents within graphs.
pub struct EdgeDocument;

impl EdgeDocument {
    /// Create a new Edge document with basic properties.
    /// 
    /// # Arguments
    /// 
    /// * `label` - Type of relationship (e.g., "knows", "works_for")
    /// * `weight` - Strength of the relationship (typically 0.0 to 1.0)
    /// * `directed` - Whether the edge is directional
    /// 
    /// # Returns
    /// 
    /// Properties map that can be used to create a Document with DocumentType::Edge
    pub fn new(label: &str, weight: f64, directed: bool) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Set core edge properties
        properties.insert("label".to_string(), Value::String(label.to_string()));
        properties.insert("weight".to_string(), Value::F64(weight));
        properties.insert("directed".to_string(), Value::Boolean(directed));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        
        properties
    }
    
    /// Create a relationship edge between people.
    /// 
    /// # Arguments
    /// 
    /// * `relationship_type` - Type of relationship ("knows", "friend", "colleague")
    /// * `strength` - Strength of relationship (0.0 to 1.0)
    /// * `directed` - Whether the relationship is directional
    /// 
    /// # Returns
    /// 
    /// Properties map for a relationship edge
    pub fn create_relationship_edge(relationship_type: &str, strength: f64, directed: bool) -> AdaptiveMap<String, Value> {
        Self::new(relationship_type, strength, directed)
    }
    
    /// Set the weight of an edge.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to edge properties
    /// * `weight` - New weight value
    pub fn set_weight(properties: &mut AdaptiveMap<String, Value>, weight: f64) {
        properties.insert("weight".to_string(), Value::F64(weight));
    }
    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Metadata extracted from a graph document.
#[derive(Debug, Clone)]
pub struct GraphMetadata {
    /// Display name of the graph
    pub name: String,
    /// Type of graph (directed, undirected, mixed)
    pub graph_type: String,
    /// Current number of nodes in the graph
    pub node_count: u32,
    /// Current number of edges in the graph
    pub edge_count: u32,
    /// Layout algorithm being used
    pub layout_algorithm: String,
    /// Whether automatic layout is enabled
    pub auto_layout: bool,
    /// Creation timestamp (nanoseconds since epoch)
    pub created_at: u64,
    /// Last modification timestamp (nanoseconds since epoch)
    pub modified_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let properties = GraphDocument::new("Test Graph", "directed");
        
        assert!(GraphDocument::validate(&properties));
        
        let metadata = GraphDocument::get_metadata(&properties);
        assert_eq!(metadata.name, "Test Graph");
        assert_eq!(metadata.graph_type, "directed");
        assert_eq!(metadata.node_count, 0);
        assert_eq!(metadata.edge_count, 0);
    }
    
    #[test]
    fn test_node_creation() {
        let properties = NodeDocument::create_person_node("Alice", 100.0, 200.0);
        
        assert_eq!(properties.get("label").unwrap(), &Value::String("Person".to_string()));
        assert_eq!(properties.get("name").unwrap(), &Value::String("Alice".to_string()));
        assert_eq!(properties.get("x").unwrap(), &Value::F64(100.0));
        assert_eq!(properties.get("y").unwrap(), &Value::F64(200.0));
    }
    
    #[test]
    fn test_edge_creation() {
        let properties = EdgeDocument::create_relationship_edge("knows", 0.8, true);
        
        assert_eq!(properties.get("label").unwrap(), &Value::String("knows".to_string()));
        assert_eq!(properties.get("weight").unwrap(), &Value::F64(0.8));
        assert_eq!(properties.get("directed").unwrap(), &Value::Boolean(true));
    }
    
    #[test]
    fn test_graph_counts() {
        let mut properties = GraphDocument::new("Test", "directed");
        
        GraphDocument::increment_node_count(&mut properties);
        GraphDocument::increment_node_count(&mut properties);
        GraphDocument::increment_edge_count(&mut properties);
        
        let metadata = GraphDocument::get_metadata(&properties);
        assert_eq!(metadata.node_count, 2);
        assert_eq!(metadata.edge_count, 1);
        
        GraphDocument::decrement_node_count(&mut properties);
        let metadata = GraphDocument::get_metadata(&properties);
        assert_eq!(metadata.node_count, 1);
    }
} 