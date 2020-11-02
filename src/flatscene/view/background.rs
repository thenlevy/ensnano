/*
This file contains fragment of code that were originally published in the `lyon` crate
Original source: https://github.com/nical/lyon/blob/master/examples/wgpu/src/main.rs
The original source was distributed under the MIT License by Nicolas Silva.
A copy of the original license is available in thirdparties/lyon/LICENSE
*/

use super::*;
use lyon::tessellation::*;
use lyon::geom::*;
use wgpu::util::DeviceExt;

pub struct Background {
    pipeline: wgpu::RenderPipeline,
    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,
}


impl Background {
    pub fn new(device: &Device, globals_layout: &wgpu::BindGroupLayout, depth_stencil_state: &Option<wgpu::DepthStencilStateDescriptor>) -> Self {
        let mut bg_geometry: VertexBuffers<BgPoint, u16> = VertexBuffers::new();
        let mut fill_tess = FillTessellator::new();

        fill_tess
            .tessellate_rectangle(
                &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(&mut bg_geometry, Custom),
            )
            .unwrap();

        let bg_vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&bg_geometry.vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let bg_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&bg_geometry.indices),
            usage: wgpu::BufferUsage::INDEX,
        });

        let bg_vs_module =
            &device.create_shader_module(wgpu::include_spirv!("background.vert.spv"));
        let bg_fs_module =
            &device.create_shader_module(wgpu::include_spirv!("background.frag.spv"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[globals_layout],
            push_constant_ranges: &[],
            label: None,
        });

        
        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &bg_vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &bg_fs_module,
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
            depth_stencil_state: depth_stencil_state.clone(),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<BgPoint>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 0,
                    }],
                }],
            },
            sample_count: SAMPLE_COUNT,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            label: None,
        });

        Self {
            pipeline: bg_pipeline,
            vbo: bg_vbo,
            ibo: bg_ibo,
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.ibo.slice(..));
        render_pass.set_vertex_buffer(0, self.vbo.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }

}


#[repr(C)]
#[derive(Copy, Clone)]
struct BgPoint {
    point: [f32; 2],
}
unsafe impl bytemuck::Pod for BgPoint {}
unsafe impl bytemuck::Zeroable for BgPoint {}


pub struct Custom;

impl FillVertexConstructor<BgPoint> for Custom {
    fn new_vertex(&mut self, vertex: lyon::tessellation::FillVertex) -> BgPoint {
        BgPoint {
            point: vertex.position().to_array(),
        }
    }
}
