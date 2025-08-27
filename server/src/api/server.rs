//! HTTP server implementation for Massive Graph API

use axum::{
    http::{
        header::{CONTENT_TYPE, AUTHORIZATION},
        Method,
    },
    routing::{delete, get, post},
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
pub fn create_app<S: crate::storage::StorageImpl>(storage: std::sync::Arc<crate::storage::Store<S>>) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
        .allow_origin(Any);

    // Build the complete router with all routes
    Router::new()
        // Root route
        .route("/", get(handlers::root_handler))
        
        // Document routes
        .route("/api/documents", post(handlers::create_document))
        .route("/api/documents/:id", get(handlers::get_document))

        .route("/api/documents/:id", delete(handlers::delete_document))
        
        // Delta routes - Document deltas
        .route("/api/documents/:id/deltas", post(handlers::apply_document_deltas))
        .route("/api/documents/:id/deltas", get(handlers::get_document_deltas))
        
        // System routes
        .route("/health", get(handlers::health_check))
        .route("/info", get(handlers::system_info))
        
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
pub async fn start_server<S: crate::storage::StorageImpl>(addr: SocketAddr, storage: std::sync::Arc<crate::storage::Store<S>>) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Massive Graph API server on {}", addr);
    
    let app = create_app(storage);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("API documentation available at http://{}/", addr);
    tracing::info!("Health check available at http://{}/health", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
