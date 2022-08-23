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
use std::rc::Rc;
use std::cell::RefCell;

use iced_wgpu::wgpu;
use wgpu::{
    include_spirv, BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline,
    StencilStateDescriptor,
};

use crate::consts::*;
use crate::utils::{create_buffer_with_data, texture::Texture};
use super::bindgroup_manager::UniformBindGroup;
use ultraviolet::Vec3;

const PLANE_SIZE: i32 = 10;

// TODO make it a parameter
const INTER_HELIX_GAP: f32 = 0.65;
const LINE_COLOR: u32 = 0xA0000000;
const LINE_WIDTH: f32 = INTER_HELIX_GAP / 100.;

#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
struct VertexRaw {
    pub position: [f32 ; 3],
    pub color: [f32 ; 4],
}

impl VertexRaw {
    pub fn buffer_desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<VertexRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[ 
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                }
            ]
        }
    }
}

struct Vertex {
    position: Vec3,
    color: u32,
}

impl Vertex {
    pub fn to_raw(&self) -> VertexRaw {
        VertexRaw {
            position: self.position.into(),
            color: [
                ((self.color & 0xFF0000) >> 16) as f32 / 255.,
                ((self.color & 0xFF00) >> 8) as f32 / 255.,
                (self.color & 0xFF) as f32 / 255.,
                ((self.color & 0xFF000000) >> 24) as f32 / 255.,
            ],
        }
    }

    pub fn new(position: Vec3, color: u32) -> Self {
        Self {
            position,
            color
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    origin: Vec3,
    right: Vec3,
    up: Vec3,
}

impl Plane {
    pub fn new(origin: Vec3, right: Vec3, up: Vec3) -> Self {
        Self {
            origin,
            right,
            up
        }
    }

    fn vertices(&self, color: u32) -> Vec<Vertex> {
        let mut ret = Vec::new();
        let right = -self.right * PLANE_SIZE as f32 * INTER_HELIX_GAP;
        let top = self.up * PLANE_SIZE as f32 * INTER_HELIX_GAP;
        ret.push(Vertex::new(self.origin - right - top, color));
        ret.push(Vertex::new(self.origin - right + top, color));
        ret.push(Vertex::new(self.origin + right - top, color));
        ret.push(Vertex::new(self.origin - right + top, color));
        ret.push(Vertex::new(self.origin + right - top, color));
        ret.push(Vertex::new(self.origin + right + top, color));
        for i in -PLANE_SIZE..PLANE_SIZE {
            let shift = self.right * INTER_HELIX_GAP * i as f32;
            println!("shift {:?}", shift);
            let width = LINE_WIDTH * right;
            let front = 0.001 * self.right.cross(self.up);
            ret.push(Vertex::new(self.origin + shift - top + front, LINE_COLOR));
            ret.push(Vertex::new(self.origin + shift + top + front, LINE_COLOR));
            ret.push(Vertex::new(self.origin + shift + width - top + front, LINE_COLOR));

            ret.push(Vertex::new(self.origin + shift + top + front, LINE_COLOR));
            ret.push(Vertex::new(self.origin + shift + width - top + front, LINE_COLOR));
            ret.push(Vertex::new(self.origin + shift + width + top + front, LINE_COLOR));
        }
        ret
    }

    fn update_buffer(&self, color: u32, vertex_buffer: &mut Option<wgpu::Buffer>, device: Rc<Device>) {
        let raw_vertices = self.vertices(color).iter().map(|v| v.to_raw()).collect::<Vec<_>>();
        *vertex_buffer = Some(create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(raw_vertices.as_slice()),
            wgpu::BufferUsage::VERTEX,
        ));
    }
}



/// A structure that handles a pipepline to draw planes
pub struct PlaneDrawer {
    device: Rc<Device>,
    /// An update in the axis defining the planes to be drawn
    new_plane: Option<Plane>,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    /// The vertices to draw in order to draw the plane
    vertex_buffer: Option<wgpu::Buffer>,
    viewer: Rc<RefCell<UniformBindGroup>>,
    color: u32,
}


impl PlaneDrawer {
    pub fn new(color: u32, viewer: Rc<RefCell<UniformBindGroup>>, device: Rc<Device>) -> Self {
        Self {
            color,
            device,
            new_plane: None,
            vertex_buffer: None,
            pipeline: None,
            viewer,
        }
    }

    pub fn new_plane(&mut self, plane: Option<Plane>) {
        if plane.is_some() {
            self.new_plane = plane;
        } else {
            self.vertex_buffer = None;
        }
    }
        
    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, viewer_bind_group: &'a wgpu::BindGroup) {
        self.update_plane();
        if self.vertex_buffer.is_some() {
            if self.pipeline.is_none() {
                self.pipeline = Some(self.create_pipeline());
            }

            render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.set_bind_group(VIEWER_BINDING_ID, viewer_bind_group, &[]);

            let nb_vertex = 6 * ( 2 * PLANE_SIZE + 1) as u32;
            render_pass.draw(0..nb_vertex, 0..1);
        }
    }

    fn update_plane(&mut self) {
        if let Some(plane) = self.new_plane.take() {
            plane.update_buffer(self.color, &mut self.vertex_buffer, self.device.clone());
        }
    }

    fn create_pipeline(&self) -> RenderPipeline {
        let vertex_module = self.device.create_shader_module(include_spirv!("plane_vert.spv"));
        let fragment_module = self.device.create_shader_module(include_spirv!("plane_frag.spv"));
        let render_pipeline_layout =
            self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &self.viewer.borrow().get_layout(),
                ],
                push_constant_ranges: &[],
                label: Some("render_pipeline_layout"),
            });

        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let color_blend =
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            };

        let alpha_blend =
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            };

        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vertex_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fragment_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                clamp_depth: false,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format,
                color_blend,
                alpha_blend,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[VertexRaw::buffer_desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            label: Some("plane drawer"),
        })
    }
}

