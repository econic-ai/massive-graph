//! Global constants used throughout the Massive Graph codebase
//! 
//! This module contains compile-time constants that are shared across
//! multiple modules to ensure consistency and avoid magic numbers.

/// Base62 character set used for human-readable IDs
/// 
/// This character set provides 62 possible characters (0-9, a-z, A-Z)
/// for generating human-readable identifiers while maintaining good
/// performance characteristics for binary operations.
pub const BASE62_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Size of each memory chunk in bytes (64MB)
/// 
/// This chunk size is optimized for:
/// - Large enough to amortize allocation overhead
/// - Small enough to avoid excessive memory waste
/// - Aligned with typical OS virtual memory page sizes
/// - Good balance for cache locality and memory usage
pub const CHUNK_SIZE: usize = 64 * 1024 * 1024;

/// Size of each memory page in bytes (16MB)
/// 
/// This page size is optimized for:
/// - Large enough to amortize allocation overhead
/// - Small enough to avoid excessive memory waste
/// - Aligned with typical OS virtual memory page sizes
/// - Good balance for cache locality and memory usage
pub const PAGE_SIZE: usize = 16 * 1024 * 1024;

/// Length of ID8 in bytes (8 characters)
/// 
/// Used for delta IDs and other identifiers that need less
/// entropy than ID16 provides.
pub const ID8_LENGTH: usize = 8;

/// Maximum number of characters in a base62 ID
/// 
/// This determines the fixed size of ID16 and ID32 types.
/// 16 characters provides sufficient entropy while maintaining
/// readability and performance.
pub const ID16_LENGTH: usize = 16;

/// Length of ID32 in bytes (32 characters)
/// 
/// Used for user IDs and other identifiers that need more
/// entropy than ID16 provides.
pub const ID32_LENGTH: usize = 32;

/// Default number of worker threads for delta processing
/// 
/// This should generally match the number of CPU cores
/// but can be overridden based on workload characteristics.
pub const DEFAULT_WORKER_THREADS: usize = 8;

/// Maximum delta operations per batch
/// 
/// Limits the number of operations processed in a single
/// batch to prevent memory exhaustion and ensure responsive
/// processing under high load.
pub const MAX_DELTA_OPERATIONS_PER_BATCH: usize = 1000;

/// Default timeout for network operations in milliseconds
/// 
/// Used for WebSocket connections, HTTP requests, and
/// other network operations that need reasonable timeouts.
pub const DEFAULT_NETWORK_TIMEOUT_MS: u64 = 5000;

/// Maximum document size in bytes (16MB)
/// 
/// Prevents individual documents from consuming excessive
/// memory and ensures predictable performance characteristics.
pub const MAX_DOCUMENT_SIZE: usize = 16 * 1024 * 1024;

/// Cache line size for memory alignment
/// 
/// Used to align data structures to cache boundaries
/// for optimal CPU cache performance.
pub const CACHE_LINE_SIZE: usize = 64;

/// Worker thread park timeout in milliseconds
/// 
/// How long worker threads sleep when no work is available.
/// Short enough to be responsive, long enough to avoid busy waiting.
pub const WORKER_PARK_TIMEOUT_MS: u64 = 1;

/// Default heartbeat interval in seconds
/// 
/// Used for WebSocket connections and other persistent connections
/// to detect disconnections and keep connections alive.
pub const DEFAULT_HEARTBEAT_INTERVAL_S: u64 = 30;

/// Default request timeout in seconds
/// 
/// Maximum time to wait for HTTP requests to complete.
pub const DEFAULT_REQUEST_TIMEOUT_S: u64 = 30;

/// Default sync interval in seconds
/// 
/// How often to synchronize state between nodes in distributed mode.
pub const DEFAULT_SYNC_INTERVAL_S: u64 = 5;

/// Default batch timeout in milliseconds
/// 
/// Maximum time to wait before processing a partial batch of deltas.
pub const DEFAULT_BATCH_TIMEOUT_MS: u64 = 10;

// Memory size constants

/// Standard buffer size for network operations (64KB)
pub const NETWORK_BUFFER_SIZE: usize = 64 * 1024;

/// Default memory pool size (256MB)
pub const DEFAULT_MEMORY_POOL_SIZE: usize = 256 * 1024 * 1024;

/// Default maximum memory usage (1GB on WASM)
#[cfg(target_arch = "wasm32")]
pub const DEFAULT_MAX_MEMORY: usize = 1024 * 1024 * 1024; // 1GB for WASM

/// Default maximum memory usage (8GB on native)
#[cfg(not(target_arch = "wasm32"))]
pub const DEFAULT_MAX_MEMORY: usize = 8 * 1024 * 1024 * 1024; // 8GB for native

/// Maximum log file size (100MB)
pub const MAX_LOG_FILE_SIZE: usize = 100 * 1024 * 1024;

/// Minimum memory requirement (1MB)
pub const MIN_MEMORY_REQUIREMENT: usize = 1024 * 1024;

/// Maximum worker threads allowed
pub const MAX_WORKER_THREADS: usize = 1024;

// Delta and stream storage constants

/// Number of slots per delta chunk (64k slots)
/// 
/// Each delta chunk pre-allocates 64,000 slots to ensure stable memory
/// addresses for zero-copy operations. This size balances memory efficiency
/// with allocation overhead.
pub const SLOTS_PER_CHUNK: usize = 64_000;

/// Number of slots per stream chunk (16k slots)
/// 
/// Stream chunks use smaller allocation units since applications typically
/// create more streams with moderate entry counts. This reduces memory
/// waste whilst maintaining pre-allocation benefits.
pub const STREAM_SLOTS_PER_CHUNK: usize = 16_000;

/// Size of document header in bytes (64 bytes)
/// 
/// Document headers are fixed-size structures aligned to cache boundaries
/// for optimal performance. The 64-byte size matches typical CPU cache
/// line sizes for efficient memory access.
pub const DOCUMENT_HEADER_SIZE: usize = 64; 