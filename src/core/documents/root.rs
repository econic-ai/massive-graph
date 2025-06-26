/// Root document implementation for top-level container documents.
/// 
/// Root documents serve as top-level namespaces that contain other documents
/// as children. They provide organizational boundaries and access control
/// scopes within the unified document architecture.
/// 
/// Examples of root documents:
/// - User workspaces
/// - Project containers  
/// - Organization boundaries
/// - Application namespaces

use crate::core::types::document::{DocumentType, Value, AdaptiveMap};

/// Root document builder and utilities.
pub struct RootDocument;

impl RootDocument {
    /// Create a new root document with the specified name.
    /// 
    /// Root documents are top-level containers that organize other documents
    /// into logical namespaces. They contain metadata about the container
    /// and maintain a children list for documents within their scope.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Human-readable name for the root document
    /// * `description` - Optional description of the root document's purpose
    /// 
    /// # Returns
    /// 
    /// AdaptiveMap containing the root document properties
    /// 
    /// # Example
    /// 
    /// ```
    /// use massive_graph::core::documents::RootDocument;
    /// 
    /// let workspace = RootDocument::new("My Workspace", Some("Personal project workspace"));
    /// ```
    pub fn new(name: &str, description: Option<&str>) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Core metadata
        properties.insert("name".to_string(), Value::String(name.to_string()));
        properties.insert("doc_type".to_string(), Value::U8(DocumentType::Root as u8));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        // Optional description
        if let Some(desc) = description {
            properties.insert("description".to_string(), Value::String(desc.to_string()));
        }
        
        // Container properties
        properties.insert("children".to_string(), Value::Array(Vec::new()));
        properties.insert("child_count".to_string(), Value::U32(0));
        
        // Access control metadata (for future use)
        properties.insert("public".to_string(), Value::Boolean(false));
        properties.insert("permissions".to_string(), Value::Array(Vec::new()));
        
        properties
    }
    
    /// Create a public root document that can be accessed by multiple users.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Human-readable name for the root document
    /// * `description` - Optional description of the root document's purpose
    /// 
    /// # Returns
    /// 
    /// AdaptiveMap containing the public root document properties
    pub fn new_public(name: &str, description: Option<&str>) -> AdaptiveMap<String, Value> {
        let mut properties = Self::new(name, description);
        properties.insert("public".to_string(), Value::Boolean(true));
        properties
    }
    
    /// Create a root document for a specific organization or team.
    /// 
    /// # Arguments
    /// 
    /// * `name` - Human-readable name for the root document
    /// * `organization` - Organization or team identifier
    /// * `description` - Optional description of the root document's purpose
    /// 
    /// # Returns
    /// 
    /// AdaptiveMap containing the organization root document properties
    pub fn new_organization(name: &str, organization: &str, description: Option<&str>) -> AdaptiveMap<String, Value> {
        let mut properties = Self::new(name, description);
        properties.insert("organization".to_string(), Value::String(organization.to_string()));
        properties.insert("org_scoped".to_string(), Value::Boolean(true));
        properties
    }
    
    /// Get the current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_document_creation() {
        let root_doc = RootDocument::new("Test Workspace", Some("A test workspace"));
        
        // Check core properties
        assert_eq!(
            root_doc.get("name").and_then(|v| if let Value::String(s) = v { Some(s) } else { None }),
            Some(&"Test Workspace".to_string())
        );
        
        assert_eq!(
            root_doc.get("description").and_then(|v| if let Value::String(s) = v { Some(s) } else { None }),
            Some(&"A test workspace".to_string())
        );
        
        assert_eq!(
            root_doc.get("doc_type").and_then(|v| if let Value::U8(t) = v { Some(*t) } else { None }),
            Some(DocumentType::Root as u8)
        );
        
        // Check container properties
        assert_eq!(
            root_doc.get("child_count").and_then(|v| if let Value::U32(c) = v { Some(*c) } else { None }),
            Some(0)
        );
        
        // Check access control
        assert_eq!(
            root_doc.get("public").and_then(|v| if let Value::Boolean(p) = v { Some(*p) } else { None }),
            Some(false)
        );
    }
    
    #[test]
    fn test_public_root_document() {
        let root_doc = RootDocument::new_public("Public Workspace", None);
        
        assert_eq!(
            root_doc.get("public").and_then(|v| if let Value::Boolean(p) = v { Some(*p) } else { None }),
            Some(true)
        );
    }
    
    #[test]
    fn test_organization_root_document() {
        let root_doc = RootDocument::new_organization("Team Workspace", "Engineering", Some("Engineering team workspace"));
        
        assert_eq!(
            root_doc.get("organization").and_then(|v| if let Value::String(s) = v { Some(s) } else { None }),
            Some(&"Engineering".to_string())
        );
        
        assert_eq!(
            root_doc.get("org_scoped").and_then(|v| if let Value::Boolean(s) = v { Some(*s) } else { None }),
            Some(true)
        );
    }
    
    #[test]
    fn test_root_document_without_description() {
        let root_doc = RootDocument::new("Simple Workspace", None);
        
        assert_eq!(
            root_doc.get("name").and_then(|v| if let Value::String(s) = v { Some(s) } else { None }),
            Some(&"Simple Workspace".to_string())
        );
        
        // Should not have description property
        assert!(root_doc.get("description").is_none());
    }
} 