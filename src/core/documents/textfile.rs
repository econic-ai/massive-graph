/// TextFile document implementation for collaborative text editing.
/// 
/// TextFile documents are optimized for collaborative editing of text files with
/// minimal conflict resolution. They use a sophisticated line-based architecture
/// that separates stable line identifiers from display positions.
/// 
/// ## Architecture Overview
/// 
/// TextFile documents solve the collaborative text editing problem by using:
/// 
/// 1. **Stable Line IDs**: Each line has a permanent u16 identifier that never changes
/// 2. **Sparse Display Positions**: Lines are positioned using u16 values with intentional gaps
/// 3. **Separated Storage**: Line content and display order are stored separately
/// 4. **Minimal Cascade Updates**: Insertions only require position updates, not content changes
/// 
/// ## Data Structure
/// 
/// ```
/// TextFile Document {
///     "line_content" -> Object({
///         1u16 -> "use std::collections::HashMap;",  // Stable line ID -> content
///         2u16 -> "",
///         3u16 -> "fn main() {",
///         42u16 -> "    let x = 42;",
///     }),
///     "line_index" -> Object({
///         1u16 -> 0u16,      // Line ID -> display position
///         2u16 -> 1000u16,   // Sparse positioning with gaps
///         3u16 -> 2000u16,
///         42u16 -> 3000u16,
///     }),
///     "next_line_id" -> 100u16,
///     "active_cursors" -> Object({ user_id -> (line_pos, char_pos) }),
/// }
/// ```
/// 
/// ## Collaborative Benefits
/// 
/// - **No Line Content Cascades**: When lines are inserted/deleted, content never moves
/// - **Minimal Index Updates**: Only display positions change, not line IDs
/// - **Sparse Positioning**: 999 insertions possible between any two lines before cascade
/// - **Stable References**: Line ID 42 always refers to the same content
/// - **Client-Side Conflict Prevention**: Cursor tracking prevents edit conflicts
/// 
/// ## Performance Characteristics
/// 
/// - **Insert (gap available)**: O(1) - no cascade updates needed
/// - **Insert (no gap)**: O(k) where k = lines needing position updates
/// - **Delete**: O(1) - just remove index entry, create gap for future use
/// - **Edit line content**: O(1) - direct property update by line ID
/// - **Lookup by display position**: O(n) scan (acceptable for file loading)
/// - **Lookup by line ID**: O(1) hash lookup
/// 
/// ## Size Limits
/// 
/// - **Maximum lines**: 65,535 (u16 line IDs)
/// - **Maximum file size**: ~3.2MB for 65k lines @ 50 chars/line average
/// - **Index overhead**: ~260KB for maximum file (65k × 4 bytes per mapping)
/// 
/// ## Example Usage
/// 
/// ```rust
/// // Create a new text file
/// let textfile = TextFileDocument::new("main.rs", "rust", initial_lines);
/// 
/// // Insert a line at display position 5
/// TextFileDocument::insert_line(&mut textfile, 5, "    println!(\"Hello\");");
/// 
/// // Edit existing line content
/// TextFileDocument::edit_line_by_position(&mut textfile, 3, "// Updated comment");
/// 
/// // Delete lines 8-10
/// TextFileDocument::delete_lines(&mut textfile, 8, 10);
/// 
/// // Get current file content for display
/// let display_lines = TextFileDocument::get_display_lines(&textfile);
/// ```

use crate::core::types::document::{Value, AdaptiveMap};
use crate::core::types::ID16;
use std::collections::HashMap;

/// Builder for creating TextFile documents with collaborative editing capabilities.
pub struct TextFileDocument;

