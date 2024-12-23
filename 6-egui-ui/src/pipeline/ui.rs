use anyhow::Result;
use bevy_ecs::{
    prelude::resource_changed,
    schedule::{IntoSystemConfigs, Schedule},
    system::{Res, ResMut, Resource},
    world::World,
};
use wgpu::TextureFormat;

use crate::gpu::GpuContext;

use super::present::FrameBuffer;

pub fn setup_ui(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;

    let pipeline = EguiRenderer::new(
        &gpu.device,
        TextureFormat::Rgba16Float,
        None,
        1,
        &gpu.window,
    );
    let app = egui_demo_lib::DemoWindows::default();
    let ui = EguiState {
        renderer: pipeline,
        app,
    };

    world.insert_resource(ui);

    schedule.add_systems(frame_buffer_changed_system.run_if(resource_changed::<FrameBuffer>));

    Ok(())
}

pub fn frame_buffer_changed_system(
    frame_buffer: Res<FrameBuffer>,
    gpu: Res<GpuContext>,
    mut pipeline: ResMut<EguiState>,
) {
    let new_size = gpu.window.inner_size();
    let new_scale = gpu.window.scale_factor();
}

// =============================== UI RESOURCE ===============================
#[derive(Resource)]
pub struct EguiState {
    pub(crate) renderer: EguiRenderer,
    pub(crate) app: egui_demo_lib::DemoWindows,
}
unsafe impl Send for EguiState {}
unsafe impl Sync for EguiState {}
impl EguiState {
    pub fn run_app(&mut self) {
        self.app.ui(&self.renderer.context());
    }
}

// =============================== RENDERER ===============================
use egui::Context;
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureView};
use egui_wgpu::{wgpu, Renderer, ScreenDescriptor};
use egui_winit::{EventResponse, State};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024), // default dimension is 2048
        );
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn ppp(&mut self, v: f32) {
        self.context().set_pixels_per_point(v);
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!("begin_frame must be called before end_frame_and_draw can be called!");
        }

        self.ppp(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: egui_wgpu::wgpu::Operations {
                    load: egui_wgpu::wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }

        self.frame_started = false;
    }
}
