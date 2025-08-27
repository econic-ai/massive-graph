/// Parse error type definitions for the Massive Graph system - Empty Shell
/// 
/// This module contains minimal parsing error type definitions to be built upon.

/// Error types for parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Placeholder variant
    Placeholder,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseError")
    }
}

impl std::error::Error for ParseError {}
