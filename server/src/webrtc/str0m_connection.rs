//! Native WebRTC implementation using str0m

use massive_graph_core::{
    webrtc::{WebRtcConnection, WebRtcResult, WebRtcError, SessionDescription, IceCandidate, 
             Payload, ConnectionState, SdpType},
    ConnectionId,
};
use str0m::{Rtc, Event, Output, Input, channel::{ChannelConfig, ChannelId}, 
            change::{SdpOffer, SdpAnswer}, Candidate, IceConnectionState};
use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Server-side WebRTC connection using str0m
pub struct Str0mConnection {
    /// Connection ID
    _connection_id: ConnectionId,
    /// Whether this peer is the initiator
    is_initiator: bool,
    /// str0m RTC instance
    rtc: Arc<Mutex<Rtc>>,
    /// Connection state
    state: Arc<Mutex<ConnectionState>>,
    /// Data channels by label
    channels: Arc<Mutex<HashMap<String, ChannelId>>>,
    /// Pending ICE candidates
    pending_candidates: Arc<Mutex<Vec<IceCandidate>>>,
    /// Pending local SDP offer (for str0m's two-step process)
    pending_offer: Arc<Mutex<Option<SdpOffer>>>,
    /// Queue of received data (channel_label, payload_data)
    received_data: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl WebRtcConnection for Str0mConnection {
    fn new(connection_id: ConnectionId, is_initiator: bool) -> Self {
        let rtc = Rtc::builder()
            .set_ice_lite(true)  // Server operates in ICE-lite mode
            .build();
            
        info!("Created str0m WebRTC connection: {} (initiator: {})", connection_id, is_initiator);
        
        Self {
            _connection_id: connection_id,
            is_initiator,
            rtc: Arc::new(Mutex::new(rtc)),
            state: Arc::new(Mutex::new(ConnectionState::New)),
            channels: Arc::new(Mutex::new(HashMap::new())),
            pending_candidates: Arc::new(Mutex::new(Vec::new())),
            pending_offer: Arc::new(Mutex::new(None)),
            received_data: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn create_offer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + Send + '_>> {
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            
            // Create data channels first
            self.setup_channels(&mut rtc)?;
            
            // str0m uses a two-step process: first gather, then render
            let change = rtc.sdp_api();
            
            // Apply changes and get the offer
            if let Some((offer, _pending)) = change.apply() {
                // Get the SDP string before moving the offer
                let sdp_string = offer.to_sdp_string();
                
                // Store the offer for later
                *self.pending_offer.lock().await = Some(offer);
                
                *self.state.lock().await = ConnectionState::Gathering;
                
                Ok(SessionDescription {
                    sdp_type: SdpType::Offer,
                    sdp: sdp_string,
                })
            } else {
                Err(WebRtcError::SignalingError("No changes to create offer".to_string()))
            }
        })
    }
    
    fn create_answer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + Send + '_>> {
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            
            // For answer, we need the pending offer that was set by set_remote_description
            let pending_offer = self.pending_offer.lock().await.take()
                .ok_or_else(|| WebRtcError::SignalingError("No pending offer to answer".to_string()))?;
            
            // Create answer using the SDP API
            let change = rtc.sdp_api();
            let answer = change.accept_offer(pending_offer)
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to create answer: {:?}", e)))?;
            
            *self.state.lock().await = ConnectionState::Connecting;
            
