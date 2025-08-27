/// Utility functions for common operations across the codebase

/// Get current timestamp in nanoseconds since epoch
/// 
/// This is used throughout the system for timestamping entries, deltas,
/// and other time-sensitive operations. Using nanoseconds provides
/// high precision for ordering operations.
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
} 