//! Platform-agnostic WebRTC connection trait

use crate::types::ConnectionId;
use crate::webrtc::{Payload, SessionDescription, IceCandidate};
use std::future::Future;
use std::pin::Pin;

/// Result type for WebRTC operations
pub type WebRtcResult<T> = Result<T, WebRtcError>;

/// WebRTC-specific errors
#[derive(Debug, thiserror::Error)]
pub enum WebRtcError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    /// Data channel error
    #[error("Data channel error: {0}")]
    DataChannelError(String),
    
    /// Signaling error
    #[error("Signaling error: {0}")]
    SignalingError(String),
    
    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

/// Platform-agnostic trait for WebRTC connections
pub trait WebRtcConnection: Send + Sync {
    /// Create a new connection
    fn new(connection_id: ConnectionId, is_initiator: bool) -> Self
    where
        Self: Sized;
    
    /// Create an offer to establish connection
    fn create_offer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + Send + '_>>;
    
    /// Create an answer to an offer
    fn create_answer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + Send + '_>>;
    
    /// Set remote description (offer or answer)
    fn set_remote_description(&mut self, desc: SessionDescription) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>>;
    
    /// Set local description (offer or answer)
    fn set_local_description(&mut self, desc: SessionDescription) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>>;
    
    /// Add an ICE candidate
    fn add_ice_candidate(&mut self, candidate: IceCandidate) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>>;
    
    /// Get local ICE candidates (may be called multiple times as candidates are gathered)
    fn get_local_candidates(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Vec<IceCandidate>>> + Send + '_>>;
    
    /// Create data channels (command, send, receive)
    fn create_data_channels(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>>;
    
    /// Send data on a specific channel
    fn send_on_channel(&mut self, channel: &str, payload: Payload) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>>;
    
    /// Poll for received data on any channel
    fn poll_received_data(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Option<(String, Payload)>>> + Send + '_>>;
    
    /// Check if connection is established
    fn is_connected(&self) -> bool;
    
    /// Close the connection
    fn close(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>>;
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Initial state
    New,
    /// Gathering ICE candidates
    Gathering,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Disconnected
    Disconnected,
    /// Failed
    Failed,
    /// Closed
    Closed,
}

/// Data channel state
#[derive(Debug, Clone, PartialEq)]
pub enum DataChannelState {
    /// Connecting
    Connecting,
    /// Open and ready
    Open,
    /// Closing
    Closing,
    /// Closed
    Closed,
}
