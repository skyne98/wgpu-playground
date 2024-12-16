use anyhow::Result;
use bevy_ecs::{
    component::Component,
    prelude::resource_changed,
    schedule::{IntoSystemConfigs, Schedule},
    system::{Res, ResMut, Resource},
    world::World,
};
use egui::{FullOutput, TexturesDelta};
use egui_demo_lib::DemoWindows;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::Platform;
use wgpu::{core::device, TextureFormat};

use crate::{
    texture::{self, Texture},
    ui::{create_app, create_platform, create_render_pass},
    uniform::Uniforms,
    vertex::{DepthVertex, Vertex},
    GpuContext,
};

use super::{present::FrameBuffer, GPUPipeline, GPUPipelineBuilder};

pub fn setup_ui(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;
    let frame_buffer = world
        .get_resource::<FrameBuffer>()
        .ok_or_else(|| anyhow::anyhow!("FrameBuffer resource not found"))?;

    let pipeline = UiPipeline::new(&gpu, frame_buffer.texture.texture.format())?;

    world.insert_resource(pipeline);

    schedule.add_systems(frame_buffer_changed_system.run_if(resource_changed::<FrameBuffer>));

    Ok(())
}

pub fn frame_buffer_changed_system(
    frame_buffer: ResMut<FrameBuffer>,
    gpu: Res<GpuContext>,
    uniforms: Res<Uniforms>,
) {
    // Update the uniforms resolution
}

// =============================== PIPELINE ===============================
#[derive(Resource)]
pub struct UiPipeline {
    pub platform: Platform,
    pub render_pass: RenderPass,
    pub app: DemoWindows,
    width: u32,
    height: u32,
    scale: f64,
}
unsafe impl Send for UiPipeline {}
unsafe impl Sync for UiPipeline {}
impl UiPipeline {
    pub fn new(gpu: &GpuContext, format: TextureFormat) -> Result<Self> {
        let platform = create_platform(gpu.config.width, gpu.config.height, gpu.scale);
        let render_pass = create_render_pass(&gpu.device, format);
        let app = create_app();

        Ok(Self {
            platform,
            render_pass,
            app,
            width: gpu.config.width,
            height: gpu.config.height,
            scale: gpu.scale,
        })
    }

    pub fn render(
        &mut self,
        elapsed: f64,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> TexturesDelta {
        self.platform.update_time(elapsed);

        // Begin to draw the UI frame.
        self.platform.begin_frame();
        self.app.ui(&self.platform.context());

        let full_output = self.platform.end_frame(None);
        let paint_jobs = self
            .platform
            .context()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.width,
            physical_height: self.height,
            scale_factor: self.scale as f32,
        };

        let tdelta: egui::TexturesDelta = full_output.textures_delta;
        let egui_rpass = &mut self.render_pass;
        egui_rpass
            .add_textures(&device, &queue, &tdelta)
            .expect("add texture ok");
        egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

        // Record all render passes.
        egui_rpass
            .execute(
                encoder,
                &target,
                &paint_jobs,
                &screen_descriptor,
                Some(wgpu::Color::BLACK),
            )
            .unwrap();

        tdelta
    }

    pub fn clean_up(&mut self, tdelta: TexturesDelta) {
        self.render_pass
            .remove_textures(tdelta)
            .expect("remove texture ok");
    }
}
