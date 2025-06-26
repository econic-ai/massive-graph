/// Module containing fixed-size identifier types optimized for efficient storage and comparison.
/// Uses base62 encoding [0-9a-zA-Z] for human-readable string representation while maintaining
/// fixed memory layout for zero-copy operations.
pub mod ids {
    use std::fmt;
    use std::str::FromStr;
    use rand::{Rng, rng};

    /// Fixed-size 16-byte identifier optimized for document references.
    /// Uses base62 encoding for human-readable representation while maintaining
    /// a fixed memory layout for zero-copy operations.
    /// 
    /// Memory Layout:
    /// - [u8; 16] - Fixed array of 16 bytes
    /// 
    /// The #[repr(transparent)] ensures the struct has the same ABI as the underlying array,
    /// enabling direct transmutation in zero-copy operations.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ID16([u8; 16]);

    /// Fixed-size 8-byte identifier optimized for delta/operation tracking.
    /// Uses base62 encoding for human-readable representation while maintaining
    /// a fixed memory layout for zero-copy operations.
    /// 
    /// Memory Layout:
    /// - [u8; 8] - Fixed array of 8 bytes
    /// 
    /// The #[repr(transparent)] ensures the struct has the same ABI as the underlying array,
    /// enabling direct transmutation in zero-copy operations.
    #[repr(transparent)]
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct ID8([u8; 8]);    

    impl ID16 {
        /// Create a new ID from a 16-byte array
        pub fn new(bytes: [u8; 16]) -> Self {
            ID16(bytes)
        }
        
        /// Get the underlying bytes
        pub fn as_bytes(&self) -> &[u8; 16] {
            &self.0
        }
        
        /// Generate a random 16-character base62 ID
        /// Base62 uses [0-9a-zA-Z] (digits, lowercase, uppercase)
        pub fn random() -> Self {
            const BASE62_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let mut rng = rng();
            let mut bytes = [0u8; 16];
            
            for i in 0..16 {
                // Generate a random index into the BASE62_CHARS array
                let idx = rng.random_range(0..BASE62_CHARS.len());
                bytes[i] = BASE62_CHARS[idx];
            }
            
            ID16(bytes)
        }
    }

    impl ID8 {
        /// Create a new ID from a 8-byte array
        pub fn new(bytes: [u8; 8]) -> Self {
            ID8(bytes)
        }
        
        /// Get the underlying bytes
        pub fn as_bytes(&self) -> &[u8; 8] {
            &self.0
        }
        
        /// Generate a random 8-character base62 ID
        /// Base62 uses [0-9a-zA-Z] (digits, lowercase, uppercase)
        pub fn random() -> Self {
            const BASE62_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let mut rng = rng();
            let mut bytes = [0u8; 8];
            
            for i in 0..8 {
                // Generate a random index into the BASE62_CHARS array
                let idx = rng.random_range(0..BASE62_CHARS.len());
                bytes[i] = BASE62_CHARS[idx];
            }
            
            ID8(bytes)
        }
    }    

    /// String conversion for ID16
    impl fmt::Display for ID16 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", String::from_utf8_lossy(&self.0))
        }
    }

    /// String conversion for ID8
    impl fmt::Display for ID8 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", String::from_utf8_lossy(&self.0))
        }
    }    

    impl FromStr for ID16 {
        type Err = &'static str;
        
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() != 16 {
                return Err("ID must be exactly 16 characters");
            }
            
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(s.as_bytes());
            Ok(ID16(bytes))
        }
    }

    impl FromStr for ID8 {
        type Err = &'static str;
        
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() != 8 {
                return Err("ID8 must be exactly 8 characters");
            }
            
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(s.as_bytes());
            Ok(ID8(bytes))
        }
    }
}

// Use more concise names in the rest of the code
pub use ids::{ID16, ID8};

