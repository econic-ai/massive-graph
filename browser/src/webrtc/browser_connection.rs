//! Browser WebRTC implementation using web-sys

use massive_graph_core::{
    webrtc::{WebRtcResult, WebRtcError, SessionDescription, IceCandidate,
             Payload, ConnectionState, SdpType, DataChannelConfig},
    ConnectionId, log_info,
};
use crate::log_js;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use std::pin::Pin;
use std::future::Future;
use web_sys::{
    RtcPeerConnection, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcSessionDescriptionInit, RtcIceCandidateInit, RtcSdpType,
    RtcOfferOptions, RtcAnswerOptions, MessageEvent, RtcPeerConnectionIceEvent,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use js_sys::Reflect;

/// Browser-specific WebRTC connection trait without Send+Sync requirements
pub trait BrowserWebRtcConnection {
    /// Create an offer to establish connection
    fn create_offer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + '_>>;
    
    /// Create an answer to an offer
    fn _create_answer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + '_>>;
    
    /// Set remote description (offer or answer)
    fn set_remote_description(&mut self, desc: SessionDescription) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>>;
    
    /// Add an ICE candidate
    fn _add_ice_candidate(&mut self, candidate: IceCandidate) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>>;
    
    /// Get local ICE candidates
    fn get_local_candidates(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Vec<IceCandidate>>> + '_>>;
    
    /// Send data on a specific channel
    fn send_on_channel(&mut self, channel: &str, payload: Payload) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>>;
    
    /// Poll for received data on any channel
    fn poll_received_data(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Option<(String, Payload)>>> + '_>>;
    
    /// Check if connection is established
    fn is_connected(&self) -> bool;
    
    /// Close the connection
    fn close(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>>;
}

/// Browser-side WebRTC connection
pub struct BrowserConnection {
    /// Connection ID
    _connection_id: ConnectionId,
    /// Whether this peer is the initiator
    is_initiator: bool,
    /// WebRTC peer connection
    peer_connection: RtcPeerConnection,
    /// Connection state
    state: Arc<Mutex<ConnectionState>>,
    /// Data channels by label
    channels: Arc<Mutex<HashMap<String, RtcDataChannel>>>,
    /// Pending ICE candidates
    pending_candidates: Arc<Mutex<Vec<IceCandidate>>>,
    /// Queue of received data (channel_label, payload_data)
    received_data: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    /// Closures for event handlers (kept alive)
    _ice_candidate_closure: Closure<dyn FnMut(RtcPeerConnectionIceEvent)>,
    _connection_state_closure: Closure<dyn FnMut(JsValue)>,
}

impl BrowserConnection {
    /// Create a new browser connection
    pub fn new(connection_id: ConnectionId, is_initiator: bool) -> Self {
        // Create RTC configuration (no ICE servers for localhost)
        let config = RtcConfiguration::new();
        
        // Create peer connection
        let peer_connection = RtcPeerConnection::new_with_configuration(&config)
            .expect("Failed to create RTCPeerConnection");
        
        let state = Arc::new(Mutex::new(ConnectionState::New));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let pending_candidates = Arc::new(Mutex::new(Vec::new()));
        let received_data = Arc::new(Mutex::new(Vec::new()));
        
        // Set up ICE candidate handler
        let candidates_clone = pending_candidates.clone();
        let ice_candidate_closure = Closure::wrap(Box::new(move |event: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = event.candidate() {
                let candidate_str = Reflect::get(&candidate, &"candidate".into())
                    .unwrap()
                    .as_string()
                    .unwrap();
                let sdp_mid = Reflect::get(&candidate, &"sdpMid".into())
                    .ok()
                    .and_then(|v| v.as_string());
                let sdp_m_line_index = Reflect::get(&candidate, &"sdpMLineIndex".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32);
                
                let ice_candidate = IceCandidate {
                    candidate: candidate_str,
                    sdp_mid,
                    sdp_m_line_index,
                };
                
                candidates_clone.lock().unwrap().push(ice_candidate);
                log_info!("ICE candidate gathered");
            }
        }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
        
        peer_connection.set_onicecandidate(Some(ice_candidate_closure.as_ref().unchecked_ref()));
        
        // Set up connection state handler
        let state_clone = state.clone();
        let connection_state_closure = Closure::wrap(Box::new(move |_: JsValue| {
            // Connection state changed
            *state_clone.lock().unwrap() = ConnectionState::Connected;
            log_info!("Connection state changed");
        }) as Box<dyn FnMut(JsValue)>);
        
        peer_connection.set_onconnectionstatechange(Some(connection_state_closure.as_ref().unchecked_ref()));
        
        log_info!("Created browser WebRTC connection: {} (initiator: {})", connection_id, is_initiator);
        
        Self {
            _connection_id: connection_id,
            is_initiator,
            peer_connection,
            state,
            channels,
            pending_candidates,
            received_data,
            _ice_candidate_closure: ice_candidate_closure,
            _connection_state_closure: connection_state_closure,
        }
    }
}

impl BrowserWebRtcConnection for BrowserConnection {
    fn create_offer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + '_>> {
        Box::pin(async move {
            // Create data channels first if we're the initiator
            if self.is_initiator {
                self.setup_channels()?;
            }
            
            // Create offer
            let offer_options = RtcOfferOptions::new();
            let offer_promise = self.peer_connection.create_offer_with_rtc_offer_options(&offer_options);
            let offer = JsFuture::from(offer_promise).await
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to create offer: {:?}", e)))?;
            
            let offer_obj = RtcSessionDescriptionInit::from(offer);
            let sdp = Reflect::get(&offer_obj, &"sdp".into())
                .unwrap()
                .as_string()
                .unwrap();
            
            // Set local description
            let set_promise = self.peer_connection.set_local_description(&offer_obj);
            JsFuture::from(set_promise).await
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to set local description: {:?}", e)))?;
            
            *self.state.lock().unwrap() = ConnectionState::Gathering;
            
            Ok(SessionDescription {
                sdp_type: SdpType::Offer,
                sdp,
            })
        })
    }
    
    fn _create_answer(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<SessionDescription>> + '_>> {
        Box::pin(async move {
            // Create answer
            let answer_options = RtcAnswerOptions::new();
            let answer_promise = self.peer_connection.create_answer_with_rtc_answer_options(&answer_options);
            let answer = JsFuture::from(answer_promise).await
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to create answer: {:?}", e)))?;
            
            let answer_obj = RtcSessionDescriptionInit::from(answer);
            let sdp = Reflect::get(&answer_obj, &"sdp".into())
                .unwrap()
                .as_string()
                .unwrap();
            
            // Set local description
            let set_promise = self.peer_connection.set_local_description(&answer_obj);
            JsFuture::from(set_promise).await
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to set local description: {:?}", e)))?;
            
            *self.state.lock().unwrap() = ConnectionState::Connecting;
            
            Ok(SessionDescription {
                sdp_type: SdpType::Answer,
                sdp,
            })
        })
    }
    
    fn set_remote_description(&mut self, desc: SessionDescription) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>> {
        Box::pin(async move {
            let desc_init = RtcSessionDescriptionInit::new(match desc.sdp_type {
                SdpType::Offer => RtcSdpType::Offer,
                SdpType::Answer => RtcSdpType::Answer,
            });
            desc_init.set_sdp(&desc.sdp);
            
            let set_promise = self.peer_connection.set_remote_description(&desc_init);
            JsFuture::from(set_promise).await
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to set remote description: {:?}", e)))?;
            
            *self.state.lock().unwrap() = ConnectionState::Connecting;
            Ok(())
        })
    }
    
    fn _add_ice_candidate(&mut self, candidate: IceCandidate) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>> {
        Box::pin(async move {
            let candidate_init = RtcIceCandidateInit::new(&candidate.candidate);
            if let Some(mid) = &candidate.sdp_mid {
                candidate_init.set_sdp_mid(Some(mid));
            }
            if let Some(index) = candidate.sdp_m_line_index {
                candidate_init.set_sdp_m_line_index(Some(index as u16));
            }
            
            let add_promise = self.peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init));
            JsFuture::from(add_promise).await
                .map_err(|e| WebRtcError::SignalingError(format!("Failed to add ICE candidate: {:?}", e)))?;
            
            Ok(())
        })
    }
    
    fn get_local_candidates(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Vec<IceCandidate>>> + '_>> {
        Box::pin(async move {
            let candidates = self.pending_candidates.lock().unwrap().drain(..).collect();
            Ok(candidates)
        })
    }
    
    fn send_on_channel(&mut self, channel: &str, payload: Payload) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>> {
        let channel = channel.to_string();
        Box::pin(async move {
            let channels = self.channels.lock().unwrap();
            let data_channel = channels.get(&channel)
                .ok_or_else(|| WebRtcError::DataChannelError(format!("Channel {} not found", channel)))?;
            
            // Send the payload bytes directly
            // SAFETY: We assume the payload data is valid for the duration of this call
            let slice = unsafe { payload.as_slice() };
            
            // Send the data
            data_channel.send_with_u8_array(slice)
                .map_err(|e| WebRtcError::DataChannelError(format!("Failed to send: {:?}", e)))?;
            
            Ok(())
        })
    }
    
    fn poll_received_data(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<Option<(String, Payload)>>> + '_>> {
        Box::pin(async move {
            let mut received = self.received_data.lock().unwrap();
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
        matches!(*self.state.lock().unwrap(), ConnectionState::Connected)
    }
    
    fn close(&mut self) -> Pin<Box<dyn Future<Output = WebRtcResult<()>> + '_>> {
        Box::pin(async move {
            self.peer_connection.close();
            *self.state.lock().unwrap() = ConnectionState::Closed;
            Ok(())
        })
    }
}

impl BrowserConnection {
    /// Set up the three data channels
    fn setup_channels(&mut self) -> WebRtcResult<()> {
        // Command channel
        let command_config = Self::create_channel_init(&DataChannelConfig::command_channel());
        let command_channel = self.peer_connection.create_data_channel_with_data_channel_dict("command", &command_config);
        self.setup_channel_handlers(&command_channel, "command");
        self.channels.lock().unwrap().insert("command".to_string(), command_channel);
        
        // Send channel
        let send_config = Self::create_channel_init(&DataChannelConfig::send_channel());
        let send_channel = self.peer_connection.create_data_channel_with_data_channel_dict("send", &send_config);
        self.setup_channel_handlers(&send_channel, "send");
        self.channels.lock().unwrap().insert("send".to_string(), send_channel);
        
        // Receive channel
        let receive_config = Self::create_channel_init(&DataChannelConfig::receive_channel());
        let receive_channel = self.peer_connection.create_data_channel_with_data_channel_dict("receive", &receive_config);
        self.setup_channel_handlers(&receive_channel, "receive");
        self.channels.lock().unwrap().insert("receive".to_string(), receive_channel);
        
        log_info!("Created data channels");
        
        Ok(())
    }
    
    /// Create RtcDataChannelInit from our config
    fn create_channel_init(config: &DataChannelConfig) -> RtcDataChannelInit {
        let init = RtcDataChannelInit::new();
        init.set_ordered(config.ordered);
        if let Some(id) = config.id {
            init.set_id(id);
        }
        init.set_negotiated(config.negotiated);
        if let Some(time) = config.max_retransmit_time {
            init.set_max_retransmit_time(time);
        }
        if let Some(retransmits) = config.max_retransmits {
            init.set_max_retransmits(retransmits);
        }
        init
    }
    
    /// Set up event handlers for a data channel
    fn setup_channel_handlers(&self, channel: &RtcDataChannel, label: &str) {
        let label = label.to_string();
        
        // On open
        let open_closure = Closure::wrap(Box::new(move |_: JsValue| {
            log_info!("Data channel opened: {}", label);
        }) as Box<dyn FnMut(JsValue)>);
        channel.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
        open_closure.forget();
        
        // On message
        let label = channel.label();
        let received_data_clone = self.received_data.clone();
        let message_closure = Closure::wrap(Box::new(move |event: MessageEvent| {
            log_js!("Data channel message", {
                "channel" => &label,
                "data" => &event.data()
            });
            
            // Extract the data as bytes
            if let Ok(array_buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                let mut data = vec![0u8; uint8_array.length() as usize];
                uint8_array.copy_to(&mut data);
                
                // Store the received data
                received_data_clone.lock().unwrap().push((label.clone(), data));
                log_info!("Stored {} bytes from channel: {}", uint8_array.length(), label);
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        channel.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));
        message_closure.forget();
        
        // On error
        let label = channel.label();
        let error_closure = Closure::wrap(Box::new(move |event: JsValue| {
            log_js!("Data channel error", {
                "channel" => &label,
                "error" => &event
            });
        }) as Box<dyn FnMut(JsValue)>);
        channel.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
        error_closure.forget();
    }
}
