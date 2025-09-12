//! HTTP server implementation for Massive Graph API

use axum::{
    http::{
        header::{CONTENT_TYPE, AUTHORIZATION, ACCESS_CONTROL_ALLOW_ORIGIN},
        Method,
    },
    routing::{delete, get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use super::api_handlers;
use massive_graph_core::{
    core::{factory::ConfiguredAppState, AppState}, log_info, storage::StorageImpl
};

/// Creates the main application router with all routes and middleware
fn create_server_impl<S: StorageImpl>(app_state: Arc<AppState<S>>) -> Router {
    // CORS configuration - permissive for POC
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, ACCESS_CONTROL_ALLOW_ORIGIN])
        .allow_origin(Any)
        .allow_credentials(false);

    // Build the complete router with all routes
    Router::new()
        // Root route
        .route("/", get(api_handlers::root_handler))
        
        // Document routes
        .route("/api/documents", post(api_handlers::create_document))
        .route("/api/documents/{id}", get(api_handlers::get_document))
        .route("/api/documents/{id}", delete(api_handlers::delete_document))
        
        // Delta routes - Document deltas
        .route("/api/documents/{id}/deltas", post(api_handlers::apply_document_deltas))
        .route("/api/documents/{id}/deltas", get(api_handlers::get_document_deltas))
        
        // System routes
        .route("/health", get(api_handlers::health_check))
        .route("/info", get(api_handlers::system_info))
        
        // WebRTC routes
        .nest("/webrtc", crate::webrtc::create_webrtc_routes::<S>())
        
        // Apply middleware to ALL routes
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        // Add unified AppState
        .with_state(app_state)
}

/// Internal function to start the server with the configured router
async fn serve_api_server_with_app(addr: SocketAddr, app: Router) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    log_info!("Server listening on http://{}", addr);
    log_info!("API documentation available at http://{}/", addr);
    log_info!("Health check available at http://{}/health", addr);
    log_info!("WebRTC endpoints available at http://{}/webrtc/*", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Start the HTTP server with the configured AppState
pub async fn start_api_server(configured_app_state: ConfiguredAppState) -> Result<(), Box<dyn std::error::Error>> {
    let http_addr = configured_app_state.http_addr();
    
    log_info!("Starting Massive Graph API server on {}", http_addr);
    
    // Match once on storage type to get concrete AppState, then start server
    match configured_app_state {
        ConfiguredAppState::Simple { app_state, .. } => {
            log_info!("Starting server with SimpleStorage backend");
            let app = create_server_impl(app_state);
            serve_api_server_with_app(http_addr, app).await
        }
        ConfiguredAppState::ZeroCopy { app_state, .. } => {
            log_info!("Starting server with ZeroCopyStorage backend");
            let app = create_server_impl(app_state);
            serve_api_server_with_app(http_addr, app).await
        }
    }
}

