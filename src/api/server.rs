//! HTTP server implementation for Massive Graph API

use axum::{
    http::{
        header::{CONTENT_TYPE, AUTHORIZATION},
        Method,
    },
    routing::{delete, get, patch, post, put},
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use super::handlers;

/// Creates the main application router with all routes and middleware
pub fn create_app(storage: crate::storage::SharedStorage) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
        .allow_origin(Any);

    // Build the complete router with all routes
    Router::new()
        // Root route
        .route("/", get(handlers::root_handler))
        
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
        
        // WebSocket routes
        .route("/ws/collections", get(handlers::websocket_collections_handler))
        .route("/ws/collections/:id", get(handlers::websocket_collection_handler))
        .route("/ws/documents", get(handlers::websocket_documents_handler))
        .route("/ws/documents/:id", get(handlers::websocket_document_handler))
        
        // Apply middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        // Add storage as shared state
        .with_state(storage)
}

/// Start the HTTP server
pub async fn start_server(addr: SocketAddr, storage: crate::storage::SharedStorage) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Massive Graph API server on {}", addr);
    
    let app = create_app(storage);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("API documentation available at http://{}/", addr);
    tracing::info!("Health check available at http://{}/api/v1/health", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
