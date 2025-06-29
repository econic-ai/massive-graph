use crate::core::types::ID16;

/// User identifier - uses same format as document IDs for consistency
pub type UserID = ID16;

/// Basic user metadata for collaboration management
#[derive(Debug, Clone)]
pub struct UserMetadata {
    /// User identifier
    pub id: UserID,
    
    /// Display name for UI
    pub display_name: String,
    
    /// When user account was created
    pub created_at: u64,
    
    /// Last activity timestamp
    pub last_active: u64,
    
    /// User status
    pub status: UserStatus,
}

/// User account status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    /// Active user account
    Active,
    /// Temporarily suspended
    Suspended,
    /// Permanently disabled
    Disabled,
}

impl UserMetadata {
    /// Create new user metadata
    pub fn new(id: UserID, display_name: String) -> Self {
        let now = current_timestamp();
        Self {
            id,
            display_name,
            created_at: now,
            last_active: now,
            status: UserStatus::Active,
        }
    }
    
    /// Update last active timestamp
    pub fn touch(&mut self) {
        self.last_active = current_timestamp();
    }
    
    /// Check if user is active
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }
}

/// Get current timestamp in nanoseconds since epoch
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
} 