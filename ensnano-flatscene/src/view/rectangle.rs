/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::wgpu;
use super::Rc;
use ensnano_utils::Ndc;

use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, RenderPipeline};

const SELECT_COLOR: [f32; 4] = [0.26, 0.64, 0.85, 0.6];

pub struct Rectangle {
    corner: Option<Option<[Ndc; 2]>>,
    pipeline: RenderPipeline,
    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,
    queue: Rc<Queue>,
}

#[derive(Default, Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTR_ARRAY,
        }
    }
}

impl Rectangle {
    pub fn new(device: &Device, queue: Rc<Queue>) -> Self {
        let vs_module = device.create_shader_module(&wgpu::include_spirv!("rectangle.vert.spv"));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!("rectangle.frag.spv"));

        let vertices = [Vertex::default(); 4];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let indices = [0u16, 1, 2, 3];

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rectangle Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let targets = &[wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        }];

        let depth_stencil = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        });

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: Some(wgpu::IndexFormat::Uint16),
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rectangle pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets,
            }),
            primitive,
            depth_stencil,
            multisample: wgpu::MultisampleState {
                count: ensnano_interactor::consts::SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            pipeline: render_pipeline,
            corner: None,
            ibo: index_buffer,
            vbo: vertex_buffer,
            queue,
        }
    }

    pub fn update_corners(&mut self, corner: Option<[Ndc; 2]>) {
        self.corner = Some(corner)
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(corners) = self.corner.take() {
            self.update_vertices(corners);
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.ibo.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vbo.slice(..));
        render_pass.draw_indexed(0..4, 0, 0..1);
    }

    fn update_vertices(&mut self, corners: Option<[Ndc; 2]>) {
        let vertices = if let Some([c1, c2]) = corners {
            let min_x = c1.x.min(c2.x);
            let max_x = c1.x.max(c2.x);
            let min_y = c1.y.min(c2.y);
            let max_y = c1.y.max(c2.y);
            [
                Vertex {
                    position: [min_x, min_y],
                    color: SELECT_COLOR,
                },
                Vertex {
                    position: [min_x, max_y],
                    color: SELECT_COLOR,
                },
                Vertex {
                    position: [max_x, min_y],
                    color: SELECT_COLOR,
                },
                Vertex {
                    position: [max_x, max_y],
                    color: SELECT_COLOR,
                },
            ]
        } else {
            [Vertex::default(); 4]
        };
        self.queue
            .write_buffer(&self.vbo, 0, bytemuck::cast_slice(&vertices));
    }
}