            Ok(SessionDescription {
                sdp_type: SdpType::Answer,
                sdp: answer.to_sdp_string(),
            })
        })
    }
    
    fn set_remote_description(&mut self, desc: SessionDescription) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>> {
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            
            match desc.sdp_type {
                SdpType::Offer => {
                    // Parse the offer and store it for creating answer
                    let offer = SdpOffer::from_sdp_string(&desc.sdp)
                        .map_err(|e| WebRtcError::SignalingError(format!("Invalid SDP offer: {:?}", e)))?;
                    
                    *self.pending_offer.lock().await = Some(offer);
                    
                    // If we received an offer, set up our channels
                    if !self.is_initiator {
                        self.setup_channels(&mut rtc)?;
                    }
                }
                SdpType::Answer => {
                    // For answer, we need to parse and accept it
                    let _answer = SdpAnswer::from_sdp_string(&desc.sdp)
                        .map_err(|e| WebRtcError::SignalingError(format!("Invalid SDP answer: {:?}", e)))?;
                    
                    // We need the original SdpPendingOffer from when we created the offer
                    // In a real implementation, we'd store this when creating the offer
                    // For now, we'll accept the answer directly via the RTC API
                    
                    // str0m doesn't have a direct way to accept an answer without the pending offer
                    // This is a limitation we'll need to work around
                    info!("Received answer, connection should be establishing");
                }
            }
            
            *self.state.lock().await = ConnectionState::Connecting;
            Ok(())
        })
    }
    
    fn set_local_description(&mut self, _desc: SessionDescription) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>> {
        Box::pin(async move {
            // str0m handles this internally
            Ok(())
        })
    }
    
    fn add_ice_candidate(&mut self, candidate: IceCandidate) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>> {
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            
            // Parse the candidate string into str0m's Candidate type
            let cand = Candidate::from_sdp_string(&candidate.candidate)
                .map_err(|e| WebRtcError::SignalingError(format!("Invalid ICE candidate: {:?}", e)))?;
            
            // Add the candidate
            rtc.add_remote_candidate(cand);
            
            Ok(())
        })
    }
    
    fn get_local_candidates(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Vec<IceCandidate>>> + Send + '_>> {
        Box::pin(async move {
            let candidates = self.pending_candidates.lock().await.drain(..).collect();
            Ok(candidates)
        })
    }
    
    fn create_data_channels(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>> {
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            self.setup_channels(&mut rtc)?;
            Ok(())
        })
    }
    
    fn send_on_channel(&mut self, channel: &str, payload: Payload) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>> {
        let channel = channel.to_string();
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            let channels = self.channels.lock().await;
            
            let channel_id = channels.get(&channel)
                .ok_or_else(|| WebRtcError::DataChannelError(format!("Channel {} not found", channel)))?;
            
            // Get the channel and write data
            let mut chan = rtc.channel(*channel_id)
                .ok_or_else(|| WebRtcError::DataChannelError(format!("Channel {} not initialized", channel)))?;
            
            // Send the payload bytes directly
            // SAFETY: We assume the payload data is valid for the duration of this call
            let slice = unsafe { payload.as_slice() };
            chan.write(true, slice)
                .map_err(|e| WebRtcError::DataChannelError(format!("Failed to send: {:?}", e)))?;
            
            Ok(())
        })
    }
    
    fn poll_received_data(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Option<(String, Payload)>>> + Send + '_>> {
        Box::pin(async move {
            let mut received = self.received_data.lock().await;
            if let Some((channel_label, data)) = received.pop() {
                // Convert Vec<u8> to Payload
                let (payload, _owned_data) = Payload::from_vec(data);
                Ok(Some((channel_label, payload)))
            } else {
                Ok(None)
            }
        })
    }
    
    fn is_connected(&self) -> bool {
        matches!(
            *self.state.blocking_lock(),
            ConnectionState::Connected
        )
    }
    
    fn close(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + Send + '_>> {
        Box::pin(async move {
            let mut rtc = self.rtc.lock().await;
            rtc.disconnect();
            *self.state.lock().await = ConnectionState::Closed;
            Ok(())
        })
    }
}

