/// Module containing fixed-size identifier types optimized for efficient storage and comparison.
/// Uses base62 encoding [0-9a-zA-Z] for human-readable string representation while maintaining
/// fixed memory layout for zero-copy operations.
pub mod ids {
    use std::fmt;
    use std::str::FromStr;
    use rand::{Rng, rng};
    use std::collections::{HashMap, BTreeMap};
    use std::sync::atomic::{AtomicU64, AtomicU32, AtomicU16, AtomicU8, Ordering};
    use std::cell::OnceCell;
    use crate::constants::{BASE62_CHARS, ID16_LENGTH, ID32_LENGTH, CACHE_LINE_SIZE};

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ID16([u8; ID16_LENGTH]);

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ID8([u8; 8]);

    /// Fixed-size 32-byte identifier optimized for user identification.
    /// Uses base62 encoding for human-readable representation while maintaining
    /// a fixed memory layout for zero-copy operations.
    /// 
    /// Memory Layout:
    /// - [u8; 32] - Fixed array of 32 bytes
    /// 
    /// The #[repr(transparent)] ensures the struct has the same ABI as the underlying array,
    /// enabling direct transmutation in zero-copy operations.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ID32([u8; ID32_LENGTH]);    

    impl ID16 {
        /// Create a new ID from a 16-byte array
        pub fn new(bytes: [u8; ID16_LENGTH]) -> Self {
            ID16(bytes)
        }
        
        /// Get the underlying bytes
        pub fn as_bytes(&self) -> &[u8; ID16_LENGTH] {
            &self.0
        }
        
        /// Generate a random 16-character base62 ID
        /// Base62 uses [0-9a-zA-Z] (digits, lowercase, uppercase)
        pub fn random() -> Self {
            let mut rng = rand::rng();
            let mut bytes = [0u8; ID16_LENGTH];
            
            for i in 0..ID16_LENGTH {
                bytes[i] = BASE62_CHARS[rng.random_range(0..BASE62_CHARS.len())];
            }
            
            ID16(bytes)
        }

        /// Create an ID16 from a byte array
        pub fn from_bytes(bytes: [u8; ID16_LENGTH]) -> Self {
            ID16(bytes)
        }

        /// Convert to string representation
        pub fn as_str(&self) -> &str {
            // Safety: BASE62_CHARS only contains valid UTF-8 characters
            unsafe { std::str::from_utf8_unchecked(&self.0) }
        }
    }

    impl Default for ID16 {
        fn default() -> Self {
            ID16([b'0'; ID16_LENGTH]) // Default to all zeros (as '0' characters)
        }
    }

    impl Default for ID32 {
        fn default() -> Self {
            ID32([b'0'; ID32_LENGTH]) // Default to all zeros (as '0' characters)
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
            let mut rng = rand::rng();
            let mut bytes = [0u8; 8];
            
            for i in 0..8 {
                bytes[i] = BASE62_CHARS[rng.random_range(0..BASE62_CHARS.len())];
            }
            
            ID8(bytes)
        }
    }

    impl ID32 {
        /// Create a new ID from a 32-byte array
        pub fn new(bytes: [u8; ID32_LENGTH]) -> Self {
            ID32(bytes)
        }
        
        /// Get the underlying bytes
        pub fn as_bytes(&self) -> &[u8; ID32_LENGTH] {
            &self.0
        }
        
        /// Generate a random 32-character base62 ID
        /// Base62 uses [0-9a-zA-Z] (digits, lowercase, uppercase)
        pub fn random() -> Self {
            let mut rng = rand::rng();
            let mut bytes = [0u8; ID32_LENGTH];
            
            for i in 0..ID32_LENGTH {
                bytes[i] = BASE62_CHARS[rng.random_range(0..BASE62_CHARS.len())];
            }
            
            ID32(bytes)
        }

        /// Create an ID32 from a byte array
        pub fn from_bytes(bytes: [u8; ID32_LENGTH]) -> Self {
            ID32(bytes)
        }

        /// Convert to string representation
        pub fn as_str(&self) -> &str {
            // Safety: BASE62_CHARS only contains valid UTF-8 characters
            unsafe { std::str::from_utf8_unchecked(&self.0) }
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

    /// String conversion for ID32
    impl fmt::Display for ID32 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.as_str())
        }
    }    

    impl FromStr for ID16 {
        type Err = &'static str;
        
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() != ID16_LENGTH {
                return Err("ID must be exactly 16 characters");
            }
            
            let mut bytes = [0u8; ID16_LENGTH];
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

    impl FromStr for ID32 {
        type Err = &'static str;
        
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() != ID32_LENGTH {
                return Err("ID32 must be exactly 32 characters");
            }
            
            let mut bytes = [0u8; ID32_LENGTH];
            bytes.copy_from_slice(s.as_bytes());
            Ok(ID32(bytes))
        }
    }
}

