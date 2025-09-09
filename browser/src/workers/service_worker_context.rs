use wasm_bindgen::prelude::*;
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
}