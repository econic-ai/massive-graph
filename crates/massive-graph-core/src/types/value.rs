
/// Wire format type identifiers - one-to-one with Value variants
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    // Primitives
    /// Null value
    Null = 0,
    /// Boolean value
    Bool = 1,
    /// i64 standard integer
    Int = 2,
    /// f64 standard float
    Float = 3,
    /// Timestamp value
    Timestamp = 4,

    // Special cases
    /// Undefined value
    Undefined = 8,
    
    // Variable length
    /// String value
    String = 16,
    /// Binary data
    Binary = 17,
    
    // Collections
    /// Array collection
    Array = 32,
    /// Map collection
    Map = 33,
    /// Object collection
    Object = 34,
    /// Set collection
    Collection = 35,
    
    // Spreadsheet
    /// State value
    State = 40,
    /// Derived value
    Derived = 41,
    /// Goal value
    Goal = 42,
    /// Optimizable value
    Optimizable = 43,
    /// Effect value
    Effect = 44,

    // Graph / Tree components
    /// Graph node
    Node = 50,
    /// Graph edge
    Edge = 51,
    /// Hyper edge
    HyperEdge = 52,

    /// Everything below this is also a document type
    /// The design is such that traversal can be seamless
    /// regardless of whether a value in a tree or a document.
    /// It should make no difference to performance or usability.

    // Direct refs (simple)
    /// Document reference
    DocumentRef = 128,
    /// Document version reference
    DocumentVersionRef = 129,
    /// Delta reference
    DeltaRef = 130,
    /// User reference
    UserRef = 131,

    // Structured types (complex)
    /// Tree structure
    Tree = 144,
    /// Graph structure
    Graph = 145,
    /// State graph structure
    StateGraph = 146,

    // Streams are linked lists
    /// Text stream
    TextStream = 160,
    /// Binary stream
    BinaryStream = 161,
    /// Delta stream
    DeltaStream = 162,
    /// Document stream
    DocumentStream = 163,
    /// Event stream
    EventStream = 164,
    
    /// All of these are designed for collaboration
    /// They use a BTree piece table for active state
    /// with periodic flattening for optimisation.

    // Statistical
    /// Matrix data structure
    Matrix = 176,
    /// Tensor data structure
    Tensor = 177,

    // Files
    /// Text file
    TextFile = 192,
    /// Binary file
    BinaryFile = 193,
    /// Table data structure
    Table = 194,

    // Reserved for system use
    /// System value
    System = 208,
    /// Event value
    Event = 209,

}

impl ValueType {
    /// Convert a u8 value to ValueType
    pub fn from_u8(value: u8) -> Self {
        // Safe because we control the wire format
        unsafe { std::mem::transmute(value) }
    }
}


/// Value reading from wire format - zero-copy view into chunk memory
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct Value<'a> {
    // Raw wire bytes - the complete value including type byte
    raw_bytes: &'a [u8],
    
    // Cached/parsed header for fast access (derived from raw_bytes)
    value_type: ValueType,    // First byte of raw_bytes
    data_offset: u8,          // Where actual data starts (after type + length)
    data_len: u32,            // Length of data (0 for fixed-size primitives)
}

impl<'a> Value<'a> {
    /// Parse from wire bytes
    pub fn from_bytes(raw_bytes: &'a [u8]) -> Self {
        let value_type = ValueType::from_u8(raw_bytes[0]);
        
        let (data_offset, data_len) = match value_type {
            // Fixed size - no length prefix
            ValueType::Null | ValueType::Undefined => (1, 0),
            ValueType::Bool => (1, 1),
            ValueType::Int | ValueType::Float | ValueType::Timestamp => (1, 8),

            // References - fixed size
            // ValueType::TextStream | ValueType::BinaryStream | ValueType::DeltaStream | ValueType::DocumentStream | ValueType::EventStream => (1, 16),            
            // ValueType::TextFile | ValueType::BinaryFile | ValueType::Table => (1, 16),            

            // Variable size - has varint length prefix
            _ => {
                let (len, bytes_read) = decode_varint(&raw_bytes[1..]);
                (1 + bytes_read as u8, len)
            }
        };
        
        Value {
            raw_bytes,
            value_type,
            data_offset,
            data_len,
        }
    }
    
    /// Get data bytes (excluding type and length prefix)
    pub fn data(&self) -> &'a [u8] {
        &self.raw_bytes[self.data_offset as usize..]
    }
    
    /// Get total size of this value in wire format
    pub fn total_size(&self) -> usize {
        self.raw_bytes.len()
    }
}


/// Decode a varint from bytes, returning (value, bytes_read)
fn decode_varint(bytes: &[u8]) -> (u32, usize) {
    let mut value = 0u32;
    let mut shift = 0;
    let mut bytes_read = 0;
    
    for byte in bytes {
        bytes_read += 1;
        value |= ((byte & 0x7F) as u32) << shift;
        
        if byte & 0x80 == 0 {
            // MSB is 0, this is the last byte
            break;
        }
        shift += 7;
        
        if bytes_read >= 5 {
            // Max 5 bytes for u32
            break;
        }
    }
    
    (value, bytes_read)
}