//! HTTP handlers for WebRTC signaling

use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use massive_graph_core::{
    webrtc::{ConnectionRequest, ConnectionResponse, IceCandidateRequest, IceCandidateResponse,
              WebRtcConnection},
    ConnectionId,
};
use massive_graph_core::{core::AppState, storage::StorageImpl};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};
use std::collections::HashMap;
use super::Str0mConnection;

/// Storage for active WebRTC connections
pub type ConnectionStore = Arc<Mutex<HashMap<ConnectionId, Arc<Mutex<Str0mConnection>>>>>;

/// Initialize WebRTC connection
pub async fn connect_handler<S: StorageImpl>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ConnectionRequest>,
) -> Result<Json<ConnectionResponse>, StatusCode> {
    info!("WebRTC connection request from client: {}", request.client_id);
    
    // Get or create connection store
    let store = get_connection_store(&state).await;
    
    // Create server connection
    let server_id = ConnectionId::random();
    let mut connection = Str0mConnection::new(server_id.clone(), false);
    
    let response = if let Some(offer) = request.offer {
        // Set remote offer
        if let Err(e) = connection.set_remote_description(offer).await {
            error!("Failed to set remote description: {}", e);
            return Ok(Json(ConnectionResponse {
                server_id,
                answer: None,
                success: false,
                error: Some(format!("Failed to process offer: {}", e)),
            }));
        }
        
        // Create answer
        match connection.create_answer().await {
            Ok(answer) => {
                // Store connection
                store.lock().await.insert(server_id.clone(), Arc::new(Mutex::new(connection)));
                
                ConnectionResponse {
                    server_id,
                    answer: Some(answer),
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("Failed to create answer: {}", e);
                ConnectionResponse {
                    server_id,
                    answer: None,
                    success: false,
                    error: Some(format!("Failed to create answer: {}", e)),
                }
            }
        }
    } else {
        // No offer provided, just register the connection
        store.lock().await.insert(server_id.clone(), Arc::new(Mutex::new(connection)));
        
        ConnectionResponse {
            server_id,
            answer: None,
            success: true,
            error: None,
        }
    };
    
    Ok(Json(response))
}

/// Exchange ICE candidates
pub async fn ice_candidate_handler<S: StorageImpl>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<IceCandidateRequest>,
) -> Result<Json<IceCandidateResponse>, StatusCode> {
    info!("ICE candidate from connection: {}", request.connection_id);
    
    let store = get_connection_store(&state).await;
    let connections = store.lock().await;
    
    if let Some(connection) = connections.get(&request.connection_id) {
        let mut conn = connection.lock().await;
        
        match conn.add_ice_candidate(request.candidate).await {
            Ok(_) => Ok(Json(IceCandidateResponse {
                success: true,
                error: None,
            })),
            Err(e) => {
                error!("Failed to add ICE candidate: {}", e);
                Ok(Json(IceCandidateResponse {
                    success: false,
                    error: Some(format!("Failed to add candidate: {}", e)),
                }))
            }
        }
    } else {
        Ok(Json(IceCandidateResponse {
            success: false,
            error: Some("Connection not found".to_string()),
        }))
    }
}

/// Health check for WebRTC service
pub async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "WebRTC service healthy")
}

/// Get or create the connection store
async fn get_connection_store<S: StorageImpl>(_state: &Arc<AppState<S>>) -> ConnectionStore {
    // For now, we'll use a static store
    // In production, this should be part of AppState
    static STORE: once_cell::sync::OnceCell<ConnectionStore> = once_cell::sync::OnceCell::new();
    
    STORE.get_or_init(|| {
        Arc::new(Mutex::new(HashMap::new()))
    }).clone()
}

/// Create WebRTC routes
pub fn create_webrtc_routes<S: StorageImpl>() -> axum::Router<Arc<AppState<S>>> {
    use axum::routing::{post, get};
    
    axum::Router::new()
        .route("/webrtc/connect", post(connect_handler))
        .route("/webrtc/ice-candidate", post(ice_candidate_handler))
        .route("/webrtc/health", get(health_handler))
}
