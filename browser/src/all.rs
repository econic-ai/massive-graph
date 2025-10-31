//! # Massive Graph Browser WASM
//!
//! WebAssembly bindings for the Massive Graph database WebRTC POC.

use wasm_bindgen::prelude::*;


// Re-export logging macros for use in worker modules and main lib
pub(crate) use massive_graph_core::log_info;

// // Use log_info in this module to avoid unused import warning
// use crate::log_info;

// Module declarations
mod workers;
mod utils;
mod webrtc;

// Re-export worker structs for WASM bindings
pub use workers::{DedicatedWorker, BrowserApp, ServiceWorkerContext};

// Re-export WebRTC manager for WASM bindings
pub use webrtc::WebRtcWorkerManager;



/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Initialize WASM environment
#[wasm_bindgen(start)]
pub fn init() {
    log_info!("⭕ Massive Graph WASM initialised...");
}use massive_graph_core::types::ID16;
use wasm_bindgen::prelude::*;
use crate::{log_info, log_js, js_val};
use web_sys::{MessagePort, ServiceWorker};
use std::collections::HashMap;
use wasm_bindgen::JsCast;

/// Browser application wrapper for WASM (Main Thread UI Worker)
#[wasm_bindgen]
pub struct BrowserApp {
    tab_id: String,
    _instance_id: String,
    is_initialized: bool,
    
    // Service Worker communication
    service_worker: Option<ServiceWorker>,
    service_worker_port: Option<MessagePort>,
    
    // SharedArrayBuffers (created during initialize)
    control_buffer: Option<js_sys::SharedArrayBuffer>,
    notifications_buffer: Option<js_sys::SharedArrayBuffer>,
    deltas_buffer: Option<js_sys::SharedArrayBuffer>,
    data_buffer: Option<js_sys::SharedArrayBuffer>,
    worker_buffers: HashMap<String, js_sys::SharedArrayBuffer>,
    
    // Active workers
    active_workers: HashMap<String, web_sys::Worker>,
}

#[wasm_bindgen]
impl BrowserApp {
    /// Create a new browser application instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Set panic hook for better error messages in console
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        
        let instance_id = ID16::random().to_string();
        let tab_id = ID16::random().to_string();
        
        log_info!(
            "🔥 Creating instance {} for tab: {}", 
            instance_id, tab_id
        );
        
