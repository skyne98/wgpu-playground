use anyhow::Result;
use bevy_ecs::{
    component::Component,
    event::{Event, EventReader, Events},
    observer::{Observer, Trigger, TriggerEvent},
    schedule::Schedule,
    system::{Res, ResMut, Resource, RunSystemOnce},
    world::World,
};
use debouncer::Debouncer;
use gpu::{setup_gpu, GpuContext};
use pipeline::{
    depth::{setup_depth, DepthTexture},
    diffuse::setup_diffuse,
    present::{setup_frame_buffer, setup_present, FrameBuffer},
    render::setup_rendering,
    GPUPipeline, GPUPipelineBuilder,
};
use pollster::FutureExt;
use std::{sync::Arc, time::Duration};
use time::{setup_time, TimeContext};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use tracing_tracy::client::{frame_name, ProfiledAllocator};
use uniform::{setup_uniforms, Uniforms};
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

mod debouncer;
mod gpu;
mod pipeline;
mod texture;
mod time;
mod uniform;
mod vertex;

#[derive(Resource)]
pub struct ResizeState {
    pub debouncer: Debouncer<PhysicalSize<u32>>,
}

impl Default for ResizeState {
    fn default() -> Self {
        Self {
            debouncer: Debouncer::new(Duration::from_millis(100)),
        }
    }
}

#[derive(Event)]
pub struct ResizeEvent {
    pub size: PhysicalSize<u32>,
}

fn resize_system(
    mut resize_state: ResMut<ResizeState>,
    mut gpu: ResMut<GpuContext>,
    mut depth_texture: ResMut<DepthTexture>,
    mut uniforms: ResMut<Uniforms>,
    mut frame_buffer: ResMut<FrameBuffer>,
    time: Res<TimeContext>,
) {
    resize_state.debouncer.tick(time.delta);

    if let Some(size) = resize_state.debouncer.get() {
        info!("Resize event: {:?}", size);
        gpu.resize(&size);
        frame_buffer
            .texture
            .resize(&gpu.device, &gpu.queue, size.width, size.height);
        depth_texture.resize(&gpu.device, size.width, size.height);
        let resolution = [size.width as f32, size.height as f32];
        uniforms.update_resolution(&gpu, resolution);
    }
}

// Application handling
struct Application {
    world: World,
    schedule: Schedule,
}

impl Application {
    pub fn new() -> Self {
        let world = World::default();

        Self {
            world,
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
        setup_gpu(&mut self.world, &mut self.schedule, window).expect("Failed to setup GPU");
        setup_uniforms(&mut self.world, &mut self.schedule).expect("Failed to setup uniforms");
        setup_frame_buffer(&mut self.world, &mut self.schedule)
            .expect("Failed to setup frame buffer");
        setup_diffuse(&mut self.world, &mut self.schedule)
            .expect("Failed to setup diffuse pipeline");
        setup_depth(&mut self.world, &mut self.schedule).expect("Failed to setup depth pipeline");
        setup_vertex_buffers(&mut self.world, &mut self.schedule)
            .expect("Failed to setup vertex buffers");
        setup_present(&mut self.world, &mut self.schedule)
            .expect("Failed to setup present pipeline");
        setup_rendering(&mut self.world, &mut self.schedule).expect("Failed to setup rendering");

        self.world.insert_resource(ResizeState::default());
        self.world.add_observer(
            |trigger: Trigger<ResizeEvent>, mut resize_state: ResMut<ResizeState>| {
                let size = (*trigger.event()).size;
                resize_state.debouncer.push(size);
            },
        );
        self.schedule.add_systems(resize_system);
        self.world.flush();
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
                WindowEvent::Resized(size) => {
                    self.world.trigger(ResizeEvent { size });
                }
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
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer()),
    )
    .expect("setup tracing");
    better_panic::install();

    pollster::block_on(run())?;
    Ok(())
}
