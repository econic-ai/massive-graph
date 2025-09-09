//! Signaling protocol for WebRTC connection establishment

use serde::{Serialize, Deserialize};
use crate::types::ConnectionId;

/// SDP type for session descriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdpType {
    /// Offer to establish connection
    Offer,
    /// Answer to an offer
    Answer,
}

/// Session Description Protocol data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDescription {
    /// SDP type (offer or answer)
    #[serde(rename = "type")]
    pub sdp_type: SdpType,
    /// SDP content
    pub sdp: String,
}

/// ICE candidate for connectivity establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    /// ICE candidate string
    pub candidate: String,
    /// Media stream identification
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    /// Media stream index
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_m_line_index: Option<u32>,
}

/// Request to initiate WebRTC connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    /// Client's connection ID
    pub client_id: ConnectionId,
    /// Optional SDP offer
    pub offer: Option<SessionDescription>,
}

/// Response to connection request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionResponse {
    /// Server's connection ID
    pub server_id: ConnectionId,
    /// SDP answer if offer was provided
    pub answer: Option<SessionDescription>,
    /// Status of the request
    pub success: bool,
    /// Optional error message
    pub error: Option<String>,
}

/// ICE candidate exchange request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidateRequest {
    /// Connection ID of the sender
    pub connection_id: ConnectionId,
    /// ICE candidate to add
    pub candidate: IceCandidate,
}

/// ICE candidate exchange response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidateResponse {
    /// Whether the candidate was accepted
    pub success: bool,
    /// Optional error message
    pub error: Option<String>,
}

/// Data channel configuration
#[derive(Debug, Clone)]
pub struct DataChannelConfig {
    /// Channel label
    pub label: String,
    /// Whether the channel is ordered
    pub ordered: bool,
    /// Maximum retransmission time in milliseconds
    pub max_retransmit_time: Option<u16>,
    /// Maximum number of retransmissions
    pub max_retransmits: Option<u16>,
    /// Protocol for the channel
    pub protocol: String,
    /// Whether this channel is negotiated out-of-band
    pub negotiated: bool,
    /// Channel ID for negotiated channels
    pub id: Option<u16>,
}

impl DataChannelConfig {
    /// Create configuration for command channel
    pub fn command_channel() -> Self {
        Self {
            label: "command".to_string(),
            ordered: true,
            max_retransmit_time: None,
            max_retransmits: None,
            protocol: "".to_string(),
            negotiated: true,
            id: Some(0),
        }
    }
    
    /// Create configuration for send channel (browser → server)
    pub fn send_channel() -> Self {
        Self {
            label: "send".to_string(),
            ordered: false,  // Unordered for speed
            max_retransmit_time: Some(100),  // 100ms max retransmit
            max_retransmits: None,
            protocol: "".to_string(),
            negotiated: true,
            id: Some(1),
        }
    }
    
    /// Create configuration for receive channel (server → browser)
    pub fn receive_channel() -> Self {
        Self {
            label: "receive".to_string(),
            ordered: false,  // Unordered for speed
            max_retransmit_time: Some(100),  // 100ms max retransmit
            max_retransmits: None,
            protocol: "".to_string(),
            negotiated: true,
            id: Some(2),
        }
    }
}
