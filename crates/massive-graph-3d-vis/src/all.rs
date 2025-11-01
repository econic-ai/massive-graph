use cgmath::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, view_proj: cgmath::Matrix4<f32>) {
        self.view_proj = view_proj.into();
    }
}

// Cube vertices with colors
pub const VERTICES: &[Vertex] = &[
    // Front face (red)
    Vertex { position: [-1.0, -1.0,  1.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [ 1.0, -1.0,  1.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-1.0,  1.0,  1.0], color: [1.0, 0.0, 0.0] },
    
    // Back face (green)
    Vertex { position: [-1.0, -1.0, -1.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [ 1.0, -1.0, -1.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [-1.0,  1.0, -1.0], color: [0.0, 1.0, 0.0] },
];

pub const INDICES: &[u16] = &[
    // Front face
    0, 1, 2,  2, 3, 0,
    // Back face
    4, 6, 5,  6, 4, 7,
    // Left face
    4, 0, 3,  3, 7, 4,
    // Right face
    1, 5, 6,  6, 2, 1,
    // Top face
    3, 2, 6,  6, 7, 3,
    // Bottom face
    4, 5, 1,  1, 0, 4,
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub position: [f32; 3],
    pub color: [f32; 3], 
    pub scale: f32,
    pub _padding: f32, // Ensure 16-byte alignment for GPU
}

impl InstanceData {
    pub fn new(position: [f32; 3], color: [f32; 3], scale: f32) -> Self {
        Self {
            position,
            color,
            scale,
            _padding: 0.0,
        }
    }
    
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Instance position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Instance color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Instance scale
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

// Combined gizmo geometry - cylindrical axis lines with arrow tips
pub const GIZMO_VERTICES: &[Vertex] = &[
    // X-axis cylindrical line (red) - 8 vertices forming octagonal cross-section
    Vertex { position: [-1.2,  0.03, 0.0], color: [1.0, 0.0, 0.0] },  // 0
    Vertex { position: [-1.2,  0.02, 0.02], color: [1.0, 0.0, 0.0] },  // 1
    Vertex { position: [-1.2,  0.0, 0.03], color: [1.0, 0.0, 0.0] },   // 2
    Vertex { position: [-1.2, -0.02, 0.02], color: [1.0, 0.0, 0.0] },  // 3
    Vertex { position: [-1.2, -0.03, 0.0], color: [1.0, 0.0, 0.0] },   // 4
    Vertex { position: [-1.2, -0.02, -0.02], color: [1.0, 0.0, 0.0] }, // 5
    Vertex { position: [-1.2,  0.0, -0.03], color: [1.0, 0.0, 0.0] },  // 6
    Vertex { position: [-1.2,  0.02, -0.02], color: [1.0, 0.0, 0.0] }, // 7
    
    Vertex { position: [ 1.2,  0.03, 0.0], color: [1.0, 0.0, 0.0] },   // 8
    Vertex { position: [ 1.2,  0.02, 0.02], color: [1.0, 0.0, 0.0] },  // 9
    Vertex { position: [ 1.2,  0.0, 0.03], color: [1.0, 0.0, 0.0] },   // 10
    Vertex { position: [ 1.2, -0.02, 0.02], color: [1.0, 0.0, 0.0] },  // 11
    Vertex { position: [ 1.2, -0.03, 0.0], color: [1.0, 0.0, 0.0] },   // 12
    Vertex { position: [ 1.2, -0.02, -0.02], color: [1.0, 0.0, 0.0] }, // 13
    Vertex { position: [ 1.2,  0.0, -0.03], color: [1.0, 0.0, 0.0] },  // 14
    Vertex { position: [ 1.2,  0.02, -0.02], color: [1.0, 0.0, 0.0] }, // 15
    
    // Y-axis cylindrical line (green) - 8 vertices forming octagonal cross-section
    Vertex { position: [ 0.03, -1.2, 0.0], color: [0.0, 1.0, 0.0] },   // 16
    Vertex { position: [ 0.02, -1.2, 0.02], color: [0.0, 1.0, 0.0] },  // 17
    Vertex { position: [ 0.0, -1.2, 0.03], color: [0.0, 1.0, 0.0] },   // 18
    Vertex { position: [-0.02, -1.2, 0.02], color: [0.0, 1.0, 0.0] },  // 19
    Vertex { position: [-0.03, -1.2, 0.0], color: [0.0, 1.0, 0.0] },   // 20
    Vertex { position: [-0.02, -1.2, -0.02], color: [0.0, 1.0, 0.0] }, // 21
    Vertex { position: [ 0.0, -1.2, -0.03], color: [0.0, 1.0, 0.0] },  // 22
    Vertex { position: [ 0.02, -1.2, -0.02], color: [0.0, 1.0, 0.0] }, // 23
    
    Vertex { position: [ 0.03,  1.2, 0.0], color: [0.0, 1.0, 0.0] },   // 24
    Vertex { position: [ 0.02,  1.2, 0.02], color: [0.0, 1.0, 0.0] },  // 25
    Vertex { position: [ 0.0,  1.2, 0.03], color: [0.0, 1.0, 0.0] },   // 26
    Vertex { position: [-0.02,  1.2, 0.02], color: [0.0, 1.0, 0.0] },  // 27
    Vertex { position: [-0.03,  1.2, 0.0], color: [0.0, 1.0, 0.0] },   // 28
    Vertex { position: [-0.02,  1.2, -0.02], color: [0.0, 1.0, 0.0] }, // 29
    Vertex { position: [ 0.0,  1.2, -0.03], color: [0.0, 1.0, 0.0] },  // 30
    Vertex { position: [ 0.02,  1.2, -0.02], color: [0.0, 1.0, 0.0] }, // 31
    
    // Z-axis cylindrical line (orange) - 8 vertices forming octagonal cross-section
    Vertex { position: [ 0.03, 0.0, -1.2], color: [1.0, 0.5, 0.0] },   // 32
    Vertex { position: [ 0.02, 0.02, -1.2], color: [1.0, 0.5, 0.0] },  // 33
    Vertex { position: [ 0.0, 0.03, -1.2], color: [1.0, 0.5, 0.0] },   // 34
    Vertex { position: [-0.02, 0.02, -1.2], color: [1.0, 0.5, 0.0] },  // 35
    Vertex { position: [-0.03, 0.0, -1.2], color: [1.0, 0.5, 0.0] },   // 36
    Vertex { position: [-0.02, -0.02, -1.2], color: [1.0, 0.5, 0.0] }, // 37
    Vertex { position: [ 0.0, -0.03, -1.2], color: [1.0, 0.5, 0.0] },  // 38
    Vertex { position: [ 0.02, -0.02, -1.2], color: [1.0, 0.5, 0.0] }, // 39
    
    Vertex { position: [ 0.03, 0.0,  1.2], color: [1.0, 0.5, 0.0] },   // 40
    Vertex { position: [ 0.02, 0.02,  1.2], color: [1.0, 0.5, 0.0] },  // 41
    Vertex { position: [ 0.0, 0.03,  1.2], color: [1.0, 0.5, 0.0] },   // 42
    Vertex { position: [-0.02, 0.02,  1.2], color: [1.0, 0.5, 0.0] },  // 43
    Vertex { position: [-0.03, 0.0,  1.2], color: [1.0, 0.5, 0.0] },   // 44
    Vertex { position: [-0.02, -0.02,  1.2], color: [1.0, 0.5, 0.0] }, // 45
    Vertex { position: [ 0.0, -0.03,  1.2], color: [1.0, 0.5, 0.0] },  // 46
    Vertex { position: [ 0.02, -0.02,  1.2], color: [1.0, 0.5, 0.0] }, // 47
    
    // X-axis positive arrow cone (red) at +X - larger octagonal base
    Vertex { position: [ 1.0,  0.15, 0.0], color: [1.0, 0.0, 0.0] },    // 48
    Vertex { position: [ 1.0,  0.11, 0.11], color: [1.0, 0.0, 0.0] },   // 49
    Vertex { position: [ 1.0,  0.0, 0.15], color: [1.0, 0.0, 0.0] },    // 50
    Vertex { position: [ 1.0, -0.11, 0.11], color: [1.0, 0.0, 0.0] },   // 51
    Vertex { position: [ 1.0, -0.15, 0.0], color: [1.0, 0.0, 0.0] },    // 52
    Vertex { position: [ 1.0, -0.11, -0.11], color: [1.0, 0.0, 0.0] },  // 53
    Vertex { position: [ 1.0,  0.0, -0.15], color: [1.0, 0.0, 0.0] },   // 54
    Vertex { position: [ 1.0,  0.11, -0.11], color: [1.0, 0.0, 0.0] },  // 55
    Vertex { position: [ 1.3, 0.0, 0.0], color: [1.0, 0.0, 0.0] },      // 56 - tip
    
    // X-axis negative arrow cone (red) at -X - larger octagonal base
    Vertex { position: [-1.0,  0.15, 0.0], color: [1.0, 0.0, 0.0] },    // 57
    Vertex { position: [-1.0,  0.11, 0.11], color: [1.0, 0.0, 0.0] },   // 58
    Vertex { position: [-1.0,  0.0, 0.15], color: [1.0, 0.0, 0.0] },    // 59
    Vertex { position: [-1.0, -0.11, 0.11], color: [1.0, 0.0, 0.0] },   // 60
    Vertex { position: [-1.0, -0.15, 0.0], color: [1.0, 0.0, 0.0] },    // 61
    Vertex { position: [-1.0, -0.11, -0.11], color: [1.0, 0.0, 0.0] },  // 62
    Vertex { position: [-1.0,  0.0, -0.15], color: [1.0, 0.0, 0.0] },   // 63
    Vertex { position: [-1.0,  0.11, -0.11], color: [1.0, 0.0, 0.0] },  // 64
    Vertex { position: [-1.3, 0.0, 0.0], color: [1.0, 0.0, 0.0] },      // 65 - tip
    
    // Y-axis positive arrow cone (green) at +Y - larger octagonal base
    Vertex { position: [ 0.15,  1.0, 0.0], color: [0.0, 1.0, 0.0] },    // 66
    Vertex { position: [ 0.11,  1.0, 0.11], color: [0.0, 1.0, 0.0] },   // 67
    Vertex { position: [ 0.0,  1.0, 0.15], color: [0.0, 1.0, 0.0] },    // 68
    Vertex { position: [-0.11,  1.0, 0.11], color: [0.0, 1.0, 0.0] },   // 69
    Vertex { position: [-0.15,  1.0, 0.0], color: [0.0, 1.0, 0.0] },    // 70
    Vertex { position: [-0.11,  1.0, -0.11], color: [0.0, 1.0, 0.0] },  // 71
    Vertex { position: [ 0.0,  1.0, -0.15], color: [0.0, 1.0, 0.0] },   // 72
    Vertex { position: [ 0.11,  1.0, -0.11], color: [0.0, 1.0, 0.0] },  // 73
    Vertex { position: [0.0,  1.3, 0.0], color: [0.0, 1.0, 0.0] },      // 74 - tip
    
    // Y-axis negative arrow cone (green) at -Y - larger octagonal base
    Vertex { position: [ 0.15, -1.0, 0.0], color: [0.0, 1.0, 0.0] },    // 75
    Vertex { position: [ 0.11, -1.0, 0.11], color: [0.0, 1.0, 0.0] },   // 76
    Vertex { position: [ 0.0, -1.0, 0.15], color: [0.0, 1.0, 0.0] },    // 77
    Vertex { position: [-0.11, -1.0, 0.11], color: [0.0, 1.0, 0.0] },   // 78
    Vertex { position: [-0.15, -1.0, 0.0], color: [0.0, 1.0, 0.0] },    // 79
    Vertex { position: [-0.11, -1.0, -0.11], color: [0.0, 1.0, 0.0] },  // 80
    Vertex { position: [ 0.0, -1.0, -0.15], color: [0.0, 1.0, 0.0] },   // 81
    Vertex { position: [ 0.11, -1.0, -0.11], color: [0.0, 1.0, 0.0] },  // 82
    Vertex { position: [0.0, -1.3, 0.0], color: [0.0, 1.0, 0.0] },      // 83 - tip
    
    // Z-axis positive arrow cone (orange) at +Z - larger octagonal base
    Vertex { position: [ 0.15, 0.0,  1.0], color: [1.0, 0.5, 0.0] },    // 84
    Vertex { position: [ 0.11, 0.11,  1.0], color: [1.0, 0.5, 0.0] },   // 85
    Vertex { position: [ 0.0, 0.15,  1.0], color: [1.0, 0.5, 0.0] },    // 86
    Vertex { position: [-0.11, 0.11,  1.0], color: [1.0, 0.5, 0.0] },   // 87
    Vertex { position: [-0.15, 0.0,  1.0], color: [1.0, 0.5, 0.0] },    // 88
    Vertex { position: [-0.11, -0.11,  1.0], color: [1.0, 0.5, 0.0] },  // 89
    Vertex { position: [ 0.0, -0.15,  1.0], color: [1.0, 0.5, 0.0] },   // 90
    Vertex { position: [ 0.11, -0.11,  1.0], color: [1.0, 0.5, 0.0] },  // 91
    Vertex { position: [0.0, 0.0,  1.3], color: [1.0, 0.5, 0.0] },      // 92 - tip
    
    // Z-axis negative arrow cone (orange) at -Z - larger octagonal base
    Vertex { position: [ 0.15, 0.0, -1.0], color: [1.0, 0.5, 0.0] },    // 93
    Vertex { position: [ 0.11, 0.11, -1.0], color: [1.0, 0.5, 0.0] },   // 94
    Vertex { position: [ 0.0, 0.15, -1.0], color: [1.0, 0.5, 0.0] },    // 95
    Vertex { position: [-0.11, 0.11, -1.0], color: [1.0, 0.5, 0.0] },   // 96
    Vertex { position: [-0.15, 0.0, -1.0], color: [1.0, 0.5, 0.0] },    // 97
    Vertex { position: [-0.11, -0.11, -1.0], color: [1.0, 0.5, 0.0] },  // 98
    Vertex { position: [ 0.0, -0.15, -1.0], color: [1.0, 0.5, 0.0] },   // 99
    Vertex { position: [ 0.11, -0.11, -1.0], color: [1.0, 0.5, 0.0] },  // 100
    Vertex { position: [0.0, 0.0, -1.3], color: [1.0, 0.5, 0.0] },      // 101 - tip
];

pub const GIZMO_INDICES: &[u16] = &[
    // X-axis cylindrical line (red) - connect octagonal faces
    // Side triangles connecting front and back octagons
    0, 8, 1,   1, 8, 9,   1, 9, 2,   2, 9, 10,   2, 10, 3,   3, 10, 11,
    3, 11, 4,  4, 11, 12, 4, 12, 5,  5, 12, 13,  5, 13, 6,   6, 13, 14,
    6, 14, 7,  7, 14, 15, 7, 15, 0,  0, 15, 8,
    
    // Y-axis cylindrical line (green) - connect octagonal faces
    16, 24, 17,  17, 24, 25,  17, 25, 18,  18, 25, 26,  18, 26, 19,  19, 26, 27,
    19, 27, 20,  20, 27, 28,  20, 28, 21,  21, 28, 29,  21, 29, 22,  22, 29, 30,
    22, 30, 23,  23, 30, 31,  23, 31, 16,  16, 31, 24,
    
    // Z-axis cylindrical line (orange) - connect octagonal faces
    32, 40, 33,  33, 40, 41,  33, 41, 34,  34, 41, 42,  34, 42, 35,  35, 42, 43,
    35, 43, 36,  36, 43, 44,  36, 44, 37,  37, 44, 45,  37, 45, 38,  38, 45, 46,
    38, 46, 39,  39, 46, 47,  39, 47, 32,  32, 47, 40,
    
    // Arrow cones (rendered as triangles) - octagonal cones with 8 faces each
    // X-axis positive arrow (vertices 48-56)
    48, 56, 49,  49, 56, 50,  50, 56, 51,  51, 56, 52,  52, 56, 53,  53, 56, 54,  54, 56, 55,  55, 56, 48,
    
    // X-axis negative arrow (vertices 57-65)
    57, 65, 58,  58, 65, 59,  59, 65, 60,  60, 65, 61,  61, 65, 62,  62, 65, 63,  63, 65, 64,  64, 65, 57,
    
    // Y-axis positive arrow (vertices 66-74)
    66, 74, 67,  67, 74, 68,  68, 74, 69,  69, 74, 70,  70, 74, 71,  71, 74, 72,  72, 74, 73,  73, 74, 66,
    
    // Y-axis negative arrow (vertices 75-83)
    75, 83, 76,  76, 83, 77,  77, 83, 78,  78, 83, 79,  79, 83, 80,  80, 83, 81,  81, 83, 82,  82, 83, 75,
    
    // Z-axis positive arrow (vertices 84-92)
    84, 92, 85,  85, 92, 86,  86, 92, 87,  87, 92, 88,  88, 92, 89,  89, 92, 90,  90, 92, 91,  91, 92, 84,
    
    // Z-axis negative arrow (vertices 93-101)
    93, 101, 94,  94, 101, 95,  95, 101, 96,  96, 101, 97,  97, 101, 98,  98, 101, 99,  99, 101, 100,  100, 101, 93,
]; use crate::camera::Camera;
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
} mod utils;
mod performance;
mod types;
mod camera;
mod math;
mod main_view;
mod gizmo_view;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, console};
use wgpu::util::DeviceExt;
use cgmath::Point3;

// Re-export from modules
pub use performance::{PerformanceSnapshot, PerformanceTracker, now};
pub use types::{Vertex, Uniforms, VERTICES, INDICES, InstanceData, GIZMO_VERTICES, GIZMO_INDICES};
pub use camera::Camera;
pub use math::{Frustum, BoundingSphere};
pub use main_view::{MainView, RenderableObject};
pub use gizmo_view::GizmoView;

#[wasm_bindgen]
pub struct CubeRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    width: u32,
    height: u32,
    render_pipeline: wgpu::RenderPipeline,
    gizmo_render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    
    // Gizmo geometry buffer
    gizmo_vertex_buffer: wgpu::Buffer,
    gizmo_index_buffer: wgpu::Buffer,
    gizmo_num_indices: u32,
    
    // Instance buffer for GPU instancing
    instance_buffer: wgpu::Buffer,
    instance_data: Vec<InstanceData>,
    max_instances: u32,
    
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    
    // Gizmo uniform buffer (separate from main view)
    gizmo_uniform_buffer: wgpu::Buffer,
    gizmo_uniform_bind_group: wgpu::BindGroup,
    
    // Depth buffer for 3D rendering
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    
    // View system
    main_view: MainView,
    gizmo_view: GizmoView,
    
    // Command buffer optimization - cache descriptors
    command_encoder_desc: wgpu::CommandEncoderDescriptor<'static>,
    texture_view_desc: wgpu::TextureViewDescriptor<'static>,
    
    // Background color
    background_color: wgpu::Color,
    
    // Performance tracking
    performance_tracker: PerformanceTracker,
    
    // Cached geometry metrics (updated only on scene changes)
    cached_object_count: u32,
    cached_edge_count: u32,
    cached_vertex_count: u32,
    cached_index_count: u32,
}

#[wasm_bindgen]
impl CubeRenderer {
    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        (depth_texture, depth_view)
    }

    fn parse_hex_color(hex: &str) -> Result<wgpu::Color, JsValue> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(JsValue::from_str("Invalid hex color format. Expected #RRGGBB"));
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| JsValue::from_str("Invalid hex color format"))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| JsValue::from_str("Invalid hex color format"))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| JsValue::from_str("Invalid hex color format"))?;
        
        Ok(wgpu::Color {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: 1.0,
        })
    }

    #[wasm_bindgen(constructor)]
    pub async fn new(canvas: HtmlCanvasElement) -> Result<CubeRenderer, JsValue> {
        Self::new_with_background(canvas, "#dddddd").await
    }

    #[wasm_bindgen]
    pub async fn new_force_webgl(canvas: HtmlCanvasElement) -> Result<CubeRenderer, JsValue> {
        Self::new_force_webgl_with_background(canvas, "#dddddd").await
    }

    #[wasm_bindgen]
    pub async fn new_with_background(canvas: HtmlCanvasElement, background_color: &str) -> Result<CubeRenderer, JsValue> {
        Self::new_with_backend_and_background(canvas, false, background_color).await
    }

    #[wasm_bindgen]
    pub async fn new_force_webgl_with_background(canvas: HtmlCanvasElement, background_color: &str) -> Result<CubeRenderer, JsValue> {
        Self::new_with_backend_and_background(canvas, true, background_color).await
    }

    async fn new_with_backend_and_background(canvas: HtmlCanvasElement, force_webgl: bool, background_color: &str) -> Result<CubeRenderer, JsValue> {
        utils::set_panic_hook();
        
        // Parse background color from hex string
        let bg_color = Self::parse_hex_color(background_color)?;
        
        console::log_1(&format!("Canvas width: {}, height: {}", canvas.width(), canvas.height()).into());

        let width = canvas.width();
        let height = canvas.height();

        if force_webgl {
            console::log_1(&"🔧 TESTING: Forcing WebGL backend".into());
            return Self::create_webgl_renderer(canvas, width, height, bg_color).await;
        }

        // Try WebGPU first, fall back to WebGL if it fails
        console::log_1(&"🚀 Attempting to use WebGPU backend...".into());
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions {
                gl: wgpu::GlBackendOptions {
                    gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
                    fence_behavior: wgpu::GlFenceBehavior::default(),
                },
                ..Default::default()
            },
        });

        // Clone canvas for potential fallback use
        let canvas_clone = canvas.clone();
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;

        let adapter_result = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await;

        let adapter = match adapter_result {
            Ok(adapter) => {
                let backend = adapter.get_info().backend;
                console::log_1(&format!("✅ Adapter acquired successfully using: {:?}", backend).into());
                adapter
            }
            Err(e) => {
                console::log_1(&format!("❌ WebGPU adapter request failed: {:?}", e).into());
                console::log_1(&"🔄 Falling back to WebGL...".into());
                
                // Use the reusable WebGL function for fallback
                return Self::create_webgl_renderer(canvas_clone, width, height, bg_color).await;
            }
        };

        let result = Self::create_with_adapter_and_surface(adapter, surface, width, height, bg_color).await?;
        
        Ok(result)
    }

    async fn create_webgl_renderer(
        canvas: HtmlCanvasElement,
        width: u32,
        height: u32,
        bg_color: wgpu::Color,
    ) -> Result<CubeRenderer, JsValue> {
        let webgl_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions {
                gl: wgpu::GlBackendOptions {
                    gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
                    fence_behavior: wgpu::GlFenceBehavior::default(),
                },
                ..Default::default()
            },
        });
        
        let webgl_surface = webgl_instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsValue::from_str(&format!("Failed to create WebGL surface: {:?}", e)))?;
        
        let webgl_adapter = webgl_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&webgl_surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to request WebGL adapter: {:?}", e)))?;
        
        console::log_1(&format!("✅ WebGL adapter acquired successfully using: {:?}", webgl_adapter.get_info().backend).into());
        
        Self::create_with_adapter_and_surface(webgl_adapter, webgl_surface, width, height, bg_color).await
    }

    async fn create_with_adapter_and_surface(
        adapter: wgpu::Adapter,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        bg_color: wgpu::Color,
    ) -> Result<CubeRenderer, JsValue> {
        // Log adapter information and limits
        let adapter_info = adapter.get_info();
        let adapter_limits = adapter.limits();
        console::log_1(&format!("📊 Adapter backend: {:?}", adapter_info.backend).into());
        console::log_1(&format!("📊 Adapter limits: max_buffer_size = {}MB", adapter_limits.max_buffer_size / (1024 * 1024)).into());

        // Choose appropriate limits based on backend and request maximum buffer size when possible
        let device_limits = match adapter_info.backend {
            wgpu::Backend::BrowserWebGpu => {
                console::log_1(&"🚀 Requesting maximum WebGPU limits with higher buffer size".into());
                let mut limits = wgpu::Limits::default();
                // Request the maximum buffer size supported by the adapter
                limits.max_buffer_size = adapter_limits.max_buffer_size;
                console::log_1(&format!("🚀 Requesting max_buffer_size = {}MB", limits.max_buffer_size / (1024 * 1024)).into());
                limits
            }
            wgpu::Backend::Gl => {
                console::log_1(&"🔧 Using WebGL2 downlevel limits".into());
                wgpu::Limits::downlevel_webgl2_defaults()
            }
            _ => {
                console::log_1(&"⚠️ Unknown backend, using default limits".into());
                wgpu::Limits::default()
            }
        };

        // Store the requested buffer size for comparison
        let requested_buffer_size = device_limits.max_buffer_size;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: device_limits,
                    memory_hints: wgpu::MemoryHints::default(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;

        console::log_1(&"✅ Device created successfully".into());
        
        // Log the actual device limits that were granted
        let actual_device_limits = device.limits();
        console::log_1(&format!("📊 Actual device limits: max_buffer_size = {}MB", 
            actual_device_limits.max_buffer_size / (1024 * 1024)).into());
        console::log_1(&format!("📊 Comparison: Requested={}MB, Granted={}MB", 
            requested_buffer_size / (1024 * 1024),
            actual_device_limits.max_buffer_size / (1024 * 1024)).into());

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create gizmo shader
        let gizmo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gizmo_shader.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), InstanceData::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create gizmo render pipeline with the gizmo shader
        let gizmo_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gizmo Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &gizmo_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gizmo_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indices = INDICES.len() as u32;

        console::log_1(&"🎉 CubeRenderer created successfully!".into());
        
        // Use 80% of max buffer size to leave room for other buffers and safety margin
        let max_safe_buffer_size = (actual_device_limits.max_buffer_size as f64 * 0.8) as u64;
        
        // Calculate max instances based on InstanceData size and actual device buffer limit
        let instance_data_size = std::mem::size_of::<InstanceData>() as u64;
        let max_instances = (max_safe_buffer_size / instance_data_size) as u32;
        
        // Log the calculated limits
        console::log_1(&format!(
            "📊 Instance buffer: max_size={}MB, instance_size={}bytes, max_instances={}", 
            max_safe_buffer_size / (1024 * 1024), 
            instance_data_size, 
            max_instances
        ).into());
        
        // Calculate maximum supported grid size for reference
        let max_grid_size = (max_instances as f64).cbrt().floor() as u32;
        console::log_1(&format!(
            "📏 Maximum supported grid: {}x{}x{} = {} cubes", 
            max_grid_size, max_grid_size, max_grid_size, max_grid_size.pow(3)
        ).into());
        
        // Create instance buffer for GPU instancing
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<InstanceData>() * max_instances as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create depth texture for 3D rendering
        let (depth_texture, depth_view) = Self::create_depth_texture(&device, width, height);

        // Create gizmo uniform buffer
        let gizmo_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let gizmo_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: gizmo_uniform_buffer.as_entire_binding(),
            }],
            label: Some("gizmo_uniform_bind_group"),
        });

        // Create gizmo geometry buffers
        let gizmo_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Vertex Buffer"),
            contents: bytemuck::cast_slice(GIZMO_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let gizmo_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Index Buffer"),
            contents: bytemuck::cast_slice(GIZMO_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let gizmo_num_indices = GIZMO_INDICES.len() as u32;

        // Create the renderer instance
        let renderer = Self {
            surface,
            device,
            queue,
            config,
            width,
            height,
            render_pipeline,
            gizmo_render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            background_color: bg_color,
            performance_tracker: PerformanceTracker::new(),
            command_encoder_desc: wgpu::CommandEncoderDescriptor {
                label: None,
                ..Default::default()
            },
            texture_view_desc: wgpu::TextureViewDescriptor::default(),
            depth_texture,
            depth_view,
            instance_buffer,
            instance_data: Vec::new(),
            max_instances,
            cached_object_count: 0,
            cached_edge_count: 0,
            cached_vertex_count: 0,
            cached_index_count: 0,
            main_view: MainView::new(width, height),
            gizmo_view: GizmoView::new(width, height),
            gizmo_uniform_buffer,
            gizmo_uniform_bind_group,
            gizmo_vertex_buffer,
            gizmo_index_buffer,
            gizmo_num_indices,
        };
        
        console::log_1(&"🎯 Renderer created with clean state (no default objects)".into());
        
        Ok(renderer)
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {           
            // Update dimensions
            self.width = width;
            self.height = height;
            self.config.width = width;
            self.config.height = height;
            
            // Reconfigure surface
            self.surface.configure(&self.device, &self.config);
            
            // Recreate depth texture for new size
            let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, width, height);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
            
            // Update view dimensions
            self.main_view.resize(width, height);
            self.gizmo_view.resize(width, height);
            
        }
    }

    #[wasm_bindgen]
    pub fn zoom(&mut self, delta: f32) {
        self.main_view.zoom(delta);
    }

    #[wasm_bindgen]
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.main_view.pan(delta_x, delta_y);
    }

    #[wasm_bindgen]
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.main_view.rotate(delta_x, delta_y);
    }
    
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        // Track that render() was called (for FPS calculation)
        self.performance_tracker.track_render_call();
        
        // Only render if something has actually changed
        if !self.main_view.is_dirty() && !self.gizmo_view.is_dirty() {
            return Ok(()); // Skip entire render cycle - previous frame stays visible
        }
        
        // Start tracking actual render work (dirty frame)
        self.performance_tracker.start_actual_render();
        
        // Update main view if dirty and get rotation for gizmo
        let main_rotation = self.main_view.update_if_dirty();
        
        // Update gizmo view from main camera rotation
        self.gizmo_view.update_from_main_camera(main_rotation);
        
        // Update gizmo view if dirty
        self.gizmo_view.update_if_dirty();
        
        // Update uniforms
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[*self.main_view.get_uniforms()]),
        );
        
        // Update gizmo uniform buffer with gizmo's view matrix
        self.queue.write_buffer(
            &self.gizmo_uniform_buffer,
            0,
            bytemuck::cast_slice(&[*self.gizmo_view.get_uniforms()]),
        );
        
        // Combine instance data from all views
        self.instance_data.clear();
        
        // Add main view instances
        self.instance_data.extend_from_slice(self.main_view.get_instance_data());
        
        // Update instance buffer with main view data only
        if !self.instance_data.is_empty() {
            let byte_data = bytemuck::cast_slice(&self.instance_data);
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                byte_data,
            );
        }
        
        // Render to GPU
        let output = self.surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&self.texture_view_desc);

        let mut encoder = self
            .device
            .create_command_encoder(&self.command_encoder_desc);

        // Render each view in its own viewport
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render main view with full viewport
            let (x, y, w, h) = self.main_view.get_viewport_region(self.width, self.height);
            render_pass.set_viewport(x, y, w, h, 0.0, 1.0);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.main_view.get_instance_data().len() as u32);
            
            // Render gizmo view if enabled
            if self.gizmo_view.is_enabled() {
                let (gx, gy, gw, gh) = self.gizmo_view.get_viewport_region(self.width, self.height);
                render_pass.set_viewport(gx, gy, gw, gh, 0.0, 1.0);
                render_pass.set_pipeline(&self.gizmo_render_pipeline);
                render_pass.set_bind_group(0, &self.gizmo_uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.gizmo_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.gizmo_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.gizmo_num_indices, 0, 0..1);
            }
        }

        // Submit command buffer
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // End performance tracking
        self.performance_tracker.end_actual_render();

        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_performance_snapshot(&mut self) -> Option<PerformanceSnapshot> {
        // Force create a snapshot for JavaScript to consume
        // This is called periodically from JavaScript, not every frame
        if self.performance_tracker.has_frames() {
            let now = now();
            let snapshot = self.performance_tracker.create_snapshot_with_renderer_data(
                now,
                self.cached_object_count,
                self.cached_edge_count,
                self.cached_vertex_count,
                self.cached_index_count,
                self.calculate_current_memory_usage(),
                self.calculate_scene_size_memory_bytes(),
                self.calculate_active_view_memory_bytes(),
                self.main_view.visible_objects,
            );
            
            Some(snapshot)
        } else {
            None
        }
    }

    #[wasm_bindgen]
    pub fn get_visible_objects(&self) -> u32 {
        self.main_view.visible_objects
    }
    
    #[wasm_bindgen]
    pub fn get_total_objects(&self) -> u32 {
        self.main_view.total_objects
    }
    
    #[wasm_bindgen]
    pub fn get_culling_ratio(&self) -> f32 {
        let total = self.main_view.total_objects;
        let visible = self.main_view.visible_objects;
        if total > 0 {
            (total - visible) as f32 / total as f32
        } else {
            0.0
        }
    }

    #[wasm_bindgen]
    pub fn create_test_objects(&mut self, count: u32) {
        self.main_view.clear_objects();
        
        // Create a grid of cubes for testing frustum culling
        let grid_size = (count as f32).cbrt().ceil() as i32;
        let spacing = 3.0;
        let offset = (grid_size as f32 - 1.0) * spacing * 0.5;
        
        for x in 0..grid_size {
            for y in 0..grid_size {
                for z in 0..grid_size {
                    if self.main_view.objects.len() >= count as usize {
                        break;
                    }
                    
                    let position = Point3::new(
                        x as f32 * spacing - offset,
                        y as f32 * spacing - offset,
                        z as f32 * spacing - offset,
                    );
                    
                    self.main_view.objects.push(RenderableObject::new(position, 2.0));
                }
                if self.main_view.objects.len() >= count as usize {
                    break;
                }
            }
            if self.main_view.objects.len() >= count as usize {
                break;
            }
        }
        
        self.main_view.total_objects = self.main_view.objects.len() as u32;
        self.main_view.mark_dirty(); // Mark view as dirty to trigger update
        
        let total_objects = self.main_view.objects.len() as u32;
        console::log_1(&format!("Created {} test objects for frustum culling", total_objects).into());
    }
    
    #[wasm_bindgen]
    pub fn add_object(&mut self, x: f32, y: f32, z: f32, radius: f32) {
        self.main_view.add_object(x, y, z, radius);
        
        // Update cached geometry metrics after scene change
        self.update_geometry_metrics();
    }
    
    #[wasm_bindgen]
    pub fn enable_instancing_demo_with_size(&mut self, grid_size: u32) {
        // Create a grid of colorful cubes to demonstrate instancing
        self.main_view.clear_objects();
        
        let grid_size = grid_size as i32;
        
        // Calculate total number of cubes needed
        let total_cubes = (grid_size as u64).pow(3);
        
        // Check if we exceed our buffer capacity
        if total_cubes > self.max_instances as u64 {
            let max_grid_size = (self.max_instances as f64).cbrt().floor() as u32;
            console::log_1(&format!("❌ ERROR: {}x{}x{} grid needs {} cubes, but buffer supports max {} cubes", 
                grid_size, grid_size, grid_size, total_cubes, self.max_instances).into());
            console::log_1(&format!("📏 Maximum supported grid size: {}x{}x{} = {} cubes", 
                max_grid_size, max_grid_size, max_grid_size, max_grid_size.pow(3)).into());
            
            // Fall back to maximum safe grid size
            let safe_grid_size = max_grid_size as i32;
            console::log_1(&format!("🔧 Using safe grid size: {}x{}x{}", safe_grid_size, safe_grid_size, safe_grid_size).into());
            self.enable_instancing_demo_with_size(safe_grid_size as u32);
            return;
        }
        
        // Simple logic: total space is always 1.0 unit
        // For N cubes along an axis: each cube diameter = 1.0/N
        let cube_diameter = 1.0f32 / grid_size as f32;
        let cube_size = cube_diameter / 2.0f32; // radius = diameter / 2
        
        // Spacing between cube centers = cube diameter (touching cubes, no gaps)
        let spacing = cube_diameter + cube_size * 3.0f32;
        
        // Create grid positions centered around origin
        for i in 0..grid_size {
            for j in 0..grid_size {
                for k in 0..grid_size {
                    // Convert grid indices to centered positions
                    let x = if grid_size == 1 {
                        0.0f32 // Single cube at origin
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
                    self.main_view.objects.push(RenderableObject::new(position, cube_size));
                }
            }
        }
        
        self.main_view.total_objects = self.main_view.objects.len() as u32;
        self.main_view.mark_dirty(); // Mark view as dirty to trigger update
        
        let total_spread = grid_size as f32 * cube_diameter;
        console::log_1(&format!("🎨 Instancing demo: {}x{}x{} grid = {} cubes (cube_diameter: {:.3}, cube_radius: {:.3}, total_spread: {:.3})", 
            grid_size, grid_size, grid_size, self.main_view.total_objects, cube_diameter, cube_size, total_spread).into());
        
        // Update cached geometry metrics after scene change
        self.update_geometry_metrics();
    }
    
    // Update cached geometry metrics - call only when scene changes
    fn update_geometry_metrics(&mut self) {
        self.cached_object_count = self.main_view.objects.len() as u32;
        
        // For current cube-based system
        if !self.main_view.objects.is_empty() {
            self.cached_vertex_count = (self.main_view.objects.len() as u32) * (VERTICES.len() as u32);
            self.cached_index_count = (self.main_view.objects.len() as u32) * (INDICES.len() as u32);
            self.cached_edge_count = (self.main_view.objects.len() as u32) * 12; // 12 edges per cube
        } else {
            self.cached_vertex_count = 0;
            self.cached_index_count = 0; 
            self.cached_edge_count = 0;
        }
        
        console::log_1(&format!("📐 Geometry metrics updated: {} objects, {} vertices, {} indices, {} edges", 
            self.cached_object_count, self.cached_vertex_count, self.cached_index_count, self.cached_edge_count).into());
    }
    
    // Calculate current GPU memory usage in bytes
    fn calculate_current_memory_usage(&self) -> u64 {
        let vertex_buffer_size = std::mem::size_of_val(VERTICES) as u64;
        let index_buffer_size = std::mem::size_of_val(INDICES) as u64;
        let instance_buffer_size = (std::mem::size_of::<InstanceData>() * self.max_instances as usize) as u64;
        let uniform_buffer_size = std::mem::size_of::<Uniforms>() as u64;
        let depth_texture_size = (self.width * self.height * 4) as u64; // 32-bit depth = 4 bytes per pixel
        
        vertex_buffer_size + index_buffer_size + instance_buffer_size + uniform_buffer_size + depth_texture_size
    }
    
    // Calculate total scene memory (all objects, regardless of visibility)
    fn calculate_scene_size_memory_bytes(&self) -> u64 {
        if self.main_view.total_objects == 0 {
            return 0;
        }
        
        // Per-object memory calculation
        let instance_data_per_object = std::mem::size_of::<InstanceData>() as u64;
        
        // The vertex and index buffers are shared across all objects, so we only scale instance data
        let shared_geometry_memory = std::mem::size_of_val(VERTICES) as u64 + std::mem::size_of_val(INDICES) as u64;
        let instance_memory = (self.main_view.total_objects as u64) * instance_data_per_object;
        
        shared_geometry_memory + instance_memory
    }
    
    // Calculate active view memory (visible objects only, post-culling)
    fn calculate_active_view_memory_bytes(&self) -> u64 {
        if self.main_view.visible_objects == 0 {
            return 0;
        }
        
        // Per-visible-object memory calculation
        let instance_data_per_object = std::mem::size_of::<InstanceData>() as u64;
        
        // The vertex and index buffers are still shared, but we only count visible instance data
        let shared_geometry_memory = if self.main_view.visible_objects > 0 { 
            std::mem::size_of_val(VERTICES) as u64 + std::mem::size_of_val(INDICES) as u64 
        } else { 
            0 
        };
        let visible_instance_memory = (self.main_view.visible_objects as u64) * instance_data_per_object;
        
        shared_geometry_memory + visible_instance_memory
    }

    // Gizmo control methods
    #[wasm_bindgen]
    pub fn enable_gizmo(&mut self) {
        self.gizmo_view.enable();
        // Force immediate re-render by marking uniforms dirty
        self.gizmo_view.mark_dirty();
    }
    
    #[wasm_bindgen]
    pub fn disable_gizmo(&mut self) {
        self.gizmo_view.disable();
        // Force immediate re-render by marking uniforms dirty
        self.gizmo_view.mark_dirty();
    }
    
    #[wasm_bindgen]
    pub fn is_gizmo_enabled(&self) -> bool {
        let enabled = self.gizmo_view.is_enabled();
        enabled
    }
} use cgmath::{Vector3, Matrix4, Point3, InnerSpace, EuclideanSpace};

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vector3<f32>,
    pub distance: f32,
}

