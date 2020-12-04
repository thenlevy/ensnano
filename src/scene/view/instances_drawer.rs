//! This modules defines the [Instanciable](Instanciable) trait. Types that implement the
//! `Instanciable` trait can be turned into instances that can be drawn by an
//! [InstanceDrawer](InstanceDrawer).

use super::drawable::VertexRaw;
use crate::consts::*;
use crate::utils::bindgroup_manager::DynamicBindGroup;
use crate::utils::create_buffer_with_data;
use crate::utils::texture::Texture;
use iced_wgpu::wgpu;
use std::marker::PhantomData;
use std::rc::Rc;
use wgpu::{
    BindGroupLayoutDescriptor, Device, PrimitiveTopology, Queue, RenderPass, RenderPipeline,
    ShaderModule,
};

/// A type that represents a vertex
pub trait Vertexable {
    /// The raw type that is sent to the shaders
    type RawType: bytemuck::Pod + bytemuck::Zeroable;
    /// The vertex state decriptor used to create the pipeline
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
    /// Convert self into a raw vertex.
    fn to_raw(&self) -> Self::RawType;
}

/// A type that provides additional ressources needed to draw a mesh
pub trait RessourceProvider {
    /// Descritpion of the additional ressources (eg textures) needed to draw the mesh.
    fn ressources_layout() -> &'static [wgpu::BindGroupLayoutEntry] {
        &[]
    }
    /// Descritpion of the additional ressources (eg textures) needed to draw the mesh.
    fn ressources(&self) -> &[wgpu::BindGroupEntry] {
        &[]
    }
}

impl RessourceProvider for () {}

/// A type that represents a mesh
pub trait Instanciable {
    /// The type that represents the vertices of the mesh
    type Vertex: Vertexable;
    /// The type that will represents the instance data
    type RawInstance: bytemuck::Pod + bytemuck::Zeroable;
    /// The type that will provide additional ressources needed to draw the mesh
    type Ressource: RessourceProvider;
    /// The vertices of the mesh
    fn vertices() -> Vec<Self::Vertex>;
    /// The indices used to draw the mesh
    fn indices() -> Vec<u16>;
    /// The primitive topology used to draw the mesh
    fn primitive_topology() -> PrimitiveTopology;
    /// The vertex shader used to draw the mesh
    fn vertex_module(device: &Device) -> ShaderModule;
    /// The fragment shader used to draw the mesh
    fn fragment_module(device: &Device) -> ShaderModule;
    /// Return the data that will represent self in the shader
    fn to_raw_instance(&self) -> Self::RawInstance;

    /// Return the content of the vertex buffer
    fn raw_vertices() -> Vec<<Self::Vertex as Vertexable>::RawType> {
        Self::vertices()
            .iter()
            .map(Vertexable::to_raw)
            .collect::<Vec<_>>()
    }
}

/// An object that draws an instanced mesh
pub struct InstanceDrawer<D: Instanciable> {
    /// The pipeline that will render the mesh
    pipeline: RenderPipeline,
    /// The vertex buffer used to draw the mesh
    vertex_buffer: wgpu::Buffer,
    /// The index buffer used to draw the mesh
    index_buffer: wgpu::Buffer,
    /// The bind group containing the instances data
    instances: DynamicBindGroup,
    /// The bind group containing the additional ressources need to draw the mesh
    additional_bind_group: wgpu::BindGroup,
    /// The number of instances
    nb_instances: u32,
    _phantom_data: PhantomData<D>,
}

impl<D: Instanciable> InstanceDrawer<D> {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: BindGroupLayoutDescriptor<'static>,
        models_desc: BindGroupLayoutDescriptor<'static>,
        ressource: D::Ressource,
    ) -> Self {
        let index_buffer = create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(D::indices().as_slice()),
            wgpu::BufferUsage::INDEX,
        );
        let vertex_buffer = create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(D::raw_vertices().as_slice()),
            wgpu::BufferUsage::VERTEX,
        );

        let pipeline = Self::create_pipeline(
            &device,
            viewer_desc,
            models_desc,
            D::vertex_module(&device),
            D::fragment_module(&device),
            D::primitive_topology(),
        );
        let instances = DynamicBindGroup::new(device.clone(), queue);

        let additional_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: D::Ressource::ressources_layout(),
            });

        let additional_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &additional_bind_group_layout,
            entries: ressource.ressources(),
        });

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            instances,
            nb_instances: 0,
            additional_bind_group,
            _phantom_data: PhantomData,
        }
    }

    pub fn new_instances(&mut self, instances: Vec<D>) {
        let raw_instances: Vec<D::RawInstance> =
            instances.iter().map(|d| d.to_raw_instance()).collect();
        self.instances.update(raw_instances.as_slice());
        self.nb_instances = instances.len() as u32;
    }

    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    ) {
        let pipeline = &self.pipeline;
        render_pass.set_pipeline(pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..));
        render_pass.set_bind_group(0, viewer_bind_group, &[]);
        render_pass.set_bind_group(1, model_bind_group, &[]);
        render_pass.set_bind_group(2, self.instances.get_bindgroup(), &[]);
        render_pass.set_bind_group(3, &self.additional_bind_group, &[]);

        let nb_index = D::indices().len() as u32;
        render_pass.draw_indexed(0..nb_index, 0, 0..self.nb_instances);
    }

    fn create_pipeline(
        device: &Device,
        viewer_bind_group_layout_desc: wgpu::BindGroupLayoutDescriptor<'static>,
        models_bind_group_layout_desc: wgpu::BindGroupLayoutDescriptor<'static>,
        vertex_module: ShaderModule,
        fragment_module: ShaderModule,
        primitive_topology: PrimitiveTopology,
    ) -> RenderPipeline {
        let viewer_bind_group_layout =
            device.create_bind_group_layout(&viewer_bind_group_layout_desc);
        let models_bind_group_layout =
            device.create_bind_group_layout(&models_bind_group_layout_desc);

        // gather the ressources, [instance, additional ressources]
        let instance_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::StorageBuffer {
                dynamic: false,
                min_binding_size: None,
                readonly: true,
            },
            count: None,
        };

        let instance_bind_group_layout_desc = BindGroupLayoutDescriptor {
            label: None,
            entries: &[instance_entry],
        };
        let instance_bind_group_layout =
            device.create_bind_group_layout(&instance_bind_group_layout_desc);
        let additional_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: D::Ressource::ressources_layout(),
            });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &viewer_bind_group_layout,
                    &models_bind_group_layout,
                    &instance_bind_group_layout,
                    &additional_bind_group_layout,
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

        let sample_count = SAMPLE_COUNT;

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
            primitive_topology,
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
            sample_count,
            sample_mask: !0,
            alpha_to_coverage_enabled: true,
            label: Some("render pipeline"),
        })
    }
}
