use crate::{core::utils::current_timestamp, types::ValueType};

/// Field address with parameters for V2 wire format
/// Parent requirement is predefined in descriptor, not sent on wire
pub struct FieldAddress {
    /// Schema version
    pub schema_version: u16,
    /// Field index
    pub field_index: u32,
    /// Raw parameter data following the field descriptor's expected groups
    pub params_raw: (*const u8, usize),
}

impl FieldAddress {
    /// Create parameter iterator based on field descriptor's param groups
    pub fn params_iter<'a>(&self, param_groups: &'a [ParamGroup]) -> ParamIterator<'a> {
        ParamIterator::new(self.params_raw.0, self.params_raw.1, param_groups)
    }
}

/// Iterator over parameter groups without allocation
pub struct ParamIterator<'a> {
    ptr: *const u8,
    remaining: usize,
    groups: &'a [ParamGroup],
    current_group: usize,
}

impl<'a> ParamIterator<'a> {
    fn new(ptr: *const u8, len: usize, groups: &'a [ParamGroup]) -> Self {
        Self {
            ptr,
            remaining: len,
            groups,
            current_group: 0,
        }
    }
    
    /// Get next parameter group's data
    pub fn next_group(&mut self) -> Option<ParamGroupData> {
        if self.current_group >= self.groups.len() || self.remaining == 0 {
            return None;
        }
        
        let group_type = &self.groups[self.current_group];
        self.current_group += 1;
        
        // Read count byte
        if self.remaining < 1 {
            return None;
        }
        let count = unsafe { *self.ptr };
        self.ptr = unsafe { self.ptr.add(1) };
        self.remaining -= 1;
        
        let start_ptr = self.ptr;
        let start_remaining = self.remaining;
        
        // Calculate how much data this group consumes
        match group_type {
            ParamGroup::KeySet => {
                // Skip over count keys (each has length prefix + data)
                for _ in 0..count {
                    if self.remaining < 1 {
                        return None;
                    }
                    let key_len = unsafe { *self.ptr } as usize;
                    let total = 1 + key_len;
                    if self.remaining < total {
                        return None;
                    }
                    self.ptr = unsafe { self.ptr.add(total) };
                    self.remaining -= total;
                }
            }
            ParamGroup::ArraySet => {
                // Skip over array parameters (type byte + data)
                for _ in 0..count {
                    if self.remaining < 1 {
                        return None;
                    }
                    let param_type = unsafe { *self.ptr };
                    self.ptr = unsafe { self.ptr.add(1) };
                    self.remaining -= 1;
                    
                    // Skip data based on type
                    let data_size = match param_type {
                        0x01 => 1, // Single index (varint)
                        0x02 => { // Multiple indices
                            if self.remaining < 1 { return None; }
                            let n = unsafe { *self.ptr } as usize;
                            1 + n // count + n varints
                        }
                        0x03 => 2, // Range (2 varints)
                        0x04 => { // Multiple ranges
                            if self.remaining < 1 { return None; }
                            let n = unsafe { *self.ptr } as usize;
                            1 + n * 2 // count + n range pairs
                        }
                        0x05 => { // Dimensions
                            if self.remaining < 1 { return None; }
                            let n = unsafe { *self.ptr } as usize;
                            1 + n // count + n dimension values
                        }
                        _ => return None,
                    };
                    
                    if self.remaining < data_size {
                        return None;
                    }
                    self.ptr = unsafe { self.ptr.add(data_size) };
                    self.remaining -= data_size;
                }
            }
        }
        
        let consumed = start_remaining - self.remaining;
        
        Some(ParamGroupData {
            group_type: group_type.clone(),
            count,
            data: (start_ptr, consumed),
        })
    }
}

/// Raw parameter group data
pub struct ParamGroupData {
    /// Group type
    pub group_type: ParamGroup,
    /// Count
    pub count: u8,
    /// Pointer to raw data including the count byte
    pub data: (*const u8, usize),
}

/// Parameter group types determined by field descriptor
#[derive(Clone, Debug, PartialEq)]
pub enum ParamGroup {
    /// Map key selection - "users.{}"
    KeySet,
    /// Array element selection - "items[]"
    ArraySet,
}