impl Plane {
    pub fn new(normal: Vector3<f32>, distance: f32) -> Self {
        Self { normal, distance }
    }
    
    pub fn from_point_normal(point: Point3<f32>, normal: Vector3<f32>) -> Self {
        let normalized = normal.normalize();
        let distance = -normalized.dot(point.to_vec());
        Self { normal: normalized, distance }
    }
    
    // Distance from point to plane (positive = in front, negative = behind)
    pub fn distance_to_point(&self, point: Point3<f32>) -> f32 {
        self.normal.dot(point.to_vec()) + self.distance
    }
}

#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Plane; 6], // left, right, bottom, top, near, far
}

impl Frustum {
    // Extract frustum planes from view-projection matrix
    pub fn from_view_proj_matrix(view_proj: Matrix4<f32>) -> Self {
        // Extract frustum planes using Gribb-Hartmann method
        // Each plane is extracted from the view-projection matrix rows
        
        let m = view_proj;
        
        // Left plane: m[3] + m[0]
        let left = Plane::new(
            Vector3::new(m[3][0] + m[0][0], m[3][1] + m[0][1], m[3][2] + m[0][2]).normalize(),
            m[3][3] + m[0][3]
        );
        
        // Right plane: m[3] - m[0]  
        let right = Plane::new(
            Vector3::new(m[3][0] - m[0][0], m[3][1] - m[0][1], m[3][2] - m[0][2]).normalize(),
            m[3][3] - m[0][3]
        );
        
        // Bottom plane: m[3] + m[1]
        let bottom = Plane::new(
            Vector3::new(m[3][0] + m[1][0], m[3][1] + m[1][1], m[3][2] + m[1][2]).normalize(),
            m[3][3] + m[1][3]
        );
        
        // Top plane: m[3] - m[1]
        let top = Plane::new(
            Vector3::new(m[3][0] - m[1][0], m[3][1] - m[1][1], m[3][2] - m[1][2]).normalize(),
            m[3][3] - m[1][3]
        );
        
        // Near plane: m[3] + m[2]
        let near = Plane::new(
            Vector3::new(m[3][0] + m[2][0], m[3][1] + m[2][1], m[3][2] + m[2][2]).normalize(),
            m[3][3] + m[2][3]
        );
        
        // Far plane: m[3] - m[2]
        let far = Plane::new(
            Vector3::new(m[3][0] - m[2][0], m[3][1] - m[2][1], m[3][2] - m[2][2]).normalize(),
            m[3][3] - m[2][3]
        );
        
        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }
    
