use anyhow::{Context, Result};

pub struct RenderPassBuilder<'a> {
    encoder: &'a mut wgpu::CommandEncoder,
    label: Option<&'a str>,
    color_view: Option<&'a wgpu::TextureView>,
    depth_view: Option<(&'a wgpu::TextureView, f32)>,
}

impl<'a> RenderPassBuilder<'a> {
    pub fn new(encoder: &'a mut wgpu::CommandEncoder) -> Self {
        Self {
            encoder,
            label: None,
            color_view: None,
            depth_view: None,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_color_view(mut self, view: &'a wgpu::TextureView) -> Self {
        self.color_view = Some(view);
        self
    }

    pub fn with_depth(mut self, view: &'a wgpu::TextureView, clear_value: f32) -> Self {
        self.depth_view = Some((view, clear_value));
        self
    }

    pub fn build(self) -> Result<wgpu::RenderPass<'a>> {
        let color_view = self.color_view.context("No color attachment provided")?;

        let depth_stencil_attachment =
            self.depth_view.map(
                |(view, clear_value)| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_value),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                },
            );

        Ok(self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        }))
    }
}