/// Array parameter types (require type byte in wire format)
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum ArrayParamType {
    /// Single index
    Index = 0x01,        // Single index
    /// Multiple indices
    Indices = 0x02,      // Multiple indices
    /// Single range
    Range = 0x03,        // Single range
    /// Multiple ranges
    Ranges = 0x04,       // Multiple ranges
    /// Tensor/matrix dimensions
    Dimensions = 0x05,   // Tensor/matrix dimensions
}

/// Field descriptor stored in schema
#[derive(Clone, Debug)]
pub struct FieldDescriptor {
    /// The field path (e.g., "users[].profile.{}.name")
    /// Only alphanumeric segments, with [], {}, . as special markers
    pub path: String,
    
    /// The data type of this field
    pub value_type: ValueType,
    
    /// Whether this field requires a parent context (path starts with *)
    pub requires_parent: bool,
    
    /// Ordered parameter groups expected by this field
    pub param_groups: Vec<ParamGroup>,
    
    /// Creation timestamp
    pub created_at: u64,

    // TODO we may need references to previous schema versions here for backward compatibility
}

impl FieldDescriptor {
    /// Create new field descriptor, parsing path to determine structure
    pub fn new(path: String, value_type: ValueType) -> Result<Self, String> {
        let (requires_parent, clean_path) = if path.starts_with('*') {
            let mut stripped = path[1..].to_string();
            // If after stripping * we have a leading dot, remove it too
            if stripped.starts_with('.') {
                stripped = stripped[1..].to_string();
            }
            (true, stripped)
        } else {
            (false, path)
        };
        
        let param_groups = Self::parse_and_validate_path(&clean_path)?;
        
        Ok(Self {
            path: clean_path,
            value_type,
            requires_parent,
            param_groups,
            created_at: current_timestamp(),
        })
    }
    
    /// Parse and validate path, returning parameter groups
    fn parse_and_validate_path(path: &str) -> Result<Vec<ParamGroup>, String> {
        if path.is_empty() {
            return Err("Invalid path: empty path not allowed".to_string());
        }
        
        if path.starts_with('.') {
            return Err("Invalid path: cannot start with '.'".to_string());
        }
        
        let mut groups = Vec::new();
        let mut chars = path.chars().peekable();
        let mut in_segment = false;
        let mut after_dot = true; // We start as if after a dot to require initial segment
        
        while let Some(ch) = chars.next() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' => {
                    in_segment = true;
                    after_dot = false;
                }
                '.' => {
                    if after_dot {
                        return Err("Invalid path: cannot have consecutive dots".to_string());
                    }
                    if !in_segment {
                        return Err("Invalid path: dot must follow a segment or bracket pair".to_string());
                    }
                    in_segment = false;
                    after_dot = true;
                }
                '[' => {
                    if let Some(&']') = chars.peek() {
                        chars.next(); // consume ']'
                        groups.push(ParamGroup::ArraySet);
                        in_segment = true;
                        after_dot = false;
                    } else {
                        return Err("Invalid path: '[' must be immediately followed by ']'".to_string());
                    }
                }
                '{' => {
                    if let Some(&'}') = chars.peek() {
                        chars.next(); // consume '}'
                        groups.push(ParamGroup::KeySet);
                        in_segment = true;
                        after_dot = false;
                    } else {
                        return Err("Invalid path: '{' must be immediately followed by '}'".to_string());
                    }
                }
                ']' => return Err("Invalid path: unexpected ']' without matching '['".to_string()),
                '}' => return Err("Invalid path: unexpected '}' without matching '{'".to_string()),
                _ => return Err(format!("Invalid character '{}' in path", ch)),
            }
        }
        
        if after_dot {
            return Err("Invalid path: cannot end with '.'".to_string());
        }
        
        Ok(groups)
    }
}

/// Parameters for encoding (high-level API)
#[derive(Clone, Debug)]
pub struct FieldParams {
    /// List of parameter groups
    pub groups: Vec<ParamGroupValues>,
}

/// Values for a parameter group
#[derive(Clone, Debug)]
pub enum ParamGroupValues {
    /// Map key selection - "users.{}"
    KeySet(Vec<String>),
    /// Array element selection - "items[]"
    ArraySet(Vec<ArrayParam>),
}

