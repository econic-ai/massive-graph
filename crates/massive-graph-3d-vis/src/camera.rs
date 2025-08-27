use cgmath::prelude::*;
use crate::performance::now;

pub struct Camera {
    // Camera state
    pub distance: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub rotation_x: f32, // Pitch (up/down rotation)
    pub rotation_y: f32, // Yaw (left/right rotation)
    
    // Matrix caching for performance
    cached_projection_matrix: cgmath::Matrix4<f32>,
    cached_view_matrix: cgmath::Matrix4<f32>,
    cached_view_proj_matrix: cgmath::Matrix4<f32>,
    projection_dirty: bool,
    view_dirty: bool,
    
    // Screen dimensions for aspect ratio
    width: u32,
    height: u32,
    
    // Optimal distance tracking (for scene-based cameras only)
    original_optimal_distance: Option<f32>,
    target_coverage: Option<f32>,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            distance: 5.0,
            pan_x: 0.0,
            pan_y: 0.0,
            rotation_x: 0.0,
            rotation_y: 0.0,
            cached_projection_matrix: cgmath::Matrix4::identity(),
            cached_view_matrix: cgmath::Matrix4::identity(),
            cached_view_proj_matrix: cgmath::Matrix4::identity(),
            projection_dirty: true,
            view_dirty: true,
            width,
            height,
            original_optimal_distance: None,
            target_coverage: None,
        }
    }
    
    /// Create a new camera with optimal distance calculated for a standardized scene
    pub fn new_for_scene(
        width: u32, 
        height: u32, 
        target_coverage: f32
    ) -> Self {
        let optimal_distance = Self::calculate_distance_for_viewport(
            width, 
            target_coverage
        );
        
        Self {
            distance: optimal_distance,
            pan_x: 0.0,
            pan_y: 0.0,
            rotation_x: 0.0,
            rotation_y: 0.0,
            cached_projection_matrix: cgmath::Matrix4::identity(),
            cached_view_matrix: cgmath::Matrix4::identity(),
            cached_view_proj_matrix: cgmath::Matrix4::identity(),
            projection_dirty: true,
            view_dirty: true,
            width,
            height,
            original_optimal_distance: Some(optimal_distance),
            target_coverage: Some(target_coverage),
        }
    }
    
    /// Calculate optimal camera distance considering viewport dimensions
    fn calculate_distance_for_viewport(
        width: u32,
        target_coverage: f32,
    ) -> f32 {
        // Distance calculation based purely on width
        // Assumption: wider screens need closer camera, narrower screens need farther camera
        let reference_width = 2500.0; // Width where distance feels optimal
        let reference_distance = 10.0; // Optimal distance for reference width
        
        // Inverse relationship: distance = reference_distance * (reference_width / current_width)
        let distance = reference_distance * (reference_width / width as f32) / target_coverage;
        
        // Clamp to reasonable bounds
        distance.clamp(0.5, 50.0)
    }
    
    pub fn zoom(&mut self, delta: f32) {
        // Zoom by adjusting camera distance
        // Positive delta = zoom in (get closer), negative = zoom out (get farther)
        let zoom_sensitivity = 0.1;
        let zoom_factor = 1.0 + (delta * zoom_sensitivity);
        
        self.distance /= zoom_factor;
        
        // Clamp camera distance to reasonable bounds
        self.distance = self.distance.clamp(0.1, 50.0);
        
        // Mark view matrix as dirty (camera changed)
        self.mark_view_dirty();
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Pan by adjusting the target position in screen space
        // Scale pan sensitivity based on camera distance (further = larger pan movements)
        let pan_sensitivity = 0.001 * self.distance;
        
        // For screen-space panning, we want:
        // - Horizontal mouse movement -> horizontal screen movement (relative to camera)
        // - Vertical mouse movement -> vertical screen movement (relative to camera)
        
        // Get camera's right and up vectors in world space
        // Right vector: perpendicular to camera forward and world up
        let cos_yaw = self.rotation_y.cos();
        let sin_yaw = self.rotation_y.sin();
        let right = cgmath::Vector3::new(cos_yaw, 0.0, sin_yaw);
        
        // Up vector: always world up for natural panning feel
        let up = cgmath::Vector3::new(0.0, 1.0, 0.0);
        
        // Apply pan movement in screen-relative directions
        // Invert deltaX so dragging right moves view right
        let pan_offset = -delta_x * pan_sensitivity * right + delta_y * pan_sensitivity * up;
        
        self.pan_x += pan_offset.x;
        self.pan_y += pan_offset.y;
        
        // Mark view matrix as dirty (camera changed)
        self.mark_view_dirty();
    }

    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        // Rotate camera around target (orbital rotation)
        let rotation_sensitivity = 0.001;
        
        // Invert deltas so dragging feels like rotating the object directly
        self.rotation_y -= delta_x * rotation_sensitivity; // Yaw (left/right) - unlimited
        self.rotation_x -= delta_y * rotation_sensitivity; // Pitch (up/down) - unlimited, no normalization
        
        // Allow unlimited yaw rotation (no clamping)
        // Normalize yaw to keep it in reasonable range but allow continuous rotation
        if self.rotation_y > std::f32::consts::PI {
            self.rotation_y -= 2.0 * std::f32::consts::PI;
        } else if self.rotation_y < -std::f32::consts::PI {
            self.rotation_y += 2.0 * std::f32::consts::PI;
        }
        
        // For pitch, do NOT normalize - let it accumulate freely
        // This prevents the disorienting flip when crossing π boundaries
        
        // Mark view matrix as dirty (camera changed)
        self.mark_view_dirty();
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            
            // For scene-based cameras, recalculate optimal distance and preserve zoom factor
            if let (Some(original_optimal), Some(target_coverage)) = 
                (self.original_optimal_distance, self.target_coverage) {
                
                // Calculate current zoom factor (current distance / original optimal distance)
                let zoom_factor = self.distance / original_optimal;
                
                // Calculate new optimal distance for new canvas size
                let new_optimal_distance = Self::calculate_distance_for_viewport(
                    width,
                    target_coverage
                );
                
                // Apply the same zoom factor to the new optimal distance
                self.distance = new_optimal_distance * zoom_factor;
                
                // Update the stored optimal distance
                self.original_optimal_distance = Some(new_optimal_distance);
                
                // Mark view as dirty since distance changed
                self.mark_view_dirty();
            }
            
            // Mark projection matrix as dirty (resize)  
            self.mark_projection_dirty();
        }
    }
    
    // Check if any matrices need updating
    pub fn is_dirty(&self) -> bool {
        self.projection_dirty || self.view_dirty
    }
    
    // Get the combined view-projection matrix, calculating if needed
    pub fn get_view_proj_matrix(&mut self) -> cgmath::Matrix4<f32> {
        if self.projection_dirty || self.view_dirty {
            self.update_matrices();
        }
        self.cached_view_proj_matrix
    }
    
    // Update matrices only if needed, with timing for performance tracking
    pub fn update_matrices(&mut self) -> f64 {
        let start_time = now();
        
        // Recalculate projection matrix only if it's dirty (resize)
        if self.projection_dirty {
            let aspect = self.width as f32 / self.height as f32;
            self.cached_projection_matrix = cgmath::perspective(cgmath::Deg(45.0), aspect, 0.1, 100.0);
            self.projection_dirty = false;
        }
        
        // Recalculate view matrix only if it's dirty (camera moved)
        if self.view_dirty {
            // Create orbital camera system
            // 1. Apply rotations around the target
            let rotation_y = cgmath::Matrix4::from_angle_y(cgmath::Rad(self.rotation_y));
            let rotation_x = cgmath::Matrix4::from_angle_x(cgmath::Rad(self.rotation_x));
            let rotation_matrix = rotation_y * rotation_x;
            
            // 2. Position camera at distance from target
            let camera_offset = cgmath::Vector3::new(0.0, 0.0, self.distance);
            let rotated_offset = rotation_matrix.transform_vector(camera_offset);
            
            // 3. Apply pan offset to target position
            let target = cgmath::Point3::new(self.pan_x, self.pan_y, 0.0);
            let camera_position = target + rotated_offset;
            
            // 4. Calculate proper up vector that accounts for camera orientation
            // Use the camera's local up vector instead of world up to prevent flipping
            let camera_up = rotation_matrix.transform_vector(cgmath::Vector3::new(0.0, 1.0, 0.0));
            
            // 5. Create view matrix with proper up vector
            self.cached_view_matrix = cgmath::Matrix4::look_at_rh(
                camera_position,
                target,
                camera_up,
            );
            self.view_dirty = false;
        }

        // Combine cached matrices (this is very fast since matrices are already calculated)
        let model = cgmath::Matrix4::identity(); // Static model matrix
        self.cached_view_proj_matrix = self.cached_projection_matrix * self.cached_view_matrix * model;
        
        let end_time = now();
        end_time - start_time
    }
    
    // Get current camera state for performance tracking
    pub fn get_state(&self) -> (f32, f32, f32, f32, f32) {
        (self.distance, self.pan_x, self.pan_y, self.rotation_x, self.rotation_y)
    }
    
    // Get current camera dimensions
    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    // Helper method to mark view matrix as dirty (camera changed)
    pub fn mark_view_dirty(&mut self) {
        self.view_dirty = true;
    }
    
    // Helper method to mark projection matrix as dirty (resize)  
    fn mark_projection_dirty(&mut self) {
        self.projection_dirty = true;
    }
} 