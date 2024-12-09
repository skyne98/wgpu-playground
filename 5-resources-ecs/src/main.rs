use anyhow::Result;
use pipeline::{GPUPipeline, GPUPipelineBuilder};
use pollster::FutureExt;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use tracing_tracy::client::{frame_name, ProfiledAllocator};
use vertex::{DepthVertex, Vertex, DEPTH_VERTICES, VERTICES};
use wgpu::{
    util::DeviceExt, Adapter, Device, Instance, Queue, RenderPipeline, Surface, SurfaceCapabilities,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[global_allocator]
static GLOBAL: ProfiledAllocator<std::alloc::System> =
    ProfiledAllocator::new(std::alloc::System, 100);

mod pipeline;
mod texture;
mod uniform;
mod vertex;

// GPU Context handling
struct GpuContext<'a> {
    device: Device,
    queue: Queue,
    surface: Surface<'a>,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    depth_sampler: wgpu::Sampler,
    config: wgpu::SurfaceConfiguration,
}

impl<'a> GpuContext<'a> {
    pub fn new(window: &'a Window) -> Result<Self> {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;
        let adapter = Self::create_adapter(&instance, &surface)?;
        let (device, queue) = Self::create_device(&adapter)?;
        let surface_caps = surface.get_capabilities(&adapter);
        let config = Self::create_surface_config(window.inner_size(), surface_caps);

        surface.configure(&device, &config);

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&Default::default());
        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Depth Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            device,
            queue,
            surface,
            depth,
            depth_view,
            depth_sampler,
            config,
        })
    }

    fn create_adapter(instance: &Instance, surface: &Surface) -> Result<Adapter> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .ok_or_else(|| anyhow::anyhow!("No adapter found"))
    }

    fn create_device(adapter: &Adapter) -> Result<(Device, Queue)> {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    label: None,
                },
                None,
            )
            .block_on()
            .map_err(|e| e.into())
    }

    fn create_surface_config(
        size: PhysicalSize<u32>,
        capabilities: SurfaceCapabilities,
    ) -> wgpu::SurfaceConfiguration {
        let formats = capabilities.formats.iter().map(|f| *f).collect::<Vec<_>>();
        let supports_hdr = formats.iter().any(|format| {
            matches!(
                format,
                wgpu::TextureFormat::Bgra8UnormSrgb
                    | wgpu::TextureFormat::Rgba16Float
                    | wgpu::TextureFormat::Rgba32Float // Add other HDR formats as needed
            )
        });
        info!("Surface supports HDR: {}", supports_hdr);
        // List all formats supported by the surface
        info!("Supported surface formats: {:#?}", formats);
        let format = formats
            .iter()
            .cloned()
            .max_by(|a, b| {
                let a_score = GpuContext::format_score(*a);
                let b_score = GpuContext::format_score(*b);
                a_score.cmp(&b_score)
            })
            .unwrap_or(formats[0].clone());
        info!("Using surface format: {:?}", format);

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: capabilities
                .present_modes
                .iter()
                .cloned()
                .max_by(|a, b| Self::present_mode_score(*a).cmp(&Self::present_mode_score(*b)))
                .unwrap_or(wgpu::PresentMode::AutoNoVsync),
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    fn format_score(format: wgpu::TextureFormat) -> u32 {
        match format {
            // Assign higher scores to preferred formats
            wgpu::TextureFormat::Rgba16Float => 9,
            wgpu::TextureFormat::Rgba32Float => 8,
            wgpu::TextureFormat::Bgra8UnormSrgb => 7,
            wgpu::TextureFormat::Rgba8UnormSrgb => 6,
            _ => 0, // Default score for other formats
        }
    }

    fn present_mode_score(present_mode: wgpu::PresentMode) -> u32 {
        match present_mode {
            wgpu::PresentMode::AutoVsync => 11,
            wgpu::PresentMode::Mailbox => 10,
            wgpu::PresentMode::Fifo => 9,
            wgpu::PresentMode::Immediate => 8,
            wgpu::PresentMode::AutoNoVsync => 7,
            _ => 0,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);

        // Recreate depth texture
        self.depth = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.depth_view = self.depth.create_view(&Default::default());
    }
}

// Renderer handles all drawing operations
struct Renderer {
    _window: Arc<Window>, // Keep window alive as long as renderer exists
    gpu: GpuContext<'static>,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    diffuse_layout: wgpu::BindGroupLayout,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,

    // ================== DRAWING DEPTH ==================
    depth_pipeline: GPUPipeline,
    depth_layout: wgpu::BindGroupLayout,
    depth_bind_group: wgpu::BindGroup,
    depth_vertices: wgpu::Buffer,
    depth_num_vertices: u32,
    uniforms: uniform::Uniforms,
    uniforms_buffer: wgpu::Buffer,

    overall_time: f32,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let gpu: GpuContext<'_> = unsafe { std::mem::transmute(GpuContext::new(&window)?) };

