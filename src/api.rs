//! # API Module
//!
//! This module provides the HTTP API interface for Massive Graph, including:
//! - RESTful endpoints for collections, documents, and deltas
//! - WebSocket endpoints for real-time subscriptions
//! - System health and information endpoints

pub mod handlers;
pub mod server;

// Re-export commonly used items
pub use handlers::*;
pub use server::{create_app, start_server}; 