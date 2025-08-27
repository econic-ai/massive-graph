// Vertex shader

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
} 