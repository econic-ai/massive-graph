/// Core reusable data structures
pub mod spsc;
pub mod mph_delta_index;
pub mod segmented_stream;
pub mod zerocopy_storage;

// Export the main types
pub use spsc::SpscRing;
pub use segmented_stream::{SegmentedStream, StreamPagePool, Cursor};
pub use mph_delta_index::OptimisedIndex;