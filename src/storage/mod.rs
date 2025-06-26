//! Storage and persistence layer

/// Mem Store
pub mod mem_store;

/// Engine
pub mod engine;

/// Factory
pub mod factory;

/// Re-export main storage types
pub use mem_store::MemStore;
pub use factory::{create_storage, create_shared_storage, SharedStorage, StorageFactoryError};
pub use crate::core::{DocumentStorage, DeltaProcessor};