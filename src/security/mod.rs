pub mod user;
pub mod collaboration;
pub mod permissions;
 
// Re-export commonly used types
pub use user::{UserID, UserMetadata};
pub use collaboration::CollaborationKey;
pub use permissions::{PermissionSet, Permission}; 