        BrowserApp { 
            tab_id,
            _instance_id: instance_id,
            is_initialized: false,
            service_worker: None,
            service_worker_port: None,
            control_buffer: None,
            notifications_buffer: None,
            deltas_buffer: None,
            data_buffer: None,
            worker_buffers: HashMap::new(),
            active_workers: HashMap::new(),
        }
    }
    
    /// Get the tab ID (method form for JS compatibility)
    pub fn get_tab_id(&self) -> String {
        self.tab_id.clone()
    }
    
    /// Initialize the BrowserApp with Service Worker and create SharedArrayBuffers
    pub fn initialize(&mut self) -> Result<(), JsValue> {
        if self.is_initialized {
            return Ok(());
        }
        
        log_info!("🔥 Initializing...");
        
        // Create SharedArrayBuffers
        self.create_shared_buffers()?;
        
        // Register with Service Worker
        self.register_with_service_worker()?;
        
        // Spawn the dedicated UI helper worker
        self.spawn_dedicated_worker("dedicated-ui-helper")?;
        
        self.is_initialized = true;
        log_info!("🔥 Initialization complete");
        
        Ok(())
    }
    
    /// Create all required SharedArrayBuffers
    fn create_shared_buffers(&mut self) -> Result<(), JsValue> {
        log_info!("🔥 Creating SharedArrayBuffers...");
        
        // Control plane buffer (1MB) - Commands and coordination
        self.control_buffer = Some(js_sys::SharedArrayBuffer::new(1024 * 1024));
        
        // Notifications buffer (1MB) - Field change notifications
        self.notifications_buffer = Some(js_sys::SharedArrayBuffer::new(1024 * 1024));
        
        // Deltas buffer (10MB) - Delta operations queue
        self.deltas_buffer = Some(js_sys::SharedArrayBuffer::new(10 * 1024 * 1024));
        
        // Data buffer (100MB) - Document/graph storage
        self.data_buffer = Some(js_sys::SharedArrayBuffer::new(100 * 1024 * 1024));
        
        log_info!("🔥 SharedArrayBuffers created - Control: 1MB, Notifications: 1MB, Deltas: 10MB, Data: 100MB");
        
        Ok(())
    }
    
    /// Register this tab with the Service Worker
    fn register_with_service_worker(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window available"))?;
        let navigator = window.navigator();
        let sw_container = navigator.service_worker();
        
        // Get the controller (active service worker)
        if let Some(controller) = sw_container.controller() {
            self.service_worker = Some(controller.clone());
            
            // Create MessageChannel for bidirectional communication
            let channel = web_sys::MessageChannel::new()?;
            self.service_worker_port = Some(channel.port1());
            
            // Set up message handler
            let port = channel.port1();
            let tab_id_clone = self.tab_id.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                // We'll handle this in handle_service_worker_message
                // Now we can log the entire event object directly!
                log_js!("🔥 Message from Service Worker", { 
                    "tabId" => &tab_id_clone, 
                    "event" => js_val!(&event),
                    "data" => js_val!(&event.data())
                });
            }) as Box<dyn FnMut(_)>);
            
            port.set_onmessage(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
            
            // Send REGISTER_TAB message
            let message = js_sys::Object::new();
            js_sys::Reflect::set(&message, &"type".into(), &"REGISTER_TAB".into())?;
            js_sys::Reflect::set(&message, &"tabId".into(), &self.tab_id.clone().into())?;
            
            let transfer = js_sys::Array::new();
            transfer.push(&channel.port2());
            
            controller.post_message_with_transferable(&message, &transfer)?;
            
            log_info!("🔥 Registered with Service Worker");
        } else {
            log_info!("🔥 No Service Worker controller available");
        }
        
        Ok(())
    }
    
    /// Send REGISTER_WORKER message to Service Worker
    pub fn register_worker(&self, worker_id: &str, worker_type: &str) -> Result<(), JsValue> {
        if let Some(ref sw) = self.service_worker {
            let message = js_sys::Object::new();
            js_sys::Reflect::set(&message, &"type".into(), &"REGISTER_WORKER".into())?;
            js_sys::Reflect::set(&message, &"tabId".into(), &self.tab_id.clone().into())?;
            js_sys::Reflect::set(&message, &"workerId".into(), &worker_id.into())?;
            js_sys::Reflect::set(&message, &"workerType".into(), &worker_type.into())?;
            
            sw.post_message(&message)?;
            
            log_js!("🔥 Registered worker with Service Worker", {
                "workerId" => worker_id,
                "workerType" => worker_type
            });
        }
        
        Ok(())
    }
    
    /// Spawn a dedicated worker and register it
    pub fn spawn_dedicated_worker(&mut self, worker_type: &str) -> Result<String, JsValue> {
        let worker_id = ID16::random().to_string();
        
        // Create a buffer for this worker
        let worker_buffer = js_sys::SharedArrayBuffer::new(1024 * 1024); // 1MB per worker
        self.worker_buffers.insert(worker_id.clone(), worker_buffer);
        
        // For now, we'll use Web Workers directly even with atomics support
        // std::thread::spawn doesn't work in browser WASM yet
        // TODO: In the future, we can explore wasm-bindgen-rayon or other solutions
        
        log_info!("🔥 Creating Web Worker for {} ({})", worker_id, worker_type);
        let worker = self.create_worker_with_wasm(&worker_id, worker_type)?;
        self.active_workers.insert(worker_id.clone(), worker);
        
        // Note: When using the threaded build, the SharedArrayBuffers can be
        // accessed from multiple threads using atomics for synchronization
        
        // Register with Service Worker
        self.register_worker(&worker_id, worker_type)?;
        
        log_js!("🔥 Spawned dedicated worker", {
            "workerId" => &worker_id,
            "workerType" => worker_type
        });
        
        Ok(worker_id)
    }
    
    /// Create a worker that loads our WASM module
    fn create_worker_with_wasm(&self, worker_id: &str, worker_type: &str) -> Result<web_sys::Worker, JsValue> {
        // Use external TypeScript worker file
        let worker_url = "/src/lib/massive-graph/dedicated-worker.ts";
        
        // Create worker with module type to support ES6 imports
        let worker_options = web_sys::WorkerOptions::new();
        worker_options.set_type(web_sys::WorkerType::Module);
        
        let worker = web_sys::Worker::new_with_options(worker_url, &worker_options)?;
        
        // Set up error handler
        let worker_id_clone = worker_id.to_string();
        let onerror = Closure::wrap(Box::new(move |event: JsValue| {
            // Log the raw error event
            log_js!("🔥❌ Worker error", {
                "workerId" => &worker_id_clone,
                "error" => js_val!(&event)
            });
        }) as Box<dyn FnMut(_)>);
        worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();
        
        // Set up message handler for READY/ERROR responses
        let worker_id_clone2 = worker_id.to_string();
        let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            if let Ok(msg_type) = js_sys::Reflect::get(&data, &"type".into()) {
                match msg_type.as_string().as_deref() {
                    Some("READY") => {
                        log_info!("🔥✅ Worker {} is ready", worker_id_clone2);
                    }
                    Some("ERROR") => {
                        if let Ok(error) = js_sys::Reflect::get(&data, &"error".into()) {
                            log_info!("🔥❌ Worker {} initialization error: {:?}", worker_id_clone2, error);
                        }
                    }
                    _ => {}
                }
            }
        }) as Box<dyn FnMut(_)>);
        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
        
        // Send initialization message
        let init_msg = js_sys::Object::new();
        js_sys::Reflect::set(&init_msg, &"type".into(), &"INIT".into())?;
        js_sys::Reflect::set(&init_msg, &"tabId".into(), &self.tab_id.clone().into())?;
        js_sys::Reflect::set(&init_msg, &"workerId".into(), &worker_id.into())?;
        js_sys::Reflect::set(&init_msg, &"workerType".into(), &worker_type.into())?;
        
        // Create buffers object with all distinct buffers
        let buffers = js_sys::Object::new();
        js_sys::Reflect::set(&buffers, &"control".into(), self.control_buffer.as_ref().unwrap())?;
        js_sys::Reflect::set(&buffers, &"notifications".into(), self.notifications_buffer.as_ref().unwrap())?;
        js_sys::Reflect::set(&buffers, &"deltas".into(), self.deltas_buffer.as_ref().unwrap())?;
        js_sys::Reflect::set(&buffers, &"data".into(), self.data_buffer.as_ref().unwrap())?;
        js_sys::Reflect::set(&buffers, &"worker".into(), self.worker_buffers.get(worker_id).unwrap())?;
        
        js_sys::Reflect::set(&init_msg, &"buffers".into(), &buffers)?;
        
        // Transfer the SharedArrayBuffers
        // let transfer = js_sys::Array::new();
        // Note: SharedArrayBuffers are not transferable, they're shareable
        // So we just send them as-is
        
        worker.post_message(&init_msg)?;
        
        Ok(worker)
    }

    // ========== FIELD NOTIFICATION SYSTEM (kept for FieldNotifier) ==========
    
    /// Register to watch a field for changes
    pub fn register_field_watch(&self, field_id: u32) {
        log_info!("🔥 Registering field watch for field {}", field_id);
        // TODO: Implement field watch registration via control buffer
    }
    
    /// Unregister field watching
    pub fn unregister_field_watch(&self, field_id: u32) {
        log_info!("🔥 Unregistering field watch for field {}", field_id);
        // TODO: Implement field watch unregistration via control buffer
    }
    
    /// Get field metadata (version, offset, size)
    pub fn get_field_info(&self, field_id: u32) -> JsValue {
        log_info!("🔥 Getting field info for field {}", field_id);
        
        // Return mock field info object
        let info = js_sys::Object::new();
        js_sys::Reflect::set(&info, &"version".into(), &JsValue::from(1u32)).unwrap();
        js_sys::Reflect::set(&info, &"offset".into(), &JsValue::from(0u32)).unwrap();
        js_sys::Reflect::set(&info, &"size".into(), &JsValue::from(0u32)).unwrap();
        
        info.into()
    }
    
    /// Get field value as string
    pub fn get_field_as_string(&self, field_id: u32) -> String {
        log_info!("🔥 Getting field {} as string", field_id);
        // TODO: Read from SharedArrayBuffer and decode as UTF-8
        format!("field_{}_value", field_id)
    }
    
    /// Get field value as bytes
    pub fn get_field_as_bytes(&self, field_id: u32) -> js_sys::Uint8Array {
        log_info!("🔥 Getting field {} as bytes", field_id);
        // TODO: Read raw bytes from SharedArrayBuffer
        let mock_data = vec![0u8; 8];
        js_sys::Uint8Array::from(&mock_data[..])
    }
    
    /// Get field value as number
    pub fn get_field_as_number(&self, field_id: u32) -> f64 {
        log_info!("🔥 Getting field {} as number", field_id);
        // TODO: Read from SharedArrayBuffer and interpret as number
        field_id as f64
    }
    
    /// Get field value as object (JSON)
    pub fn get_field_as_object(&self, field_id: u32) -> JsValue {
        log_info!("🔥 Getting field {} as object", field_id);
        // TODO: Read from SharedArrayBuffer, decode as UTF-8, parse as JSON
        let mock_obj = js_sys::Object::new();
        js_sys::Reflect::set(&mock_obj, &"fieldId".into(), &JsValue::from(field_id)).unwrap();
        js_sys::Reflect::set(&mock_obj, &"value".into(), &"mock_data".into()).unwrap();
        
        mock_obj.into()
    }
    
    /// Request a field update
    pub fn request_field_update(&self, field_id: u32, _value: &JsValue) -> Result<(), JsValue> {
        log_info!("🔥 Requesting field update for field {}", field_id);
        
        // TODO: Implement field update via control buffer
        // For now, just log the request
        log_js!("📝 Field update requested", {
            "fieldId" => field_id.to_string(),
            "tabId" => &self.tab_id
        });
        
        Ok(())
    }
}pub mod dedicated_worker_app;
pub mod browser_app;
pub mod service_worker_context;

