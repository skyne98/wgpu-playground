use anyhow::Result;
use bevy_ecs::{schedule::Schedule, system::Resource, world::World};

use crate::{
    texture::Texture,
    uniform::{Uniforms, UniformsData},
    vertex::DepthVertex,
    GpuContext,
};

use super::{GPUPipeline, GPUPipelineBuilder};

pub fn setup_depth(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;
    let uniforms = world
        .get_resource::<Uniforms>()
        .ok_or_else(|| anyhow::anyhow!("UniformsData resource not found"))?;

    let depth_texture = DepthTexture::new(&gpu, gpu.config.width, gpu.config.height)?;

    let diffuse_bind_group_layout = DepthBindGroupLayout::new(&gpu)?;
    let diffuse_bind_group = DepthBindGroup::new(
        &gpu,
        &depth_texture,
        &diffuse_bind_group_layout,
        &uniforms.buffer,
    )?;
    let diffuse_pipeline = DepthPipeline::new(&gpu, &diffuse_bind_group_layout)?;
    world.insert_resource(diffuse_bind_group_layout);
    world.insert_resource(diffuse_bind_group);
    world.insert_resource(depth_texture);
    world.insert_resource(diffuse_pipeline);

    Ok(())
}

// =============================== BIND GROUP ===============================
#[derive(Resource)]
pub struct DepthBindGroupLayout {
    pub layout: wgpu::BindGroupLayout,
}
impl DepthBindGroupLayout {
    pub fn new(gpu: &GpuContext) -> Result<Self> {
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

        Ok(Self {
            layout: depth_layout,
        })
    }
}

#[derive(Resource)]
pub struct DepthBindGroup {
    pub bind_group: wgpu::BindGroup,
}
impl DepthBindGroup {
    pub fn new(
        gpu: &GpuContext,
        depth_texture: &DepthTexture,
        layout: &DepthBindGroupLayout,
        uniforms_buffer: &wgpu::Buffer,
    ) -> Result<Self> {
        let depth_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth_texture.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&depth_texture.texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: UniformsData::as_entire_binding(&uniforms_buffer),
                },
            ],
            label: Some("depth_bind_group"),
        });

        Ok(Self {
            bind_group: depth_bind_group,
        })
    }
}

// =============================== PIPELINE ===============================
#[derive(Resource)]
pub struct DepthPipeline {
    pub pipeline: GPUPipeline,
}
impl DepthPipeline {
    pub fn new(gpu: &GpuContext, bind_group_layout: &DepthBindGroupLayout) -> Result<Self> {
        let depth_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Depth Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/depth.wgsl").into()),
            });
        let depth_pipeline = GPUPipelineBuilder::new(&gpu.device)
            .label("Depth Pipeline")
            .bind_group_layout(&bind_group_layout.layout)
            .vertex_shader(&depth_shader, "vs_main")
            .fragment_shader(&depth_shader, "fs_main")
            .vertex_buffer_layout(DepthVertex::desc())
            .default_color_target(gpu.config.format)
            .depth_stencil_state(None)
            .default_multisample_state()
            .default_primitive_state()
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            pipeline: depth_pipeline,
        })
    }
}

// =============================== TEXTURE ===============================
#[derive(Resource)]
pub struct DepthTexture {
    pub texture: Texture,
}
impl DepthTexture {
    pub fn new(gpu: &GpuContext, width: u32, height: u32) -> Result<Self> {
        let texture = Texture::depth_texture(&gpu.device, width, height);
        Ok(Self { texture })
    }
}