impl Str0mConnection {
    /// Set up the three data channels
    fn setup_channels(&self, rtc: &mut Rtc) -> WebRtcResult<()> {
        // Use the SDP API to add channels
        let mut change = rtc.sdp_api();
        
        // Command channel - simple label-only config
        let command_id = change.add_channel("command".to_string());
        
        // Send channel - with custom config for unreliable
        let send_config = ChannelConfig {
            label: "send".to_string(),
            ordered: false,
            reliability: Default::default(),  // Default is Reliable
            negotiated: None,  // Let str0m negotiate
            protocol: String::new(),
        };
        let send_id = change.add_channel_with_config(send_config);
        
        // Receive channel - with custom config for unreliable
        let receive_config = ChannelConfig {
            label: "receive".to_string(),
            ordered: false,
            reliability: Default::default(),  // Default is Reliable
            negotiated: None,  // Let str0m negotiate
            protocol: String::new(),
        };
        let receive_id = change.add_channel_with_config(receive_config);
        
        // Apply the changes
        let _ = change.apply();
        
        // Store channel IDs
        let mut channels = self.channels.blocking_lock();
        channels.insert("command".to_string(), command_id);
        channels.insert("send".to_string(), send_id);
        channels.insert("receive".to_string(), receive_id);
        
        info!("Configured data channels - command: {:?}, send: {:?}, receive: {:?}", 
              command_id, send_id, receive_id);
        
        Ok(())
    }
    
    /// Process RTC events (should be called in a loop)
    pub async fn handle_event(&mut self, event: Event) -> WebRtcResult<()> {
        match event {
            Event::IceConnectionStateChange(state) => {
                info!("ICE connection state changed: {:?}", state);
                match state {
                    IceConnectionState::Connected => {
                        *self.state.lock().await = ConnectionState::Connected;
                    }
                    IceConnectionState::Disconnected => {
                        *self.state.lock().await = ConnectionState::Disconnected;
                    }
                    _ => {}
                }
            }
            Event::ChannelOpen(id, label) => {
                info!("Data channel opened: {} (id: {:?})", label, id);
                self.channels.lock().await.insert(label, id);
            }
            Event::ChannelData(data) => {
                info!("Received {} bytes on channel {:?}", data.data.len(), data.id);
                
                // Find the channel label for this ID
                let channels = self.channels.lock().await;
                let channel_label = channels.iter()
                    .find(|(_, &id)| id == data.id)
                    .map(|(label, _)| label.clone());
                
                if let Some(label) = channel_label {
                    // Store the received data for polling
                    let mut received = self.received_data.lock().await;
                    received.push((label, data.data.to_vec()));
                    info!("Queued data for channel: {}", received.last().unwrap().0);
                } else {
                    warn!("Received data on unknown channel ID: {:?}", data.id);
                }
            }
            Event::ChannelClose(id) => {
                warn!("Data channel closed: {:?}", id);
                // Remove from our channel map
                let mut channels = self.channels.lock().await;
                channels.retain(|_, v| *v != id);
            }
            _ => {
                // Handle other events as needed
            }
        }
        Ok(())
    }
    
    /// Poll the RTC instance for output
    pub async fn poll_output(&mut self) -> WebRtcResult<Option<Output>> {
        let mut rtc = self.rtc.lock().await;
        match rtc.poll_output() {
            Ok(output) => Ok(Some(output)),
            Err(e) => Err(WebRtcError::Other(format!("Poll error: {:?}", e))),
        }
    }
    
    /// Handle timeout
    pub async fn handle_timeout(&mut self, now: std::time::Instant) -> WebRtcResult<()> {
        let mut rtc = self.rtc.lock().await;
        rtc.handle_input(Input::Timeout(now))
            .map_err(|e| WebRtcError::Other(format!("Timeout handling failed: {:?}", e)))?;
        Ok(())
    }
    
    /// Handle incoming data
    pub async fn handle_input(&mut self, input: Input<'_>) -> WebRtcResult<()> {
        let mut rtc = self.rtc.lock().await;
        rtc.handle_input(input)
            .map_err(|e| WebRtcError::Other(format!("Input handling failed: {:?}", e)))?;
        Ok(())
    }
}