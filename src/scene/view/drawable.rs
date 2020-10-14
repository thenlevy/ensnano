use crate::consts::*;
use crate::utils::create_buffer_with_data;
use crate::utils::texture::Texture;
use iced_wgpu::wgpu;
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
        let raw_vertices = self
            .vertices(fake)
            .iter()
            .map(|v| v.to_raw(Self::use_alpha()))
            .collect::<Vec<_>>();
        *vertex_buffer = Some(create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(raw_vertices.as_slice()),
            wgpu::BufferUsage::VERTEX,
        ));
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
            device.clone().as_ref(),
            bytemuck::cast_slice(D::indices().as_slice()),
            wgpu::BufferUsage::INDEX,
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
            render_pass.set_index_buffer(self.index_buffer.slice(..));
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
            .create_shader_module(include_spirv!("plane_vert.spv"));
        let fragment_module = self
            .device
            .create_shader_module(include_spirv!("plane_frag.spv"));
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

        let color_blend = if !fake {
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            }
        } else {
            wgpu::BlendDescriptor::REPLACE
        };
        let alpha_blend = if !fake {
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            }
        } else {
            wgpu::BlendDescriptor::REPLACE
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                primitive_topology: self.primitive_topology,
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
                    stencil: wgpu::StencilStateDescriptor {
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

#[derive(Debug, Clone, Copy)]
struct VertexRaw {
    pub position: [f32; 3],
    pub color: [f32; 4],
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
                },
            ],
        }
    }
}

pub struct Vertex {
    position: Vec3,
    color: u32,
    fake: bool,
}

impl Vertex {
    fn to_raw(&self, use_alpha: bool) -> VertexRaw {
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
