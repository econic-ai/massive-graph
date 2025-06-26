//! Storage factory for creating storage implementations based on configuration

use crate::core::config::{StorageConfig, StorageType};
use crate::storage::MemStore;
use std::sync::{Arc, Mutex};

/// Storage factory error
#[derive(Debug)]
pub enum StorageFactoryError {
    /// Unsupported storage type
    UnsupportedStorageType(StorageType),
    /// Storage initialization failed
    InitializationFailed(String),
}

impl std::fmt::Display for StorageFactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageFactoryError::UnsupportedStorageType(storage_type) => {
                write!(f, "Unsupported storage type: {:?}", storage_type)
            }
            StorageFactoryError::InitializationFailed(msg) => {
                write!(f, "Storage initialization failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for StorageFactoryError {}

/// Storage wrapper for thread-safe access
pub type SharedStorage = Arc<Mutex<MemStore>>;

/// Create a storage implementation based on configuration
pub fn create_storage(config: &StorageConfig) -> Result<MemStore, StorageFactoryError> {
    match config.storage_type {
        StorageType::Memory => {
            let store = MemStore::new();
            Ok(store)
        }
        StorageType::Disk => {
            // TODO: Implement disk storage
            Err(StorageFactoryError::UnsupportedStorageType(StorageType::Disk))
        }
        StorageType::Distributed => {
            // TODO: Implement distributed storage
            Err(StorageFactoryError::UnsupportedStorageType(StorageType::Distributed))
        }
    }
}

/// Create a shared storage implementation based on configuration
pub fn create_shared_storage(config: &StorageConfig) -> Result<SharedStorage, StorageFactoryError> {
    let storage = create_storage(config)?;
    Ok(Arc::new(Mutex::new(storage)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::StorageConfig;

    #[test]
    fn test_memory_storage_creation() {
        let config = StorageConfig {
            storage_type: StorageType::Memory,
            ..Default::default()
        };
        
        let storage = create_storage(&config).unwrap();
        assert_eq!(storage.document_count(), 0);
    }

    #[test]
    fn test_shared_memory_storage_creation() {
        let config = StorageConfig {
            storage_type: StorageType::Memory,
            ..Default::default()
        };
        
        let shared_storage = create_shared_storage(&config).unwrap();
        let storage = shared_storage.lock().unwrap();
        assert_eq!(storage.document_count(), 0);
    }

    #[test]
    fn test_disk_storage_unsupported() {
        let config = StorageConfig {
            storage_type: StorageType::Disk,
            ..Default::default()
        };
        
        let result = create_storage(&config);
        assert!(result.is_err());
        
        if let Err(StorageFactoryError::UnsupportedStorageType(StorageType::Disk)) = result {
            // Expected error
        } else {
            panic!("Expected UnsupportedStorageType(Disk) error");
        }
    }

    #[test]
    fn test_distributed_storage_unsupported() {
        let config = StorageConfig {
            storage_type: StorageType::Distributed,
            ..Default::default()
        };
        
        let result = create_storage(&config);
        assert!(result.is_err());
        
        if let Err(StorageFactoryError::UnsupportedStorageType(StorageType::Distributed)) = result {
            // Expected error
        } else {
            panic!("Expected UnsupportedStorageType(Distributed) error");
        }
    }
} 