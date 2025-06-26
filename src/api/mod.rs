//! # API Module
//!
//! This module provides the HTTP API interface for Massive Graph, including:
//! - RESTful endpoints for collections, documents, and deltas
//! - WebSocket endpoints for real-time subscriptions
//! - System health and information endpoints
//!
//! ## Endpoints Overview
//!
//! ### Collection Operations
//! - `POST /api/v1/collections` - Create collection
//! - `GET /api/v1/collections` - List collections with pagination
//! - `GET /api/v1/collections/{id}` - Get collection by ID
//! - `PUT /api/v1/collections/{id}` - Update collection metadata
//! - `DELETE /api/v1/collections/{id}` - Delete collection
//!
//! ### Document Operations
//! - `POST /api/v1/documents` - Create document
//! - `GET /api/v1/documents` - List documents with pagination  
//! - `GET /api/v1/documents/{id}` - Get document by ID
//! - `PUT /api/v1/documents/{id}` - Update/replace document
//! - `PATCH /api/v1/documents/{id}` - Partial update document
//! - `DELETE /api/v1/documents/{id}` - Delete document
//!
//! ### Delta Operations
//! - `POST /api/v1/collections/{id}/deltas` - Apply array of deltas to collection
//! - `POST /api/v1/documents/{id}/deltas` - Apply array of deltas to document
//! - `GET /api/v1/collections/{id}/deltas` - Get recent deltas for collection
//! - `GET /api/v1/documents/{id}/deltas` - Get recent deltas for document
//! - `GET /api/v1/deltas/since/{timestamp}` - Get all deltas since timestamp
//!
//! ### Real-time Subscriptions
//! - `WebSocket /ws/collections` - Subscribe to all collection changes
//! - `WebSocket /ws/collections/{id}` - Subscribe to specific collection changes
//! - `WebSocket /ws/documents` - Subscribe to all document changes
//! - `WebSocket /ws/documents/{id}` - Subscribe to specific document changes
//!
//! ### System Essentials
//! - `GET /api/v1/health` - Health check
//! - `GET /api/v1/info` - Database info and capabilities

pub mod handlers;
pub mod server;

// Re-export commonly used items
pub use handlers::*;
pub use server::{create_app, start_server}; 