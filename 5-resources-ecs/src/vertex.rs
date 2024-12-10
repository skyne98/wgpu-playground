use anyhow::Result;
use bevy_ecs::{
    schedule::Schedule,
    system::{Res, ResMut, Resource},
    world::World,
};
use wgpu::util::DeviceExt;

use crate::{gpu::GpuContext, time::TimeContext};

pub fn setup_vertex_buffers(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let gpu = world
        .get_resource::<GpuContext>()
        .ok_or_else(|| anyhow::anyhow!("Gpu resource not found"))?;

    let vertex_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
    let num_vertices = VERTICES.len() as u32;

    let depth_vertex_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Depth Vertex Buffer"),
            contents: bytemuck::cast_slice(DEPTH_VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
    let num_depth_vertices = DEPTH_VERTICES.len() as u32;

    world.insert_resource(VertexBuffers {
        vertex_buffer,
        depth_vertex_buffer,
        num_vertices,
        num_depth_vertices,
    });

    schedule.add_systems(rotate_vertices_system);

    Ok(())
}

pub fn rotate_vertices_system(
    gpu: Res<GpuContext>,
    time: Res<TimeContext>,
    vertex_buffers: ResMut<VertexBuffers>,
) {
    // Update the vertex buffer with new data
    let new_vertices = rotated_vertices(time.total);
    gpu.queue.write_buffer(
        &vertex_buffers.vertex_buffer,
        0,
        bytemuck::cast_slice(&new_vertices),
    );
}

#[derive(Resource)]
pub struct VertexBuffers {
    pub vertex_buffer: wgpu::Buffer,
    pub depth_vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_depth_vertices: u32,
}

// =================================== VERTEX ===================================
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
];

pub fn rotated_vertices(time: f32) -> [Vertex; 3] {
    let rotation = glam::Mat4::from_rotation_y(time * std::f32::consts::PI);
    // Create orthographic projection matrix
    let ortho = glam::Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, -1.5, 1.5);

    let vertices = VERTICES
        .iter()
        .map(|v| glam::Vec3::new(v.position[0], v.position[1], v.position[2]))
        .collect::<Vec<_>>();

    let rotated = [vertices[0], vertices[1], vertices[2]].map(|v| {
        // Apply rotation then projection
        let rotated = rotation.transform_vector3(v);
        let transformed = ortho.project_point3(rotated);
        Vertex {
            position: [transformed.x, transformed.y, transformed.z],
            color: [1.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
        }
    });

    [
        Vertex {
            color: VERTICES[0].color,
            tex_coords: VERTICES[0].tex_coords,
            ..rotated[0]
        },
        Vertex {
            color: VERTICES[1].color,
            tex_coords: VERTICES[1].tex_coords,
            ..rotated[1]
        },
        Vertex {
            color: VERTICES[2].color,
            tex_coords: VERTICES[2].tex_coords,
            ..rotated[2]
        },
    ]
}

// ========================== DEPTH VERTEX ==========================
pub const DEPTH_VERTICES: &[DepthVertex] = &[
    // FILL THE WHOLE SCREEN
    DepthVertex {
        position: [-1.0, 1.0, 0.0],
    },
    DepthVertex {
        position: [-1.0, -1.0, 0.0],
    },
    DepthVertex {
        position: [1.0, -1.0, 0.0],
    },
    DepthVertex {
        position: [1.0, -1.0, 0.0],
    },
    DepthVertex {
        position: [1.0, 1.0, 0.0],
    },
    DepthVertex {
        position: [-1.0, 1.0, 0.0],
    },
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DepthVertex {
    position: [f32; 3],
}

impl DepthVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
