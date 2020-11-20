use iced_wgpu::wgpu;
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};
use wgpu::{include_spirv, Device, Queue, RenderPass, RenderPipeline};

use super::{
    bindgroup_manager::{DynamicBindGroup, UniformBindGroup},
    CameraPtr, ProjectionPtr, Uniforms,
};
use crate::consts::*;
pub use crate::design::{Grid, GridDivision, GridType, GridTypeDescr, Parameters};
use crate::utils::texture::Texture;

mod texture;

#[derive(Debug, Clone)]
pub struct GridInstance {
    pub grid: Grid,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub color: u32,
    pub design: usize,
    pub id: usize,
}

impl GridInstance {
    fn to_raw(&self) -> GridInstanceRaw {
        use crate::utils::instance::Instance;
        GridInstanceRaw {
            model: Mat4::from_translation(self.grid.position)
                * self.grid.orientation.into_matrix().into_homogeneous(),
            min_x: self.min_x as f32,
            max_x: self.max_x as f32,
            min_y: self.min_y as f32,
            max_y: self.max_y as f32,
            grid_type: self.grid.grid_type.descr() as u32,
            color: Instance::color_from_u32(self.color).truncated(),
        }
    }

    /// Return x >= 0 so that orgin + x axis is on the grid, or None if such an x does not exist.
    fn ray_intersection(&self, origin: Vec3, axis: Vec3) -> Option<GridIntersection> {
        let ret = self.grid.ray_intersection(origin, axis)?;
        if ret < 0. {
            return None;
        }
        let (x, y) = {
            let intersection = origin + ret * axis;
            let vec = intersection - self.grid.position;
            let x_dir = Vec3::unit_z().rotated_by(self.grid.orientation);
            let y_dir = Vec3::unit_y().rotated_by(self.grid.orientation);
            (vec.dot(x_dir), vec.dot(y_dir))
        };
        if self.contains_point(x, y) {
            let (x, y) = self.grid.grid_type.interpolate(&self.grid.parameters, x, y);

            Some(GridIntersection {
                depth: ret,
                grid_id: self.id,
                design_id: self.design,
                x,
                y,
            })
        } else {
            None
        }
    }

    fn convert_coord(&self, x: f32, y: f32) -> (f32, f32) {
        match self.grid.grid_type {
            GridType::Square(_) => {
                let r =
                    self.grid.parameters.helix_radius * 2. + self.grid.parameters.inter_helix_gap;
                (x / r, y / r)
            }
            GridType::Honeycomb(_) => {
                let r =
                    self.grid.parameters.helix_radius * 2. + self.grid.parameters.inter_helix_gap;
                (x * 2. / (3f32.sqrt() * r), (y - r / 2.) * 2. / (3. * r))
            }
        }
    }

    fn contains_point(&self, x: f32, y: f32) -> bool {
        let (x, y) = self.convert_coord(x, y);
        x >= self.min_x as f32 - 0.025
            && x <= self.max_x as f32 + 0.025
            && y >= -self.max_y as f32 - 0.025
            && y <= -self.min_y as f32 + 0.025
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct GridInstanceRaw {
    pub model: Mat4,    // padding 0
    pub min_x: f32,     // padding 1
    pub max_x: f32,     // padding 2
    pub min_y: f32,     // padding 3
    pub max_y: f32,     // padding 0
    pub color: Vec3,    // padding 3
    pub grid_type: u32, // padding 0
}

unsafe impl bytemuck::Zeroable for GridInstanceRaw {}
unsafe impl bytemuck::Pod for GridInstanceRaw {}

#[repr(C)]
#[derive(Copy, Clone)]
struct ParametersRaw {
    pub helix_radius: f32,
    pub inter_helix_gap: f32,
    pub _padding: [f32; 2],
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
    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    instances: Vec<GridInstance>,
    selected: Option<(usize, usize)>,
    candidate: Option<(usize, usize)>,
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
            parameters: parameters_bg,
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

        Self {
            device,
            new_instances: Some(Rc::new(vec![])),
            number_instances: 0,
            new_viewer_data: None,
            bind_groups,
            pipeline: None,
            texture_bind_group,
            texture_bind_group_layout,
            instances: vec![],
            selected: None,
            candidate: None,
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
        if let Some(instances) = self.new_instances.take() {
            self.instances = (*instances).clone();
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
        self.update_colors();
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
            alpha_to_coverage_enabled: true,
            label: Some("render pipeline"),
        })
    }

    pub fn intersect(&self, origin: Vec3, direction: Vec3) -> Option<GridIntersection> {
        let mut ret = None;
        let mut depth = std::f32::INFINITY;
        for (n, g) in self.instances.iter().enumerate() {
            if let Some(intersection) = g.ray_intersection(origin, direction) {
                if intersection.depth < depth {
                    ret = Some(intersection.clone());
                    depth = intersection.depth;
                }
            }
        }
        ret
    }

    pub fn set_candidate_grid(&mut self, grid: Option<(u32, u32)>) {
        self.candidate = grid.map(|(a, b)| (a as usize, b as usize))
    }

    pub fn set_selected_grid(&mut self, grid: Option<(u32, u32)>) {
        self.selected = grid.map(|(a, b)| (a as usize, b as usize))
    }

    fn update_colors(&mut self) {
        for instance in self.instances.iter_mut() {
            if self.selected == Some((instance.design, instance.id)) {
                instance.color = 0xFF_00_00
            } else if self.candidate == Some((instance.design, instance.id)) {
                instance.color = 0x00_FF_00
            } else {
                instance.color = 0x00_00_FF
            }
        }
        let instances_data: Vec<_> = self.instances.iter().map(|i| i.to_raw()).collect();
        self.bind_groups.update_instances(instances_data.as_slice());
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

#[derive(Clone)]
pub struct GridIntersection {
    pub depth: f32,
    pub design_id: usize,
    pub grid_id: usize,
    pub x: isize,
    pub y: isize,
}
