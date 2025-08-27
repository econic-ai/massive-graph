use std::sync::atomic::AtomicPtr;
use super::StreamId;

/// Stream reference - supports append-only linked lists
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct AppendOnlyStream {
    stream_id: StreamId,
    stream_type: StreamType,       // Type of stream
    head: *const Node,           // First node
    tail: AtomicPtr<Node>,       // Last node for O(1) append
    last_processed: AtomicPtr<Node>, // Processing cursor
}

/// Node in a stream
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct Node {
    // TODO: This is a placeholder for the actual data type.
    // data_ref: Option,         // Points to any data type in chunk
    next: AtomicPtr<Node>,      // Next in stream
}

/// Stream types for different data patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamType {
    /// Ordered delta operations
    DeltaStream,
    /// Document version snapshots
    DocumentStream,
    /// Text append operations
    TextStream,
    /// Binary data chunks
    BinaryStream,
}