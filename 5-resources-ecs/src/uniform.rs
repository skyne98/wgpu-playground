use crate::gpu::GpuContext;
use anyhow::Result;
use bevy_ecs::{schedule::Schedule, system::Resource, world::World};
use wgpu::util::DeviceExt;

pub fn setup_uniforms(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;

    let uniforms = Uniforms::new(gpu);
    world.insert_resource(uniforms);

    Ok(())
}

#[derive(Resource)]
pub struct Uniforms {
    pub data: UniformsData,
    pub buffer: wgpu::Buffer,
}
impl Uniforms {
    pub fn new(gpu: &GpuContext) -> Self {
        let data = UniformsData::new(
            [gpu.config.width as f32, gpu.config.height as f32],
            gpu.config.format.is_srgb(),
        );
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniforms_buffer"),
                contents: bytemuck::cast_slice(&[data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        Self { data, buffer }
    }
    pub fn update_resolution(&mut self, gpu: &GpuContext, resolution: [f32; 2]) {
        self.data.resolution = resolution;
        gpu.queue
            .write_buffer(&self.buffer, 0, self.data.as_bytes());
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsData {
    pub resolution: [f32; 2],
    pub srgb_surface: f32,
    pub _padding: f32, // Add padding to match 16-byte alignment
}

impl UniformsData {
    pub fn new(resolution: [f32; 2], srgb_surface: bool) -> Self {
        Self {
            resolution,
            srgb_surface: if srgb_surface { 1.0 } else { 0.0 },
            _padding: 0.0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    pub fn as_entire_binding<'a>(buffer: &'a wgpu::Buffer) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset: 0,
            size: None,
        })
    }
}
