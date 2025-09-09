//! WebRTC HTTP endpoints for SDP exchange
//! 
//! This module provides HTTP endpoints for WebRTC signaling and SDP exchange.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{ConnectionId, comms::{ConnectionManager as CoreConnectionManager}};
use crate::log_info;

/// Global WebRTC state for the server
/// This is separate from the main AppState to keep the POC simple
#[derive(Clone)]
pub struct Network {
    /// Connection manager from core
    pub connection_manager: Arc<CoreConnectionManager>,
    /// Our server connection ID
    pub node_id: ConnectionId,
}

/// SDP data for exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpData {
    /// SDP type (offer or answer)
    pub sdp_type: String,
    /// SDP content
    pub sdp: String,
    /// Connection ID of the peer
    pub connection_id: ConnectionId,
}

/// Request to initiate WebRTC connection
#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    /// Client's connection ID as a string
    pub connection_id: String,
    /// Optional SDP offer
    pub sdp_offer: Option<String>,
}

/// Response to connection request
#[derive(Debug, Serialize)]
pub struct ConnectResponse {
    /// Server's connection ID as string
    pub server_id: String,
    /// SDP answer if offer was provided
    pub sdp_answer: Option<String>,
    /// Status message
    pub message: String,
}

impl Network {
    /// Create a new WebRTC state
    pub fn new() -> Self {
        let node_id = ConnectionId::random();
        let connection_manager = Arc::new(CoreConnectionManager::new(node_id.clone(), true));
        
        log_info!("WebRTC server initialized with Node ID: {}", node_id);
        
        Self {
            connection_manager,
            node_id,
        }
    }
}