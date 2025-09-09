//! Integration of WebRTC into the dedicated worker

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use massive_graph_core::{
    webrtc::{ConnectionRequest, ConnectionResponse, IceCandidateRequest, TestMessage, IceCandidate},
    ConnectionId, log_info,
};
use crate::log_js;
use web_sys::{Request, RequestInit, Response, Headers};
use super::{BrowserConnection, BrowserWebRtcConnection};
use std::rc::Rc;
use std::cell::RefCell;

/// WebRTC manager for the dedicated worker
#[wasm_bindgen]
pub struct WebRtcWorkerManager {
    /// Server URL
    server_url: String,
    /// Our connection ID
    connection_id: ConnectionId,
    /// Active connection (using Rc/RefCell for single-threaded WASM)
    connection: Option<Rc<RefCell<BrowserConnection>>>,
}

#[wasm_bindgen]
impl WebRtcWorkerManager {
    /// Create a new WebRTC worker manager
    #[wasm_bindgen(constructor)]
    pub fn new(server_url: Option<String>) -> Self {
        let server_url = server_url.unwrap_or_else(|| "http://localhost:8080".to_string());
        let connection_id = ConnectionId::random();
        
        log_info!("Created WebRTC worker manager with ID: {}", connection_id);
        
        Self {
            server_url,
            connection_id,
            connection: None,
        }
    }
    
    /// Connect to the server
    pub async fn connect(&mut self) -> Result<(), JsValue> {
        log_info!("Initiating WebRTC connection to server");
        
        // Create browser connection
        let connection = BrowserConnection::new(self.connection_id.clone(), true);
        let connection_rc = Rc::new(RefCell::new(connection));
        
        // Create offer
        let offer = connection_rc.borrow_mut().create_offer().await
            .map_err(|e| JsValue::from_str(&format!("Failed to create offer: {}", e)))?;
        
        // Send connection request with offer
        let request = ConnectionRequest {
            client_id: self.connection_id.clone(),
            offer: Some(offer),
        };
        
        let response = self.send_http_request("/webrtc/connect", &request).await?;
        let conn_response: ConnectionResponse = serde_wasm_bindgen::from_value(response)?;
        
        if !conn_response.success {
            return Err(JsValue::from_str(&format!("Connection failed: {:?}", conn_response.error)));
        }
        
        // Set remote answer
        if let Some(answer) = conn_response.answer {
            connection_rc.borrow_mut().set_remote_description(answer).await
                .map_err(|e| JsValue::from_str(&format!("Failed to set answer: {}", e)))?;
        }
        
        // Store connection
        self.connection = Some(connection_rc);
        
        // Start ICE candidate exchange
        self.start_ice_exchange();
        
        log_info!("WebRTC connection initiated with server: {}", conn_response.server_id);
        
        Ok(())
    }
    
    /// Send a test ping message
    pub async fn send_ping(&self, message: String) -> Result<(), JsValue> {
        if let Some(connection) = &self.connection {
            let timestamp = js_sys::Date::now() as u64;
            let test_msg = TestMessage::Ping {
                timestamp,
                payload: message,
            };
            
            let (payload, _data) = test_msg.to_payload()
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize: {}", e)))?;
            
            connection.borrow_mut().send_on_channel("command", payload).await
                .map_err(|e| JsValue::from_str(&format!("Failed to send: {}", e)))?;
            
            log_info!("Sent ping message");
            Ok(())
        } else {
            Err(JsValue::from_str("Not connected"))
        }
    }
    
    /// Check if connected
    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &self.connection {
            connection.borrow().is_connected()
        } else {
            false
        }
    }
    
    /// Close the connection
    pub async fn close(&mut self) -> Result<(), JsValue> {
        if let Some(connection) = &self.connection {
            connection.borrow_mut().close().await
                .map_err(|e| JsValue::from_str(&format!("Failed to close: {}", e)))?;
        }
        self.connection = None;
        Ok(())
    }
}

impl WebRtcWorkerManager {
    /// Start ICE candidate exchange
    fn start_ice_exchange(&self) {
        if let Some(connection) = &self.connection {
            let connection = connection.clone();
            let server_url = self.server_url.clone();
            
            spawn_local(async move {
                // Periodically check for local candidates and send them
                loop {
                    gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
                    
                    let candidates = connection.borrow_mut().get_local_candidates().await.unwrap_or_default();
                    
                    for candidate in candidates {
                        if let Err(e) = Self::send_ice_candidate(&server_url, candidate).await {
                            log_js!("Failed to send ICE candidate", { "error" => &e });
                        }
                    }
                    
                    // Check if connected
                    if connection.borrow().is_connected() {
                        log_info!("WebRTC connection established!");
                        break;
                    }
                }
            });
        }
    }
    
    /// Send ICE candidate to server
    async fn send_ice_candidate(server_url: &str, candidate: IceCandidate) -> Result<(), JsValue> {
        let url = format!("{}/webrtc/ice-candidate", server_url);
        
        let headers = Headers::new()?;
        headers.set("Content-Type", "application/json")?;
        
        let request_data = serde_wasm_bindgen::to_value(&IceCandidateRequest {
            connection_id: ConnectionId::random(), // TODO: Use actual connection ID
            candidate,
        })?;
        
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_headers(&headers);
        opts.set_body(&js_sys::JSON::stringify(&request_data)?.into());
        
        let request = Request::new_with_str_and_init(&url, &opts)?;
        let window = web_sys::window().unwrap();
        let response = JsFuture::from(window.fetch_with_request(&request)).await?;
        let response: Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", response.status())));
        }
        
        Ok(())
    }
    
    /// Send HTTP request
    async fn send_http_request<T: serde::Serialize>(&self, path: &str, data: &T) -> Result<JsValue, JsValue> {
        let url = format!("{}{}", self.server_url, path);
        
        let headers = Headers::new()?;
        headers.set("Content-Type", "application/json")?;
        
        let request_data = serde_wasm_bindgen::to_value(data)?;
        
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_headers(&headers);
        opts.set_body(&js_sys::JSON::stringify(&request_data)?.into());
        
        let request = Request::new_with_str_and_init(&url, &opts)?;
        let window = web_sys::window().unwrap();
        let response = JsFuture::from(window.fetch_with_request(&request)).await?;
        let response: Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", response.status())));
        }
        
        let json = JsFuture::from(response.json()?).await?;
        Ok(json)
    }
}

use wasm_bindgen_futures::JsFuture;