pub use dedicated_worker_app::DedicatedWorker;
pub use browser_app::BrowserApp;
pub use service_worker_context::ServiceWorkerContext;use wasm_bindgen::prelude::*;
use crate::{log_info, log_js};
use std::collections::HashMap;
use web_sys::MessagePort;

/// Service Worker context for WASM - handles tab tracking and message routing
#[wasm_bindgen]
pub struct ServiceWorkerContext {
    initialized: bool,
    connected_tabs: HashMap<String, MessagePort>,
    // Track workers per tab: tab_id -> Vec<(worker_id, worker_type)>
    tab_workers: HashMap<String, Vec<(String, String)>>,
}

#[wasm_bindgen]
impl ServiceWorkerContext {
    /// Create a new service worker context
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Set panic hook for better error messages in console
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        
        log_info!("🐕 Initializing WASM context for Service Worker");
        
        ServiceWorkerContext {
            initialized: false,
            connected_tabs: HashMap::new(),
            tab_workers: HashMap::new(),
        }
    }
    
    /// Initialize the context
    pub fn initialize(&mut self) {
        log_info!("🐕 Context initialized");
        self.initialized = true;
    }
    
    /// Check if initialized
    #[wasm_bindgen(getter)]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get the number of connected tabs
    #[wasm_bindgen(getter)]
    pub fn tab_count(&self) -> usize {
        self.connected_tabs.len()
    }
    
    /// Handle incoming messages
    pub fn handle_message(&mut self, event: &web_sys::MessageEvent) -> Result<(), JsValue> {
        // Parse the message data
        let data = event.data();
        let message_obj = js_sys::Object::from(data);
        
        // Get message type
        let msg_type = js_sys::Reflect::get(&message_obj, &"type".into())?
            .as_string()
            .unwrap_or_default();
            
        // Get tab ID if present
        let tab_id = js_sys::Reflect::get(&message_obj, &"tabId".into())?
            .as_string();
            
        // Log the message as a JS object for consistency with JS console output
        log_js!("🐕 Message received by Service Worker:", { "type" => &msg_type, "tabId" => &tab_id });
        
        match msg_type.as_str() {
            "PING" => self.handle_ping(event),
            "REGISTER_TAB" => self.handle_register_tab(event, tab_id),
            "UNREGISTER_TAB" => self.handle_unregister_tab(tab_id),
            "REGISTER_WORKER" => self.handle_register_worker(event),
            _ => {
                log_info!("🐕 Unknown message type: {}", msg_type);
                Ok(())
            }
        }
    }
    
    
    // Private message handlers
    fn handle_ping(&self, event: &web_sys::MessageEvent) -> Result<(), JsValue> {
        log_info!("🐕 Handling PING");
        
        // Get the port from the event
        let ports = event.ports();
        if ports.length() > 0 {
            let port_value = ports.get(0);
            let port = MessagePort::from(port_value);
            
            // Create response
            let response = js_sys::Object::new();
            js_sys::Reflect::set(&response, &"type".into(), &"PONG".into())?;
            js_sys::Reflect::set(&response, &"timestamp".into(), &js_sys::Date::now().into())?;
            
            // Send response
            port.post_message(&response)?;
        }
        
        Ok(())
    }
    
    fn handle_register_tab(&mut self, event: &web_sys::MessageEvent, tab_id: Option<String>) -> Result<(), JsValue> {
        if let Some(tab_id) = tab_id {
            let ports = event.ports();
            if ports.length() > 0 {
                let port_value = ports.get(0);
                let port = MessagePort::from(port_value);
                
                // Store the connection
                self.connected_tabs.insert(tab_id.clone(), port.clone());
                
                log_info!("🐕 Tab registered: {}. Total tabs: {}", tab_id, self.connected_tabs.len());
                
                // Send acknowledgment
                let response = js_sys::Object::new();
                js_sys::Reflect::set(&response, &"type".into(), &"REGISTERED".into())?;
                js_sys::Reflect::set(&response, &"tabId".into(), &tab_id.into())?;
                js_sys::Reflect::set(&response, &"totalTabs".into(), &(self.connected_tabs.len() as u32).into())?;
                
                port.post_message(&response)?;
            } else {
                log_info!("🐕 No port provided for tab registration");
            }
        } else {
            log_info!("🐕 No tab ID provided for registration");
        }
        
        Ok(())
    }
    
    fn handle_unregister_tab(&mut self, tab_id: Option<String>) -> Result<(), JsValue> {
        if let Some(tab_id) = tab_id {
            // Remove tab
            if self.connected_tabs.remove(&tab_id).is_some() {
                log_info!("🐕 Tab unregistered: {}. Remaining tabs: {}", tab_id, self.connected_tabs.len());
            }
            
            // Remove all workers associated with this tab
            if let Some(workers) = self.tab_workers.remove(&tab_id) {
                log_info!("🐕 Unregistering {} workers for tab {}", workers.len(), tab_id);
                for (worker_id, worker_type) in workers {
                    log_info!("🐕 Unregistered worker {} (type: {})", worker_id, worker_type);
                }
            }
        }
        
        Ok(())
    }
    
    fn handle_register_worker(&mut self, event: &web_sys::MessageEvent) -> Result<(), JsValue> {
        let message_obj = event.data();
        
        let tab_id = js_sys::Reflect::get(&message_obj, &"tabId".into())?
            .as_string();
        let worker_id = js_sys::Reflect::get(&message_obj, &"workerId".into())?
            .as_string();
        let worker_type = js_sys::Reflect::get(&message_obj, &"workerType".into())?
            .as_string();
            
        if let (Some(tab_id), Some(worker_id), Some(worker_type)) = (tab_id, worker_id, worker_type) {
            // Track this worker under its tab
            let workers = self.tab_workers.entry(tab_id.clone()).or_insert_with(Vec::new);
            workers.push((worker_id.clone(), worker_type.clone()));
            
            log_js!("🔧 Worker registered:", {
                "tabId" => &tab_id,
                "workerId" => &worker_id,
                "workerType" => &worker_type,
                "totalWorkersForTab" => workers.len() as i32
            });
        } else {
            log_info!("🐕 Invalid worker registration - missing required fields");
        }
        
        Ok(())
    }
}use wasm_bindgen::prelude::*;
use crate::{js_val, log_info, log_js};

