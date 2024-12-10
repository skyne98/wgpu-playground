use crate::{gpu::GpuContext, ResizeEvent};
use anyhow::Result;
use bevy_ecs::{
    observer::Trigger,
    schedule::Schedule,
    system::{Res, ResMut, Resource},
    world::World,
};
use wgpu::util::DeviceExt;

pub fn setup_uniforms(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("GpuContext resource not found"))?;

    let uniforms = Uniforms::new(gpu);
    world.insert_resource(uniforms);

    world.add_observer(
        |trigger: Trigger<ResizeEvent>, gpu: Res<GpuContext>, mut uniforms: ResMut<Uniforms>| {
            let new_size = trigger.event().size;
            uniforms.update(&gpu, [new_size.width as f32, new_size.height as f32]);
        },
    );

    Ok(())
}

#[derive(Resource)]
pub struct Uniforms {
    pub data: UniformsData,
    pub buffer: wgpu::Buffer,
}
impl Uniforms {
    pub fn new(gpu: &GpuContext) -> Self {
        let data = UniformsData::new([gpu.config.width as f32, gpu.config.height as f32]);
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniforms_buffer"),
                contents: bytemuck::cast_slice(&[data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        Self { data, buffer }
    }
    pub fn update(&mut self, gpu: &GpuContext, resolution: [f32; 2]) {
        self.data.resolution = resolution;
        gpu.queue
            .write_buffer(&self.buffer, 0, self.data.as_bytes());
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsData {
    pub resolution: [f32; 2],
}

impl UniformsData {
    pub fn new(resolution: [f32; 2]) -> Self {
        Self { resolution }
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