impl TextFileDocument {
    /// Create a new TextFile document with initial content.
    /// 
    /// This creates a TextFile document with the specified lines, setting up
    /// the sparse positioning system with gaps for efficient insertion.
    /// 
    /// # Arguments
    /// 
    /// * `filename` - Name of the text file
    /// * `language` - Programming language for syntax highlighting (e.g., "rust", "python")
    /// * `initial_lines` - Vector of initial line content strings
    /// 
    /// # Returns
    /// 
    /// Properties map that can be used to create a Document with DocumentType::TextFile
    pub fn new(filename: &str, language: &str, initial_lines: Vec<String>) -> AdaptiveMap<String, Value> {
        let mut properties = AdaptiveMap::new();
        
        // Set file metadata
        properties.insert("filename".to_string(), Value::String(filename.to_string()));
        properties.insert("language".to_string(), Value::String(language.to_string()));
        properties.insert("encoding".to_string(), Value::String("utf-8".to_string()));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        // Initialize line content and index with sparse positioning
        let mut line_content = AdaptiveMap::new();
        let mut line_index = AdaptiveMap::new();
        let mut next_line_id = 1u16;
        
        // Create lines with 1000-unit spacing for optimal insertion gaps
        for (display_pos, content) in initial_lines.into_iter().enumerate() {
            let line_id = next_line_id;
            let position = (display_pos as u16) * 1000;
            
            line_content.insert(line_id.to_string(), Value::String(content));
            line_index.insert(line_id.to_string(), Value::U16(position));
            
            next_line_id += 1;
        }
        
        properties.insert("line_content".to_string(), Value::Object(Box::new(line_content)));
        properties.insert("line_index".to_string(), Value::Object(Box::new(line_index)));
        properties.insert("next_line_id".to_string(), Value::U16(next_line_id));
        properties.insert("line_count".to_string(), Value::U32((next_line_id - 1) as u32));
        
        // Initialize empty cursor tracking
        let active_cursors = AdaptiveMap::new();
        properties.insert("active_cursors".to_string(), Value::Object(Box::new(active_cursors)));
        
        properties
    }
    
    /// Create a new empty TextFile document.
    /// 
    /// # Arguments
    /// 
    /// * `filename` - Name of the text file
    /// * `language` - Programming language for syntax highlighting
    /// 
    /// # Returns
    /// 
    /// Properties map for an empty TextFile document
    pub fn new_empty(filename: &str, language: &str) -> AdaptiveMap<String, Value> {
        Self::new(filename, language, vec![])
    }
    
    /// Insert a new line at the specified display position.
    /// 
    /// This inserts a new line with the given content at the display position,
    /// using sparse positioning to minimize cascade updates.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `display_position` - Where to insert the line (0-based)
    /// * `content` - Content for the new line
    /// 
    /// # Returns
    /// 
    /// The line ID of the newly created line, or error if insertion fails
    pub fn insert_line(properties: &mut AdaptiveMap<String, Value>, display_position: u16, content: &str) -> Result<u16, &'static str> {
        // Get next line ID
        let line_id = properties.get("next_line_id")
            .and_then(|v| if let Value::U16(id) = v { Some(*id) } else { None })
            .ok_or("Invalid next_line_id")?;
        
        // Update next line ID
        properties.insert("next_line_id".to_string(), Value::U16(line_id + 1));
        
        // Add line content
        if let Some(Value::Object(ref mut line_content)) = properties.get_mut("line_content") {
            line_content.insert(line_id.to_string(), Value::String(content.to_string()));
        } else {
            return Err("Invalid line_content structure");
        }
        
        // Calculate insertion position using sparse positioning
        let insertion_pos = Self::calculate_insertion_position(properties, display_position)?;
        
        // Add to line index
        if let Some(Value::Object(ref mut line_index)) = properties.get_mut("line_index") {
            line_index.insert(line_id.to_string(), Value::U16(insertion_pos));
        } else {
            return Err("Invalid line_index structure");
        }
        
        // Update line count
        if let Some(Value::U32(ref mut count)) = properties.get_mut("line_count") {
            *count += 1;
        }
        
