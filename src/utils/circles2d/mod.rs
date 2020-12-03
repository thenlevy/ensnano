use iced_wgpu::wgpu;
use std::rc::Rc;
use ultraviolet::Vec2;
use wgpu::{include_spirv, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

use crate::consts::*;
use crate::utils::bindgroup_manager::DynamicBindGroup;
use crate::utils::texture::Texture;
mod textures;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CircleInstance {
    pub center: Vec2,
    pub radius: f32,
    _padding: u32,
}

impl CircleInstance {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self {
            center,
            radius,
            _padding: 0,
        }
    }
}

unsafe impl bytemuck::Zeroable for CircleInstance {}
unsafe impl bytemuck::Pod for CircleInstance {}

pub struct CircleDrawer {
    device: Rc<Device>,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<CircleInstance>>>,
    /// The number of instance to draw.
    number_instances: usize,
    /// The data sent the the GPU
    instances_bg: DynamicBindGroup,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl CircleDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        encoder: &mut wgpu::CommandEncoder,
        globals_layout: &BindGroupLayout,
    ) -> Self {
        let instances_bg = DynamicBindGroup::new(device.clone(), queue.clone());
        let circle_texture = textures::CircleTexture::new(device.clone().as_ref(), encoder);
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: true,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&circle_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&circle_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let mut ret = Self {
            device,
            new_instances: None,
            number_instances: 0,
            pipeline: None,
            texture_bind_group,
            texture_bind_group_layout,
            instances_bg,
        };
        let pipeline = ret.create_pipeline(globals_layout);
        ret.pipeline = Some(pipeline);
        ret
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        self.update_instances();
        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(1, self.instances_bg.get_bindgroup(), &[]);
        render_pass.set_bind_group(TEXTURE_BINDING_ID, &self.texture_bind_group, &[]);
        render_pass.draw(0..4, 0..self.number_instances as u32);
    }

    pub fn new_instances(&mut self, instances: Rc<Vec<CircleInstance>>) {
        self.new_instances = Some(instances)
    }

    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().cloned().collect();
            self.instances_bg.update(instances_data.as_slice());
        }
    }

    /// Create a render pipepline. This function is meant to be called once, before drawing for the
    /// first time.
    fn create_pipeline(&self, globals_layout: &BindGroupLayout) -> RenderPipeline {
        let vertex_module = self
            .device
            .create_shader_module(include_spirv!("circle.vert.spv"));
        let fragment_module = self
            .device
            .create_shader_module(include_spirv!("circle.frag.spv"));
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        globals_layout,
                        &self.instances_bg.get_layout(),
                        &self.texture_bind_group_layout,
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
                    vertex_buffers: &[],
                },
                sample_count: SAMPLE_COUNT,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
                label: Some("render pipeline"),
            })
    }
}
