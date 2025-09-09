//! Simple binary protocol for WebRTC control channel
//! 
//! Defines Command and Event messages that can be sent over the control channel.

use serde::{Serialize, Deserialize};
use crate::types::ConnectionId;
use bincode::serde::{encode_to_vec, decode_from_slice};
use bincode::config;

/// Commands that can be sent over the control channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Register with the server using a connection ID
    Register { 
        /// Connection ID of the sender
        connection_id: ConnectionId 
    },
    
    /// Ping message to test connection
    Ping { 
        /// Timestamp when ping was sent
        timestamp: u64 
    },
}

/// Events that can be received over the control channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Request to establish a new connection
    ConnectToMe { 
        /// New connection ID to use for the secondary connection
        new_connection_id: ConnectionId 
    },
    
    /// Response to a ping
    Pong { 
        /// Original timestamp from the ping
        timestamp: u64 
    },
}

/// Serialize a command to binary format
pub fn serialize_command(cmd: &Command) -> Result<Vec<u8>, bincode::error::EncodeError> {
    encode_to_vec(cmd, config::standard())
}

/// Deserialize a command from binary format
pub fn deserialize_command(data: &[u8]) -> Result<Command, bincode::error::DecodeError> {
    let (command, _) = decode_from_slice(data, config::standard())?;
    Ok(command)
}

/// Serialize an event to binary format
pub fn serialize_event(event: &Event) -> Result<Vec<u8>, bincode::error::EncodeError> {
    encode_to_vec(event, config::standard())
}

/// Deserialize an event from binary format
pub fn deserialize_event(data: &[u8]) -> Result<Event, bincode::error::DecodeError> {
    let (event, _) = decode_from_slice(data, config::standard())?;
    Ok(event)
}
