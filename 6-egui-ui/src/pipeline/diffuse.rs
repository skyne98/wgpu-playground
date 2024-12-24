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
    vertex::{DepthVertex, Vertex},
    GpuContext,
};

use super::{present::FrameBuffer, GPUPipeline, GPUPipelineBuilder};

pub fn setup_diffuse(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;

    let diffuse_bind_group_layout = DiffuseBindGroupLayout::new(&gpu)?;
    let diffuse_bytes = include_bytes!("../../../assets/stone.png");
    let diffuse_texture =
        texture::Texture::from_bytes(&gpu.device, &gpu.queue, diffuse_bytes, "diffuse_texture")?;
    let diffuse_bind_group =
        DiffuseBindGroup::new(&gpu, &diffuse_bind_group_layout, &diffuse_texture)?;
    let diffuse_pipeline = DiffusePipeline::new(&gpu, &diffuse_bind_group_layout)?;

    world.insert_resource(diffuse_bind_group_layout);
    world.insert_resource(diffuse_bind_group);
    world.insert_resource(diffuse_pipeline);

    Ok(())
}

// =============================== BIND GROUP ===============================
#[derive(Resource)]
pub struct DiffuseBindGroupLayout {
    pub layout: wgpu::BindGroupLayout,
}
impl DiffuseBindGroupLayout {
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
                            // This should match the filterable field of the
                            // corresponding Texture entry above.
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
pub struct DiffuseBindGroup {
    pub bind_group: wgpu::BindGroup,
}
impl DiffuseBindGroup {
    pub fn new(
        gpu: &GpuContext,
        layout: &DiffuseBindGroupLayout,
        texture: &Texture,
    ) -> Result<Self> {
        let diffuse_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            ],
            label: Some("diffuse_bind_group"),
        });

        Ok(Self {
            bind_group: diffuse_bind_group,
        })
    }
}

// =============================== PIPELINE ===============================
#[derive(Resource)]
pub struct DiffusePipeline {
    pub pipeline: GPUPipeline,
}
impl DiffusePipeline {
    pub fn new(gpu: &GpuContext, bind_group_layout: &DiffuseBindGroupLayout) -> Result<Self> {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("diffuse_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/diffuse.wgsl").into()),
            });
        let diffuse_pipeline = GPUPipelineBuilder::new(&gpu.device)
            .label("diffuse_pipeline")
            .bind_group_layout(&bind_group_layout.layout)
            .vertex_shader(&shader, "vs_main")
            .fragment_shader(&shader, "fs_main")
            .vertex_buffer_layout(Vertex::desc())
            .default_color_target(wgpu::TextureFormat::Rgba16Float)
            .default_depth_stencil_state()
            .default_multisample_state()
            .default_primitive_state()
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            pipeline: diffuse_pipeline,
        })
    }
}