        // ================== TEXTURE ==================
        let diffuse_bytes = include_bytes!("../../assets/stone.png");
        let diffuse_texture = texture::Texture::from_bytes(
            &gpu.device,
            &gpu.queue,
            diffuse_bytes,
            "diffuse_texture",
        )?;
        let diffuse_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            // This should match the filterable field of the
                            // corresponding Texture entry above.
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });
        let diffuse_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &diffuse_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
            });

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&diffuse_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // 1.
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
                cache: None,
            });

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let num_vertices = VERTICES.len() as u32;

        // ================== DRAWING DEPTH ==================
        let depth_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("depth_bind_group_layout"),
            });
        let uniforms = uniform::Uniforms::new([gpu.config.width as f32, gpu.config.height as f32]);
        let uniforms_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniforms Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let depth_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &depth_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gpu.depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniforms.as_entire_binding(&uniforms_buffer),
                },
            ],
            label: Some("depth_bind_group"),
        });
        let depth_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Depth Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/depth.wgsl").into()),
            });
        let depth_pipeline = GPUPipelineBuilder::new(&gpu.device)
            .label("Depth Pipeline")
            .bind_group_layout(&depth_layout)
            .vertex_shader(&depth_shader, "vs_main")
            .vertex_buffer_layout(DepthVertex::desc())
            .fragment_shader(&depth_shader, "fs_main")
            .default_color_target(gpu.config.format)
            .depth_stencil_state(None)
            .default_multisample_state()
            .default_primitive_state()
            .build()
            .expect("Failed to create depth pipeline");

        let depth_vertices = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Depth Vertex Buffer"),
                contents: bytemuck::cast_slice(DEPTH_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let depth_num_vertices = DEPTH_VERTICES.len() as u32;

        Ok(Self {
            _window: window,
            gpu,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            diffuse_layout: diffuse_bind_group_layout,
            diffuse_bind_group,
            diffuse_texture,

            // ================== DRAWING DEPTH ==================
            depth_pipeline,
            depth_layout,
            depth_bind_group,
            depth_vertices,
            depth_num_vertices,

            uniforms,
            uniforms_buffer,

            overall_time: 0.0,
        })
    }

    pub fn render(&mut self, delta: f32) -> Result<()> {
        let _render_guard = tracing_tracy::client::Client::running()
            .expect("client must be running")
            .non_continuous_frame(frame_name!("rendering"));

        let output = self.gpu.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());

        // Update the vertex buffer with new data
        let new_vertices = vertex::rotated_vertices(self.overall_time);
        self.gpu
            .queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&new_vertices));

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        // DRAWING DEPTH
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.depth_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.depth_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.depth_vertices.slice(..));
            render_pass.draw(0..self.depth_num_vertices, 0..1);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        drop(_render_guard);

        let _present_guard = tracing_tracy::client::Client::running()
            .expect("client must be running")
            .non_continuous_frame(frame_name!("presenting"));
        output.present();
        drop(_present_guard);

        self.overall_time += delta;

        Ok(())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.gpu.resize(new_size);

        // Recreate all resources related to the depth texture
        self.depth_bind_group = self
            .gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.depth_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.gpu.depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.gpu.depth_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniforms.as_entire_binding(&self.uniforms_buffer),
                    },
                ],
                label: Some("depth_bind_group"),
            });

        // Put new data into the resolution buffer
        self.uniforms.resolution = [new_size.width as f32, new_size.height as f32];
        self.gpu.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }
}

struct Engine {
    window: Arc<Window>,
    renderer: Renderer,
    last_time: std::time::Instant,
    frame_time_history: Vec<f32>,
}

impl Engine {
    pub fn new(window: Window) -> Result<Self> {
        let window = Arc::new(window);
        let renderer = Renderer::new(window.clone())?;
        Ok(Self {
            window,
            renderer,
            last_time: std::time::Instant::now(),
            frame_time_history: Vec::new(),
        })
    }

    pub fn render(&mut self) -> Result<()> {
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;

        // Calculate the average frame time
        self.frame_time_history.push(delta);
        if self.frame_time_history.len() > 2000 {
            self.frame_time_history.remove(0);
        }
        let average_frame_time: f32 =
            self.frame_time_history.iter().sum::<f32>() / self.frame_time_history.len() as f32;
        // Sort frame times for percentile calculations
        let mut sorted_times = self.frame_time_history.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate 95th and 99th percentiles
        let percentile_95_idx = ((self.frame_time_history.len() as f32 * 0.95) as usize).max(0);
        let percentile_99_idx = ((self.frame_time_history.len() as f32 * 0.99) as usize).max(0);

        let percentile_95 = sorted_times.get(percentile_95_idx).copied().unwrap_or(0.0);
        let percentile_99 = sorted_times.get(percentile_99_idx).copied().unwrap_or(0.0);
        self.window.set_title(&format!(
            "Frame time: {:.2}ms (95th: {:.2}ms, 99th: {:.2}ms)",
            average_frame_time * 1000.0,
            percentile_95 * 1000.0,
            percentile_99 * 1000.0
        ));

        // Tracy
        tracing_tracy::client::Client::running()
            .expect("client must be running")
            .frame_mark();

        self.renderer.render(delta)?;
        Ok(())
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.renderer.resize(size);
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}

// Application handling
struct Application {
    engine: Option<Engine>,
}

impl Application {
    pub fn new() -> Self {
        Self { engine: None }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("WGPU Engine")
                    .with_inner_size(Size::Logical(LogicalSize::new(800.0, 600.0)))
                    .with_min_inner_size(Size::Logical(LogicalSize::new(400.0, 300.0))),
            )
            .unwrap();

        self.engine = Some(Engine::new(window).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(engine) = &mut self.engine {
            if engine.window().id() == window_id {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(size) => engine.resize(size),
                    WindowEvent::RedrawRequested => {
                        let _ = engine.render();
                    }
                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(engine) = &self.engine {
            engine.window().request_redraw();
        }
    }
}

pub async fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = Application::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn main() -> Result<()> {
    let env_filter = EnvFilter::from_default_env()
        .add_directive("wgpu=warn".parse().unwrap())
        .add_directive("winit=warn".parse().unwrap())
        .add_directive("naga=warn".parse().unwrap())
        .add_directive("debug".parse().unwrap());

    // Initialize the subscriber with the filter
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_tracy::TracyLayer::default())
            .with(env_filter),
    )
    .expect("setup tracy layer");
    better_panic::install();
    pollster::block_on(run())?;
    Ok(())
}
