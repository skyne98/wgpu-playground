#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub resolution: [f32; 2],
}

impl Uniforms {
    pub fn new(resolution: [f32; 2]) -> Self {
        Self { resolution }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    pub fn as_entire_binding<'a>(&self, buffer: &'a wgpu::Buffer) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset: 0,
            size: None,
        })
    }
}
