//! Segmented stream

/// Segmented stream implementation
pub mod segmented_stream;

// Export the main types
pub use segmented_stream::SegmentedStream;
pub use segmented_stream::StreamPagePool;
pub use segmented_stream::Cursor;