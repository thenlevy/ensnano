
use std::rc::Rc;
use iced_wgpu::wgpu;
use wgpu::{Device, Queue, RenderPipeline};
use super::{DrawArea, CameraPtr};
use crate::PhySize;
use crate::consts::*;
use crate::utils::bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use crate::utils::texture::Texture;

pub struct View {
    device: Rc<Device>,
    queue: Rc<Queue>,
    background_pipeline: RenderPipeline,
    window_size: PhySize,
    depth_texture: Texture,

}

impl View {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhySize,
        camera: CameraPtr,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(device.as_ref(), &window_size, SAMPLE_COUNT);
        let globals =
            UniformBindGroup::new(device.clone(), queue.clone(), camera.borrow().get_globals());

        let depth_stencil_state = Some(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor::IGNORE,
                back: wgpu::StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        });
        let background_pipeline = background_pipeline(device.clone().as_ref(), depth_stencil_state);
        Self {
            device,
            queue,
            background_pipeline,
            window_size,
            depth_texture,
        }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        area: DrawArea,
    ) {
        let clear_color = wgpu::Color {
            r: 1.,
            g: 0.,
            b: 0.,
            a: 1.,
        };

        let msaa_texture = if SAMPLE_COUNT > 1 {
            Some(crate::utils::texture::Texture::create_msaa_texture(
                self.device.clone().as_ref(),
                &self.window_size,
                SAMPLE_COUNT,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ))
        } else {
            None
        };

        let attachment = if msaa_texture.is_some() {
            msaa_texture.as_ref().unwrap()
        } else {
            target
        };

        let resolve_target = if msaa_texture.is_some() {
            Some(target)
        } else {
            None
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: true,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: true,
                }),
            }),
        });
        render_pass.set_viewport(
            area.position.x as f32,
            area.position.y as f32,
            area.size.width as f32,
            area.size.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            area.position.x,
            area.position.y,
            area.size.width,
            area.size.height,
        );
        render_pass.set_pipeline(&self.background_pipeline);
        render_pass.draw(0..4, 0..1);
    }
}

fn background_pipeline(device: &Device, depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>) -> RenderPipeline {
    let vs_module = &device.create_shader_module(wgpu::include_spirv!("view/background.vert.spv"));
    let fs_module = &device.create_shader_module(wgpu::include_spirv!("view/background.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[],
        push_constant_ranges: &[],
        label: None,
    });

    let desc = wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            ..Default::default()
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: SAMPLE_COUNT,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&desc)
}

