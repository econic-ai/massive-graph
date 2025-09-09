//! Platform-agnostic connection management for WebRTC
//! 
//! This module provides a ConnectionManager that works on both server (native)
//! and browser (WASM) environments. It manages WebRTC connections and handles
//! the connection lifecycle.

use dashmap::DashMap;
use crate::types::ConnectionId;

/// Manages WebRTC connections in a platform-agnostic way
pub struct ConnectionManager {
    /// Our own connection ID
    pub connection_id: ConnectionId,
    
    /// Whether this is running on the server
    pub is_server: bool,
    
    /// Active connections mapped by their connection IDs
    connections: DashMap<ConnectionId, ConnectionState>,
}

/// State of a single connection
#[derive(Debug, Clone)]
pub struct ConnectionState {
    /// Remote connection ID
    pub remote_id: ConnectionId,
    
    /// Whether this is the main control connection
    pub is_control: bool,
    
    /// Connection status
    pub status: ConnectionStatus,
}

/// Connection status
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Connection is being established
    Connecting,
    /// Connection is active
    Connected,
    /// Connection is closed
    Closed,
}

impl ConnectionManager {
    /// Create a new ConnectionManager
    pub fn new(connection_id: ConnectionId, is_server: bool) -> Self {
        Self {
            connection_id,
            is_server,
            connections: DashMap::new(),
        }
    }
    
    /// Register a new connection
    pub fn add_connection(&self, remote_id: ConnectionId, is_control: bool) {
        let state = ConnectionState {
            remote_id: remote_id.clone(),
            is_control,
            status: ConnectionStatus::Connecting,
        };
        self.connections.insert(remote_id, state);
    }
    
    /// Update connection status
    pub fn update_status(&self, remote_id: &ConnectionId, status: ConnectionStatus) {
        if let Some(mut connection) = self.connections.get_mut(remote_id) {
            connection.status = status;
        }
    }
    
    /// Remove a connection
    pub fn remove_connection(&self, remote_id: &ConnectionId) {
        self.connections.remove(remote_id);
    }
    
    /// Get all active connections
    pub fn get_active_connections(&self) -> Vec<ConnectionId> {
        self.connections
            .iter()
            .filter(|entry| entry.status == ConnectionStatus::Connected)
            .map(|entry| entry.remote_id.clone())
            .collect()
    }
    
    /// Check if a connection exists
    pub fn has_connection(&self, remote_id: &ConnectionId) -> bool {
        self.connections.contains_key(remote_id)
    }
    
    /// Get connection state
    pub fn get_connection(&self, remote_id: &ConnectionId) -> Option<ConnectionState> {
        self.connections.get(remote_id).map(|entry| entry.clone())
    }
}
