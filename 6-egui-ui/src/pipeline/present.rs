use anyhow::Result;
use bevy_ecs::{
    component::Component,
    prelude::resource_changed,
    schedule::{IntoSystemConfigs, Schedule},
    system::{Res, ResMut, Resource},
    world::World,
};

use crate::{
    shaders::present::bind_groups::{BindGroup0, BindGroupLayout0},
    texture::{self, Texture},
    uniform::Uniforms,
    vertex::{DepthVertex, Vertex},
    GpuContext,
};

use super::{GPUPipeline, GPUPipelineBuilder};

pub fn setup_present(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;
    let frame_buffer = world
        .get_resource::<FrameBuffer>()
        .ok_or_else(|| anyhow::anyhow!("Texture resource not found"))?;
    let uniform = world
        .get_resource::<Uniforms>()
        .ok_or_else(|| anyhow::anyhow!("Uniform resource not found"))?;

    let bind_group_layout = PresentBindGroupLayout::new(&gpu)?;
    let bind_group =
        PresentBindGroup::new(&gpu, &bind_group_layout, &frame_buffer.texture, uniform)?;
    let pipeline = PresentPipeline::new(&gpu, &bind_group_layout)?;

    world.insert_resource(bind_group_layout);
    world.insert_resource(bind_group);
    world.insert_resource(pipeline);

    schedule.add_systems(frame_buffer_changed_system.run_if(resource_changed::<FrameBuffer>));

    Ok(())
}
pub fn setup_frame_buffer(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;

    let texture =
        Texture::frame_buffer_texture(&gpu.device, gpu.config.width, gpu.config.height, None, 1);
    let frame_buffer = FrameBuffer { texture };

    world.insert_resource(frame_buffer);

    Ok(())
}

pub fn frame_buffer_changed_system(
    frame_buffer: ResMut<FrameBuffer>,
    gpu: Res<GpuContext>,
    mut present_bind_group: ResMut<PresentBindGroup>,
    present_bind_group_layout: Res<PresentBindGroupLayout>,
    uniforms: Res<Uniforms>,
) {
    present_bind_group.recreate(
        &gpu.device,
        &present_bind_group_layout,
        &frame_buffer.texture,
        &uniforms,
    );
}

// =============================== FRAME BUFFER ===============================
#[derive(Resource)]
pub struct FrameBuffer {
    pub texture: Texture,
}

// =============================== BIND GROUP ===============================
#[derive(Resource)]
pub struct PresentBindGroup {
    pub bind_group: BindGroup0,
}
impl PresentBindGroup {
    pub fn new(gpu: &GpuContext, texture: &Texture, uniforms_buffer: &Uniforms) -> Result<Self> {
        let bind_group = BindGroup0::from_bindings(
            &gpu.device,
            BindGroupLayout0 {
                t_diffuse: &texture.view,
                s_diffuse: &texture.sampler,
                uniforms: wgpu::BufferBinding {
                    buffer: &uniforms_buffer.buffer,
                    offset: 0,
                    size: None,
                },
            },
        );

        Ok(Self { bind_group })
    }
    pub fn recreate(
        &mut self,
        device: &wgpu::Device,
        texture: &Texture,
        uniforms_buffer: &Uniforms,
    ) {
        self.bind_group = BindGroup0::from_bindings(
            device,
            BindGroupLayout0 {
                t_diffuse: &texture.view,
                s_diffuse: &texture.sampler,
                uniforms: wgpu::BufferBinding {
                    buffer: &uniforms_buffer.buffer,
                    offset: 0,
                    size: None,
                },
            },
        );
    }
}

// =============================== PIPELINE ===============================
#[derive(Resource)]
pub struct PresentPipeline {
    pub pipeline: GPUPipeline,
}
impl PresentPipeline {
    pub fn new(gpu: &GpuContext, bind_group: &PresentBindGroup) -> Result<Self> {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("present_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/present.wgsl").into()),
            });
        let pipeline = GPUPipelineBuilder::new(&gpu.device)
            .label("present_pipeline")
            .bind_group_layout(&bind_group.bind_group.
            .vertex_shader(&shader, "vs_main")
            .fragment_shader(&shader, "fs_main")
            .default_color_target(gpu.config.format)
            .depth_stencil_state(None)
            .default_multisample_state()
            .default_primitive_state()
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self { pipeline })
    }
}
