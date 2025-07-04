//! Storage factory for creating storage implementations based on configuration

use tracing::info;

use crate::core::config::{StorageConfig, StorageType};
use crate::core::types::ID32;
use crate::storage::MemStore;

/// UserID is an alias for ID32
pub type UserID = ID32;

/// Storage factory error
#[derive(Debug)]
pub enum StorageFactoryError {
    /// Unsupported storage type
    UnsupportedStorageType(StorageType),
    /// Storage initialization failed
    InitializationFailed(String),
    ConfigError(String),
    InitializationError(String),
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
            StorageFactoryError::ConfigError(msg) => write!(f, "Storage config error: {}", msg),
            StorageFactoryError::InitializationError(msg) => write!(f, "Storage initialization error: {}", msg),
        }
    }
}

impl std::error::Error for StorageFactoryError {}

/// Create a storage implementation based on configuration
/// 
/// # Arguments
/// 
/// * `storage_type` - Storage type configuration
/// * `user_id` - User ID for the storage instance
/// 
/// # Returns
/// 
/// * `Ok(MemStore)` - Successfully created storage
/// * `Err(StorageFactoryError)` - Configuration or initialization error
pub fn create_storage(storage_type: StorageType, user_id: UserID) -> Result<MemStore, StorageFactoryError> {
    info!("Creating storage: {:?}", storage_type);
    match storage_type {
        StorageType::Memory => {
            let store = MemStore::new(user_id);
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
        
        let storage = create_storage(StorageType::Memory, ID32::random()).unwrap();
        assert_eq!(storage.document_count(), 0);
    }

    #[test]
    fn test_disk_storage_unsupported() {
        let config = StorageConfig {
            storage_type: StorageType::Disk,
            ..Default::default()
        };
        
        let result = create_storage(StorageType::Disk, ID32::random());
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
        
        let result = create_storage(StorageType::Distributed, ID32::random());
        assert!(result.is_err());
        
        if let Err(StorageFactoryError::UnsupportedStorageType(StorageType::Distributed)) = result {
            // Expected error
        } else {
            panic!("Expected UnsupportedStorageType(Distributed) error");
        }
    }
} 