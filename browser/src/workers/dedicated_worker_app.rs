use wasm_bindgen::prelude::*;
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
