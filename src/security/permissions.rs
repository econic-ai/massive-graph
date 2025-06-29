use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

/// Individual permission that can be granted to users
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    /// Read document content and metadata
    Read,
    /// Modify document properties
    Write,
    /// Add/remove child documents
    ManageChildren,
    /// Share document with other users
    Share,
    /// Delete the document
    Delete,
    /// Administrative access (all permissions)
    Admin,
}

/// Set of permissions for a collaboration context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionSet {
    /// Individual permissions granted
    permissions: BTreeSet<Permission>,
}

impl PermissionSet {
    /// Create empty permission set
    pub fn new() -> Self {
        Self {
            permissions: BTreeSet::new(),
        }
    }
    
    /// Create permission set with read-only access
    pub fn read_only() -> Self {
        let mut permissions = BTreeSet::new();
        permissions.insert(Permission::Read);
        Self { permissions }
    }
    
    /// Create permission set with read-write access
    pub fn read_write() -> Self {
        let mut permissions = BTreeSet::new();
        permissions.insert(Permission::Read);
        permissions.insert(Permission::Write);
        permissions.insert(Permission::ManageChildren);
        Self { permissions }
    }
    
    /// Create permission set with full collaboration access
    pub fn collaborator() -> Self {
        let mut permissions = BTreeSet::new();
        permissions.insert(Permission::Read);
        permissions.insert(Permission::Write);
        permissions.insert(Permission::ManageChildren);
        permissions.insert(Permission::Share);
        Self { permissions }
    }
    
    /// Create permission set with administrative access
    pub fn admin() -> Self {
        let mut permissions = BTreeSet::new();
        permissions.insert(Permission::Admin);
        Self { permissions }
    }
    
    /// Add a permission to the set
    pub fn grant(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }
    
    /// Remove a permission from the set
    pub fn revoke(&mut self, permission: Permission) {
        self.permissions.remove(&permission);
    }
    
    /// Check if a specific permission is granted
    pub fn has_permission(&self, permission: Permission) -> bool {
        // Admin permission grants all others
        self.permissions.contains(&Permission::Admin) || 
        self.permissions.contains(&permission)
    }
    
    /// Check if user can read
    pub fn can_read(&self) -> bool {
        self.has_permission(Permission::Read)
    }
    
    /// Check if user can write
    pub fn can_write(&self) -> bool {
        self.has_permission(Permission::Write)
    }
    
    /// Check if user can manage children
    pub fn can_manage_children(&self) -> bool {
        self.has_permission(Permission::ManageChildren)
    }
    
    /// Check if user can share
    pub fn can_share(&self) -> bool {
        self.has_permission(Permission::Share)
    }
    
    /// Check if user can delete
    pub fn can_delete(&self) -> bool {
        self.has_permission(Permission::Delete)
    }
    
    /// Check if user has admin access
    pub fn is_admin(&self) -> bool {
        self.has_permission(Permission::Admin)
    }
}

impl Default for PermissionSet {
    fn default() -> Self {
        panic!("PermissionSet cannot be defaulted");
    }
}

impl Hash for PermissionSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the sorted permissions for deterministic hashing
        for permission in &self.permissions {
            permission.hash(state);
        }
    }
} 