/// Module containing the unified document model.
/// All entities (documents, edges, indexes, statistics, etc.) are represented as documents with children.
pub mod document {
    use super::ids::ID16;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Document type identifiers for different kinds of hypergraph entities.
    /// Each type determines how the document's properties and children should be interpreted.
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum DocumentType {

        /// Root document container (top-level namespace)
        Root = 0,

        // Core content types
        /// Generic document with no specific structure
        Generic = 1,
        /// Text document with string content
        Text = 2,
        /// Binary document with raw byte content
        Binary = 3,
        /// JSON document with structured data
        Json = 4,
        
        // Hypergraph structures  
        /// Root graph container
        Graph = 11,
        /// Individual node in the hypergraph
        Node = 12,
        /// Hyperedge connecting multiple children
        Edge = 13,
                
        // Index types (replace old grouping systems)
        /// Index by document type for fast type-based queries
        TypeIndex = 20,
        /// Index by name/tag for fast name-based lookups
        NameIndex = 21,
        /// Index by property values for fast property-based queries
        PropertyIndex = 22,
        
        // ML and analytics (now as documents with children)
        /// ML weights for a set of documents
        WeightSet = 30,
        /// Statistical analysis for a cohort
        StatisticalModel = 31,
        
        // Collections and organization
        /// Collection of related documents
        Collection = 40,
        /// Group of documents with shared properties
        Group = 41,
        
        // System types
        /// System metadata document
        Metadata = 50,
        
        // Stream types
        /// Raw binary data with server timestamps
        BinaryStream = 60,
        /// Text/JSON strings with server timestamps
        TextStream = 61,
    }

    /// Adaptive map that automatically chooses optimal internal structure.
    /// Uses BTreeMap for small collections (better cache locality) and HashMap for large ones (O(1) scaling).
    #[derive(Debug, Clone)]
    pub struct AdaptiveMap<K, V> 
    where 
        K: Clone + Ord + std::hash::Hash,
        V: Clone,
    {
        data: MapData<K, V>,
    }

    /// Internal data storage for AdaptiveMap
    #[derive(Debug, Clone)]
    enum MapData<K, V> 
    where 
        K: Clone + Ord + std::hash::Hash,
        V: Clone,
    {
        Small(std::collections::BTreeMap<K, V>),    // < 50 items
        Large(HashMap<K, V>),                       // >= 50 items
    }

    /// Implementation of AdaptiveMap with explicit match statements
    impl<K, V> AdaptiveMap<K, V>
    where
        K: Clone + Ord + std::hash::Hash,
        V: Clone,
    {
        /// Create a new empty adaptive map using BTreeMap (good for small collections)
        pub fn new() -> Self {
            Self {
                data: MapData::Small(std::collections::BTreeMap::new()),
            }
        }

        /// Create a new empty adaptive map using HashMap (good for large collections)
        pub fn new_large() -> Self {
            Self {
                data: MapData::Large(HashMap::new()),
            }
        }

        /// Insert a key-value pair
        pub fn insert(&mut self, key: K, value: V) -> Option<V> {
            match &mut self.data {
                MapData::Small(btree) => btree.insert(key, value),
                MapData::Large(hashmap) => hashmap.insert(key, value),
            }
        }

        /// Get a value by key
        pub fn get<Q>(&self, key: &Q) -> Option<&V>
        where
            K: std::borrow::Borrow<Q>,
            Q: Ord + std::hash::Hash + ?Sized,
        {
            match &self.data {
                MapData::Small(btree) => btree.get(key),
                MapData::Large(hashmap) => hashmap.get(key),
            }
        }

        /// Get a mutable reference to a value by key
        pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
        where
            K: std::borrow::Borrow<Q>,
            Q: Ord + std::hash::Hash + ?Sized,
        {
            match &mut self.data {
                MapData::Small(btree) => btree.get_mut(key),
                MapData::Large(hashmap) => hashmap.get_mut(key),
            }
        }

        /// Remove a key-value pair
        pub fn remove(&mut self, key: &K) -> Option<V> {
            match &mut self.data {
                MapData::Small(btree) => btree.remove(key),
                MapData::Large(hashmap) => hashmap.remove(key),
            }
        }

        /// Get the number of key-value pairs
        pub fn len(&self) -> usize {
            match &self.data {
                MapData::Small(btree) => btree.len(),
                MapData::Large(hashmap) => hashmap.len(),
            }
        }

        /// Check if the map is empty
        pub fn is_empty(&self) -> bool {
            match &self.data {
                MapData::Small(btree) => btree.is_empty(),
                MapData::Large(hashmap) => hashmap.is_empty(),
            }
        }

        /// Iterate over key-value pairs
        pub fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_> {
            match &self.data {
                MapData::Small(btree) => Box::new(btree.iter()),
                MapData::Large(hashmap) => Box::new(hashmap.iter()),
            }
        }

        /// Check if this map uses BTreeMap internally
        pub fn is_small(&self) -> bool {
            matches!(self.data, MapData::Small(_))
        }

        /// Check if this map uses HashMap internally
        pub fn is_large(&self) -> bool {
            matches!(self.data, MapData::Large(_))
        }
    }

