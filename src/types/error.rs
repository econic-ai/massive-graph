//! Error types and handling for Massive Graph Database
//! 
//! This module defines all error types used throughout the system,
//! optimized for zero-cost error propagation and clear diagnostics.

use thiserror::Error;

/// Main result type used throughout the crate
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the Massive Graph Database
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration errors..
    #[error("Configuration error: {0}")]
    Config(String),

    /// Storage layer errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Network communication errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Delta operation errors
    #[error("Delta operation error: {0}")]
    Delta(#[from] DeltaError),

    /// Graph operation errors
    #[error("Graph operation error: {0}")]
    Graph(#[from] GraphError),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),

    /// Authorization and permission errors
    #[error("Permission denied: {0}")]
    Permission(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal system errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// I/O errors from std
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Prometheus metrics errors
    #[error("Metrics error: {0}")]
    Metrics(#[from] prometheus::Error),
}

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    /// Memory allocation failed
    #[error("Memory allocation failed")]
    OutOfMemory,

    /// Disk I/O operation failed
    #[error("Disk I/O failed: {0}")]
    DiskIo(#[from] std::io::Error),

    /// Corruption detected in stored data
    #[error("Data corruption detected: {0}")]
    Corruption(String),

    /// Index operation failed
    #[error("Index operation failed: {0}")]
    Index(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    Transaction(String),
}

/// Network communication errors
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Connection failed or lost
    #[error("Connection error: {0}")]
    Connection(String),

    /// Protocol violation
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Timeout occurred
    #[error("Operation timed out")]
    Timeout,

    /// Message too large
    #[error("Message too large: {size} bytes (max: {max_size})")]
    MessageTooLarge { 
        /// Actual message size in bytes
        size: usize, 
        /// Maximum allowed message size in bytes
        max_size: usize 
    },

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
}

/// Delta operation errors
#[derive(Error, Debug)]
pub enum DeltaError {
    /// Delta conflict during merge
    #[error("Delta conflict: {0}")]
    Conflict(String),

    /// Invalid delta format
    #[error("Invalid delta format: {0}")]
    InvalidFormat(String),

    /// Delta sequence error
    #[error("Delta sequence error: expected {expected}, got {actual}")]
    SequenceError { 
        /// Expected sequence number
        expected: u64, 
        /// Actual sequence number received
        actual: u64 
    },

    /// Delta too large
    #[error("Delta too large: {size} bytes")]
    TooLarge { 
        /// Size of the delta in bytes
        size: usize 
    },
}

/// Graph operation errors
#[derive(Error, Debug)]
pub enum GraphError {
    /// Node not found
    #[error("Node not found: {id}")]
    NodeNotFound { 
        /// ID of the missing node
        id: String 
    },

    /// Edge not found
    #[error("Edge not found: {id}")]
    EdgeNotFound { 
        /// ID of the missing edge
        id: String 
    },

    /// Cycle detected where not allowed
    #[error("Cycle detected in graph")]
    CycleDetected,

    /// Invalid graph structure
    #[error("Invalid graph structure: {0}")]
    InvalidStructure(String),

    /// Property not found
    #[error("Property not found: {key}")]
    PropertyNotFound { 
        /// Key of the missing property
        key: String 
    },

    /// Type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { 
        /// Expected type name
        expected: String, 
        /// Actual type name received
        actual: String 
    },
}

/// Serialization/deserialization errors
#[derive(Error, Debug)]
pub enum SerializationError {
    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// MessagePack serialization error
    #[error("MessagePack error: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),

    /// MessagePack deserialization error
    #[error("MessagePack decode error: {0}")]
    MessagePackDecode(#[from] rmp_serde::decode::Error),

    /// Bincode serialization error
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    /// Unsupported format
    #[error("Unsupported serialization format: {0}")]
    UnsupportedFormat(String),
}

impl Error {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a permission error
    pub fn permission(msg: impl Into<String>) -> Self {
        Self::Permission(msg.into())
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    /// Create an already exists error
    pub fn already_exists(resource: impl Into<String>) -> Self {
        Self::AlreadyExists(resource.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Network(NetworkError::Timeout)
                | Error::Network(NetworkError::Connection(_))
                | Error::Storage(StorageError::DiskIo(_))
        )
    }

    /// Check if this is a client error (4xx equivalent)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Error::InvalidInput(_)
                | Error::Permission(_)
                | Error::NotFound(_)
                | Error::AlreadyExists(_)
                | Error::Graph(_)
                | Error::Serialization(_)
        )
    }

    /// Check if this is a server error (5xx equivalent)
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Error::Internal(_) | Error::Storage(_) | Error::Network(_)
        )
    }
} 