/// Core reusable data structures
pub mod spsc;
pub mod optimised_index;
pub mod segmented_stream;


// Export the main types
pub use spsc::SpscRing;
pub use segmented_stream::{SegmentedStream, StreamPagePool, Cursor};
pub use optimised_index::OptimisedIndex;