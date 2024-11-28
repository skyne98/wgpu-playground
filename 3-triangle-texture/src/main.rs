use anyhow::Result;
use pollster::FutureExt;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;
use vertex::{Vertex, VERTICES};
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

mod texture;
mod vertex;

// GPU Context handling
struct GpuContext<'a> {
    device: Device,
    queue: Queue,
    surface: Surface<'a>,
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

        Ok(Self {
            device,
            queue,
            surface,
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
            // Assign higher scores to preferred present modes
            wgpu::PresentMode::Fifo => 10,
            wgpu::PresentMode::Mailbox => 9,
            wgpu::PresentMode::Immediate => 8,
            _ => 0, // Default score for other present modes
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
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
                depth_stencil: None, // 1.
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
                usage: wgpu::BufferUsages::VERTEX,
            });

        let num_vertices = VERTICES.len() as u32;

        Ok(Self {
            _window: window,
            gpu,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            diffuse_layout: diffuse_bind_group_layout,
            diffuse_bind_group,
            diffuse_texture,
        })
    }

    pub fn render(&mut self) -> Result<()> {
        let output = self.gpu.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets
                    Some(wgpu::RenderPassColorAttachment {
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
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.gpu.resize(new_size);
    }
}

struct Engine {
    window: Arc<Window>,
    renderer: Renderer,
}

impl Engine {
    pub fn new(window: Window) -> Result<Self> {
        let window = Arc::new(window);
        let renderer = Renderer::new(window.clone())?;
        Ok(Self { window, renderer })
    }

    pub fn render(&mut self) -> Result<()> {
        self.renderer.render()
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
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    better_panic::install();
    pollster::block_on(run())?;
    Ok(())
}