/// Dedicated worker for background processing (WebRTC, Delta, etc.)
#[wasm_bindgen]
pub struct DedicatedWorker {
    worker_id: String,
    tab_id: String,
    worker_type: String,
    is_initialized: bool,
    
    // SharedArrayBuffers
    control_buffer: Option<js_sys::SharedArrayBuffer>,
    notifications_buffer: Option<js_sys::SharedArrayBuffer>,
    deltas_buffer: Option<js_sys::SharedArrayBuffer>,
    data_buffer: Option<js_sys::SharedArrayBuffer>,
    worker_buffer: Option<js_sys::SharedArrayBuffer>,
}

#[wasm_bindgen]
impl DedicatedWorker {
    /// Create a new dedicated worker
    #[wasm_bindgen(constructor)]
    pub fn new(tab_id: String, worker_id: String, worker_type: String) -> Self {
        log_js!("🔷 DedicatedWorker created", { 
            "workerId" => &worker_id,
            "workerType" => &worker_type, 
            "tabId" => &tab_id 
        });
        
        Self {
            worker_id,
            tab_id,
            worker_type,
            is_initialized: false,
            control_buffer: None,
            notifications_buffer: None,
            deltas_buffer: None,
            data_buffer: None,
            worker_buffer: None,
        }
    }
    
