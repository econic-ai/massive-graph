use crate::camera::Camera;
use crate::types::{InstanceData, Uniforms};
use cgmath::Point3;
use web_sys::console;

pub struct GizmoView {
    pub camera: Camera,
    
    // Gizmo state
    enabled: bool,
    
    // Cached uniforms and matrices
    uniforms: Uniforms,
    uniforms_dirty: bool,
    
    // Last main camera rotation for change detection
    last_main_rotation: Option<(f32, f32)>,
    
    // Instance data for this view (3 axis cubes)
    instance_data: Vec<InstanceData>,
    
    // Viewport configuration
    margin: u32,
    size: u32,
}

impl GizmoView {
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        // Create orthographic camera for gizmo
        let mut camera = Camera::new(150, 150); // Fixed gizmo size
        
        // Set up orthographic projection at fixed distance
        camera.distance = 5.0;
        camera.pan_x = 0.0;
        camera.pan_y = 0.0;
        camera.rotation_x = 0.0;
        camera.rotation_y = 0.0;
        
        let mut gizmo = Self {
            camera,
            enabled: false,
            uniforms: Uniforms::new(),
            uniforms_dirty: true,
            last_main_rotation: None,
            instance_data: Vec::new(),
            margin: 20,
            size: 150,
        };
        
        // Create the 3 axis cubes
        gizmo.create_axis_cubes();
        
        gizmo
    }
    
    pub fn resize(&mut self, canvas_width: u32, canvas_height: u32) {
        // Gizmo camera size doesn't change, but we might need to recalculate viewport position
        // The actual viewport position is calculated in get_viewport_region()
    }
    
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    pub fn mark_dirty(&mut self) {
        self.uniforms_dirty = true;
    }
    
    pub fn get_viewport_region(&self, canvas_width: u32, canvas_height: u32) -> (f32, f32, f32, f32) {
        if !self.enabled {
            return (0.0, 0.0, 0.0, 0.0); // No viewport when disabled
        }
        
        // Bottom-right corner with margin
        let x = canvas_width.saturating_sub(self.size + self.margin) as f32;
        let y = canvas_height.saturating_sub(self.size + self.margin) as f32;
        
        (x, y, self.size as f32, self.size as f32)
    }
    
    pub fn is_dirty(&self) -> bool {
        self.uniforms_dirty
    }
    
    pub fn update_from_main_camera(&mut self, main_rotation: Option<(f32, f32)>) -> bool {
        if !self.enabled {
            return false;
        }
        
        // Check if main camera rotation has changed
        let rotation_changed = match (main_rotation, self.last_main_rotation) {
            (Some((new_x, new_y)), Some((old_x, old_y))) => {
                (new_x - old_x).abs() > 0.001 || (new_y - old_y).abs() > 0.001
            }
            (Some(_), None) => true, // First time
            (None, _) => false, // No main camera update
        };
        
        if rotation_changed {
            if let Some((rotation_x, rotation_y)) = main_rotation {
                // Copy rotation from main camera
                self.camera.rotation_x = rotation_x;
                self.camera.rotation_y = rotation_y;
                
                // Keep gizmo camera fixed at origin with fixed distance
                self.camera.distance = 5.0;
                self.camera.pan_x = 0.0;
                self.camera.pan_y = 0.0;
                
                // CRITICAL: Mark camera as dirty after changing rotation values!
                // Without this, update_matrices() will skip the view matrix recalculation
                self.camera.mark_view_dirty();
                
                // console::log_1(&format!("🎯 Gizmo rotation applied: x={:.3}, y={:.3}", rotation_x, rotation_y).into());
                
                // Update camera matrices immediately with new rotation
                let matrix_time = self.camera.update_matrices();
                
                self.last_main_rotation = main_rotation;
                self.uniforms_dirty = true;
                
                return true;
            }
        }
        
        false
    }
    
    pub fn update_if_dirty(&mut self) -> bool {
        if !self.enabled {
            return false;
        }
        
        if self.uniforms_dirty {
            // Update camera matrices
            self.camera.update_matrices();
            
            // Update uniforms with the combined matrix
            let view_proj_matrix = self.camera.get_view_proj_matrix();
            self.uniforms.update_view_proj(view_proj_matrix);
            
            self.uniforms_dirty = false;
            
            return true;
        }
        
        false
    }
    
    pub fn get_uniforms(&self) -> &Uniforms {
        &self.uniforms
    }
    
    pub fn get_instance_data(&self) -> &[InstanceData] {
        if self.enabled {
            &self.instance_data
        } else {
            &[] // Return empty slice when disabled
        }
    }
    
    fn create_axis_cubes(&mut self) {
        // The gizmo now uses static geometry (GIZMO_VERTICES/GIZMO_INDICES)
        // No instance data needed - the complete gizmo is baked into the vertex buffer
        self.instance_data.clear();
        
        // Add a single "identity" instance at origin with unit scale for the entire gizmo
        self.instance_data.push(InstanceData::new(
            [0.0, 0.0, 0.0], // Position at origin
            [1.0, 1.0, 1.0], // White color (actual colors come from vertex data) 
            1.0,             // Unit scale
        ));
    }
} 