use wasm_bindgen::prelude::*;
use web_sys::Performance;
use std::collections::VecDeque;

// WASM-compatible performance timing
fn performance() -> Performance {
    web_sys::window()
        .expect("should have a window in this context")
        .performance()
        .expect("performance should be available")
}

pub fn now() -> f64 {
    performance().now()
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct PerformanceSnapshot {
    pub timestamp: f64,            // Milliseconds since session start
    pub render_calls_per_sec: f64, // Total render() calls per second (likely 60 FPS)
    pub actual_renders_per_sec: f64, // Actual rendering work per second (when dirty)
    pub dirty_ratio: f64,          // % of render calls that actually render (dirty ratio)
    pub frame_count: u32,          // Total render calls since start
    pub dirty_frame_count: u32,    // Total actual renders since start
    
    // Simplified timing (only when actually rendering)
    pub avg_render_time_ms: f64,   // Average time for actual renders
    
    // Geometry metrics (cached, updated only on scene changes)
    pub object_count: u32,         // Total objects in scene
    pub edge_count: u32,           // Total edges in scene
    pub vertex_count: u32,         // Total vertices in scene template
    pub index_count: u32,          // Total indices for all objects
    
    // Memory metrics (simplified)
    pub memory_usage_mb: f64,      // Current GPU memory usage in MB
    pub scene_size_memory_mb: f64, // Total memory for all objects in scene
    pub active_view_memory_mb: f64, // Memory for currently visible objects (post-culling)
    pub visible_objects: u32,      // Number of visible objects after culling
}

pub struct PerformanceTracker {
    // Track render calls vs actual renders
    render_calls: VecDeque<f64>,       // timestamps of all render() calls
    actual_renders: VecDeque<f64>,     // timestamps of actual rendering work (dirty frames)
    render_times: VecDeque<(f64, f64)>, // (timestamp, duration) for actual renders only
    
    session_start: f64,
    total_render_calls: u32,
    total_actual_renders: u32,
    
    // Export interval (100ms = 10Hz)
    export_interval: f64,
    last_export: f64,
    
    // Track current render start time
    current_render_start: Option<f64>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        let now = now();
        Self {
            render_calls: VecDeque::new(),
            actual_renders: VecDeque::new(),
            render_times: VecDeque::new(),
            session_start: now,
            total_render_calls: 0,
            total_actual_renders: 0,
            export_interval: 100.0, // 100ms
            last_export: now,
            current_render_start: None,
        }
    }
    
    // Clean old timestamps from all queues - call this regularly to prevent stale data
    fn clean_old_data(&mut self) {
        let now = now();
        let one_second_ago = now - 1000.0;
        
        // Clean old render calls
        while let Some(&timestamp) = self.render_calls.front() {
            if timestamp < one_second_ago {
                self.render_calls.pop_front();
            } else {
                break;
            }
        }
        
        // Clean old actual renders  
        while let Some(&timestamp) = self.actual_renders.front() {
            if timestamp < one_second_ago {
                self.actual_renders.pop_front();
            } else {
                break;
            }
        }
        
        // Clean old render times
        while let Some(&(timestamp, _)) = self.render_times.front() {
            if timestamp < one_second_ago {
                self.render_times.pop_front();
            } else {
                break;
            }
        }
    }
    
    // Called every time render() function is invoked
    pub fn track_render_call(&mut self) {
        let now = now();
        self.render_calls.push_back(now);
        self.total_render_calls += 1;
        
        // Clean old data whenever we track a call
        self.clean_old_data();
    }
    
    // Called when render() actually does work (dirty frame)
    pub fn start_actual_render(&mut self) {
        let now = now();
        self.actual_renders.push_back(now);
        self.total_actual_renders += 1;
        
        // Clean old data whenever we start a render
        self.clean_old_data();
        
        // Track current render start time
        self.current_render_start = Some(now);
    }
    
    // Called when actual rendering work completes
    pub fn end_actual_render(&mut self) -> Option<PerformanceSnapshot> {
        let end_time = now();
        let start_time = self.current_render_start.expect("Render start time not set");
        let duration = end_time - start_time;
        
        // Store render time
        self.render_times.push_back((start_time, duration));
        
        // Clean old data whenever we end a render
        self.clean_old_data();
        
        // Check if we should export a snapshot
        if end_time - self.last_export >= self.export_interval {
            self.last_export = end_time;
            return Some(self.create_snapshot(end_time));
        }
        
        None
    }
    
    // Create snapshot with renderer data
    pub fn create_snapshot_with_renderer_data(
        &mut self, 
        now: f64,
        cached_object_count: u32,
        cached_edge_count: u32,
        cached_vertex_count: u32,
        cached_index_count: u32,
        total_memory_usage_bytes: u64,
        scene_size_memory_bytes: u64,
        active_view_memory_bytes: u64,
        visible_objects: u32,
    ) -> PerformanceSnapshot {
        // Clean old data before creating snapshot - this is critical!
        self.clean_old_data();
        
        let mut snapshot = self.create_snapshot(now);
        
        // Add cached geometry metrics
        snapshot.object_count = cached_object_count;
        snapshot.edge_count = cached_edge_count;
        snapshot.vertex_count = cached_vertex_count;
        snapshot.index_count = cached_index_count;
        
        // Calculate memory metrics in MB
        snapshot.memory_usage_mb = total_memory_usage_bytes as f64 / (1024.0 * 1024.0);
        snapshot.scene_size_memory_mb = scene_size_memory_bytes as f64 / (1024.0 * 1024.0);
        snapshot.active_view_memory_mb = active_view_memory_bytes as f64 / (1024.0 * 1024.0);
        snapshot.visible_objects = visible_objects;
        
        snapshot
    }
    
    fn create_snapshot(&self, now: f64) -> PerformanceSnapshot {
        // Calculate render calls per second
        let render_calls_per_sec = self.render_calls.len() as f64;
        
        // Calculate actual renders per second  
        let actual_renders_per_sec = self.actual_renders.len() as f64;
        
        // Calculate dirty ratio
        let dirty_ratio = if render_calls_per_sec > 0.0 {
            actual_renders_per_sec / render_calls_per_sec
        } else {
            0.0
        };
        
        // Calculate average render time for actual renders
        let avg_render_time_ms = if !self.render_times.is_empty() {
            let total_time: f64 = self.render_times.iter().map(|(_, duration)| *duration).sum();
            total_time / self.render_times.len() as f64
        } else {
            0.0
        };
        
        PerformanceSnapshot {
            timestamp: now - self.session_start,
            render_calls_per_sec,
            actual_renders_per_sec,
            dirty_ratio,
            frame_count: self.total_render_calls,
            dirty_frame_count: self.total_actual_renders,
            avg_render_time_ms,
            object_count: 0,
            edge_count: 0,
            vertex_count: 0,
            index_count: 0,
            memory_usage_mb: 0.0,
            scene_size_memory_mb: 0.0,
            active_view_memory_mb: 0.0,
            visible_objects: 0,
        }
    }
    
    pub fn has_frames(&self) -> bool {
        !self.render_calls.is_empty() || !self.actual_renders.is_empty()
    }
} 