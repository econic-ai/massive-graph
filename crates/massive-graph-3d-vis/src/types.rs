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
]; 