/// Array parameter values
#[derive(Clone, Debug)]
pub enum ArrayParam {
    /// Single index
    Index(u32),
    /// Multiple indices
    Indices(Vec<u32>),
    /// Single range
    Range(u32, u32),
    /// Multiple ranges
    Ranges(Vec<(u32, u32)>),
    /// Tensor/matrix dimensions
    Dimensions(Vec<u32>),
}

impl ArrayParam {
    /// Get the type byte for wire encoding
    pub fn type_byte(&self) -> u8 {
        match self {
            ArrayParam::Index(_) => ArrayParamType::Index as u8,
            ArrayParam::Indices(_) => ArrayParamType::Indices as u8,
            ArrayParam::Range(_, _) => ArrayParamType::Range as u8,
            ArrayParam::Ranges(_) => ArrayParamType::Ranges as u8,
            ArrayParam::Dimensions(_) => ArrayParamType::Dimensions as u8,
        }
    }
    
    /// Calculate encoded size (excluding type byte)
    pub fn encoded_size(&self) -> usize {
        match self {
            ArrayParam::Index(_) => 1, // 1 varint
            ArrayParam::Indices(v) => 1 + v.len(), // count + values
            ArrayParam::Range(_, _) => 2, // 2 varints
            ArrayParam::Ranges(v) => 1 + v.len() * 2, // count + pairs
            ArrayParam::Dimensions(v) => 1 + v.len(), // count + values
        }
    }
}

