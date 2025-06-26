/// Binary document implementation for large binary data storage.
/// 
/// Binary documents store large binary data (videos, images, executables) with
/// streaming capabilities. They use the BinaryStream value type for efficient
/// chunk-based access and range requests.
/// 
/// ## Architecture
/// 
/// Binary documents store data as a `BinaryStream` property, enabling:
/// - Chunk-based streaming for large files
/// - Range requests for video seeking
/// - Metadata properties for MIME type, filename, etc.
/// - Efficient append operations for real-time data
/// 
/// ## Example Usage
/// 
/// ```rust
/// // Create a binary document for a video file
/// let video_doc = BinaryDocument::new(
///     "video.mp4",
///     "video/mp4", 
///     video_data
/// );
/// 
/// // Stream specific time range
/// let chunk = video_doc.get_range(start_time, end_time)?;
/// ```

use crate::core::types::document::{Value, AdaptiveMap, AppendOnlyStream};

/// Builder for creating binary documents with proper structure and validation.
pub struct BinaryDocument;

impl BinaryDocument {
    /// Create a new binary document from complete binary data.
    /// 
    /// This creates a document with the binary data stored as a BinaryStream
    /// property, along with metadata properties for filename and MIME type.
    /// 
    /// # Arguments
    /// 
    /// * `filename` - Original filename for the binary data
    /// * `mime_type` - MIME type (e.g., "video/mp4", "image/png")
    /// * `data` - Complete binary data as byte vector
    /// 
    /// # Returns
    /// 
    /// Properties map that can be used to create a Document with DocumentType::Binary
    pub fn new(filename: &str, mime_type: &str, data: Vec<u8>) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Create binary stream with the complete data
        let mut stream = AppendOnlyStream::new();
        stream.append(data);
        
