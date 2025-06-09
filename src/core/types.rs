//! High-performance type definitions for Massive Graph Database
//! 
//! This module contains optimized data types designed for maximum performance,
//! zero-copy operations, and minimal memory overhead.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// High-performance 16-byte ID optimized for hashing and comparison
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct NodeId([u8; 16]);

/// High-performance 16-byte edge ID
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EdgeId([u8; 16]);

/// Timestamp type optimized for ordering and comparison
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Timestamp(u64);

/// Version number for optimistic concurrency control
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Version(u64);

/// Compact property key using interned strings for efficiency
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PropertyKey(Arc<str>);

/// Optimized value type supporting zero-copy operations
#[derive(Clone, Debug)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// 64-bit signed integer
    Int(i64),
    /// 64-bit unsigned integer
    UInt(u64),
    /// 64-bit floating point
    Float(f64),
    /// String value with zero-copy bytes
    String(Arc<str>),
    /// Binary data with zero-copy bytes (simplified for now)
    Bytes(Vec<u8>),
    /// Array of values (simplified for now)
    Array(Vec<Value>),
    /// Object/map of key-value pairs (simplified for now)
    Object(HashMap<PropertyKey, Value>),
    /// Reference to another node
    NodeRef(NodeId),
    /// Reference to an edge
    EdgeRef(EdgeId),
}

/// Property map optimized for concurrent access
pub type Properties = HashMap<PropertyKey, Value>;

/// Edge direction for traversal optimization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// Outgoing edge (from this node)
    Out,
    /// Incoming edge (to this node)
    In,
    /// Either direction
    Both,
}

/// Label type for nodes and edges with interned strings
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Label(Arc<str>);

// Implementations for NodeId
impl NodeId {
    /// Create a new random node ID
    pub fn new() -> Self {
        let mut bytes = [0u8; 16];
        // Use a fast random number generator
        use std::collections::hash_map::DefaultHasher;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let nanos = now.as_nanos() as u64;
        
        // Combine timestamp with thread ID for uniqueness
        let thread_id = std::thread::current().id();
        let mut hasher = DefaultHasher::new();
        nanos.hash(&mut hasher);
        thread_id.hash(&mut hasher);
        
        let hash1 = hasher.finish();
        
        // Second hash for additional entropy
        let mut hasher = DefaultHasher::new();
        hash1.hash(&mut hasher);
        let hash2 = hasher.finish();
        
        bytes[0..8].copy_from_slice(&hash1.to_ne_bytes());
        bytes[8..16].copy_from_slice(&hash2.to_ne_bytes());
        
        Self(bytes)
    }
    
    /// Create from byte array
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    
    /// Get byte array
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
    
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Parse from string representation
    pub fn from_string(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() == 16 {
            let mut array = [0u8; 16];
            array.copy_from_slice(&bytes);
            Ok(Self(array))
        } else {
            Err(hex::FromHexError::InvalidStringLength)
        }
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl Hash for NodeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Use first 8 bytes for fast hashing
        let hash = u64::from_ne_bytes([
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5], self.0[6], self.0[7],
        ]);
        hash.hash(state);
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", hex::encode(&self.0[..4]))
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// Similar implementations for EdgeId
impl EdgeId {
    pub fn new() -> Self {
        let mut bytes = [0u8; 16];
        use std::collections::hash_map::DefaultHasher;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let nanos = now.as_nanos() as u64;
        
        let thread_id = std::thread::current().id();
        let mut hasher = DefaultHasher::new();
        nanos.hash(&mut hasher);
        thread_id.hash(&mut hasher);
        b"edge".hash(&mut hasher); // Distinguish from NodeId
        
        let hash1 = hasher.finish();
        
        let mut hasher = DefaultHasher::new();
        hash1.hash(&mut hasher);
        let hash2 = hasher.finish();
        
        bytes[0..8].copy_from_slice(&hash1.to_ne_bytes());
        bytes[8..16].copy_from_slice(&hash2.to_ne_bytes());
        
        Self(bytes)
    }
    
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
    
    pub fn to_string(&self) -> String {
        hex::encode(self.0)
    }
    
    pub fn from_string(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() == 16 {
            let mut array = [0u8; 16];
            array.copy_from_slice(&bytes);
            Ok(Self(array))
        } else {
            Err(hex::FromHexError::InvalidStringLength)
        }
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl Hash for EdgeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let hash = u64::from_ne_bytes([
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5], self.0[6], self.0[7],
        ]);
        hash.hash(state);
    }
}

impl fmt::Debug for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EdgeId({})", hex::encode(&self.0[..4]))
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// Timestamp implementations
impl Timestamp {
    /// Create timestamp from current time
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self(nanos)
    }
    
    /// Create from nanoseconds since epoch
    pub fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }
    
    /// Get nanoseconds since epoch
    pub fn as_nanos(&self) -> u64 {
        self.0
    }
    
    /// Get microseconds since epoch
    pub fn as_micros(&self) -> u64 {
        self.0 / 1_000
    }
    
    /// Get milliseconds since epoch
    pub fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }
    
    /// Get seconds since epoch
    pub fn as_secs(&self) -> u64 {
        self.0 / 1_000_000_000
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timestamp({}ms)", self.as_millis())
    }
}

