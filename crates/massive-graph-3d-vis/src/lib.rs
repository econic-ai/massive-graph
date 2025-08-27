mod utils;
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
} 