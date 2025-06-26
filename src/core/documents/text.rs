/// Text document implementation for simple text content.
/// 
/// Text documents store simple text content with basic metadata properties.
/// Unlike TextFile documents which are optimized for collaborative editing,
/// Text documents are for simple string content with standard properties.
/// 
/// ## Architecture
/// 
/// Text documents store content as a simple String value with metadata:
/// - Direct string content property
/// - Language/encoding metadata
/// - Basic timestamps and versioning
/// - Simple structure for non-collaborative text
/// 
/// ## Example Usage
/// 
/// ```rust
/// // Create a text document for a note
/// let note = TextDocument::new(
///     "My note content here",
///     Some("en"),
///     Some("plain")
/// );
/// 
/// // Update the content
/// TextDocument::set_content(&mut note, "Updated content");
/// ```

use crate::core::types::document::{Value, AdaptiveMap};

/// Builder for creating text documents with proper structure and validation.
pub struct TextDocument;

impl TextDocument {
    /// Create a new text document with the specified content and metadata.
    /// 
    /// This creates a simple text document with string content and optional
    /// metadata properties for language and content type.
    /// 
    /// # Arguments
    /// 
    /// * `content` - The text content as a string
    /// * `language` - Optional language code (e.g., "en", "es", "fr")
    /// * `content_type` - Optional content type (e.g., "plain", "markdown", "html")
    /// 
    /// # Returns
    /// 
    /// Properties map that can be used to create a Document with DocumentType::Text
    pub fn new(content: &str, language: Option<&str>, content_type: Option<&str>) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Set core content and metadata
        properties.insert("content".to_string(), Value::String(content.to_string()));
        properties.insert("length".to_string(), Value::U64(content.len() as u64));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        // Set optional metadata
        if let Some(lang) = language {
            properties.insert("language".to_string(), Value::String(lang.to_string()));
        }
        
        if let Some(ctype) = content_type {
            properties.insert("content_type".to_string(), Value::String(ctype.to_string()));
        }
        
