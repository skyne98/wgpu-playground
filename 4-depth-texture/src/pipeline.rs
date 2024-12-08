use std::num::NonZero;

use wgpu::PrimitiveState;

pub struct GPUPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_pipeline_layout: wgpu::PipelineLayout,
}

impl GPUPipeline {
    pub fn new(
        render_pipeline_layout: wgpu::PipelineLayout,
        render_pipeline: wgpu::RenderPipeline,
    ) -> Self {
        Self {
            render_pipeline,
            render_pipeline_layout,
        }
    }
}

// Define the GPUPipelineBuilder struct
pub struct GPUPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<&'a str>,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    vertex_shader: Option<(&'a wgpu::ShaderModule, &'a str)>,
    fragment_shader: Option<(&'a wgpu::ShaderModule, &'a str)>,
    vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    color_targets: Vec<Option<wgpu::ColorTargetState>>,
    primitive_state: Option<wgpu::PrimitiveState>,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    multisample_state: Option<wgpu::MultisampleState>,
    multiview: Option<NonZero<u32>>,
}

impl<'a> GPUPipelineBuilder<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            device,
            label: None,
            bind_group_layouts: vec![],
            vertex_shader: None,
            fragment_shader: None,
            vertex_buffers: vec![],
            color_targets: vec![],
            primitive_state: None,
            depth_stencil_state: None,
            multisample_state: None,
            multiview: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn bind_group_layout(mut self, layout: &'a wgpu::BindGroupLayout) -> Self {
        self.bind_group_layouts.push(layout);
        self
    }
    pub fn vertex_shader(mut self, shader: &'a wgpu::ShaderModule, entry_point: &'a str) -> Self {
        self.vertex_shader = Some((shader, entry_point));
        self
    }
    pub fn fragment_shader(mut self, shader: &'a wgpu::ShaderModule, entry_point: &'a str) -> Self {
        self.fragment_shader = Some((shader, entry_point));
        self
    }
    pub fn vertex_buffer_layout(mut self, layout: wgpu::VertexBufferLayout<'a>) -> Self {
        self.vertex_buffers.push(layout);
        self
    }
    pub fn color_target(mut self, target: wgpu::ColorTargetState) -> Self {
        self.color_targets.push(Some(target));
        self
    }
    pub fn primitive_state(mut self, state: wgpu::PrimitiveState) -> Self {
        self.primitive_state = Some(state);
        self
    }
    pub fn depth_stencil_state(mut self, state: Option<wgpu::DepthStencilState>) -> Self {
        self.depth_stencil_state = state;
        self
    }
    pub fn multisample_state(mut self, state: wgpu::MultisampleState) -> Self {
        self.multisample_state = Some(state);
        self
    }
    pub fn multiview(mut self, multiview: NonZero<u32>) -> Self {
        self.multiview = Some(multiview);
        self
    }

    // Utilities
    pub fn default_color_target(mut self, format: wgpu::TextureFormat) -> Self {
        self.color_targets.push(Some(wgpu::ColorTargetState {
            format: format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }));
        self
    }
    pub fn default_depth_stencil_state(mut self) -> Self {
        self.depth_stencil_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        self
    }
    pub fn depth_stencil_disabled(mut self) -> Self {
        self.depth_stencil_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        self
    }
    pub fn default_multisample_state(mut self) -> Self {
        self.multisample_state = Some(wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        });
        self
    }
    pub fn default_primitive_state(mut self) -> Self {
        self.primitive_state = Some(PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        });
        self
    }

    pub fn build(self) -> Result<GPUPipeline, &'static str> {
        if self.vertex_shader.is_none() {
            return Err("Vertex shader is required");
        }
        let vertex_shader = self.vertex_shader.expect("Vertex shader is required");

        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: self.label,
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            });

        let vertex_state = wgpu::VertexState {
            module: vertex_shader.0,
            entry_point: Some(vertex_shader.1),
            buffers: &self.vertex_buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        let fragment_state = self
            .fragment_shader
            .map(|(shader, entry)| wgpu::FragmentState {
                module: shader,
                entry_point: Some(entry),
                targets: &self.color_targets,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });

        let render_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: self.label,
                layout: Some(&layout),
                vertex: vertex_state,
                fragment: fragment_state,
                primitive: self
                    .primitive_state
                    .unwrap_or(wgpu::PrimitiveState::default()),
                depth_stencil: self.depth_stencil_state,
                multisample: self
                    .multisample_state
                    .unwrap_or(wgpu::MultisampleState::default()),
                multiview: self.multiview,
                cache: None,
            });

        Ok(GPUPipeline::new(layout, render_pipeline))
    }
}
