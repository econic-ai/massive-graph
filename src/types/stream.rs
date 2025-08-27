use std::sync::atomic::AtomicPtr;
use super::StreamId;

/// Stream reference - supports append-only linked lists
pub struct AppendOnlyStream {
    stream_id: StreamId,
    stream_type: StreamType,       // Type of stream
    head: *const Node,           // First node
    tail: AtomicPtr<Node>,       // Last node for O(1) append
    last_processed: AtomicPtr<Node>, // Processing cursor
}

pub struct Node {
    // TODO: This is a placeholder for the actual data type.
    data_ref: Node,         // Points to any data type in chunk
    
    next: AtomicPtr<Node>,      // Next in stream
}

/// Stream types for different data patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamType {
    DeltaStream,    // Ordered delta operations
    DocumentStream,  // Document version snapshots
    TextStream,     // Text append operations
    BinaryStream,   // Binary data chunks
}