        // Set up binary document properties
        properties.insert("filename".to_string(), Value::String(filename.to_string()));
        properties.insert("mime_type".to_string(), Value::String(mime_type.to_string()));
        properties.insert("content".to_string(), Value::BinaryStream(Box::new(stream)));
        properties.insert("size".to_string(), Value::U64(0)); // Will be updated by stream size
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        
        properties
    }
    
    /// Create a new empty binary document for streaming data.
    /// 
    /// This creates a binary document with an empty stream, suitable for
    /// real-time data streaming where data will be appended over time.
    /// 
    /// # Arguments
    /// 
    /// * `filename` - Filename for the streaming binary data
    /// * `mime_type` - MIME type for the streaming data
    /// 
    /// # Returns
    /// 
    /// Properties map with empty BinaryStream ready for append operations
    pub fn new_streaming(filename: &str, mime_type: &str) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Create empty binary stream for streaming
        let stream = AppendOnlyStream::new();
        
        properties.insert("filename".to_string(), Value::String(filename.to_string()));
        properties.insert("mime_type".to_string(), Value::String(mime_type.to_string()));
        properties.insert("content".to_string(), Value::BinaryStream(Box::new(stream)));
        properties.insert("size".to_string(), Value::U64(0));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        properties.insert("streaming".to_string(), Value::Boolean(true));
        
        properties
    }
    
    /// Append binary data to an existing binary document stream.
    /// 
    /// This adds new binary data to the document's BinaryStream, updating
    /// the size metadata accordingly.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to document properties
    /// * `data` - Binary data to append to the stream
    /// 
    /// # Returns
    /// 
    /// Timestamp of the appended entry, or error if document structure is invalid
    pub fn append_data(properties: &mut AdaptiveMap<String, Value>, data: Vec<u8>) -> Result<u64, &'static str> {
        let data_size = data.len() as u64;
        
        // Get mutable reference to the binary stream
        if let Some(Value::BinaryStream(ref mut stream)) = properties.get_mut("content") {
            let timestamp = stream.append(data);
            
            // Update size metadata
            if let Some(Value::U64(ref mut size)) = properties.get_mut("size") {
                *size += data_size;
            }
            
            // Update last modified timestamp
            properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
            
            Ok(timestamp)
        } else {
            Err("Document does not contain a valid binary stream")
        }
    }
    
    /// Get binary data from a specific time range.
    /// 
    /// This retrieves binary data entries from the stream within the specified
    /// timestamp range, useful for video seeking or partial data retrieval.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties containing the binary stream
    /// * `start_time` - Start timestamp (nanoseconds since epoch)
    /// * `end_time` - End timestamp (nanoseconds since epoch)
    /// 
    /// # Returns
    /// 
    /// Vector of binary data chunks within the time range
    pub fn get_range(properties: &AdaptiveMap<String, Value>, start_time: u64, end_time: u64) -> Result<Vec<&Vec<u8>>, &'static str> {
        if let Some(Value::BinaryStream(stream)) = properties.get("content") {
            let entries = stream.range(start_time, end_time);
            Ok(entries.into_iter().map(|entry| &entry.data).collect())
        } else {
            Err("Document does not contain a valid binary stream")
        }
    }
    
    /// Get the latest N binary data entries.
    /// 
    /// This retrieves the most recent binary data entries from the stream,
    /// useful for displaying recent data or implementing "tail" functionality.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties containing the binary stream
    /// * `count` - Number of latest entries to retrieve
    /// 
    /// # Returns
    /// 
    /// Vector of the latest binary data chunks
    pub fn get_latest(properties: &AdaptiveMap<String, Value>, count: usize) -> Result<Vec<&Vec<u8>>, &'static str> {
        if let Some(Value::BinaryStream(stream)) = properties.get("content") {
            let entries = stream.latest(count);
            Ok(entries.into_iter().map(|entry| &entry.data).collect())
        } else {
            Err("Document does not contain a valid binary stream")
        }
    }
    
    /// Get metadata about the binary document.
    /// 
    /// This extracts common metadata properties from the binary document,
    /// providing a convenient way to access file information.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties to extract metadata from
    /// 
    /// # Returns
    /// 
    /// BinaryMetadata struct with filename, MIME type, size, etc.
    pub fn get_metadata(properties: &AdaptiveMap<String, Value>) -> BinaryMetadata {
        BinaryMetadata {
            filename: properties.get("filename")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            mime_type: properties.get("mime_type")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            size: properties.get("size")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            created_at: properties.get("created_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            modified_at: properties.get("modified_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None }),
            is_streaming: properties.get("streaming")
                .and_then(|v| if let Value::Boolean(b) = v { Some(*b) } else { None })
                .unwrap_or(false),
        }
    }
    
    /// Validate that a document has the correct structure for a binary document.
    /// 
    /// This checks that all required properties are present and have the correct
    /// types for a valid binary document.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties to validate
    /// 
    /// # Returns
    /// 
    /// True if the document is a valid binary document structure
    pub fn validate(properties: &AdaptiveMap<String, Value>) -> bool {
        // Check required properties exist with correct types
        let has_filename = properties.get("filename")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_mime_type = properties.get("mime_type")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_content = properties.get("content")
            .map(|v| matches!(v, Value::BinaryStream(_)))
            .unwrap_or(false);
            
        let has_size = properties.get("size")
            .map(|v| matches!(v, Value::U64(_)))
            .unwrap_or(false);
        
        has_filename && has_mime_type && has_content && has_size
    }
    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Metadata extracted from a binary document.
/// 
/// This struct provides a convenient way to access common metadata
/// properties from binary documents without parsing the Value enum.
#[derive(Debug, Clone)]
pub struct BinaryMetadata {
    /// Original filename of the binary data
    pub filename: String,
    /// MIME type of the binary data
    pub mime_type: String,
    /// Total size of binary data in bytes
    pub size: u64,
    /// Creation timestamp (nanoseconds since epoch)
    pub created_at: u64,
    /// Last modification timestamp (nanoseconds since epoch)
    pub modified_at: Option<u64>,
    /// Whether this is a streaming binary document
    pub is_streaming: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_document_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let properties = BinaryDocument::new("test.bin", "application/octet-stream", data.clone());
        
        assert!(BinaryDocument::validate(&properties));
        
        let metadata = BinaryDocument::get_metadata(&properties);
        assert_eq!(metadata.filename, "test.bin");
        assert_eq!(metadata.mime_type, "application/octet-stream");
        assert!(!metadata.is_streaming);
    }
    
    #[test]
    fn test_streaming_binary_document() {
        let mut properties = BinaryDocument::new_streaming("stream.dat", "application/octet-stream");
        
        assert!(BinaryDocument::validate(&properties));
        
        let data1 = vec![1, 2, 3];
        let data2 = vec![4, 5, 6];
        
        BinaryDocument::append_data(&mut properties, data1).unwrap();
        BinaryDocument::append_data(&mut properties, data2).unwrap();
        
        let metadata = BinaryDocument::get_metadata(&properties);
        assert_eq!(metadata.size, 6);
        assert!(metadata.is_streaming);
    }
} 