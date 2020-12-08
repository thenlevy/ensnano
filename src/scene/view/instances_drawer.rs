//! This modules defines the [Instanciable](Instanciable) trait. Types that implement the
//! `Instanciable` trait can be turned into instances that can be drawn by an
//! [InstanceDrawer](InstanceDrawer).

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
    fn ressources(&self) -> Vec<wgpu::BindGroupEntry> {
        Vec::new()
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
    /// The vertices of the mesh.
    ///
    /// The vertices must be the same for all the instances drawn by an
    /// `Instanciable`. However, vertices can depend on the particular instantiation of the type
    /// that implements `Instanciable`. In that case, the implementation of `Instanciable` must
    /// overwrite the [`custom_vertices`](`custom_vertices`) method.
    fn vertices() -> Vec<Self::Vertex>
    where
        Self: Sized;
    /// The indices used to draw the mesh.
    ///
    /// The indices must be the same for all the instances drawn by an
    /// `Instanciable`. However, indices can depend on the particular instantiation of the type
    /// that implements `Instanciable`. In that case, the implementation of `Instanciable` must
    /// overwrite the [`custom_indices`](`custom_indices`) method.
    fn indices() -> Vec<u16>
    where
        Self: Sized;
    /// The primitive topology used to draw the mesh
    fn primitive_topology() -> PrimitiveTopology
    where
        Self: Sized;
    /// The vertex shader used to draw the mesh
    fn vertex_module(device: &Device) -> ShaderModule
    where
        Self: Sized;
    /// The fragment shader used to draw the mesh
    fn fragment_module(device: &Device) -> ShaderModule
    where
        Self: Sized;
    /// Return the data that will represent self in the shader
    fn to_raw_instance(&self) -> Self::RawInstance;

    /// Return the content of the vertex buffer
    fn raw_vertices() -> Vec<<Self::Vertex as Vertexable>::RawType>
    where
        Self: Sized,
    {
        Self::vertices()
            .iter()
            .map(Vertexable::to_raw)
            .collect::<Vec<_>>()
    }

    /// Return the vertices of the mesh, if they depends on `self`.
    fn custom_vertices(&self) -> Option<Vec<Self::Vertex>> {
        None
    }

    /// Return the vertices of the mesh, if they depends on `self`.
    fn custom_indices(&self) -> Option<Vec<u16>> {
        None
    }

    /// Return the content of the vertex buffer, or `None` if `custom_vertex` is not overwriten
    fn custom_raw_vertices(&self) -> Option<Vec<<Self::Vertex as Vertexable>::RawType>> {
        self.custom_vertices()
            .map(|v| v.iter().map(Vertexable::to_raw).collect())
    }

    /// The vertex shader used to draw the mesh on fake texture. If this returns `None`, an
    /// `InstanceDrawer` drawing on a fake texture will use `self::vertex_module` instead.
    fn fake_vertex_module(_device: &Device) -> Option<ShaderModule>
    where
        Self: Sized,
    {
        None
    }

    /// The fragment shader used to draw the mesh on fake texture. If this returns `None`, an
    /// `InstanceDrawer` drawing on a fake texture will use `self::fragment_module` instead.
    fn fake_fragment_module(_device: &Device) -> Option<ShaderModule>
    where
        Self: Sized,
    {
        None
    }

    fn alpha_to_coverage_enabled() -> bool
    where
        Self: Sized,
    {
        false
    }
}

/// An object that draws an instanced mesh
pub struct InstanceDrawer<D: Instanciable + ?Sized> {
    /// The pipeline that will render the mesh
    pipeline: RenderPipeline,
    /// The vertex buffer used to draw the mesh
    vertex_buffer: wgpu::Buffer,
    /// The index buffer used to draw the mesh
    index_buffer: wgpu::Buffer,
    /// The bind group containing the instances data
    instances: DynamicBindGroup,
    /// The bind group containing the additional ressources need to draw the mesh
    additional_bind_group: Option<wgpu::BindGroup>,
    /// The number of instances
    nb_instances: u32,
    /// The number of vertex indices
    nb_indices: u32,
    _phantom_data: PhantomData<D>,
    device: Rc<Device>,
}

impl<D: Instanciable> InstanceDrawer<D> {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: &BindGroupLayoutDescriptor<'static>,
        models_desc: &BindGroupLayoutDescriptor<'static>,
        ressource: D::Ressource,
        fake: bool,
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

