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
/*
This file contains fragment of code that were originally published in the `lyon` crate
Original source: https://github.com/nical/lyon/blob/master/examples/wgpu/src/main.rs
The original source was distributed under the MIT License by Nicolas Silva.
A copy of the original license is available in thirdparties/lyon/LICENSE
*/

use super::*;
use lyon::geom::*;
use lyon::tessellation::*;
use wgpu::util::DeviceExt;

pub struct Background {
    pipeline: wgpu::RenderPipeline,
    border_pipeline: wgpu::RenderPipeline,
    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,
}

impl Background {
    pub fn new(
        device: &Device,
        globals_layout: &wgpu::BindGroupLayout,
        depth_stencil: &Option<wgpu::DepthStencilState>,
    ) -> Self {
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
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bg_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&bg_geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bg_vs_module =
            &device.create_shader_module(&wgpu::include_spirv!("background.vert.spv"));
        let bg_fs_module =
            &device.create_shader_module(&wgpu::include_spirv!("background.frag.spv"));
        let border_vs_module =
            &device.create_shader_module(&wgpu::include_spirv!("border.vert.spv"));
        let border_fs_module =
            &device.create_shader_module(&wgpu::include_spirv!("border.frag.spv"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[globals_layout],
            push_constant_ranges: &[],
            label: None,
        });

        let targets = &[wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }];

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        };

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bg_vs_module,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BgPoint>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &bg_fs_module,
                entry_point: "main",
                targets,
            }),
            depth_stencil: depth_stencil.clone(),
            primitive: primitive.clone(),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: None,
        });
        let border_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &border_vs_module,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BgPoint>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &border_fs_module,
                entry_point: "main",
                targets,
            }),
            depth_stencil: depth_stencil.clone(),
            primitive,
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: None,
        });

        Self {
            pipeline: bg_pipeline,
            border_pipeline,
            vbo: bg_vbo,
            ibo: bg_ibo,
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.ibo.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vbo.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }

    pub fn draw_border<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.border_pipeline);
        render_pass.set_index_buffer(self.ibo.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vbo.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BgPoint {
    point: [f32; 2],
}

pub struct Custom;

impl FillVertexConstructor<BgPoint> for Custom {
    fn new_vertex(&mut self, vertex: lyon::tessellation::FillVertex) -> BgPoint {
        BgPoint {
            point: vertex.position().to_array(),
        }
    }
}
