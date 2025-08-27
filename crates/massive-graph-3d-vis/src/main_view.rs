use crate::camera::Camera;
use crate::types::{InstanceData, Uniforms};
use crate::math::{Frustum, BoundingSphere};
use cgmath::Point3;

// Simple renderable object for main scene
#[derive(Debug, Clone)]
pub struct RenderableObject {
    pub position: Point3<f32>,
    pub bounding_sphere: BoundingSphere,
    pub visible: bool, // Result of frustum culling
}

impl RenderableObject {
    pub fn new(position: Point3<f32>, size: f32) -> Self {
        Self {
            position,
            bounding_sphere: BoundingSphere::for_cube(position, size),
            visible: true,
        }
    }
}

pub struct MainView {
    pub camera: Camera,
    pub objects: Vec<RenderableObject>,
    
    // Cached uniforms and matrices
    uniforms: Uniforms,
    uniforms_dirty: bool,
    current_frustum: Option<Frustum>,
    
    // Culling state
    pub visible_objects: u32,
    pub total_objects: u32,
    
    // Instance data for this view
    instance_data: Vec<InstanceData>,
}

impl MainView {
    pub fn new(width: u32, height: u32) -> Self {
        let mut main_view = Self {
            camera: Camera::new_for_scene(
                width, 
                height, 
                2.0     // target_coverage: scene takes 60% of viewport height
            ),
            objects: Vec::new(),
            uniforms: Uniforms::new(),
            uniforms_dirty: true,
            current_frustum: None,
            visible_objects: 0,
            total_objects: 0,
            instance_data: Vec::new(),
        };
        
        // Force initial camera matrix calculation to ensure proper initialization
        main_view.camera.update_matrices();
        let view_proj_matrix = main_view.camera.get_view_proj_matrix();
        main_view.uniforms.update_view_proj(view_proj_matrix);
        
        main_view
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width, height);
        // Mark as dirty to ensure camera projection matrix is updated immediately
        // This prevents aspect ratio distortion during resize
        self.mark_dirty();
    }
    
    pub fn zoom(&mut self, delta: f32) {
        self.camera.zoom(delta);
        self.uniforms_dirty = true;
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
        self.uniforms_dirty = true;
    }

    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.rotate(delta_x, delta_y);
        self.uniforms_dirty = true;
    }
    
    pub fn get_viewport_region(&self, canvas_width: u32, canvas_height: u32) -> (f32, f32, f32, f32) {
        // Main view uses full canvas
        (0.0, 0.0, canvas_width as f32, canvas_height as f32)
    }
    
    pub fn is_dirty(&self) -> bool {
        self.uniforms_dirty || self.camera.is_dirty()
    }
    
    pub fn update_if_dirty(&mut self) -> Option<(f32, f32)> {
        if self.uniforms_dirty {
            // Always update camera matrices when view is dirty (ensures proper initialization)
            self.camera.update_matrices();
            
            // Update uniforms with the combined matrix
            let view_proj_matrix = self.camera.get_view_proj_matrix();
            self.uniforms.update_view_proj(view_proj_matrix);
            
            // Extract frustum for culling
            self.current_frustum = Some(Frustum::from_view_proj_matrix(view_proj_matrix));
            
            // Perform frustum culling
            self.perform_frustum_culling();
            
            // Update instance data
            self.update_instance_data();
            
            self.uniforms_dirty = false;
            
            // Return camera rotation for dependent views
            let camera_state = self.camera.get_state();
            Some((camera_state.3, camera_state.4)) // rotation_x, rotation_y
        } else {
            None
        }
    }
    
    pub fn get_uniforms(&self) -> &Uniforms {
        &self.uniforms
    }
    
    pub fn get_instance_data(&self) -> &[InstanceData] {
        &self.instance_data
    }
    
    pub fn mark_dirty(&mut self) {
        self.uniforms_dirty = true;
    }
    
    pub fn add_object(&mut self, x: f32, y: f32, z: f32, radius: f32) {
        let position = Point3::new(x, y, z);
        let object = RenderableObject::new(position, radius * 2.0); // size = diameter
        self.objects.push(object);
        self.total_objects = self.objects.len() as u32;
        self.mark_dirty(); // Ensure view updates on next render
    }
    
    pub fn clear_objects(&mut self) {
        self.objects.clear();
        self.total_objects = 0;
        self.visible_objects = 0;
        self.mark_dirty(); // Ensure view updates on next render
    }
    
    pub fn create_grid_objects(&mut self, grid_size: u32) {
        self.clear_objects();
        
        let grid_size = grid_size as i32;
        let cube_diameter = 1.0f32 / grid_size as f32;
        let cube_size = cube_diameter / 2.0f32; // radius = diameter / 2
        let spacing = cube_diameter + cube_size * 3.0f32;
        
        // Create grid positions centered around origin
        for i in 0..grid_size {
            for j in 0..grid_size {
                for k in 0..grid_size {
                    let x = if grid_size == 1 {
                        0.0f32
                    } else {
                        (i as f32 - (grid_size as f32 - 1.0f32) / 2.0f32) * spacing
                    };
                    let y = if grid_size == 1 {
                        0.0f32
                    } else {
                        (j as f32 - (grid_size as f32 - 1.0f32) / 2.0f32) * spacing
                    };
                    let z = if grid_size == 1 {
                        0.0f32
                    } else {
                        (k as f32 - (grid_size as f32 - 1.0f32) / 2.0f32) * spacing
                    };
                    
                    let position = Point3::new(x, y, z);
                    self.objects.push(RenderableObject::new(position, cube_size));
                }
            }
        }
        
        self.total_objects = self.objects.len() as u32;
        self.mark_dirty(); // Ensure view updates on next render
    }
    
    fn perform_frustum_culling(&mut self) {
        if let Some(ref frustum) = self.current_frustum {
            self.visible_objects = 0;
            
            for object in &mut self.objects {
                object.visible = frustum.contains_sphere(
                    object.bounding_sphere.center,
                    object.bounding_sphere.radius,
                );
                
                if object.visible {
                    self.visible_objects += 1;
                }
            }
        } else {
            // No frustum available, mark all as visible
            self.visible_objects = self.total_objects;
            for object in &mut self.objects {
                object.visible = true;
            }
        }
    }

    fn update_instance_data(&mut self) {
        // Clear previous instance data
        self.instance_data.clear();
        
        // Populate instance data from visible objects only
        for object in &self.objects {
            if object.visible {
                let color = if object.position.x == 0.0 && object.position.y == 0.0 && object.position.z == 0.0 {
                    // Default cube at origin gets beautiful purple-pink color
                    [0.8, 0.2, 0.8]
                } else {
                    // Other objects get height-based coloring
                    match object.position.y {
                        y if y > 2.0 => [1.0, 0.2, 0.2], // Red for high objects
                        y if y < -2.0 => [0.2, 0.2, 1.0], // Blue for low objects
                        _ => [0.2, 1.0, 0.2], // Green for middle objects
                    }
                };
                
                self.instance_data.push(InstanceData::new(
                    [object.position.x, object.position.y, object.position.z],
                    color,
                    object.bounding_sphere.radius,
                ));
            }
        }
    }
} 