    /// Get the worker ID
    #[wasm_bindgen(getter)]
    pub fn worker_id(&self) -> String {
        self.worker_id.clone()
    }
    
    /// Get the tab ID this worker belongs to
    #[wasm_bindgen(getter)]
    pub fn tab_id(&self) -> String {
        self.tab_id.clone()
    }
    
    /// Get the worker type (WebRTCWorker, DeltaProcessor, etc.)
    #[wasm_bindgen(getter)]
    pub fn worker_type(&self) -> String {
        self.worker_type.clone()
    }
    
    /// Initialize the worker with SharedArrayBuffers
    pub fn initialize(
        &mut self,
        control: js_sys::SharedArrayBuffer,
        notifications: js_sys::SharedArrayBuffer,
        deltas: js_sys::SharedArrayBuffer,
        data: js_sys::SharedArrayBuffer,
        worker: js_sys::SharedArrayBuffer,
    ) {
        log_info!("🔷 Initializing DedicatedWorker with SharedArrayBuffers");
        
        self.control_buffer = Some(control);
        self.notifications_buffer = Some(notifications);
        self.deltas_buffer = Some(deltas);
        self.data_buffer = Some(data);
        self.worker_buffer = Some(worker);
        
        self.is_initialized = true;
        
        log_js!("🔷✅ DedicatedWorker initialized", {
            "workerId" => &self.worker_id,
            "workerType" => &self.worker_type,
            "tabId" => &self.tab_id
        });
    }
    
