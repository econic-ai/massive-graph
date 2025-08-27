//! System-wide constants

/// Base62 character set for ID generation
pub const BASE62_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Default chunk size (64MB)
pub const CHUNK_SIZE: usize = 64 * 1024 * 1024;

/// Length of ID16 identifiers
pub const ID16_LENGTH: usize = 16;

/// Length of ID32 identifiers  
pub const ID32_LENGTH: usize = 32;

/// Length of ID8 identifiers
pub const ID8_LENGTH: usize = 8;
