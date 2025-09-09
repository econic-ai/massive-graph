//! Error types and handling for Massive Graph Database
//! 
//! This module defines all error types used throughout the system,
//! optimized for zero-cost error propagation and clear diagnostics.
//! 
//! 
#[derive(Debug, Clone)]
/// Errors that can occur during parsing operations
pub enum ParseError {
   /// Not enough bytes for the expected data
   InsufficientData { 
       /// Expected number of bytes
       expected: usize, 
       /// Actual number of bytes available
       actual: usize 
   },
   
   /// Invalid operation byte
   InvalidOperation(u8),
   
   /// Invalid UTF-8 in string data
   InvalidUtf8,
   
   /// Corrupted or invalid wire format
   InvalidFormat,
}

impl std::fmt::Display for ParseError {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       match self {
           ParseError::InsufficientData { expected, actual } => {
               write!(f, "Insufficient data: expected {} bytes, got {}", expected, actual)
           }
           ParseError::InvalidOperation(op) => {
               write!(f, "Invalid operation byte: {:#x}", op)
           }
           ParseError::InvalidUtf8 => {
               write!(f, "Invalid UTF-8 encoding")
           }
           ParseError::InvalidFormat => {
               write!(f, "Invalid wire format")
           }
       }
   }
}

impl std::error::Error for ParseError {}