impl FieldParams {
    /// Create empty parameters
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }
    
    /// Add a key set
    pub fn add_keys(&mut self, keys: Vec<String>) {
        self.groups.push(ParamGroupValues::KeySet(keys));
    }
    
    /// Add array parameters
    pub fn add_array(&mut self, params: Vec<ArrayParam>) {
        self.groups.push(ParamGroupValues::ArraySet(params));
    }
    
    /// Calculate total encoded size
    pub fn encoded_size(&self) -> usize {
        let mut size = 0;
        
        for group in &self.groups {
            size += 1; // Count byte
            
            match group {
                ParamGroupValues::KeySet(keys) => {
                    for key in keys {
                        size += 1 + key.len(); // Length byte + string
                    }
                }
                ParamGroupValues::ArraySet(params) => {
                    for param in params {
                        size += 1 + param.encoded_size(); // Type byte + data
                    }
                }
            }
        }
        
        size
    }
    
    /// Encode to wire format following field descriptor's expected groups
    pub fn encode(&self, field_desc: &FieldDescriptor) -> Result<Vec<u8>, String> {
        if self.groups.len() != field_desc.param_groups.len() {
            return Err(format!(
                "Parameter group count mismatch: got {}, expected {}",
                self.groups.len(),
                field_desc.param_groups.len()
            ));
        }
        
        let mut output = Vec::with_capacity(self.encoded_size());
        
        for (i, group) in self.groups.iter().enumerate() {
            let expected = &field_desc.param_groups[i];
            
            match (group, expected) {
                (ParamGroupValues::KeySet(keys), ParamGroup::KeySet) => {
                    output.push(keys.len() as u8);
                    for key in keys {
                        output.push(key.len() as u8);
                        output.extend_from_slice(key.as_bytes());
                    }
                }
                (ParamGroupValues::ArraySet(params), ParamGroup::ArraySet) => {
                    output.push(params.len() as u8);
                    for param in params {
                        output.push(param.type_byte());
                        // Encode param data (simplified - would use actual varint encoding)
                        match param {
                            ArrayParam::Index(idx) => output.push(*idx as u8),
                            ArrayParam::Indices(indices) => {
                                output.push(indices.len() as u8);
                                for idx in indices {
                                    output.push(*idx as u8);
                                }
                            }
                            ArrayParam::Range(start, end) => {
                                output.push(*start as u8);
                                output.push(*end as u8);
                            }
                            ArrayParam::Ranges(ranges) => {
                                output.push(ranges.len() as u8);
                                for (start, end) in ranges {
                                    output.push(*start as u8);
                                    output.push(*end as u8);
                                }
                            }
                            ArrayParam::Dimensions(dims) => {
                                output.push(dims.len() as u8);
                                for dim in dims {
                                    output.push(*dim as u8);
                                }
                            }
                        }
                    }
                }
                _ => return Err("Parameter type mismatch".to_string()),
            }
        }
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_path_parsing() {
        // Test simple path
        let fd = FieldDescriptor::new("users".to_string(), ValueType::String).unwrap();
        assert_eq!(fd.param_groups.len(), 0);
        assert!(!fd.requires_parent);
        
        // Test complex path with arrays and maps
        let fd = FieldDescriptor::new("users[].profile.{}.names[]".to_string(), ValueType::String).unwrap();
        assert_eq!(fd.param_groups.len(), 3);
        assert_eq!(fd.param_groups[0], ParamGroup::ArraySet);
        assert_eq!(fd.param_groups[1], ParamGroup::KeySet);
        assert_eq!(fd.param_groups[2], ParamGroup::ArraySet);
        assert!(!fd.requires_parent);
        
        // Test parent requirement
        let fd = FieldDescriptor::new("*name".to_string(), ValueType::String).unwrap();
        assert!(fd.requires_parent);
        assert_eq!(fd.param_groups.len(), 0);
        assert_eq!(fd.path, "name"); // * should be stripped
        
        // Test parent with dot path (*.property is valid)
        let fd = FieldDescriptor::new("*.property".to_string(), ValueType::String).unwrap();
        assert!(fd.requires_parent);
        assert_eq!(fd.path, "property"); // Leading dot is stripped along with *
        
        // Test parent with longer path
        let fd = FieldDescriptor::new("*user.name".to_string(), ValueType::String).unwrap();
        assert!(fd.requires_parent);
        assert_eq!(fd.path, "user.name");
        
        // Test all valid characters
        let fd = FieldDescriptor::new("user123.profileABC".to_string(), ValueType::Int).unwrap();
        assert_eq!(fd.param_groups.len(), 0);
    }
    
    #[test]
    fn test_invalid_paths() {
        // Test unclosed bracket
        assert!(FieldDescriptor::new("users[".to_string(), ValueType::String).is_err());
        assert!(FieldDescriptor::new("users{".to_string(), ValueType::String).is_err());
        
        // Test mismatched brackets
        assert!(FieldDescriptor::new("users]".to_string(), ValueType::String).is_err());
        assert!(FieldDescriptor::new("users}".to_string(), ValueType::String).is_err());
        
        // Test non-adjacent brackets
        assert!(FieldDescriptor::new("users[x]".to_string(), ValueType::String).is_err());
        assert!(FieldDescriptor::new("users{key}".to_string(), ValueType::String).is_err());
        
        // Test invalid characters
        assert!(FieldDescriptor::new("users-profile".to_string(), ValueType::String).is_err());
        assert!(FieldDescriptor::new("users@profile".to_string(), ValueType::String).is_err());
        assert!(FieldDescriptor::new("users profile".to_string(), ValueType::String).is_err());
        
        // Test empty path
        assert!(FieldDescriptor::new("".to_string(), ValueType::String).is_err());
        assert!(FieldDescriptor::new("*".to_string(), ValueType::String).is_err()); // Just * is invalid
        
        // Test trailing dot
        assert!(FieldDescriptor::new("users.".to_string(), ValueType::String).is_err());
        
        // Test leading dot
        assert!(FieldDescriptor::new(".users".to_string(), ValueType::String).is_err());
        
        // Test double dots
        assert!(FieldDescriptor::new("users..profile".to_string(), ValueType::String).is_err());
    }
    
    #[test]
    fn test_array_param_encoding() {
        let mut params = FieldParams::new();
        
        // Test single index
        params.add_array(vec![ArrayParam::Index(42)]);
        assert_eq!(params.groups.len(), 1);
        
        // Test multiple indices
        params.add_array(vec![ArrayParam::Indices(vec![1, 2, 3])]);
        assert_eq!(params.groups.len(), 2);
        
        // Test ranges
        params.add_array(vec![ArrayParam::Range(0, 10)]);
        assert_eq!(params.groups.len(), 3);
    }
    
    #[test]
    fn test_encoded_size_calculation() {
        let mut params = FieldParams::new();
        
        // Empty params = 0 size
        assert_eq!(params.encoded_size(), 0);
        
        // Single key set with one key
        params.add_keys(vec!["test".to_string()]);
        assert_eq!(params.encoded_size(), 1 + 1 + 4); // count + length + "test"
        
        // Add array params
        params.add_array(vec![ArrayParam::Index(5)]);
        // Total size is now: KeySet(6) + ArraySet(3) = 9
        assert_eq!(params.encoded_size(), 9);
    }
}
