use iced_wgpu::wgpu;
use std::collections::HashMap;
use std::rc::Rc;
use ultraviolet::{Mat2, Vec2};
use wgpu::{include_spirv, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

use crate::consts::*;
use crate::text::{Letter, Vertex as CharVertex};
use crate::utils::bindgroup_manager::DynamicBindGroup;
use crate::utils::texture::Texture;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CharInstance {
    pub center: Vec2,
    pub rotation: Mat2,
    pub size: f32,
    pub z_index: i32,
}

unsafe impl bytemuck::Zeroable for CharInstance {}
unsafe impl bytemuck::Pod for CharInstance {}

pub struct CharDrawer {
    device: Rc<Device>,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<CharInstance>>>,
    /// The number of instance to draw.
    number_instances: usize,
    /// The data sent the the GPU
    instances_bg: DynamicBindGroup,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    letter: Rc<Letter>,
}

impl CharDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        globals_layout: &BindGroupLayout,
        character: char,
    ) -> Self {
        let instances_bg = DynamicBindGroup::new(device.clone(), queue.clone());
        let char_texture = Rc::new(Letter::new(character, device.clone(), queue.clone()));

        let new_instances = vec![CharInstance {
            center: Vec2::zero(),
            rotation: Mat2::identity(),
            z_index: -1,
            size: 1.,
        }];
        let mut ret = Self {
            device,
            new_instances: Some(Rc::new(new_instances)),
            number_instances: 0,
            pipeline: None,
            instances_bg,
            letter: char_texture.clone(),
        };
        let pipeline = ret.create_pipeline(globals_layout);
        ret.pipeline = Some(pipeline);
        ret
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        self.update_instances();
        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(1, self.instances_bg.get_bindgroup(), &[]);
        render_pass.set_bind_group(TEXTURE_BINDING_ID, &self.letter.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.letter.vertex_buffer.slice(..));
        render_pass.draw(0..4, 0..self.number_instances as u32);
    }

    pub fn new_instances(&mut self, instances: Rc<Vec<CharInstance>>) {
        self.new_instances = Some(instances)
    }

    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().cloned().collect();
            self.instances_bg.update(instances_data.as_slice());
        }
    }

    pub fn advancement_x(&self) -> f32 {
        self.letter.advance
    }

    /// Create a render pipepline. This function is meant to be called once, before drawing for the
    /// first time.
    fn create_pipeline(&self, globals_layout: &BindGroupLayout) -> RenderPipeline {
        let vertex_module = self
            .device
            .create_shader_module(include_spirv!("chars.vert.spv"));
        let fragment_module = self
            .device
            .create_shader_module(include_spirv!("chars.frag.spv"));
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        globals_layout,
                        &self.instances_bg.get_layout(),
                        &self.letter.bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                    label: Some("render_pipeline_layout"),
                });

        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let color_blend = wgpu::BlendDescriptor {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };

        let alpha_blend = wgpu::BlendDescriptor {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
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
                primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
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
                    vertex_buffers: &[CharVertex::desc()],
                },
                sample_count: SAMPLE_COUNT,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
                label: Some("render pipeline"),
            })
    }
}

pub fn char_positions(string: String, drawers: &HashMap<char, CharDrawer>) -> Vec<f32> {
    let mut ret = vec![0f32];
    let mut x = 0f32;
    for c in string.chars() {
        x += drawers.get(&c).unwrap().advancement_x();
        ret.push(x);
    }
    ret
}

pub fn height(string: String, drawers: &HashMap<char, CharDrawer>) -> f32 {
    let mut ret = 0f32;
    for c in string.chars() {
        ret = ret.max(drawers.get(&c).unwrap().letter.height)
    }
    ret
}
