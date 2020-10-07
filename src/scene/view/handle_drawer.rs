use std::rc::Rc;
use iced_wgpu::wgpu;
use wgpu::{
    include_spirv, Device, RenderPass, RenderPipeline,
    StencilStateDescriptor,
};

use crate::consts::*;
use crate::utils::{create_buffer_with_data, texture::Texture};
use ultraviolet::Vec3;

#[derive(Debug, Clone, Copy)]
struct VertexRaw {
    pub position: [f32 ; 3],
    pub color: [f32 ; 4],
}

unsafe impl bytemuck::Zeroable for VertexRaw {}
unsafe impl bytemuck::Pod for VertexRaw {}

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
                1.,
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
pub struct Handle {
    pub origin: Vec3,
    pub direction: Vec3,
    pub translation: Vec3,
    normal: Vec3,
    color: u32,
    id: u32,
    length: f32,
}

impl Handle {
    pub fn new(origin: Vec3, direction: Vec3, normal: Vec3, color: u32, id: u32, length: f32) -> Self {
        Self {
            origin,
            direction,
            translation: Vec3::zero(),
            normal,
            color,
            id,
            length,
        }
    }

    fn vertices(&self, fake: bool) -> Vec<Vertex> {
        let mut ret = Vec::new();
        let width = self.length / 30.;
        let color = if fake {
            self.id
        } else {
            self.color
        };
        for x in [-1f32, 1.].iter() {
            for y in [-1., 1.].iter() {
                for z in [0., 1.].iter() {
                    ret.push(Vertex::new(self.origin + self.normal * *x * width + *y * self.direction.cross(self.normal) * width + *z * self.direction * self.length + self.translation,color));
                }
            }
        }
        ret
    }

    fn indices() -> Vec<u16> {
        vec![
            0, 1, 2,
            1, 2, 3,
            0, 1, 5,
            0, 4, 5,
            0, 4, 6,
            0, 6, 2,
            5, 4, 6,
            5, 6, 7,
            2, 6, 7,
            3, 6, 7,
            1, 5, 7,
            1, 3, 7]
    }

    fn update_buffer(&self, vertex_buffer: &mut Option<wgpu::Buffer>, device: Rc<Device>, fake: bool) {
        let raw_vertices = self.vertices(fake).iter().map(|v| v.to_raw()).collect::<Vec<_>>();
        *vertex_buffer = Some(create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(raw_vertices.as_slice()),
            wgpu::BufferUsage::VERTEX,
        ));
    }
}



/// A structure that draw one handle
pub struct HandleDrawer {
    device: Rc<Device>,
    /// An update in the axis defining the planes to be drawn
    new_handle: Option<Handle>,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    /// The pipeline created by `self` for drawing on the fake texture
    pipeline_fake: Option<RenderPipeline>,
    /// The vertices to draw in order to draw the handle
    vertex_buffer: Option<wgpu::Buffer>,
    /// The vertices to draw in order to draw on the fake texture
    fake_vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
}


impl HandleDrawer {
    pub fn new(device: Rc<Device>) -> Self {
        let index_buffer = create_buffer_with_data(
            device.clone().as_ref(),
            bytemuck::cast_slice(Handle::indices().as_slice()),
            wgpu::BufferUsage::INDEX,
        );

        Self {
            device,
            new_handle: None,
            vertex_buffer: None,
            fake_vertex_buffer: None,
            index_buffer,
            pipeline: None,
            pipeline_fake: None,
        }
    }

    pub fn new_handle(&mut self, handle: Option<Handle>) {
        if handle.is_some() {
            self.new_handle = handle;
        } else {
            self.vertex_buffer = None;
            self.fake_vertex_buffer = None;
        }
    }
        
    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, viewer_bind_group: &'a wgpu::BindGroup, viewer_bind_group_layout: &'a wgpu::BindGroupLayout, fake: bool) {
        self.update_handle();
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
                render_pass.set_vertex_buffer(0, self.fake_vertex_buffer.as_ref().unwrap().slice(..));
            } else {
                render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            }
            render_pass.set_index_buffer(self.index_buffer.slice(..));
            render_pass.set_bind_group(VIEWER_BINDING_ID, viewer_bind_group, &[]);

            let nb_index = Handle::indices().len() as u32;
            render_pass.draw_indexed(0..nb_index, 0, 0..1);
        }
    }

    fn update_handle(&mut self) {
        if let Some(handle) = self.new_handle.take() {
            handle.update_buffer(&mut self.vertex_buffer, self.device.clone(), false);
            handle.update_buffer(&mut self.fake_vertex_buffer, self.device.clone(), true);
        }
    }

    fn create_pipeline(&self, viewer_bind_group_layout: &wgpu::BindGroupLayout, fake: bool) -> RenderPipeline {
        let vertex_module = self.device.create_shader_module(include_spirv!("plane_vert.spv"));
        let fragment_module = self.device.create_shader_module(include_spirv!("plane_frag.spv"));
        let render_pipeline_layout =
            self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    viewer_bind_group_layout
                ],
                push_constant_ranges: &[],
                label: Some("render_pipeline_layout"),
            });

        let format = if fake {
            wgpu::TextureFormat::Bgra8Unorm
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

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
            label: Some("render pipeline"),
        })
    }
}

