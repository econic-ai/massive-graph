use massive_graph_core::types::ID16;
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
}