    /// Equality comparison for AdaptiveMap across different internal structures
    impl<K, V> PartialEq for AdaptiveMap<K, V>
    where
        K: Clone + Ord + std::hash::Hash + PartialEq,
        V: Clone + PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            // Simple case: if lengths differ, they're not equal
            if self.len() != other.len() {
                return false;
            }
            
            // Compare all key-value pairs regardless of internal structure
            self.iter().all(|(k, v)| other.get(k) == Some(v))
        }
    }

    /// Value type for document properties supporting various data structures.
    /// Reduced from 48 bytes to 8 bytes by boxing large variants.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Value {
        /// Null value
        Null,
        /// Boolean true/false value
        Boolean(bool),
        /// 8-bit signed integer
        I8(i8),
        /// 16-bit signed integer
        I16(i16),
        /// 32-bit signed integer
        I32(i32),
        /// 64-bit signed integer
        I64(i64),
        /// 8-bit unsigned integer
        U8(u8),
        /// 16-bit unsigned integer
        U16(u16),
        /// 32-bit unsigned integer
        U32(u32),
        /// 64-bit unsigned integer
        U64(u64),
        /// 32-bit floating point number
        F32(f32),
        /// 64-bit floating point number
        F64(f64),
        /// UTF-8 string value
        String(String),
        /// Binary data as byte vector
        Binary(Vec<u8>),
        /// Array of values
        Array(Vec<Value>),
        /// Object with string keys and value properties
        Object(Box<AdaptiveMap<String, Value>>),
        /// Reference to another document by ID
        Reference(ID16),
        
        // Stream types for massive ordered collections
        /// Raw binary data with timestamps
        BinaryStream(Box<AppendOnlyStream<Vec<u8>>>),
        /// Text/JSON data with timestamps
        TextStream(Box<AppendOnlyStream<String>>),
    }

    /// Append-only stream optimized for massive ordered collections.
    /// Designed for lock-free concurrent access with server-generated timestamps.
    /// 
    /// Key features:
    /// - Append-only operations (no updates/deletes)
    /// - Server-generated timestamps for ordering
    /// - Efficient range queries by timestamp
    /// - Support for live subscriptions
    /// - Lock-free concurrent reads during writes
    #[derive(Debug, Clone)]
    pub struct AppendOnlyStream<T> 
    where 
        T: Clone,
    {
        /// Ordered entries by server timestamp
        entries: Vec<StreamEntry<T>>,
        
        /// Total number of entries (for quick size checks)
        entry_count: u64,
        
        /// Timestamp of the most recent entry
        latest_timestamp: u64,
        
        /// Timestamp of the oldest entry
        earliest_timestamp: u64,
    }

    /// Individual entry in an append-only stream.
    /// Contains server-generated timestamp and the actual data.
    #[derive(Debug, Clone)]
    pub struct StreamEntry<T> 
    where 
        T: Clone,
    {
        /// Server-generated timestamp (nanoseconds since epoch)
        pub timestamp: u64,
        
        /// The actual data (binary or text)
        pub data: T,
    }

    /// Equality comparison for streams
    impl<T> PartialEq for AppendOnlyStream<T>
    where
        T: Clone + PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.entries == other.entries
        }
    }

    /// Equality comparison for stream entries
    impl<T> PartialEq for StreamEntry<T>
    where
        T: Clone + PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.timestamp == other.timestamp && self.data == other.data
        }
    }

    /// AppendOnlyStream implementation with lock-free operations
    impl<T> AppendOnlyStream<T>
    where
        T: Clone,
    {
        /// Create a new empty stream with initial capacity
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
                entry_count: 0,
                latest_timestamp: 0,
                earliest_timestamp: u64::MAX,
            }
        }

        /// Append new data with server-generated timestamp, returns timestamp
        pub fn append(&mut self, data: T) -> u64 {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;

            let entry = StreamEntry { timestamp, data };
            
            self.entries.push(entry);
            self.entry_count += 1;
            self.latest_timestamp = timestamp;
            
            if self.earliest_timestamp == u64::MAX {
                self.earliest_timestamp = timestamp;
            }

            timestamp
        }

        /// Get entries in a timestamp range (inclusive)
        pub fn range(&self, start_time: u64, end_time: u64) -> Vec<&StreamEntry<T>> {
            self.entries
                .iter()
                .filter(|entry| entry.timestamp >= start_time && entry.timestamp <= end_time)
                .collect()
        }

        /// Get the latest N entries in chronological order
        pub fn latest(&self, count: usize) -> Vec<&StreamEntry<T>> {
            let start_idx = if self.entries.len() > count {
                self.entries.len() - count
            } else {
                0
            };
            self.entries[start_idx..].iter().collect()
        }

        /// Get the earliest N entries in chronological order
        pub fn earliest(&self, count: usize) -> Vec<&StreamEntry<T>> {
            let end_idx = std::cmp::min(count, self.entries.len());
            self.entries[..end_idx].iter().collect()
        }

        /// Get total number of entries in the stream
        pub fn len(&self) -> u64 {
            self.entry_count
        }

        /// Check if stream contains no entries
        pub fn is_empty(&self) -> bool {
            self.entry_count == 0
        }

        /// Get timestamp range of the stream (earliest, latest)
        pub fn timestamp_range(&self) -> Option<(u64, u64)> {
            if self.is_empty() {
                None
            } else {
                Some((self.earliest_timestamp, self.latest_timestamp))
            }
        }
    }

    /// Fixed-size document header optimized for CPU cache lines (64 bytes).
    /// Contains all metadata needed for document operations without data access.
    /// 
    /// Cache-aligned for optimal memory access patterns in concurrent scenarios.
    #[repr(C, align(64))]
    pub struct DocumentHeader {
        /// Immutable document identifier
        pub id: ID16,                           // 16 bytes
        
        /// Atomic version counter for MVCC and conflict resolution
        pub version: AtomicU64,                 // 8 bytes
        
        /// Creation timestamp (nanoseconds since epoch)
        pub created_at: u64,                    // 8 bytes
        
        /// Last modification timestamp (nanoseconds since epoch)
        pub modified_at: u64,             // 8 bytes
        
        /// Document type for dispatch optimization
        pub doc_type: DocumentType,             // 1 byte
        
        /// Total size of data segment in bytes
        pub data_size: u32,                     // 4 bytes
        
        /// Number of properties for iteration hints
        pub property_count: u16,                // 2 bytes
        
        /// Number of children for iteration hints
        pub child_count: u16,                   // 2 bytes
        
        /// Data integrity checksum
        pub checksum: u32,                      // 4 bytes
        
        /// Parent document ID (for bidirectional relationships)
        pub parent_id: ID16,                    // 16 bytes
        
        // Padding to reach 64-byte alignment
        _padding: [u8; 3],                      // 3 bytes padding
    }

    /// Zero-copy value reference without data ownership.
    /// Provides direct access to values stored in binary format.
    #[derive(Debug)]
    pub enum ValueRef<'a> {
        /// Null value reference
        Null,
        /// Boolean value reference
        Boolean(bool),
        /// 8-bit signed integer reference
        I8(i8),
        /// 16-bit signed integer reference
        I16(i16),
        /// 32-bit signed integer reference
        I32(i32),
        /// 64-bit signed integer reference
        I64(i64),
        /// 8-bit unsigned integer reference
        U8(u8),
        /// 16-bit unsigned integer reference
        U16(u16),
        /// 32-bit unsigned integer reference
        U32(u32),
        /// 64-bit unsigned integer reference
        U64(u64),
        /// 32-bit floating point reference
        F32(f32),
        /// 64-bit floating point reference
        F64(f64),
        /// String slice reference
        String(&'a str),
        /// Binary data slice reference
        Binary(&'a [u8]),
        /// Array reference for zero-copy access
        Array(ArrayRef<'a>),
        /// Object reference for zero-copy access
        Object(ObjectRef<'a>),
        /// Reference to another document by ID
        Reference(ID16),
        
        // Stream references for zero-copy access
        /// Binary stream reference for zero-copy access
        BinaryStream(StreamRef<'a, Vec<u8>>),
        /// Text stream reference for zero-copy access
        TextStream(StreamRef<'a, String>),
    }

    /// Reference to an array value stored in binary format
    #[derive(Debug)]
    pub struct ArrayRef<'a> {
        /// Raw binary data containing the array elements
        data: &'a [u8],
        /// Number of elements in the array
        element_count: u32,
    }

    /// Implementation of ArrayRef for zero-copy array access
    impl<'a> ArrayRef<'a> {
        /// Create a new array reference from raw data
        pub fn new(data: &'a [u8], element_count: u32) -> Self {
            Self { data, element_count }
        }

        /// Get the number of elements in the array
        pub fn len(&self) -> u32 {
            self.element_count
        }

        /// Check if the array is empty
        pub fn is_empty(&self) -> bool {
            self.element_count == 0
        }

        /// Get the raw binary data
        pub fn data(&self) -> &'a [u8] {
            self.data
        }

        /// Parse a specific element by index (placeholder implementation)
        pub fn get_element(&self, _index: u32) -> Option<ValueRef<'a>> {
            // TODO: Implement binary parsing for array elements
            // This would parse the binary data at the given index
            None
        }
    }

    /// Reference to an object value stored in binary format  
    #[derive(Debug)]
    pub struct ObjectRef<'a> {
        /// Raw binary data containing the object properties
        data: &'a [u8],
        /// Number of properties in the object
        property_count: u32,
    }

    /// Implementation of ObjectRef for zero-copy object access
    impl<'a> ObjectRef<'a> {
        /// Create a new object reference from raw data
        pub fn new(data: &'a [u8], property_count: u32) -> Self {
            Self { data, property_count }
        }

        /// Get the number of properties in the object
        pub fn len(&self) -> u32 {
            self.property_count
        }

        /// Check if the object has no properties
        pub fn is_empty(&self) -> bool {
            self.property_count == 0
        }

        /// Get the raw binary data
        pub fn data(&self) -> &'a [u8] {
            self.data
        }

        /// Get a property by key (placeholder implementation)
        pub fn get_property(&self, _key: &str) -> Option<ValueRef<'a>> {
            // TODO: Implement binary parsing for object properties
            // This would parse the binary data to find the property by key
            None
        }

        /// Iterator over property keys (placeholder implementation)
        pub fn keys(&self) -> impl Iterator<Item = &'a str> {
            // TODO: Implement binary parsing for property keys
            std::iter::empty()
        }
    }

    /// Reference to a stream value stored in binary format
    #[derive(Debug)]
    pub struct StreamRef<'a, T> 
    where 
        T: Clone,
    {
        /// Raw binary data containing the stream entries
        data: &'a [u8],
        /// Number of entries in the stream
        entry_count: u64,
        /// Phantom data for type safety
        _phantom: std::marker::PhantomData<T>,
    }

    /// Implementation of StreamRef for zero-copy stream access
    impl<'a, T> StreamRef<'a, T>
    where
        T: Clone,
    {
        /// Create a new stream reference from raw data
        pub fn new(data: &'a [u8], entry_count: u64) -> Self {
            Self { 
                data, 
                entry_count, 
                _phantom: std::marker::PhantomData 
            }
        }

        /// Get the number of entries in the stream
        pub fn len(&self) -> u64 {
            self.entry_count
        }

        /// Check if the stream is empty
        pub fn is_empty(&self) -> bool {
            self.entry_count == 0
        }

        /// Get the raw binary data
        pub fn data(&self) -> &'a [u8] {
            self.data
        }

        /// Get a specific entry by index (placeholder implementation)
        pub fn get_entry(&self, _index: u64) -> Option<(u64, &'a [u8])> {
            // TODO: Implement binary parsing for stream entries
            // This would return (timestamp, data) for the entry at index
            None
        }

        /// Get entries in a timestamp range (placeholder implementation)
        pub fn range(&self, _start_time: u64, _end_time: u64) -> impl Iterator<Item = (u64, &'a [u8])> {
            // TODO: Implement binary parsing for timestamp ranges
            std::iter::empty()
        }
    }

    /// Document handle providing zero-copy access to stored data.
    /// The document doesn't own data - it's a view into memory segments.
    /// 
    /// Lifetime 'a ensures document cannot outlive the underlying storage.
    pub struct Document<'a> {
        /// Reference to the fixed header
        pub header: &'a DocumentHeader,
        
        /// Raw data segment containing properties and children
        pub data: &'a [u8],
    }

    /// Document implementation providing zero-copy access patterns
    impl<'a> Document<'a> {
        /// Create a new document view from header and data
        pub fn new(header: &'a DocumentHeader, data: &'a [u8]) -> Self {
            Self { header, data }
        }

        /// Get immutable document identifier
        pub fn id(&self) -> ID16 {
            self.header.id
        }

        /// Get document type for dispatch optimization
        pub fn doc_type(&self) -> DocumentType {
            self.header.doc_type
        }

        /// Get current document version atomically
        pub fn version(&self) -> u64 {
            self.header.version.load(Ordering::Acquire)
        }

        /// Get parent document ID for bidirectional relationships
        pub fn parent(&self) -> ID16 {
            self.header.parent_id
        }

        /// Get child count without parsing children data
        pub fn child_count(&self) -> u16 {
            self.header.child_count
        }

        /// Get property count without parsing property data
        pub fn property_count(&self) -> u16 {
            self.header.property_count
        }
    }
}