    // Test if a sphere is inside the frustum
    pub fn contains_sphere(&self, center: Point3<f32>, radius: f32) -> bool {
        for plane in &self.planes {
            let distance = plane.distance_to_point(center);
            if distance < -radius {
                // Sphere is completely outside this plane
                return false;
            }
        }
        // Sphere is inside or intersecting all planes
        true
    }
    
    // Test if an axis-aligned bounding box is inside the frustum
    pub fn contains_aabb(&self, min: Point3<f32>, max: Point3<f32>) -> bool {
        for plane in &self.planes {
            // Test all 8 corners of the AABB against the plane
            let corners = [
                Point3::new(min.x, min.y, min.z),
                Point3::new(max.x, min.y, min.z),
                Point3::new(min.x, max.y, min.z),
                Point3::new(max.x, max.y, min.z),
                Point3::new(min.x, min.y, max.z),
                Point3::new(max.x, min.y, max.z),
                Point3::new(min.x, max.y, max.z),
                Point3::new(max.x, max.y, max.z),
            ];
            
            let mut all_outside = true;
            for corner in &corners {
                if plane.distance_to_point(*corner) >= 0.0 {
                    all_outside = false;
                    break;
                }
            }
            
            if all_outside {
                // All corners are outside this plane
                return false;
            }
        }
        // AABB intersects or is inside all planes
        true
    }
}

