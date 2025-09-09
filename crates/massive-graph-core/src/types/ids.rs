/// Module containing fixed-size identifier types optimized for efficient storage and comparison.
/// Uses base62 encoding [0-9a-zA-Z] for human-readable string representation while maintaining
/// fixed memory layout for zero-copy operations.

use std::fmt;
use std::str::FromStr;
use rand::{rng, Rng};
use serde::{Serialize, Deserialize};
use crate::constants::{BASE62_CHARS, ID8_LENGTH, ID16_LENGTH, ID32_LENGTH};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ID8([u8; ID8_LENGTH]);

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        let mut rng = rng();
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
        let mut rng = rng();
        let mut bytes = [0u8; 8];
        
        for i in 0..8 {
            bytes[i] = BASE62_CHARS[rng.random_range(0..BASE62_CHARS.len())];
        }
        
        ID8(bytes)
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        // Safety: BASE62_CHARS only contains valid UTF-8 characters
        unsafe { std::str::from_utf8_unchecked(&self.0) }
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
        let mut rng = rng();
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
        write!(f, "{}", self.as_str())
    }
}

/// String conversion for ID8
impl fmt::Display for ID8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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
            return Err("ID16 must be exactly 16 characters");
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