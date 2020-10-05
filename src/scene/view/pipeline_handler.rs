use crate::utils;
use iced_wgpu::wgpu;
use instance::Instance;
use light::create_light;
use mesh::{DrawModel, Mesh, Vertex};
use std::rc::Rc;
use texture::Texture;
use ultraviolet::Mat4;
use utils::{instance, light, mesh, texture};
use wgpu::{
    include_spirv, BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline,
    StencilStateDescriptor,
};

use super::{
    bindgroup_manager::{DynamicBindGroup, UniformBindGroup},
    CameraPtr, ProjectionPtr, Uniforms,
};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct ByteMat4(Mat4);

unsafe impl bytemuck::Zeroable for ByteMat4 {}
unsafe impl bytemuck::Pod for ByteMat4 {}

/// A structure that can create a pipeline which will draw several instances of the same
/// mesh.
pub struct PipelineHandler {
    device: Rc<Device>,
    /// The mesh to be drawn
    mesh: Mesh,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<Instance>>>,
    /// A possible updates to the model matrices. Must be taken into account before drawing
    /// next frame
    new_model_matrices: Option<Rc<Vec<Mat4>>>,
    /// The number of instance to draw.
    number_instances: usize,
    /// A possible update to the projection and view matrices. Must be taken into acccount before
    /// drawing next frame
    new_viewer_data: Option<Uniforms>,
    /// The data sent the the GPU
    bind_groups: BindGroups,
    /// The compiled vertex shader
    vertex_module: wgpu::ShaderModule,
    /// The compiled fragment shader
    fragment_module: wgpu::ShaderModule,
    /// The primitive used for drawing
    primitive_topology: wgpu::PrimitiveTopology,
    /// The kind of pipepline that the pipline is
    flavour: Flavour,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
}

/// The type of pipepline
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flavour {
    /// For drawing tubes and spheres in their real colors
    Real,
    /// For drawing tubes and spheres in a fake color encoding their identifier
    Fake,
    /// For drawing the selection effect
    Selected,
    /// For drawing the "under the cursor" effect
    Candidate,
    /// For drawing the phantom helices
    Phantom,
}

impl PipelineHandler {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        mesh: Mesh,
        camera: &CameraPtr,
        projection: &ProjectionPtr,
        primitive_topology: wgpu::PrimitiveTopology,
        flavour: Flavour,
    ) -> Self {
        let instances = DynamicBindGroup::new(device.clone(), queue.clone());

        let mut viewer_data = Uniforms::new();
        viewer_data.update_view_proj(camera.clone(), projection.clone());
        let viewer = UniformBindGroup::new(device.clone(), queue.clone(), &viewer_data);

        let model_matrices = DynamicBindGroup::new(device.clone(), queue.clone());

        let (light, light_layout) = create_light(device.clone().as_ref());

        let bind_groups = BindGroups {
            instances,
            viewer,
            light,
            light_layout,
            model_matrices,
        };

        let vertex_module = device.create_shader_module(include_spirv!("vert.spv"));
        let fragment_module = match flavour {
            Flavour::Real => device.create_shader_module(include_spirv!("frag.spv")),
            Flavour::Phantom => device.create_shader_module(include_spirv!("phantom.spv")),
            Flavour::Fake => device.create_shader_module(include_spirv!("fake_color.spv")),
            Flavour::Selected => device.create_shader_module(include_spirv!("selected_frag.spv")),
            Flavour::Candidate => device.create_shader_module(include_spirv!("candidate.spv")),
        };

        Self {
            device,
            mesh,
            new_instances: None,
            number_instances: 0,
            new_viewer_data: None,
            new_model_matrices: None,
            bind_groups,
            vertex_module,
            fragment_module,
            primitive_topology,
            flavour,
            pipeline: None,
        }
    }

    pub fn new_viewer(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        self.new_viewer_data = Some(Uniforms::from_view_proj(camera, projection));
    }

    pub fn new_instances(&mut self, instances: Rc<Vec<Instance>>) {
        self.new_instances = Some(instances.clone())
    }

    pub fn new_model_matrices(&mut self, matrices: Rc<Vec<Mat4>>) {
        self.new_model_matrices = Some(matrices)
    }

    pub fn update_model_matrix(&mut self, design_id: usize, matrix: Mat4) {
        self.bind_groups.update_model_matrix(design_id, matrix)
    }

    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances.take() {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().map(|i| i.to_raw()).collect();
            self.bind_groups.update_instances(instances_data.as_slice());
        }
    }

    fn update_viewer(&mut self) {
        if let Some(viewer_data) = self.new_viewer_data.take() {
            self.bind_groups.update_viewer(&viewer_data)
        }
    }

    fn update_model_matrices(&mut self) {
        if let Some(matrices) = self.new_model_matrices.take() {
            let byte_matrices: Vec<_> = matrices.iter().map(|m| ByteMat4(*m)).collect();
            self.bind_groups
                .update_model_matrices(byte_matrices.as_slice())
        }
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        if self.pipeline.is_none() {
            self.pipeline = Some(self.create_pipeline(self.device.as_ref()));
        }
        self.update_instances();
        self.update_viewer();
        self.update_model_matrices();
        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());

        render_pass.draw_mesh_instanced(
            &self.mesh,
            0..self.number_instances as u32,
            self.bind_groups.viewer.get_bindgroup(),
            &self.bind_groups.instances.get_bindgroup(),
            &self.bind_groups.light,
            &self.bind_groups.model_matrices.get_bindgroup(),
        );
    }

    fn create_pipeline(&self, device: &Device) -> RenderPipeline {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &self.bind_groups.viewer.get_layout(),
                    &self.bind_groups.instances.get_layout(),
                    &self.bind_groups.light_layout,
                    &self.bind_groups.model_matrices.get_layout(),
                ],
                push_constant_ranges: &[],
                label: Some("render_pipeline_layout"),
            });

        let format = match self.flavour {
            Flavour::Fake => wgpu::TextureFormat::Bgra8Unorm,
            _ => wgpu::TextureFormat::Bgra8UnormSrgb,
        };

        let color_blend = if self.flavour != Flavour::Fake {
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            }
        } else {
            wgpu::BlendDescriptor::REPLACE
        };
        let alpha_blend = if self.flavour != Flavour::Fake {
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            }
        } else {
            wgpu::BlendDescriptor::REPLACE
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &self.vertex_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &self.fragment_module,
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
                stencil: StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[mesh::MeshVertex::desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            label: Some("render pipeline"),
        })
    }
}

struct BindGroups {
    instances: DynamicBindGroup,
    viewer: UniformBindGroup,
    light: BindGroup,
    light_layout: BindGroupLayout,
    model_matrices: DynamicBindGroup,
}

impl BindGroups {
    fn update_model_matrices<M: bytemuck::Pod>(&mut self, matrices: &[M]) {
        self.model_matrices.update(matrices);
    }

    fn update_model_matrix(&mut self, design_id: usize, matrix: Mat4) {
        let byte_mat = ByteMat4(matrix);
        let matrix_bytes = bytemuck::bytes_of(&byte_mat);
        let offset = design_id * matrix_bytes.len();
        self.model_matrices.update_offset(offset, matrix_bytes)
    }

    fn update_instances<I: bytemuck::Pod>(&mut self, instances_data: &[I]) {
        self.instances.update(instances_data);
    }

    pub fn update_viewer<U: bytemuck::Pod>(&mut self, viewer_data: &U) {
        self.viewer.update(viewer_data);
    }
}