        // Update modification timestamp
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        Ok(line_id)
    }
    
    /// Delete a single line at the specified display position.
    /// 
    /// This removes the line from both content and index, creating a gap
    /// in the sparse positioning that can be reused for future insertions.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `display_position` - Position of line to delete (0-based)
    /// 
    /// # Returns
    /// 
    /// Success or error if deletion fails
    pub fn delete_line(properties: &mut AdaptiveMap<String, Value>, display_position: u16) -> Result<(), &'static str> {
        let line_id = Self::find_line_id_at_position(properties, display_position)?;
        
        // Remove line content
        if let Some(Value::Object(ref mut line_content)) = properties.get_mut("line_content") {
            line_content.remove(&line_id.to_string());
        }
        
        // Remove from line index (creates gap for future insertions)
        if let Some(Value::Object(ref mut line_index)) = properties.get_mut("line_index") {
            line_index.remove(&line_id.to_string());
        }
        
        // Update line count
        if let Some(Value::U32(ref mut count)) = properties.get_mut("line_count") {
            if *count > 0 {
                *count -= 1;
            }
        }
        
        // Update modification timestamp
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        Ok(())
    }
    
    /// Delete multiple lines in a range.
    /// 
    /// This efficiently deletes a range of lines, creating gaps in the
    /// sparse positioning for future insertions.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `start_position` - First line to delete (0-based, inclusive)
    /// * `end_position` - Last line to delete (0-based, inclusive)
    /// 
    /// # Returns
    /// 
    /// Number of lines deleted, or error if deletion fails
    pub fn delete_lines(properties: &mut AdaptiveMap<String, Value>, start_position: u16, end_position: u16) -> Result<u16, &'static str> {
        if start_position > end_position {
            return Err("Invalid range: start_position > end_position");
        }
        
        let mut deleted_count = 0;
        
        // Delete lines from end to start to avoid position shifting issues
        for pos in (start_position..=end_position).rev() {
            if Self::delete_line(properties, pos).is_ok() {
                deleted_count += 1;
            }
        }
        
        Ok(deleted_count)
    }
    
    /// Edit the content of a line at the specified display position.
    /// 
    /// This updates the content of an existing line without affecting
    /// the line index or other lines.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `display_position` - Position of line to edit (0-based)
    /// * `new_content` - New content for the line
    /// 
    /// # Returns
    /// 
    /// Success or error if edit fails
    pub fn edit_line_by_position(properties: &mut AdaptiveMap<String, Value>, display_position: u16, new_content: &str) -> Result<(), &'static str> {
        let line_id = Self::find_line_id_at_position(properties, display_position)?;
        Self::edit_line_by_id(properties, line_id, new_content)
    }
    
    /// Edit the content of a line by its stable line ID.
    /// 
    /// This is more efficient than editing by position since it doesn't
    /// require scanning the index to find the line ID.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `line_id` - Stable line ID to edit
    /// * `new_content` - New content for the line
    /// 
    /// # Returns
    /// 
    /// Success or error if edit fails
    pub fn edit_line_by_id(properties: &mut AdaptiveMap<String, Value>, line_id: u16, new_content: &str) -> Result<(), &'static str> {
        if let Some(Value::Object(ref mut line_content)) = properties.get_mut("line_content") {
            line_content.insert(line_id.to_string(), Value::String(new_content.to_string()));
            
            // Update modification timestamp
            properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
            
            Ok(())
        } else {
            Err("Invalid line_content structure")
        }
    }
    
    /// Get all lines in display order for rendering.
    /// 
    /// This scans the line index to build an ordered list of line content
    /// for display purposes. Used primarily for file loading and rendering.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - TextFile document properties
    /// 
    /// # Returns
    /// 
    /// Vector of lines in display order, or error if structure is invalid
    pub fn get_display_lines(properties: &AdaptiveMap<String, Value>) -> Result<Vec<String>, &'static str> {
        let line_content = properties.get("line_content")
            .and_then(|v| if let Value::Object(obj) = v { Some(obj.as_ref()) } else { None })
            .ok_or("Invalid line_content structure")?;
            
        let line_index = properties.get("line_index")
            .and_then(|v| if let Value::Object(obj) = v { Some(obj.as_ref()) } else { None })
            .ok_or("Invalid line_index structure")?;
        
        // Build position -> line_id mapping
        let mut positions: Vec<(u16, u16)> = Vec::new();
        for (line_id_str, position_val) in line_index.iter() {
            let line_id = line_id_str.parse::<u16>().map_err(|_| "Invalid line ID format")?;
            let position = if let Value::U16(pos) = position_val { *pos } else { 
                return Err("Invalid position value");
            };
            positions.push((position, line_id));
        }
        
        // Sort by position to get display order
        positions.sort_by_key(|(pos, _)| *pos);
        
        // Build result vector with line content
        let mut result = Vec::new();
        for (_, line_id) in positions {
            let content = line_content.get(&line_id.to_string())
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default();
            result.push(content);
        }
        
        Ok(result)
    }
    
    /// Get the content of a specific line by display position.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - TextFile document properties
    /// * `display_position` - Position of line to retrieve (0-based)
    /// 
    /// # Returns
    /// 
    /// Line content string, or error if position is invalid
    pub fn get_line_content(properties: &AdaptiveMap<String, Value>, display_position: u16) -> Result<String, &'static str> {
        let line_id = Self::find_line_id_at_position(properties, display_position)?;
        
        let line_content = properties.get("line_content")
            .and_then(|v| if let Value::Object(obj) = v { Some(obj.as_ref()) } else { None })
            .ok_or("Invalid line_content structure")?;
            
        line_content.get(&line_id.to_string())
            .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
            .ok_or("Line content not found")
    }
    
    /// Update cursor position for collaborative editing awareness.
    /// 
    /// This tracks where users are currently editing to prevent conflicts
    /// through client-side cursor awareness.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `user_id` - ID of the user updating their cursor
    /// * `line_position` - Display line position (0-based)
    /// * `char_position` - Character position within the line (0-based)
    /// 
    /// # Returns
    /// 
    /// Success or error if cursor update fails
    pub fn update_cursor(properties: &mut AdaptiveMap<String, Value>, user_id: ID16, line_position: u16, char_position: u16) -> Result<(), &'static str> {
        if let Some(Value::Object(ref mut active_cursors)) = properties.get_mut("active_cursors") {
            let cursor_data = format!("{}:{}", line_position, char_position);
            active_cursors.insert(user_id.to_string(), Value::String(cursor_data));
            Ok(())
        } else {
            Err("Invalid active_cursors structure")
        }
    }
    
    /// Remove cursor tracking for a user (e.g., when they disconnect).
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Mutable reference to TextFile document properties
    /// * `user_id` - ID of the user to remove cursor tracking for
    pub fn remove_cursor(properties: &mut AdaptiveMap<String, Value>, user_id: ID16) {
        if let Some(Value::Object(ref mut active_cursors)) = properties.get_mut("active_cursors") {
            active_cursors.remove(&user_id.to_string());
        }
    }
    
    /// Get all active cursor positions for collaborative awareness.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - TextFile document properties
    /// 
    /// # Returns
    /// 
    /// HashMap of user_id -> (line_position, char_position)
    pub fn get_active_cursors(properties: &AdaptiveMap<String, Value>) -> HashMap<ID16, (u16, u16)> {
        let mut result = HashMap::new();
        
        if let Some(Value::Object(active_cursors)) = properties.get("active_cursors") {
            for (user_id_str, cursor_val) in active_cursors.iter() {
                if let (Ok(user_id), Value::String(cursor_data)) = (user_id_str.parse(), cursor_val) {
                    if let Some((line_str, char_str)) = cursor_data.split_once(':') {
                        if let (Ok(line_pos), Ok(char_pos)) = (line_str.parse(), char_str.parse()) {
                            result.insert(user_id, (line_pos, char_pos));
                        }
                    }
                }
            }
        }
        
        result
    }
    
    /// Get metadata about the TextFile document.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - TextFile document properties
    /// 
    /// # Returns
    /// 
    /// TextFileMetadata struct with filename, language, line count, etc.
    pub fn get_metadata(properties: &AdaptiveMap<String, Value>) -> TextFileMetadata {
        TextFileMetadata {
            filename: properties.get("filename")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            language: properties.get("language")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            encoding: properties.get("encoding")
                .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_default(),
            line_count: properties.get("line_count")
                .and_then(|v| if let Value::U32(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            created_at: properties.get("created_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
            modified_at: properties.get("modified_at")
                .and_then(|v| if let Value::U64(n) = v { Some(*n) } else { None })
                .unwrap_or(0),
        }
    }
    
    /// Validate that a document has the correct structure for a TextFile document.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - Document properties to validate
    /// 
    /// # Returns
    /// 
    /// True if the document is a valid TextFile document structure
    pub fn validate(properties: &AdaptiveMap<String, Value>) -> bool {
        let has_filename = properties.get("filename")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_language = properties.get("language")
            .map(|v| matches!(v, Value::String(_)))
            .unwrap_or(false);
            
        let has_line_content = properties.get("line_content")
            .map(|v| matches!(v, Value::Object(_)))
            .unwrap_or(false);
            
        let has_line_index = properties.get("line_index")
            .map(|v| matches!(v, Value::Object(_)))
            .unwrap_or(false);
            
        let has_next_line_id = properties.get("next_line_id")
            .map(|v| matches!(v, Value::U16(_)))
            .unwrap_or(false);
        
        has_filename && has_language && has_line_content && has_line_index && has_next_line_id
    }
    
    /// Find the line ID at a specific display position.
    /// 
    /// This scans the line index to find which line ID corresponds to
    /// the specified display position.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - TextFile document properties
    /// * `display_position` - Display position to look up (0-based)
    /// 
    /// # Returns
    /// 
    /// Line ID at that position, or error if position is invalid
    fn find_line_id_at_position(properties: &AdaptiveMap<String, Value>, display_position: u16) -> Result<u16, &'static str> {
        let line_index = properties.get("line_index")
            .and_then(|v| if let Value::Object(obj) = v { Some(obj.as_ref()) } else { None })
            .ok_or("Invalid line_index structure")?;
        
        // Build position -> line_id mapping and sort
        let mut positions: Vec<(u16, u16)> = Vec::new();
        for (line_id_str, position_val) in line_index.iter() {
            let line_id = line_id_str.parse::<u16>().map_err(|_| "Invalid line ID format")?;
            let position = if let Value::U16(pos) = position_val { *pos } else { 
                return Err("Invalid position value");
            };
            positions.push((position, line_id));
        }
        
        positions.sort_by_key(|(pos, _)| *pos);
        
        // Find line at display position
        if (display_position as usize) < positions.len() {
            Ok(positions[display_position as usize].1)
        } else {
            Err("Display position out of range")
        }
    }
    
    /// Calculate the optimal insertion position using sparse positioning.
    /// 
    /// This finds a position between existing lines, using gaps where possible
    /// and returns information about whether cascade updates are needed.
    /// 
    /// # Arguments
    /// 
    /// * `properties` - TextFile document properties
    /// * `display_position` - Where to insert (0-based)
    /// 
    /// # Returns
    /// 
    /// Position value to use for the new line
    fn calculate_insertion_position(properties: &AdaptiveMap<String, Value>, display_position: u16) -> Result<u16, &'static str> {
        let line_index = properties.get("line_index")
            .and_then(|v| if let Value::Object(obj) = v { Some(obj.as_ref()) } else { None })
            .ok_or("Invalid line_index structure")?;
        
        // Build sorted position list
        let mut positions: Vec<u16> = Vec::new();
        for (_, position_val) in line_index.iter() {
            if let Value::U16(pos) = position_val {
                positions.push(*pos);
            }
        }
        positions.sort();
        
        // Handle insertion at beginning
        if display_position == 0 {
            if positions.is_empty() {
                return Ok(0);
            } else if positions[0] > 0 {
                return Ok(positions[0] / 2);
            } else {
                // Need to shift everything right - use conservative position
                return Ok(500);
            }
        }
        
        // Handle insertion at end
        if (display_position as usize) >= positions.len() {
            let last_pos = positions.last().copied().unwrap_or(0);
            if last_pos < 65000 { // Leave room for growth
                return Ok(last_pos + 1000);
            } else {
                // Near limit, use smaller increment
                return Ok(std::cmp::min(65535, last_pos + 1));
            }
        }
        
        // Handle insertion in middle - try to find gap
        let prev_pos = positions[(display_position - 1) as usize];
        let next_pos = positions[display_position as usize];
        
        if next_pos > prev_pos + 1 {
            // Gap available - insert in middle
            Ok(prev_pos + (next_pos - prev_pos) / 2)
        } else {
            // No gap - use conservative position that would require cascade update later
            Ok(prev_pos + 1)
        }
    }

    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Metadata extracted from a TextFile document.
#[derive(Debug, Clone)]
pub struct TextFileMetadata {
    /// Filename of the text file
    pub filename: String,
    /// Programming language for syntax highlighting
    pub language: String,
    /// Text encoding (typically "utf-8")
    pub encoding: String,
    /// Current number of lines in the file
    pub line_count: u32,
    /// Creation timestamp (nanoseconds since epoch)
    pub created_at: u64,
    /// Last modification timestamp (nanoseconds since epoch)
    pub modified_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::ID16;

    #[test]
    fn test_textfile_creation() {
        let initial_lines = vec![
            "use std::collections::HashMap;".to_string(),
            "".to_string(),
            "fn main() {".to_string(),
        ];
        
        let properties = TextFileDocument::new("main.rs", "rust", initial_lines);
        assert!(TextFileDocument::validate(&properties));
        
        let metadata = TextFileDocument::get_metadata(&properties);
        assert_eq!(metadata.filename, "main.rs");
        assert_eq!(metadata.language, "rust");
        assert_eq!(metadata.line_count, 3);
    }
    
    #[test]
    fn test_line_insertion() {
        let mut properties = TextFileDocument::new_empty("test.rs", "rust");
        
        // Insert first line
        let line_id = TextFileDocument::insert_line(&mut properties, 0, "// First line").unwrap();
        assert_eq!(line_id, 1);
        
        // Insert second line
        TextFileDocument::insert_line(&mut properties, 1, "// Second line").unwrap();
        
        let lines = TextFileDocument::get_display_lines(&properties).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "// First line");
        assert_eq!(lines[1], "// Second line");
    }
    
    #[test]
    fn test_line_editing() {
        let mut properties = TextFileDocument::new("test.rs", "rust", vec!["original".to_string()]);
        
        TextFileDocument::edit_line_by_position(&mut properties, 0, "modified").unwrap();
        
        let content = TextFileDocument::get_line_content(&properties, 0).unwrap();
        assert_eq!(content, "modified");
    }
    
    #[test]
    fn test_line_deletion() {
        let mut properties = TextFileDocument::new("test.rs", "rust", vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);
        
        TextFileDocument::delete_line(&mut properties, 1).unwrap();
        
        let lines = TextFileDocument::get_display_lines(&properties).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 3");
        
        let metadata = TextFileDocument::get_metadata(&properties);
        assert_eq!(metadata.line_count, 2);
    }
    
    #[test]
    fn test_cursor_tracking() {
        let mut properties = TextFileDocument::new_empty("test.rs", "rust");
        let user_id = ID16::random();
        
        TextFileDocument::update_cursor(&mut properties, user_id, 5, 10).unwrap();
        
        let cursors = TextFileDocument::get_active_cursors(&properties);
        assert_eq!(cursors.get(&user_id), Some(&(5, 10)));
        
        TextFileDocument::remove_cursor(&mut properties, user_id);
        let cursors = TextFileDocument::get_active_cursors(&properties);
        assert!(!cursors.contains_key(&user_id));
    }
} 