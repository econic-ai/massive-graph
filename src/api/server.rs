//! HTTP server implementation for Massive Graph API

use axum::{
    http::{
        header::{CONTENT_TYPE, AUTHORIZATION},
        HeaderValue, Method,
    },
    middleware,
    response::Json,
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use super::routes::{create_api_routes, create_websocket_routes};

/// Creates the main application router with all routes and middleware
pub fn create_app() -> Router {
    // API routes
    let api_routes = create_api_routes();
    
    // WebSocket routes  
    let ws_routes = create_websocket_routes();
    
    // Root status endpoint
    let root_route = Router::new()
        .route("/", axum::routing::get(root_handler));

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
        .allow_origin(Any);

    // Build the complete router
    Router::new()
        .merge(root_route)
        .merge(api_routes)
        .merge(ws_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
}

/// Root handler that provides API information
async fn root_handler() -> Json<serde_json::Value> {
    Json(json!({
        "name": "Massive Graph API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Real-time graph database for collaborative intelligence",
        "endpoints": {
            "health": "/api/v1/health",
            "info": "/api/v1/info",
            "collections": "/api/v1/collections",
            "documents": "/api/v1/documents",
            "deltas": "/api/v1/deltas",
            "websockets": {
                "collections": "/ws/collections",
                "documents": "/ws/documents"
            }
        },
        "documentation": "https://docs.massive-graph.dev"
    }))
}

/// Start the HTTP server
pub async fn start_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Massive Graph API server on {}", addr);
    
    let app = create_app();
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("API documentation available at http://{}/", addr);
    tracing::info!("Health check available at http://{}/api/v1/health", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
