use anyhow::Result;
use pollster::FutureExt;
use std::sync::Arc;
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
        let format = capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
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
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let gpu: GpuContext<'_> = unsafe { std::mem::transmute(GpuContext::new(&window)?) };

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
                    bind_group_layouts: &[],
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
    tracing_subscriber::fmt::init();
    better_panic::install();
    pollster::block_on(run())?;
    Ok(())
}
