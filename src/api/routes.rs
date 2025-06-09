//! API route definitions for the Massive Graph REST API

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

use super::handlers;

/// Create the main API router with all endpoints
pub fn create_api_routes() -> Router {
    Router::new()
        // Collection routes
        .route("/api/v1/collections", post(handlers::create_collection))
        .route("/api/v1/collections", get(handlers::list_collections))
        .route("/api/v1/collections/:id", get(handlers::get_collection))
        .route("/api/v1/collections/:id", put(handlers::update_collection))
        .route("/api/v1/collections/:id", delete(handlers::delete_collection))
        
        // Document routes
        .route("/api/v1/documents", post(handlers::create_document))
        .route("/api/v1/documents", get(handlers::list_documents))
        .route("/api/v1/documents/:id", get(handlers::get_document))
        .route("/api/v1/documents/:id", put(handlers::update_document))
        .route("/api/v1/documents/:id", patch(handlers::patch_document))
        .route("/api/v1/documents/:id", delete(handlers::delete_document))
        
        // Delta routes - Collection deltas
        .route("/api/v1/collections/:id/deltas", post(handlers::apply_collection_deltas))
        .route("/api/v1/collections/:id/deltas", get(handlers::get_collection_deltas))
        
        // Delta routes - Document deltas
        .route("/api/v1/documents/:id/deltas", post(handlers::apply_document_deltas))
        .route("/api/v1/documents/:id/deltas", get(handlers::get_document_deltas))
        
        // Delta routes - Global deltas
        .route("/api/v1/deltas/since/:timestamp", get(handlers::get_deltas_since))
        
        // System routes
        .route("/api/v1/health", get(handlers::health_check))
        .route("/api/v1/info", get(handlers::system_info))
}

/// Create WebSocket routes for real-time subscriptions
pub fn create_websocket_routes() -> Router {
    Router::new()
        // WebSocket endpoints for real-time subscriptions
        // Note: These are placeholder routes - WebSocket upgrading will be implemented later
        .route("/ws/collections", get(websocket_collections_handler))
        .route("/ws/collections/:id", get(websocket_collection_handler))
        .route("/ws/documents", get(websocket_documents_handler))
        .route("/ws/documents/:id", get(websocket_document_handler))
}

// Placeholder WebSocket handlers - these will be implemented with actual WebSocket logic later
async fn websocket_collections_handler() -> &'static str {
    "WebSocket endpoint for all collection changes - not yet implemented"
}

async fn websocket_collection_handler() -> &'static str {
    "WebSocket endpoint for specific collection changes - not yet implemented"
}

async fn websocket_documents_handler() -> &'static str {
    "WebSocket endpoint for all document changes - not yet implemented"
}

async fn websocket_document_handler() -> &'static str {
    "WebSocket endpoint for specific document changes - not yet implemented"
}
