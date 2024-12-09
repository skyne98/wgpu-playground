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

// Depth
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
