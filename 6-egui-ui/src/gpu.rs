use anyhow::Result;
use bevy_ecs::event::Event;
use bevy_ecs::event::EventReader;
use bevy_ecs::observer::Trigger;
use bevy_ecs::schedule::Schedule;
use bevy_ecs::system::Commands;
use bevy_ecs::system::ResMut;
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use pollster::FutureExt;
use tracing::info;
use wgpu::Adapter;
use wgpu::Device;
use wgpu::Instance;
use wgpu::Queue;
use wgpu::Surface;
use wgpu::SurfaceCapabilities;
use winit::dpi::PhysicalSize;
use winit::window::Window;

// GPU Context handling
#[derive(Resource)]
pub struct GpuContext {
    pub window: Window,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub scale: f64,
}

impl GpuContext {
    pub fn new(window: Window) -> Result<Self> {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // turn into a static borrow
        let window_static: &'static Window = unsafe { std::mem::transmute(&window) };
        let surface = instance.create_surface(window_static)?;
        let adapter = Self::create_adapter(&instance, &surface)?;
        let (device, queue) = Self::create_device(&adapter)?;
        let surface_caps = surface.get_capabilities(&adapter);
        let config = Self::create_surface_config(window.inner_size(), surface_caps);

        surface.configure(&device, &config);

        let scale = window.scale_factor();

        Ok(Self {
            window,
            device,
            queue,
            surface,
            config,
            scale,
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
            wgpu::TextureFormat::Bgra8UnormSrgb => 10,
            wgpu::TextureFormat::Rgba8UnormSrgb => 9,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 7,
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

    pub fn resize(&mut self, size: &PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }
}

pub fn setup_gpu(world: &mut World, schedule: &mut Schedule, window: Window) -> Result<()> {
    let gpu = GpuContext::new(window)?;
    world.insert_resource(gpu);
    Ok(())
}