    /// Is the worker initialized?
    #[wasm_bindgen(getter)]
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Start processing
    pub fn start(&mut self) {
        log_info!("🔧 Starting {} worker: {}", self.worker_type, self.worker_id);
        
        if !self.is_initialized {
            log_info!("🔷❌ Worker not initialized - waiting for buffers");
            return;
        }
        
        // TODO: Implement worker-specific processing
        match self.worker_type.as_str() {
            "dedicated-ui-helper" => {
                log_info!("🔷 UI Helper worker ready for tasks");
                // TODO: Set up UI helper specific tasks
            }
            _ => {
                log_info!("🔷 Unknown worker type: {}", self.worker_type);
            }
        }
    }
    
    /// Stop processing
    pub fn stop(&mut self) {
        log_info!("🔷 Stopping {} worker: {}", self.worker_type, self.worker_id);
        // TODO: Implement worker-specific cleanup
    }
    
    /// Process a message from the main thread
    pub fn process_message(&mut self, message: &JsValue) -> Result<(), JsValue> {
        log_js!("🔷 Processing message", {
            "workerId" => &self.worker_id,
            "message" => js_val!(message)
        });
        
        // TODO: Implement message handling based on worker type
        Ok(())
    }
}
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
//! Browser-side WebRTC implementation

mod browser_connection;
mod worker_integration;

pub use browser_connection::*;
pub use worker_integration::*;
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
use wasm_bindgen::prelude::*;
use web_sys::console;

/// Logs a message with a JavaScript object to the console
/// 
/// # Arguments
/// * `message` - The message prefix (e.g., "📨 Message received:")
/// * `pairs` - A slice of (key, value) tuples to create the JS object
/// 
/// # Example
/// ```
/// log_js_object("📨 Message received:", &[
///     ("type", "PING"),
///     ("tabId", "abc123"),
///     ("timestamp", &timestamp.to_string()),
/// ]);
/// ```
pub fn _log_js_object(message: &str, pairs: &[(&str, &str)]) {
    let obj = js_sys::Object::new();
    
    for (key, value) in pairs {
        js_sys::Reflect::set(&obj, &(*key).into(), &(*value).into()).unwrap();
    }
    
    console::log_2(&message.into(), &obj);
}

