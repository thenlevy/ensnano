
use iced_wgpu::wgpu;
use std::rc::Rc;
use ultraviolet::{Mat4, Vec3, Vec4, Rotor3};
use wgpu::{include_spirv, Device, Queue, RenderPass, RenderPipeline};

use super::{
    bindgroup_manager::{DynamicBindGroup, UniformBindGroup},
    CameraPtr, ProjectionPtr, Uniforms,
};
use crate::consts::*;
use crate::utils::texture::Texture;
use crate::design::Parameters;

mod texture;

#[derive(Debug, Clone, Copy)]
pub enum GridType {
    Square = 0,
    Honeycomb = 1,
}

#[derive(Debug, Clone, Copy)]
pub struct GridInstance {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub grid_type: GridType,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl GridInstance {
    fn to_raw(&self) -> GridInstanceRaw {
        GridInstanceRaw {
            model: Mat4::from_translation(self.position) * self.orientation.into_matrix().into_homogeneous(),
            min_x: self.min_x as f32,
            max_x: self.max_x as f32,
            min_y: self.min_y as f32,
            max_y: self.max_y as f32,
            grid_type: self.grid_type as u32,
            _padding: Vec3::zero(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct GridInstanceRaw {
    pub model: Mat4, // padding 0
    pub min_x: f32, // padding 1
    pub max_x: f32, // padding 2
    pub min_y: f32, // padding 3
    pub max_y: f32, // padding 0
    pub grid_type: u32, // padding 1
    pub _padding: Vec3,
}

unsafe impl bytemuck::Zeroable for GridInstanceRaw {}
unsafe impl bytemuck::Pod for GridInstanceRaw {}

#[repr(C)]
#[derive(Copy, Clone)]
struct ParametersRaw {
    pub helix_radius: f32,
    pub inter_helix_gap: f32,
    pub _padding: [f32 ; 2],
}

unsafe impl bytemuck::Zeroable for ParametersRaw {}
unsafe impl bytemuck::Pod for ParametersRaw {}

impl ParametersRaw {
    pub fn from_parameters(parameters: &Parameters) -> Self {
        Self {
            helix_radius: parameters.helix_radius,
            inter_helix_gap: parameters.inter_helix_gap,
            _padding: [0f32, 0f32],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct ByteMat4(Mat4);

unsafe impl bytemuck::Zeroable for ByteMat4 {}
unsafe impl bytemuck::Pod for ByteMat4 {}

/// A structure that manages the pipepline that draw the grids
pub struct GridDrawer {
    device: Rc<Device>,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<GridInstance>>>,
    /// The number of instance to draw.
    number_instances: usize,
    /// A possible update to the projection and view matrices. Must be taken into acccount before
    /// drawing next frame
    new_viewer_data: Option<Uniforms>,
    /// The data sent the the GPU
    bind_groups: BindGroups,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    square_texture: texture::SquareTexture,
    honney_texture: texture::HonneyTexture,
    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    parameters: Parameters,
}

impl GridDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        camera: &CameraPtr,
        projection: &ProjectionPtr,
        encoder: &mut wgpu::CommandEncoder,
        parameters: Option<Parameters>,
    ) -> Self {
        let instances = DynamicBindGroup::new(device.clone(), queue.clone());

        let mut viewer_data = Uniforms::new();
        viewer_data.update_view_proj(camera.clone(), projection.clone());
        let viewer = UniformBindGroup::new(device.clone(), queue.clone(), &viewer_data);

        let parameters = parameters.unwrap_or_default();
        let parameters_data = ParametersRaw::from_parameters(&parameters);
        let parameters_bg = UniformBindGroup::new(device.clone(), queue.clone(), &parameters_data);

        let bind_groups = BindGroups {
            instances,
            viewer,
            parameters: parameters_bg
        };

        let square_texture = texture::SquareTexture::new(device.clone().as_ref(), encoder);
        let honney_texture = texture::HonneyTexture::new(device.clone().as_ref(), encoder);

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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: true,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
                    resource: wgpu::BindingResource::TextureView(&square_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&square_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&honney_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&honney_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let new_instances = vec![GridInstance {
            position: Vec3::zero(),
            orientation: Rotor3::from_rotation_xz(-std::f32::consts::FRAC_PI_2),
            min_x: 0,
            max_x: 3,
            min_y: 0,
            max_y: 1,
            grid_type: GridType::Honeycomb,
        }];

        Self {
            device,
            new_instances: Some(Rc::new(new_instances)),
            number_instances: 0,
            new_viewer_data: None,
            bind_groups,
            pipeline: None,
            honney_texture,
            square_texture,
            texture_bind_group,
            texture_bind_group_layout,
            parameters,
        }
    }

    /// Request an update of the view and projection matrices. This matrices are provided by the camera and
    /// projection objects.
    /// These new matrices are used on the next frame
    pub fn new_viewer(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        self.new_viewer_data = Some(Uniforms::from_view_proj(camera, projection));
    }

    /// Request an update of the set of instances to draw. This update take effects on the next frame
    pub fn new_instances(&mut self, instances: Rc<Vec<GridInstance>>) {
        self.new_instances = Some(instances)
    }

    /// If one or several update of the set of instances were requested before the last call of
    /// this function, perform the most recent update.
    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().map(|i| i.to_raw()).collect();
            self.bind_groups.update_instances(instances_data.as_slice());
        }
    }

    /// If one or several update of the view and projection matrices were requested before the last call of
    /// this function, perform the most recent update.
    fn update_viewer(&mut self) {
        if let Some(viewer_data) = self.new_viewer_data.take() {
            self.bind_groups.update_viewer(&viewer_data)
        }
    }

    /// Draw the instances of the mesh on the render pass
    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        if self.pipeline.is_none() {
            self.pipeline = Some(self.create_pipeline(self.device.as_ref()));
        }
        self.update_viewer();
        self.update_instances();
        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(
            VIEWER_BINDING_ID,
            self.bind_groups.viewer.get_bindgroup(),
            &[],
        );
        render_pass.set_bind_group(
            INSTANCES_BINDING_ID,
            self.bind_groups.instances.get_bindgroup(),
            &[],
        );
        render_pass.set_bind_group(TEXTURE_BINDING_ID, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(3, &self.bind_groups.parameters.get_bindgroup(), &[]);
        render_pass.draw(0..4, 0..self.number_instances as u32);
    }

    /// Create a render pipepline. This function is meant to be called once, before drawing for the
    /// first time.
    fn create_pipeline(&self, device: &Device) -> RenderPipeline {
        let vertex_module = device.create_shader_module(include_spirv!("grid.vert.spv"));
        let fragment_module = device.create_shader_module(include_spirv!("grid.frag.spv"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &self.bind_groups.viewer.get_layout(),
                    &self.bind_groups.instances.get_layout(),
                    &self.texture_bind_group_layout,
                    &self.bind_groups.parameters.get_layout(),
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

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                vertex_buffers: &[texture::Vertex::desc()],
            },
            sample_count: SAMPLE_COUNT,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            label: Some("render pipeline"),
        })
    }
}

/// Handles the bindgroups and bindgroup layouts of a piepline.
struct BindGroups {
    instances: DynamicBindGroup,
    viewer: UniformBindGroup,
    parameters: UniformBindGroup,
}

impl BindGroups {

    fn update_instances<I: bytemuck::Pod>(&mut self, instances_data: &[I]) {
        self.instances.update(instances_data);
    }

    pub fn update_viewer<U: bytemuck::Pod>(&mut self, viewer_data: &U) {
        self.viewer.update(viewer_data);
    }

    pub fn update_parameters<U: bytemuck::Pod>(&mut self, parameters_data: &U) {
        self.parameters.update(parameters_data);
    }
}