/// Module containing delta operations for real-time synchronization and fine-grained updates.
/// Optimized for minimal wire overhead and zero-copy application.
pub mod delta {
    use super::ids::{ID16, ID8};
    use bytes::Bytes;

    /// Operation types for granular document modifications.
    /// Each operation is designed for minimal overhead and atomic application.
    /// 
    /// Bit layout for access control:
    /// - 0x00-0x7F (0-127): User operations (bit 7 = 0)
    /// - 0x80-0xFF (128-255): Privileged operations (bit 7 = 1)
    ///   - 0x80-0x9F: Admin operations (bits 7,5 = 1,0)  
    ///   - 0xA0-0xBF: Statistical operations (bits 7,5 = 1,1)
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum OpType {
        // User operations (0x00-0x7F) - bit 7 = 0
        
        // Property operations
        /// Set a property value on a document
        PropertySet = 0,
        /// Delete a property from a document
        PropertyDelete = 1,
        
        // String property operations
        /// Insert text at a specific position in a string property
        StringInsert = 2,
        /// Remove text from a specific range in a string property
        StringRemove = 3,
        
        // Numeric property operations
        /// Increment a numeric property value (integers, floats, booleans)
        PropertyIncrement = 4,
        /// Decrement a numeric property value (integers, floats, booleans)
        PropertyDecrement = 5,
        
        // Children operations
        /// Add a child relationship to a document
        ChildAdd = 10,
        /// Remove a child relationship from a document
        ChildRemove = 11,
        /// Replace all children with a new set
        ChildReplace = 12,
        /// Move a child to a new position
        ChildMove = 13,
        /// Copy a child to a new document
        ChildCopy = 14,
        
        // Stream operations
        /// Append binary data to stream
        StreamAppendBinary = 15,
        /// Append text data to stream
        StreamAppendText = 16,
        
        // Document lifecycle
        /// Create a new document
        DocumentCreate = 20,
        /// Delete an existing document
        DocumentDelete = 21,
        /// Move document to new parent
        DocumentMove = 22,
        /// Copy document with new ID
        DocumentCopy = 23,
        
        // Privileged operations (0x80-0xFF) - bit 7 = 1
        
        // Admin operations (0x80-0x9F) - bits 7,5 = 1,0
        /// Rebuild indexes for performance optimization
        Reindex = 0x80,
        /// Truncate old data/logs for storage management  
        Truncate = 0x81,
        /// Compact storage and optimize layout
        Compact = 0x82,
        /// Backup data to external storage
        Backup = 0x83,
        /// Restore data from backup
        Restore = 0x84,
        
        // Statistical/ML operations (0xA0-0xBF) - bits 7,5 = 1,1  
        /// Calculate and update ML weights
        CalculateWeight = 0xA0,
        /// Update statistical model parameters
        UpdateStatistics = 0xA1,
        /// Rebuild statistical indexes
        RebuildStatistics = 0xA2,
        /// Synchronize distributed model parameters
        SyncModel = 0xA3,
    }

    /// Zero-copy operation parsed from binary stream.
    /// References data without allocation.
    pub struct Operation<'a> {
        /// Type of operation to perform
        pub op_type: OpType,
        /// Target document ID for this operation
        pub target_id: ID16,
        /// Sequence number for operation ordering
        pub sequence: u64,
        /// Raw payload data for the operation
        pub payload: &'a [u8],
    }

    /// Delta containing multiple operations for atomic application.
    /// Designed for efficient network transmission and storage.
    pub struct Delta {
        /// Delta metadata
        pub header: DeltaHeader,
        
        /// Serialized operations
        pub data: Bytes,
    }

    /// Delta header containing batch metadata
    #[repr(C)]
    pub struct DeltaHeader {
        /// Unique delta identifier
        pub id: ID8,                            // 8 bytes
        
        /// Creation timestamp
        pub timestamp: u64,                     // 8 bytes
        
        /// Executor/author of these operations
        pub executor_id: ID16,                  // 16 bytes
        
        /// Number of operations in this delta
        pub op_count: u32,                      // 4 bytes
        
        /// Total size of operation data
        pub data_size: u32,                     // 4 bytes
        
    }


} 