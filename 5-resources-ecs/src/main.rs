use anyhow::Result;
use bevy_ecs::{event::Event, schedule::Schedule, system::Resource, world::World};
use gpu::{setup_gpu, GpuContext};
use pipeline::{
    depth::setup_depth, diffuse::setup_diffuse, render::setup_rendering, GPUPipeline,
    GPUPipelineBuilder,
};
use pollster::FutureExt;
use std::sync::Arc;
use time::setup_time;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use tracing_tracy::client::{frame_name, ProfiledAllocator};
use uniform::setup_uniforms;
use vertex::{setup_vertex_buffers, DepthVertex, Vertex, DEPTH_VERTICES, VERTICES};
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

mod gpu;
mod pipeline;
mod texture;
mod time;
mod uniform;
mod vertex;

#[derive(Event)]
struct ResizeEvent {
    size: PhysicalSize<u32>,
}

// Application handling
struct Application {
    world: World,
    schedule: Schedule,
}

impl Application {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::default(),
        }
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
            .expect("Failed to create window");

        setup_time(&mut self.world, &mut self.schedule).expect("Failed to setup time");
        setup_gpu(&mut self.world, window).expect("Failed to setup GPU");
        setup_uniforms(&mut self.world, &mut self.schedule).expect("Failed to setup uniforms");
        setup_diffuse(&mut self.world, &mut self.schedule)
            .expect("Failed to setup diffuse pipeline");
        setup_depth(&mut self.world, &mut self.schedule).expect("Failed to setup depth pipeline");
        setup_vertex_buffers(&mut self.world, &mut self.schedule)
            .expect("Failed to setup vertex buffers");
        setup_rendering(&mut self.world, &mut self.schedule).expect("Failed to setup rendering");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let current_window_id = {
            let gpu = self
                .world
                .get_resource::<GpuContext>()
                .expect("GpuContext not found");
            gpu.window.id()
        };

        if current_window_id == window_id {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => self.world.trigger(ResizeEvent { size }),
                WindowEvent::RedrawRequested => {
                    self.schedule.run(&mut self.world);
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let gpu = self
            .world
            .get_resource::<GpuContext>()
            .expect("GpuContext not found");
        gpu.window.request_redraw();
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