// Simple bounding sphere for objects
#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Point3<f32>,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(center: Point3<f32>, radius: f32) -> Self {
        Self { center, radius }
    }
    
    pub fn from_array(center: [f32; 3], radius: f32) -> Self {
        Self {
            center: Point3::new(center[0], center[1], center[2]),
            radius,
        }
    }
    
    // Create bounding sphere for a cube at given position with given size
    pub fn for_cube(position: Point3<f32>, size: f32) -> Self {
        // Cube extends from -size/2 to +size/2 in each direction
        // Sphere radius is the distance from center to corner
        let radius = (size * size * 3.0).sqrt() * 1.0; // sqrt(3) * size/2
        Self::new(position, radius)
    }
} use wasm_bindgen::prelude::*;
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
} use crate::camera::Camera;
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
} // Vertex shader

struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct InstanceInput {
    @location(2) instance_position: vec3<f32>,
    @location(3) instance_color: vec3<f32>,
    @location(4) instance_scale: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Use vertex color to choose between purple and pink faces for beautiful 3D shading
    if (model.color.r > 0.5) {
        // Red vertex (front face) -> Purple
        out.color = vec3<f32>(0.6, 0.2, 0.8); // Purple
    } else if (model.color.g > 0.5) {
        // Green vertex (back face) -> Pink
        out.color = vec3<f32>(1.0, 0.4, 0.7); // Pink
    } else {
        // Other faces -> Mix of purple and pink
        out.color = vec3<f32>(0.8, 0.3, 0.9); // Light purple-pink
    }
    
    // Scale vertex position by instance scale, then translate by instance position
    let world_position = (model.position * instance.instance_scale) + instance.instance_position;
    out.clip_position = uniforms.view_proj * vec4<f32>(world_position, 1.0);
    
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
} use wasm_bindgen::prelude::*;

// When the `console_error_panic_hook` feature is enabled, we can call the
// `set_panic_hook` function at least once during initialization, and then
// we will get better error messages if our code ever panics.
//
// For more details see
// https://github.com/rustwasm/console_error_panic_hook#readme
#[cfg(feature = "console_error_panic_hook")]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(not(feature = "console_error_panic_hook"))]
pub fn set_panic_hook() {
    // No-op when the feature is not enabled
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ( $( $t:tt )* ) => {
        log(&format!( $( $t )* ))
    }
} // Gizmo-specific vertex shader with flat colors (no instancing)

struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Use vertex color directly for clean, flat gizmo appearance
    out.color = model.color;
    
    // Transform vertex position directly (no instance scaling/translation)
    out.clip_position = uniforms.view_proj * vec4<f32>(model.position, 1.0);
    
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
} use cgmath::prelude::*;
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