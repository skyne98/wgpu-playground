use anyhow::Result;
use bevy_ecs::{
    schedule::{Condition, IntoSystemConfigs, Schedule},
    system::{IntoSystem, Res},
    world::World,
};
use tracing::error;
use tracing_tracy::client::frame_name;

use crate::{
    gpu::GpuContext,
    time::TimeContext,
    vertex::{self, VertexBuffers},
    ResizeState,
};

use super::{
    depth::{DepthBindGroup, DepthPipeline, DepthTexture},
    diffuse::{DiffuseBindGroup, DiffusePipeline},
    present::{FrameBuffer, PresentBindGroup, PresentPipeline},
};

pub fn setup_rendering(_world: &mut World, schedule: &mut Schedule) -> Result<()> {
    schedule.add_systems(render_system);
    Ok(())
}

pub fn render_system(
    time: Res<TimeContext>,
    gpu: Res<GpuContext>,
    depth: Res<DepthTexture>,
    diffuse_bind_group: Res<DiffuseBindGroup>,
    diffuse_pipeline: Res<DiffusePipeline>,
    depth_bind_group: Res<DepthBindGroup>,
    depth_pipeline: Res<DepthPipeline>,
    present_bind_group: Res<PresentBindGroup>,
    present_pipeline: Res<PresentPipeline>,
    vertex_buffers: Res<VertexBuffers>,
    frame_buffer: Res<FrameBuffer>,
) {
    let f = || -> Result<()> {
        let _render_guard = tracing_tracy::client::Client::running()
            .expect("client must be running")
            .non_continuous_frame(frame_name!("rendering"));

        let output = gpu.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());

        // Update the vertex buffer with new data
        let new_vertices = vertex::rotated_vertices(time.total);
        gpu.queue.write_buffer(
            &vertex_buffers.vertex_buffer,
            0,
            bytemuck::cast_slice(&new_vertices),
        );

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // DRAWING DIFFUSE
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("diffuse_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_buffer.texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&diffuse_pipeline.pipeline.render_pipeline);
            render_pass.set_bind_group(0, &diffuse_bind_group.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffers.vertex_buffer.slice(..));
            render_pass.draw(0..vertex_buffers.num_vertices, 0..1);
        }

        // DRAWING DEPTH
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("depth_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_buffer.texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&depth_pipeline.pipeline.render_pipeline);
            render_pass.set_bind_group(0, &depth_bind_group.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffers.depth_vertex_buffer.slice(..));
            render_pass.draw(0..vertex_buffers.num_depth_vertices, 0..1);
        }

        // PRESENT
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&present_pipeline.pipeline.render_pipeline);
            render_pass.set_bind_group(0, &present_bind_group.bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        drop(_render_guard);

        let _present_guard = tracing_tracy::client::Client::running()
            .expect("client must be running")
            .non_continuous_frame(frame_name!("presenting"));
        output.present();
        drop(_present_guard);

        tracing_tracy::client::Client::running()
            .expect("client must be running")
            .frame_mark();

        Ok(())
    };

    if let Err(e) = f() {
        // error!("Error during rendering: {:?}", e);
    }
}