// Version implementations
impl Version {
    /// Create initial version
    pub fn initial() -> Self {
        Self(1)
    }
    
    /// Increment version
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
    
    /// Get version number
    pub fn as_u64(&self) -> u64 {
        self.0
    }
    
    /// Create from u64
    pub fn from_u64(v: u64) -> Self {
        Self(v)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Version({})", self.0)
    }
}

// PropertyKey implementations
impl PropertyKey {
    /// Create new property key
    pub fn new(key: impl Into<Arc<str>>) -> Self {
        Self(key.into())
    }
    
    /// Get string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PropertyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PropertyKey({})", self.0)
    }
}

impl fmt::Display for PropertyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PropertyKey {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for PropertyKey {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// Label implementations
impl Label {
    pub fn new(label: impl Into<Arc<str>>) -> Self {
        Self(label.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Label({})", self.0)
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Label {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// Value implementations
impl Value {
    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    
    /// Get value as boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    
    /// Get value as integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::UInt(u) if *u <= i64::MAX as u64 => Some(*u as i64),
            _ => None,
        }
    }
    
    /// Get value as unsigned integer
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt(u) => Some(*u),
            Value::Int(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        }
    }
    
    /// Get value as float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::UInt(u) => Some(*u as f64),
            _ => None,
        }
    }
    
    /// Get value as string reference
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    
    /// Get value as bytes
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }
    
    /// Get estimated memory usage
    pub fn memory_usage(&self) -> usize {
        match self {
            Value::Null => 0,
            Value::Bool(_) => 1,
            Value::Int(_) | Value::UInt(_) | Value::Float(_) => 8,
            Value::String(s) => s.len(),
            Value::Bytes(b) => b.len(),
            Value::Array(arr) => {
                arr.iter().map(|v| v.memory_usage()).sum::<usize>() + arr.len() * 8
            }
            Value::Object(obj) => {
                obj.iter()
                    .map(|(k, v)| k.as_str().len() + v.memory_usage())
                    .sum::<usize>()
                    + obj.len() * 16
            }
            Value::NodeRef(_) | Value::EdgeRef(_) => 16,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::UInt(a), Value::UInt(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::NodeRef(a), Value::NodeRef(b)) => a == b,
            (Value::EdgeRef(a), Value::EdgeRef(b)) => a == b,
            // Cross-type numeric comparisons
            (Value::Int(a), Value::UInt(b)) => *a >= 0 && *a as u64 == *b,
            (Value::UInt(a), Value::Int(b)) => *b >= 0 && *a == *b as u64,
            (Value::Int(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::Int(b)) => (a - *b as f64).abs() < f64::EPSILON,
            (Value::UInt(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::UInt(b)) => (a - *b as f64).abs() < f64::EPSILON,
            _ => false,
        }
    }
}

impl Eq for Value {}

// Convenient constructors
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Value::UInt(u)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.into())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s.into())
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Bytes(b)
    }
}

impl From<&[u8]> for Value {
    fn from(b: &[u8]) -> Self {
        Value::Bytes(b.to_vec())
    }
}

impl From<NodeId> for Value {
    fn from(id: NodeId) -> Self {
        Value::NodeRef(id)
    }
}

impl From<EdgeId> for Value {
    fn from(id: EdgeId) -> Self {
        Value::EdgeRef(id)
    }
}

// Manual Serialize/Deserialize implementations for PropertyKey
impl serde::Serialize for PropertyKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PropertyKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PropertyKey(s.into()))
    }
}

// Manual Serialize/Deserialize implementations for Label
impl serde::Serialize for Label {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Label {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Label(s.into()))
    }
}

// Manual Serialize/Deserialize implementations for Value
impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Int(i) => serializer.serialize_i64(*i),
            Value::UInt(u) => serializer.serialize_u64(*u),
            Value::Float(f) => serializer.serialize_f64(*f),
            Value::String(s) => serializer.serialize_str(s),
            Value::Bytes(b) => serializer.serialize_bytes(b),
            Value::Array(arr) => arr.serialize(serializer),
            Value::Object(obj) => {
                let mut map = serializer.serialize_map(Some(obj.len()))?;
                for (k, v) in obj {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Value::NodeRef(id) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("node_ref", &id.to_string())?;
                map.end()
            }
            Value::EdgeRef(id) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("edge_ref", &id.to_string())?;
                map.end()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // For now, implement a basic deserializer
        // In production, this would be more sophisticated
        use serde_json::Value as JsonValue;
        let json_value = JsonValue::deserialize(deserializer)?;
        
        Ok(match json_value {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Bool(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(u) = n.as_u64() {
                    Value::UInt(u)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            JsonValue::String(s) => Value::String(s.into()),
            JsonValue::Array(arr) => {
                let values: Result<Vec<_>, _> = arr.into_iter()
                    .map(|v| serde_json::from_value(v))
                    .collect();
                Value::Array(values.map_err(serde::de::Error::custom)?)
            }
            JsonValue::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    let value: Value = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
                    map.insert(PropertyKey::new(k), value);
                }
                Value::Object(map)
            }
        })
    }
} 