        let vertex_module = if fake {
            D::fake_vertex_module(&device).unwrap_or_else(|| D::vertex_module(&device))
        } else {
            D::vertex_module(&device)
        };

        let fragment_module = if fake {
            D::fake_fragment_module(&device).unwrap_or_else(|| D::fragment_module(&device))
        } else {
            D::fragment_module(&device)
        };

        let pipeline = Self::create_pipeline(
            &device,
            viewer_desc,
            models_desc,
            vertex_module,
            fragment_module,
            D::primitive_topology(),
            fake,
        );
        let instances = DynamicBindGroup::new(device.clone(), queue);

        let additional_ressources_layout = D::Ressource::ressources_layout();
        let additional_bind_group = if additional_ressources_layout.len() > 0 {
            let additional_bind_group_layout =
                device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: D::Ressource::ressources_layout(),
                });

            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &additional_bind_group_layout,
                entries: ressource.ressources().as_slice(),
            }))
        } else {
            None
        };

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            instances,
            nb_instances: 0,
            nb_indices: D::indices().len() as u32,
            additional_bind_group,
            _phantom_data: PhantomData,
            device,
        }
    }

    pub fn new_instances(&mut self, instances: Vec<D>) {
        let raw_instances: Vec<D::RawInstance> =
            instances.iter().map(|d| d.to_raw_instance()).collect();
        self.instances.update(raw_instances.as_slice());
        self.nb_instances = instances.len() as u32;
        if let Some(indices) = instances.get(0).and_then(D::custom_indices) {
            self.nb_indices = indices.len() as u32;
            self.index_buffer = create_buffer_with_data(
                self.device.as_ref(),
                bytemuck::cast_slice(indices.as_slice()),
                wgpu::BufferUsage::INDEX,
            );
        }
        if let Some(vertices) = instances.get(0).and_then(D::custom_raw_vertices) {
            self.vertex_buffer = create_buffer_with_data(
                self.device.as_ref(),
                bytemuck::cast_slice(vertices.as_slice()),
                wgpu::BufferUsage::VERTEX,
            );
        }
    }

    fn create_pipeline(
        device: &Device,
        viewer_bind_group_layout_desc: &wgpu::BindGroupLayoutDescriptor<'static>,
        models_bind_group_layout_desc: &wgpu::BindGroupLayoutDescriptor<'static>,
        vertex_module: ShaderModule,
        fragment_module: ShaderModule,
        primitive_topology: PrimitiveTopology,
        fake: bool,
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

        // texture displayed on the frame requires to use srgb, texture used for object
        // identification must be in linear format
        let format = if fake {
            wgpu::TextureFormat::Bgra8Unorm
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        // We use alpha blending on texture displayed on the frame. For fake texture we simply rely
        // on depth.
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

        let sample_count = if fake { 1 } else { SAMPLE_COUNT };

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
        let render_pipeline_layout = if D::Ressource::ressources_layout().len() > 0 {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &viewer_bind_group_layout,
                    &models_bind_group_layout,
                    &instance_bind_group_layout,
                    &additional_bind_group_layout,
                ],
                push_constant_ranges: &[],
                label: Some("render_pipeline_layout"),
            })
        } else {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &viewer_bind_group_layout,
                    &models_bind_group_layout,
                    &instance_bind_group_layout,
                ],
                push_constant_ranges: &[],
                label: Some("render_pipeline_layout"),
            })
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
                vertex_buffers: &[D::Vertex::desc()],
            },
            sample_count,
            sample_mask: !0,
            alpha_to_coverage_enabled: !fake,
            label: Some("render pipeline"),
        })
    }
}

pub trait RawDrawer {
    type RawInstance;

    fn draw<'a>(
        &'a mut self,
        render_pass: &mut RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    );

    fn new_instances_raw(&mut self, instances_raw: &Vec<Self::RawInstance>);
}

impl<D: Instanciable> RawDrawer for InstanceDrawer<D> {
    type RawInstance = <D as Instanciable>::RawInstance;

    fn new_instances_raw(&mut self, instances_raw: &Vec<D::RawInstance>) {
        self.nb_instances = instances_raw.len() as u32;
        self.instances.update(instances_raw.as_slice());
    }

    fn draw<'a>(
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
        if let Some(ref additional_bind_group) = self.additional_bind_group {
            render_pass.set_bind_group(3, additional_bind_group, &[]);
        }

        render_pass.draw_indexed(0..self.nb_indices, 0, 0..self.nb_instances);
    }
}
