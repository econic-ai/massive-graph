use std::collections::BTreeSet;
use crate::core::types::ID16;
use super::{UserID, PermissionSet};

/// Key identifying a unique collaboration context
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CollaborationKey {
    /// Sorted set of participants for deterministic key generation
    pub participants: BTreeSet<UserID>,
    
    /// Permission configuration for this collaboration
    pub permissions: PermissionSet,
}

impl CollaborationKey {
    /// Create collaboration key for private documents (single user)
    pub fn private(user_id: UserID) -> Self {
        let mut participants = BTreeSet::new();
        participants.insert(user_id);
        
        Self {
            participants,
            permissions: PermissionSet::admin(),
        }
    }
    
    /// Create collaboration key for shared documents
    pub fn shared(participants: Vec<UserID>, permissions: PermissionSet) -> Self {
        let participant_set = participants.into_iter().collect();
        
        Self {
            participants: participant_set,
            permissions,
        }
    }
    
    /// Check if user is participant in this collaboration
    pub fn has_participant(&self, user_id: &UserID) -> bool {
        self.participants.contains(user_id)
    }
    
    /// Get number of participants
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }
    
    /// Check if this is a private collaboration (single user)
    pub fn is_private(&self) -> bool {
        self.participants.len() == 1
    }
} 