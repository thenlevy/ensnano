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
use super::SAMPLE_COUNT;
use ensnano_design::ultraviolet;
use ensnano_interactor::consts::*;
use ensnano_utils::create_buffer_with_data;
use ensnano_utils::texture::Texture;
use ensnano_utils::wgpu;
use std::rc::Rc;
use ultraviolet::Vec3;
use wgpu::{include_spirv, Device, RenderPass, RenderPipeline};

pub trait Drawable {
    fn indices() -> Vec<u16>;
    fn vertices(&self, fake: bool) -> Vec<Vertex>;
    fn primitive_topology() -> wgpu::PrimitiveTopology;

    fn use_alpha() -> bool {
        false
    }
    fn update_buffer(
        &self,
        vertex_buffer: &mut Option<wgpu::Buffer>,
        device: Rc<Device>,
        fake: bool,
    ) {
        *vertex_buffer = Some(create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(self.raw_vertices(fake).as_slice()),
            wgpu::BufferUsages::VERTEX,
            "drawable vertex",
        ));
    }
    fn raw_vertices(&self, fake: bool) -> Vec<VertexRaw> {
        self.vertices(fake)
            .iter()
            .map(|v| v.to_raw(Self::use_alpha()))
            .collect::<Vec<_>>()
    }
}

/// A structure that draw one object
pub struct Drawer<D: Drawable> {
    device: Rc<Device>,
    /// An update in the axis defining the planes to be drawn
    new_object: Option<D>,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    /// The pipeline created by `self` for drawing on the fake texture
    pipeline_fake: Option<RenderPipeline>,
    /// The vertices to draw in order to draw the object
    vertex_buffer: Option<wgpu::Buffer>,
    /// The vertices to draw in order to draw on the fake texture
    fake_vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
    primitive_topology: wgpu::PrimitiveTopology,
}

impl<D: Drawable> Drawer<D> {
    pub fn new(device: Rc<Device>) -> Self {
        let index_buffer = create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(D::indices().as_slice()),
            wgpu::BufferUsages::INDEX,
            "drawable index",
        );

        Self {
            device,
            new_object: None,
            vertex_buffer: None,
            fake_vertex_buffer: None,
            index_buffer,
            pipeline: None,
            pipeline_fake: None,
            primitive_topology: D::primitive_topology(),
        }
    }

    pub fn new_object(&mut self, object: Option<D>) {
        if object.is_some() {
            self.new_object = object;
        } else {
            self.vertex_buffer = None;
            self.fake_vertex_buffer = None;
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
        viewer_bind_group_layout: &'a wgpu::BindGroupLayout,
        fake: bool,
    ) {
        self.update_object();
        if self.vertex_buffer.is_some() {
            let pipeline = if fake {
                if self.pipeline_fake.is_none() {
                    self.pipeline_fake = Some(self.create_pipeline(viewer_bind_group_layout, true))
                }
                self.pipeline_fake.as_ref().unwrap()
            } else {
                if self.pipeline.is_none() {
                    self.pipeline = Some(self.create_pipeline(viewer_bind_group_layout, false));
                }
                self.pipeline.as_ref().unwrap()
            };

            render_pass.set_pipeline(pipeline);
            if fake {
                render_pass
                    .set_vertex_buffer(0, self.fake_vertex_buffer.as_ref().unwrap().slice(..));
            } else {
                render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            }
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(VIEWER_BINDING_ID, viewer_bind_group, &[]);

            let nb_index = D::indices().len() as u32;
            render_pass.draw_indexed(0..nb_index, 0, 0..1);
        }
    }

    fn update_object(&mut self) {
        if let Some(object) = self.new_object.take() {
            object.update_buffer(&mut self.vertex_buffer, self.device.clone(), false);
            object.update_buffer(&mut self.fake_vertex_buffer, self.device.clone(), true);
        }
    }

    fn create_pipeline(
        &self,
        viewer_bind_group_layout: &wgpu::BindGroupLayout,
        fake: bool,
    ) -> RenderPipeline {
        let vertex_module = self
            .device
            .create_shader_module(&include_spirv!("plane_vert.spv"));
        let fragment_module = self
            .device
            .create_shader_module(&include_spirv!("plane_frag.spv"));
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[viewer_bind_group_layout],
                    push_constant_ranges: &[],
                    label: Some("render_pipeline_layout"),
                });

        let format = if fake {
            wgpu::TextureFormat::Bgra8Unorm
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        let blend_state = if fake {
            wgpu::BlendState::REPLACE
        } else {
            wgpu::BlendState::ALPHA_BLENDING
        };

        let sample_count = if !fake { SAMPLE_COUNT } else { 1 };

        let targets = &[wgpu::ColorTargetState {
            format,
            blend: Some(blend_state),
            write_mask: wgpu::ColorWrites::ALL,
        }];
        let strip_index_format = match self.primitive_topology {
            wgpu::PrimitiveTopology::LineStrip | wgpu::PrimitiveTopology::TriangleStrip => {
                Some(wgpu::IndexFormat::Uint16)
            }
            _ => None,
        };

        let primitive = wgpu::PrimitiveState {
            topology: self.primitive_topology,
            strip_index_format,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_module,
                    entry_point: "main",
                    buffers: &[VertexRaw::buffer_desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_module,
                    entry_point: "main",
                    targets,
                }),
                primitive,
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                label: Some("render pipeline"),
                multiview: None,
            })
    }
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VertexRaw {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
impl VertexRaw {
    pub fn buffer_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTR_ARRAY,
        }
    }
}

pub struct Vertex {
    pub position: Vec3,
    pub color: u32,
    pub fake: bool,
}

impl Vertex {
    pub fn to_raw(&self, use_alpha: bool) -> VertexRaw {
        let alpha = if use_alpha || self.fake {
            ((self.color & 0xFF000000) >> 24) as f32 / 255.
        } else {
            1.
        };
        VertexRaw {
            position: self.position.into(),
            color: [
                ((self.color & 0xFF0000) >> 16) as f32 / 255.,
                ((self.color & 0xFF00) >> 8) as f32 / 255.,
                (self.color & 0xFF) as f32 / 255.,
                alpha,
            ],
        }
    }

    pub fn new(position: Vec3, color: u32, fake: bool) -> Self {
        Self {
            position,
            color,
            fake,
        }
    }
}