// Use more concise names in the rest of the code
pub use ids::{ID16, ID8, ID32};
pub use document::{Handle, DocumentType, AdaptiveMap, Value, AppendOnlyStream, StreamEntry, DocumentHeader, Document};

/// Module containing the unified document model.
/// All entities (documents, edges, indexes, statistics, etc.) are represented as documents with children.
pub mod document {
    use super::ids::ID16;
    use std::collections::{HashMap, BTreeMap};
    use std::sync::atomic::{AtomicU64, AtomicU32, AtomicU16, AtomicU8, Ordering};
    use std::cell::OnceCell;
    use std::fmt;

    /// Handle to data stored in heap memory.
    /// This is a lightweight reference that doesn't own the underlying data.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Handle(pub u64);

    impl Handle {
        /// Create a new handle with the given ID
        pub fn new(id: u64) -> Self {
            Handle(id)
        }
        
        /// Get the underlying ID
        pub fn id(&self) -> u64 {
            self.0
        }
    }

    impl fmt::Display for Handle {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Handle({})", self.0)
        }
    }

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
        
        // Stream types - immutable append-only sequential streams
        /// Raw binary data with server timestamps
        BinaryStream = 60,
        /// Text/JSON strings with server timestamps
        TextStream = 61,
        
        // Delta and sequential document streams
        /// Single delta operation document
        Delta = 70,
        
        // Index and filter types
        /// Bloom filter for fast membership testing
        BloomFilter = 80,
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
    /// Variable-sized data (strings, streams) use handles for stable references.
    /// All variants are now 8 bytes for predictable document serialization.
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
        /// Binary data as byte vector (for small fixed-size data)
        Binary(Vec<u8>),
        /// Array of values
        Array(Vec<Value>),
        /// Object with string keys and value properties
        Object(Box<AdaptiveMap<String, Value>>),
        /// Reference to another document by ID
        Reference(ID16),
        
        // Handle-based types for variable-sized pooled data
        /// Handle to string in string pool (8 bytes, stable reference)
        String(Handle),
        /// Handle to binary stream in stream pool (8 bytes, stable reference)
        BinaryStream(Handle),
        /// Handle to text stream in stream pool (8 bytes, stable reference)
        TextStream(Handle),
        /// Handle to document stream in stream pool (8 bytes, stable reference)
        DocumentStream(Handle),
    }

    /// Handle-based append-only stream optimized for massive ordered collections.
    /// Each entry is stored in a separate Box for stable references and zero reallocation.
    /// 
    /// Key features:
    /// - Immutable entries created complete at append time
    /// - Each entry in stable Box - no reallocation issues
    /// - Server-generated timestamps for ordering
    /// - Efficient range queries by timestamp
    /// - Support for live subscriptions
    /// - Lock-free concurrent reads during writes
    #[derive(Debug, Clone)]
    pub struct AppendOnlyStream<T> 
    where 
        T: Clone,
    {
        /// Each entry in its own stable Box for zero reallocation
        entries: Vec<Box<StreamEntry<T>>>,
        
        /// Timestamp index for O(log n) time-based queries
        timestamp_index: BTreeMap<u64, usize>,
        
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

    /// AppendOnlyStream implementation with handle-based stable references
    impl<T> AppendOnlyStream<T>
    where
        T: Clone,
    {
        /// Create a new empty stream with initial capacity
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
                timestamp_index: BTreeMap::new(),
                entry_count: 0,
                latest_timestamp: 0,
                earliest_timestamp: u64::MAX,
            }
        }

        /// Append new data with server-generated timestamp, returns timestamp
        /// Entry is created complete and immutable - stored in stable Box
        pub fn append(&mut self, data: T) -> u64 {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;

            let entry = Box::new(StreamEntry { timestamp, data });
            let index = self.entries.len();
            
            // Add to timestamp index for O(log n) seeking
            self.timestamp_index.insert(timestamp, index);
            
            // Store in stable Box - address never changes
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
                .map(|boxed| boxed.as_ref())
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
            self.entries[start_idx..].iter().map(|boxed| boxed.as_ref()).collect()
        }

        /// Get the earliest N entries in chronological order
        pub fn earliest(&self, count: usize) -> Vec<&StreamEntry<T>> {
            let end_idx = std::cmp::min(count, self.entries.len());
            self.entries[..end_idx].iter().map(|boxed| boxed.as_ref()).collect()
        }

        /// Get entry at specific timestamp - O(log n) lookup
        pub fn get_at_time(&self, timestamp: u64) -> Option<&StreamEntry<T>> {
            let index = self.timestamp_index.get(&timestamp)?;
            self.entries.get(*index).map(|boxed| boxed.as_ref())
        }

        /// Get entry by index - O(1) lookup
        pub fn get_entry(&self, index: usize) -> Option<&StreamEntry<T>> {
            self.entries.get(index).map(|boxed| boxed.as_ref())
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

    
    /// Document header aligned to cache line boundary for optimal performance
    #[repr(C, align(64))]
    #[derive(Debug)]
    pub struct DocumentHeader {
        pub id: ID16,                    // 16 bytes
        pub version: AtomicU64,          // 8 bytes
        pub created_at: u64,             // 8 bytes
        pub modified_at: AtomicU64,      // 8 bytes
        pub doc_type: DocumentType,      // 1 byte
        pub data_size: u32,              // 4 bytes
        pub property_count: u16,         // 2 bytes
        pub checksum: u32,               // 4 bytes
        pub parent_id: ID16,             // 16 bytes
        pub total_child_count: u16,      // 2 bytes
        pub group_count: u8,             // 1 byte
        _padding: [u8; 2],               // 2 bytes padding = 64 total
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
        /// Document stream reference for zero-copy access to ordered document IDs
        DocumentStream(StreamRef<'a, ID16>),
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

    /// Zero-copy document view that references header and data without ownership.
    /// 
    /// This struct provides access to document metadata and properties without copying data.
    /// The lifetime 'a ensures the document view cannot outlive the underlying storage.
    #[derive(Debug)]
    pub struct Document<'a> {
        /// Reference to the fixed header
        pub header: &'a DocumentHeader,
        
        /// Raw data segment containing properties and children
        pub data: &'a [u8],
        
        /// Lazy-initialized property index for O(1) property access after first lookup.
        /// Built on-demand by scanning the binary property data once.
        /// Maps property name -> (offset, length) within the data slice.
        property_index: std::cell::OnceCell<HashMap<String, (usize, usize)>>,
    }

    /// Document implementation providing zero-copy access patterns
    impl<'a> Document<'a> {
        /// Create a new document view from header and data
        pub fn new(header: &'a DocumentHeader, data: &'a [u8]) -> Self {
            Self { header, data, property_index: OnceCell::new() }
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

        /// Get the number of children in this document (calculated from data)
        pub fn child_count(&self) -> u16 {
            // TODO: Parse child data to count children
            // This would scan the binary data to count child references
            0
        }

        /// Get the number of properties in this document (calculated from data)
        pub fn property_count(&self) -> u16 {
            // Use the lazy-initialized property index to get count
            let index = self.property_index.get_or_init(|| {
                self.build_property_index()
            });
            index.len() as u16
        }
        
        /// Get a property value by name with O(1) access after first lookup.
        /// First call builds the property index by scanning binary data (O(n)).
        /// Subsequent calls use the cached index for O(1) access.
        pub fn get_property(&self, property_name: &str) -> Option<&[u8]> {
            // Get or build the property index
            let index = self.property_index.get_or_init(|| {
                self.build_property_index()
            });
            
            // O(1) lookup into the index
            if let Some((offset, length)) = index.get(property_name) {
                if *offset + *length <= self.data.len() {
                    Some(&self.data[*offset..*offset + *length])
                } else {
                    None // Corrupted data
                }
            } else {
                None
            }
        }
        
        /// Build the property index by scanning the binary property data.
        /// This is called once lazily when first property access occurs.
        fn build_property_index(&self) -> HashMap<String, (usize, usize)> {
            let mut index = HashMap::new();
            let mut offset = 0;
            
            // TODO: Implement actual binary property parsing
            // This would scan the binary format:
            // [property_count][key1_len][key1_bytes][type1][value1_len][value1_bytes]...
            //
            // For now, return empty index
            while offset < self.data.len() {
                // Parse property header from binary data
                // Extract key name, value offset, and value length
                // Insert into index: property_name -> (value_offset, value_length)
                
                // Placeholder - would implement actual binary parsing
                break;
            }
            
            index
        }
        
        /// Get all property names in this document.
        /// Uses the lazy-initialized index for efficient access.
        pub fn property_names(&self) -> Vec<&str> {
            let index = self.property_index.get_or_init(|| {
                self.build_property_index()
            });
            
            index.keys().map(|k| k.as_str()).collect()
        }
        
        /// Check if a property exists in this document.
        pub fn has_property(&self, property_name: &str) -> bool {
            let index = self.property_index.get_or_init(|| {
                self.build_property_index()
            });
            
            index.contains_key(property_name)
        }
    }
}