        properties
    }
    
    /// Create a new empty text document.
    /// 
    /// This creates a text document with empty content, ready for content
    /// to be added later.
    /// 
    /// # Returns
    /// 
    /// Properties map with empty content and basic metadata
    pub fn new_empty() -> AdaptiveMap<String, Value> {
        Self::new("", None, None)
    }
    
    /// Set the content of a text document.
    /// 
    /// This updates the text content and automatically updates the length
    /// and modification timestamp.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to document properties
    /// * `content` - New text content to set
    pub fn set_content(properties: &mut AdaptiveMap<String, Value>, content: &str) {
        properties.insert("content".to_string(), Value::String(content.to_string()));
        properties.insert("length".to_string(), Value::U64(content.len() as u64));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Get the content from a text document.
    /// 
    /// This retrieves the text content from the document properties.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties containing the text content
    /// 
    /// # Returns
    /// 
    /// The text content as a string, or empty string if not found
    pub fn get_content(properties: &AdaptiveMap<String, Value>) -> String {
        properties.get("content")
            .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
            .unwrap_or_default()
    }
    
    /// Append text to the existing content.
    /// 
    /// This adds new text to the end of the existing content, updating
    /// metadata accordingly.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to document properties
    /// * `additional_content` - Text to append to existing content
    pub fn append_content(properties: &mut AdaptiveMap<String, Value>, additional_content: &str) {
        let current_content = Self::get_content(properties);
        let new_content = format!("{}{}", current_content, additional_content);
        Self::set_content(properties, &new_content);
    }
    
    /// Prepend text to the existing content.
    /// 
    /// This adds new text to the beginning of the existing content, updating
    /// metadata accordingly.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to document properties
    /// * `prefix_content` - Text to prepend to existing content
    pub fn prepend_content(properties: &mut AdaptiveMap<String, Value>, prefix_content: &str) {
        let current_content = Self::get_content(properties);
        let new_content = format!("{}{}", prefix_content, current_content);
        Self::set_content(properties, &new_content);
    }
    
    /// Set the language of the text document.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to document properties
    /// * `language` - Language code (e.g., "en", "es", "fr")
    pub fn set_language(properties: &mut AdaptiveMap<String, Value>, language: &str) {
        properties.insert("language".to_string(), Value::String(language.to_string()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Set the content type of the text document.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to document properties
    /// * `content_type` - Content type (e.g., "plain", "markdown", "html")
    pub fn set_content_type(properties: &mut AdaptiveMap<String, Value>, content_type: &str) {
        properties.insert("content_type".to_string(), Value::String(content_type.to_string()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
    }
    
    /// Get metadata about the text document.
    /// 
    /// This extracts common metadata properties from the text document,
    /// providing a convenient way to access document information.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties to extract metadata from
    /// 
    /// # Returns
    /// 
    /// TextMetadata struct with content length, language, timestamps, etc.
    pub fn get_metadata(properties: &AdaptiveMap<String, Value>) -> TextMetadata {
        TextMetadata {
            length: properties.get("length")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            language: properties.get("language")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None }),
            content_type: properties.get("content_type")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None }),
            created_at: properties.get("created_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            modified_at: properties.get("modified_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
        }
    }
    
    /// Validate that a document has the correct structure for a text document.
    /// 
    /// This checks that all required properties are present and have the correct
    /// types for a valid text document.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties to validate
    /// 
    /// # Returns
    /// 
    /// True if the document is a valid text document structure
    pub fn validate(properties: &AdaptiveMap<String, Value>) -> bool {
        // Check required properties exist with correct types
        let has_content = properties.get("content")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_length = properties.get("length")
            .map(|v| matches!(v, Value::U64(_)))
            .unwrap_or(false);
            
        let has_created_at = properties.get("created_at")
            .map(|v| matches!(v, Value::U64(_)))
            .unwrap_or(false);
            
        let has_modified_at = properties.get("modified_at")
            .map(|v| matches!(v, Value::U64(_)))
            .unwrap_or(false);
        
        has_content && has_length && has_created_at && has_modified_at
    }
    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Metadata extracted from a text document.
/// 
/// This struct provides a convenient way to access common metadata
/// properties from text documents without parsing the Value enum.
#[derive(Debug, Clone)]
pub struct TextMetadata {
    /// Length of the text content in characters
    pub length: u64,
    /// Language code of the text content
    pub language: Option<String>,
    /// Content type (plain, markdown, html, etc.)
    pub content_type: Option<String>,
    /// Creation timestamp (nanoseconds since epoch)
    pub created_at: u64,
    /// Last modification timestamp (nanoseconds since epoch)
    pub modified_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_document_creation() {
        let properties = TextDocument::new("Hello, world!", Some("en"), Some("plain"));
        
        assert!(TextDocument::validate(&properties));
        
        let content = TextDocument::get_content(&properties);
        assert_eq!(content, "Hello, world!");
        
        let metadata = TextDocument::get_metadata(&properties);
        assert_eq!(metadata.length, 13);
        assert_eq!(metadata.language, Some("en".to_string()));
        assert_eq!(metadata.content_type, Some("plain".to_string()));
    }
    
    #[test]
    fn test_text_document_modification() {
        let mut properties = TextDocument::new_empty();
        
        TextDocument::set_content(&mut properties, "Initial content");
        assert_eq!(TextDocument::get_content(&properties), "Initial content");
        
        TextDocument::append_content(&mut properties, " appended");
        assert_eq!(TextDocument::get_content(&properties), "Initial content appended");
        
        TextDocument::prepend_content(&mut properties, "Prefix ");
        assert_eq!(TextDocument::get_content(&properties), "Prefix Initial content appended");
        
        let metadata = TextDocument::get_metadata(&properties);
        assert_eq!(metadata.length, 31);
    }
    
    #[test]
    fn test_text_document_metadata() {
        let mut properties = TextDocument::new("Test content", None, None);
        
        TextDocument::set_language(&mut properties, "fr");
        TextDocument::set_content_type(&mut properties, "markdown");
        
        let metadata = TextDocument::get_metadata(&properties);
        assert_eq!(metadata.language, Some("fr".to_string()));
        assert_eq!(metadata.content_type, Some("markdown".to_string()));
        assert!(metadata.modified_at > metadata.created_at);
    }
} 