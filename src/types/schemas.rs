use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU16};
use dashmap::DashMap;
use bytes::Bytes;

/// Schema version with major.minor semantics
/// Packed into u16 for efficient atomic operations and wire format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaVersion {
    major: u8,  // Breaking changes (reordering, restructuring)
    minor: u8,  // Compatible changes (additions, tombstones)
}

impl SchemaVersion {
    /// Pack into u16 for atomic operations
    pub fn to_u16(&self) -> u16 {
        ((self.major as u16) << 8) | (self.minor as u16)
    }
    
    /// Unpack from u16
    pub fn from_u16(val: u16) -> Self {
        SchemaVersion {
            major: (val >> 8) as u8,
            minor: (val & 0xFF) as u8,
        }
    }
}

/// Unique identifier for schema families
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaFamilyId(u32);

/// Property identifier with variable-length encoding
/// 0-127: single byte, 128-32K: two bytes, 32K+: three bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyId(pub u16);


/// Main schema structure - immutable once created
pub struct Schema {
    /// Unique identifier for this schema family
    family_id: SchemaFamilyId,
    
    /// Version of this schema
    version: SchemaVersion,
    
    /// When this schema was created (unix timestamp)
    created_at: u64,
    
    /// Pattern registry - the compression dictionary
    /// Index in vec IS the PropertyId value
    patterns: Vec<PatternEntry>,
    
    /// Wire-ready serialised form for transmission
    /// Pre-computed for efficient distribution
    wire_bytes: Bytes,
    
    /// Parent schema for inheritance (Global -> DocumentType -> Instance)
    parent: Option<(SchemaFamilyId, SchemaVersion)>,
}

/// Individual pattern in the schema
pub struct PatternEntry {
    /// Pattern template like "users[*].classDict.{}.assessments[*].score"
    pattern: String,
    
    /// Parameter types needed to hydrate this pattern
    param_types: Vec<ParamType>,
    
    /// Frequency count for optimisation decisions
    frequency: u64,
    
    /// Whether this pattern is active (false = tombstoned)
    active: bool,
}

/// Types of parameters in patterns
#[derive(Debug, Clone, Copy)]
#[repr(u8)]  // Explicit discriminant for wire format
pub enum ParamType {
    ArrayIndex = 0x01,      // [*] -> varint encoded index
    MapKey = 0x02,          // {} -> length-prefixed string
    DynamicSegment = 0x03,  // {{userId}} -> captured string segment
}

/// Manages all schemas in the system
pub struct SchemaRegistry {
    /// Sharded by family for less contention (16 shards)
    /// Hash family_id to determine shard
    schemas: [DashMap<(SchemaFamilyId, SchemaVersion), Arc<Schema>>; 16],
    
    /// Version cache for fast lookups without loading full schema
    version_cache: DashMap<SchemaFamilyId, AtomicU16>,
    
    /// Global patterns shared across families
    global_patterns: Arc<Schema>,
}

impl SchemaRegistry {
    /// Get schema with sharding for reduced contention
    pub fn get_schema(&self, family: SchemaFamilyId, version: SchemaVersion) -> Option<Arc<Schema>> {
        let shard = (family.0 as usize) % 16;
        self.schemas[shard].get(&(family, version)).map(|s| s.clone())
    }
    
    /// Register new schema version
    pub fn register_schema(&self, schema: Schema) -> Result<(), SchemaError> {
        let shard = (schema.family_id.0 as usize) % 16;
        let key = (schema.family_id, schema.version);
        
        // Check for duplicate
        if self.schemas[shard].contains_key(&key) {
            return Err(SchemaError::DuplicateVersion);
        }
        
        // Store values we need before moving schema
        let family_id = schema.family_id;
        let version = schema.version;
        
        // Now move schema into Arc
        let arc_schema = Arc::new(schema);
        self.schemas[shard].insert(key, arc_schema);
        
        // Update version cache using stored values
        self.version_cache.insert(
            family_id, 
            AtomicU16::new(version.to_u16())
        );
        
        Ok(())
    }
}

/// A family of related schemas with version history
pub struct SchemaFamily {
    /// Unique identifier
    id: SchemaFamilyId,
    
    /// Human-readable name
    name: String,
    
    /// All versions of this schema (using DashMap for thread-safety)
    versions: DashMap<SchemaVersion, Arc<Schema>>,
    
    /// Current active version for new deltas (atomically updatable)
    current: AtomicU16,  // Packed SchemaVersion
}

#[derive(Debug)]
pub enum SchemaError {
    DuplicateVersion,
    InvalidVersion,
    SchemaNotFound,
}

/// Cache-line aligned structure for hot path operations
#[repr(align(64))]
pub struct HotCache {
    /// Reference to shared schema (atomic for lock-free updates)
    schema_ptr: AtomicPtr<Schema>,
    
    /// Packed schema version for fast equality checks
    schema_version: AtomicU16,
    
    /// Most frequent patterns cached (avoid schema dereference)
    /// Covers ~90% of operations
    cached_patterns: [Option<PatternEntry>; 8],
    
    /// Cached PropertyIds for fastest lookup
    cached_ids: [PropertyId; 8],
}

/// Global schema registry accessor
fn get_schema_registry() -> &'static SchemaRegistry {
    // Implementation would return singleton or injected instance
    todo!()
}