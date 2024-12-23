use anyhow::Result;
use bevy_ecs::{
    component::Component,
    prelude::resource_changed,
    schedule::{IntoSystemConfigs, Schedule},
    system::{Res, ResMut, Resource},
    world::World,
};

use crate::{
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
pub struct PresentBindGroupLayout {
    pub layout: wgpu::BindGroupLayout,
}
impl PresentBindGroupLayout {
    pub fn new(gpu: &GpuContext) -> Result<Self> {
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
                    label: Some("diffuse_bind_group_layout"),
                });

        Ok(Self {
            layout: diffuse_bind_group_layout,
        })
    }
}

#[derive(Resource)]
pub struct PresentBindGroup {
    pub bind_group: wgpu::BindGroup,
}
impl PresentBindGroup {
    pub fn new(
        gpu: &GpuContext,
        layout: &PresentBindGroupLayout,
        texture: &Texture,
        uniforms_buffer: &Uniforms,
    ) -> Result<Self> {
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniforms_buffer.buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            label: Some("present_bind_group"),
        });

        Ok(Self { bind_group })
    }
    pub fn recreate(
        &mut self,
        device: &wgpu::Device,
        layout: &PresentBindGroupLayout,
        texture: &Texture,
        uniforms_buffer: &Uniforms,
    ) {
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniforms_buffer.buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            label: Some("present_bind_group"),
        });
    }
}

// =============================== PIPELINE ===============================
#[derive(Resource)]
pub struct PresentPipeline {
    pub pipeline: GPUPipeline,
}
impl PresentPipeline {
    pub fn new(gpu: &GpuContext, bind_group_layout: &PresentBindGroupLayout) -> Result<Self> {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("present_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/present.wgsl").into()),
            });
        let pipeline = GPUPipelineBuilder::new(&gpu.device)
            .label("present_pipeline")
            .bind_group_layout(&bind_group_layout.layout)
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