/// Macro to log with JS object format
/// 
/// # Example
/// ```
/// log_js!("📨 Message received:", {
///     "type" => msg_type,
///     "tabId" => tab_id,  // Can be Option<String>
///     "count" => count.to_string()
/// });
/// ```
/// Log to JavaScript console with structured data
#[macro_export]
macro_rules! log_js {
    ($message:expr, { $($key:expr => $value:expr),* $(,)? }) => {
        {
            let obj = js_sys::Object::new();
            $(
                let val = $crate::utils::OptionToJsValue::to_js_value(&$value);
                if !val.is_undefined() {
                    js_sys::Reflect::set(&obj, &$key.into(), &val).unwrap();
                }
            )*
            web_sys::console::log_2(&$message.into(), &obj);
        }
    };
}

/// Helper trait to convert values for JS logging
pub trait OptionToJsValue {
    fn to_js_value(&self) -> JsValue;
}

impl OptionToJsValue for Option<String> {
    fn to_js_value(&self) -> JsValue {
        match self {
            Some(v) => v.clone().into(),
            None => JsValue::UNDEFINED,
        }
    }
}

impl OptionToJsValue for String {
    fn to_js_value(&self) -> JsValue {
        self.clone().into()
    }
}

impl OptionToJsValue for &String {
    fn to_js_value(&self) -> JsValue {
        (*self).clone().into()
    }
}

impl OptionToJsValue for &str {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &Option<String> {
    fn to_js_value(&self) -> JsValue {
        match self {
            Some(v) => v.clone().into(),
            None => JsValue::UNDEFINED,
        }
    }
}

// Add implementations for numbers
impl OptionToJsValue for i32 {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &i32 {
    fn to_js_value(&self) -> JsValue {
        JsValue::from(**self)
    }
}

impl OptionToJsValue for u32 {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &u32 {
    fn to_js_value(&self) -> JsValue {
        JsValue::from(**self)
    }
}

impl OptionToJsValue for bool {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &bool {
    fn to_js_value(&self) -> JsValue {
        JsValue::from(**self)
    }
}

// Support for JsValue directly
impl OptionToJsValue for JsValue {
    fn to_js_value(&self) -> JsValue {
        self.clone()
    }
}

impl OptionToJsValue for &JsValue {
    fn to_js_value(&self) -> JsValue {
        (*self).clone()
    }
}

/// Generic wrapper for any value that can be logged
/// This allows us to log any JsValue, including complex objects like MessageEvent
pub struct LogValue<T>(pub T);

// Direct implementation for JsValue references
impl OptionToJsValue for LogValue<&JsValue> {
    fn to_js_value(&self) -> JsValue {
        self.0.clone()
    }
}

// Implementation for owned JsValue
impl OptionToJsValue for LogValue<JsValue> {
    fn to_js_value(&self) -> JsValue {
        self.0.clone()
    }
}

// Implementation for MessageEvent
impl OptionToJsValue for LogValue<&web_sys::MessageEvent> {
    fn to_js_value(&self) -> JsValue {
        self.0.clone().into()
    }
}

// Note: We can't implement AsRef<JsValue> for external types like MessageEvent
// due to Rust's orphan rules. Instead, use LogValue wrapper directly.

// Alternative: Use serde for automatic conversion of Rust structs
#[allow(dead_code)]
pub struct SerdeValue<T: serde::Serialize>(pub T);

#[allow(dead_code)]
impl<T: serde::Serialize> OptionToJsValue for SerdeValue<T> {
    fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.0).unwrap_or(JsValue::NULL)
    }
}

// Helper macro to make LogValue usage cleaner
/// Convert value to JsValue
#[macro_export]
macro_rules! js_val {
    ($val:expr) => {
        $crate::utils::LogValue($val)
    };
}

/// Extension of log_info! macro that supports JS object logging
#[macro_export]
macro_rules! log_info_js {
    // Standard log_info behavior
    ($($arg:tt)*) => {
        $crate::log_info!($($arg)*);
    };
    
    // JS object logging
    ($message:expr, object: { $($key:expr => $value:expr),* $(,)? }) => {
        {
            let obj = js_sys::Object::new();
            $(
                let val: wasm_bindgen::JsValue = match (&$value).into() {
                    Ok(v) => v,
                    Err(_) => $value.to_string().into(),
                };
                js_sys::Reflect::set(&obj, &$key.into(), &val).unwrap();
            )*
            web_sys::console::log_2(&$message.into(), &obj);
        }
    };
}

