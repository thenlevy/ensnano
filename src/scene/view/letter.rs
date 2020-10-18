use std::rc::Rc;
use iced_wgpu::wgpu;
use wgpu::{RenderPass, Device, Queue, RenderPipeline, include_spirv};
use ultraviolet::{Vec3, Vec4, Mat4, Mat3};

use crate::utils::{instance::InstanceRaw, texture::Texture};
use crate::text::{Vertex, Letter};
use crate::consts::*;
use super::{
    bindgroup_manager::{DynamicBindGroup, UniformBindGroup},
    CameraPtr, ProjectionPtr, Uniforms,
};

#[derive(Debug, Clone)]
pub struct LetterInstance {
    pub position: Vec3,
    pub color: Vec4,
}

impl LetterInstance {
    pub fn seen_by(&self, camera: CameraPtr) -> InstanceRaw {
        let matrix = Mat3::new(camera.borrow().right_vec(), camera.borrow().up_vec(), -camera.borrow().direction());
        InstanceRaw {
            model: Mat4::from_translation(self.position)
                * matrix.into_homogeneous(),
            color: self.color,
            // We do not draw the letters on fake textures
            id: Vec4::zero(),
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct ByteMat4(Mat4);

unsafe impl bytemuck::Zeroable for ByteMat4 {}
unsafe impl bytemuck::Pod for ByteMat4 {}

/// A structure that can create a pipeline which will draw several instances of the same
/// letter.
pub struct LetterDrawer {
    device: Rc<Device>,
    /// The letter to be drawn
    letter: Letter,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<LetterInstance>>>,
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
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    /// The viewer of the letter
    camera: CameraPtr,
}


impl LetterDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        character: char,
        camera: &CameraPtr,
        projection: &ProjectionPtr,
    ) -> Self {
        let letter = Letter::new(character, device.clone(), queue.clone());
        let instances = DynamicBindGroup::new(device.clone(), queue.clone());

        let mut viewer_data = Uniforms::new();
        viewer_data.update_view_proj(camera.clone(), projection.clone());
        let viewer = UniformBindGroup::new(device.clone(), queue.clone(), &viewer_data);

        let model_matrices = DynamicBindGroup::new(device.clone(), queue.clone());


        let bind_groups = BindGroups {
            instances,
            viewer,
            model_matrices,
        };


        Self {
            device,
            letter,
            new_instances: None,
            number_instances: 0,
            new_viewer_data: None,
            new_model_matrices: None,
            bind_groups,
            pipeline: None,
            camera: camera.clone(),
        }
    }


    /// Request an update of the view and projection matrices. This matrices are provided by the camera and
    /// projection objects.
    /// These new matrices are used on the next frame
    pub fn new_viewer(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        self.new_viewer_data = Some(Uniforms::from_view_proj(camera, projection));
    }

    /// Request an update of the set of instances to draw. This update take effects on the next frame
    pub fn new_instances(&mut self, instances: Rc<Vec<LetterInstance>>) {
        self.new_instances = Some(instances.clone())
    }

    /// Request an update all the model matrices
    pub fn new_model_matrices(&mut self, matrices: Rc<Vec<Mat4>>) {
        self.new_model_matrices = Some(matrices)
    }

    /// Request an update of a single model matrix
    pub fn update_model_matrix(&mut self, design_id: usize, matrix: Mat4) {
        self.bind_groups.update_model_matrix(design_id, matrix)
    }

    /// If one or several update of the set of instances were requested before the last call of
    /// this function, perform the most recent update.
    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances.take() {
            println!("updating instance");
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().map(|i| i.seen_by(self.camera.clone())).collect();
            self.bind_groups.update_instances(instances_data.as_slice());
            println!("ok");
        }
    }

    /// If one or several update of the view and projection matrices were requested before the last call of
    /// this function, perform the most recent update.
    fn update_viewer(&mut self) {
        if let Some(viewer_data) = self.new_viewer_data.take() {
            self.bind_groups.update_viewer(&viewer_data)
        }
    }

    /// If one or several update of the model matrices were requested before the last call of
    /// this function, perform the most recent update.
    fn update_model_matrices(&mut self) {
        if let Some(matrices) = self.new_model_matrices.take() {
            let byte_matrices: Vec<_> = matrices.iter().map(|m| ByteMat4(*m)).collect();
            self.bind_groups
                .update_model_matrices(byte_matrices.as_slice())
        }
    }

    /// Draw the instances of the mesh on the render pass
    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        if self.pipeline.is_none() {
            self.pipeline = Some(self.create_pipeline(self.device.as_ref()));
        }
        self.update_instances();
        self.update_viewer();
        self.update_model_matrices();
        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(VIEWER_BINDING_ID, self.bind_groups.viewer.get_bindgroup(), &[]);
        render_pass.set_bind_group(INSTANCES_BINDING_ID, self.bind_groups.instances.get_bindgroup(), &[]);
        render_pass.set_bind_group(TEXTURE_BINDING_ID, &self.letter.bind_group, &[]);
        render_pass.set_bind_group(MODEL_BINDING_ID, self.bind_groups.model_matrices.get_bindgroup(), &[]);
        render_pass.set_vertex_buffer(0, self.letter.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.letter.index_buffer.slice(..));

        render_pass.draw_indexed(0..4, 0, 0..self.number_instances as u32);
    }

    /// Create a render pipepline. This function is meant to be called once, before drawing for the
    /// first time.
    fn create_pipeline(&self, device: &Device) -> RenderPipeline {
        let vertex_module = device.create_shader_module(include_spirv!("letter.vert.spv"));
        let fragment_module = device.create_shader_module(include_spirv!("letter.frag.spv"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &self.bind_groups.viewer.get_layout(),
                    &self.bind_groups.instances.get_layout(),
                    &self.letter.bind_group_layout,
                    &self.bind_groups.model_matrices.get_layout(),
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
                vertex_buffers: &[Vertex::desc()],
            },
            sample_count: 1,
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

