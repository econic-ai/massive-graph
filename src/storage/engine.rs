//! Storage backend implementations

use crate::core::types::ID32;
use crate::storage::MemStore;
use dashmap::DashMap;

/// UserID is an alias for ID32
pub type UserID = ID32;

/// Storage trait for different backends
pub trait Storage {
    // TODO: Define storage interface
}

/// In-memory storage implementation
pub struct MemoryStorage;

impl Storage for MemoryStorage {
    // TODO: Implement storage methods
}

/// NodeStorageEngine manages all user MemStores for this node.
pub struct NodeStorageEngine {
    /// Map of user IDs to their isolated storage instances
    stores: DashMap<UserID, MemStore>,
    /// This node's ID (used as the initial user)
    node_id: UserID,
}

impl NodeStorageEngine {
    /// Initialize the storage engine with a node ID
    pub fn new(node_id: UserID) -> Self {
        let mut engine = Self {
            stores: DashMap::new(),
            node_id,
        };
        
        // Create the initial store for this node
        engine.stores.insert(node_id, MemStore::new(node_id));
        
        engine
    }
    
    /// Get or create a storage instance for a user
    pub fn get_or_create_store(&self, user_id: UserID) -> dashmap::mapref::one::RefMut<'_, UserID, MemStore> {
        self.stores.entry(user_id).or_insert_with(|| MemStore::new(user_id))
    }
    
    /// Get the node ID for this storage engine
    pub fn get_node_id(&self) -> UserID {
        self.node_id
    }
}
