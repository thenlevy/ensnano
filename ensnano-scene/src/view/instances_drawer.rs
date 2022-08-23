/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
//! This modules defines the [Instanciable](Instanciable) trait. Types that implement the
//! `Instanciable` trait can be turned into instances that can be drawn by an
//! [InstanceDrawer](InstanceDrawer).

use ensnano_interactor::consts::*;
use ensnano_utils::bindgroup_manager::DynamicBindGroup;
use ensnano_utils::create_buffer_with_data;
use ensnano_utils::texture::Texture;
use ensnano_utils::wgpu;
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
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
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

    /// This methods allows the ressource tho provide the vertex buffer. If the return value is
    /// Some, it takes priority over the Instanciable's vertices.
    fn vertex_buffer_desc() -> Option<wgpu::VertexBufferLayout<'static>>
    where
        Self: Sized,
    {
        None
    }

    /// This methods allows the ressource tho provide the vertex buffer. If the return value is
    /// Some, it takes priority over the Instanciable's vertices.
    fn vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        None
    }

    /// This methods allows the ressource tho provide the index buffer. If the return value is
    /// Some, it takes priority over the Instanciable's indices.
    fn index_buffer(&self) -> Option<&wgpu::Buffer> {
        None
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

    /// The vertex shader used to draw the outline of the mesh. If this returns `None`, an
    /// `InstanceDrawer` drawing the outline will use `self::vertex_module` instead.
    fn outline_vertex_module(_device: &Device) -> Option<ShaderModule>
    where
        Self: Sized,
    {
        None
    }

    /// The fragment shader used to draw the outline of the mesh. If this returns `None`, an
    /// `InstanceDrawer` drawing the outline will use `self::fragment_module` instead.
    fn outline_fragment_module(_device: &Device) -> Option<ShaderModule>
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

    /// The method can be overwritten to disable depth test
    fn depth_test() -> bool
    where
        Self: Sized,
    {
        true
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
    ressource: D::Ressource,
    device: Rc<Device>,
    label: String,
}

impl<D: Instanciable> InstanceDrawer<D> {
    pub fn new<S: AsRef<str>>(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: &BindGroupLayoutDescriptor<'static>,
        models_desc: &BindGroupLayoutDescriptor<'static>,
        ressource: D::Ressource,
        fake: bool,
        label: S,
    ) -> Self {
        Self::init(
            device,
            queue,
            viewer_desc,
            models_desc,
            ressource,
            fake,
            false,
            false,
            label,
        )
    }

    pub fn new_outliner<S: AsRef<str>>(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: &BindGroupLayoutDescriptor<'static>,
        models_desc: &BindGroupLayoutDescriptor<'static>,
        ressource: D::Ressource,
        label: S,
    ) -> Self {
        Self::init(
            device,
            queue,
            viewer_desc,
            models_desc,
            ressource,
            false,
            false,
            true,
            label,
        )
    }

    pub fn new_wireframe<S: AsRef<str>>(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: &BindGroupLayoutDescriptor<'static>,
        models_desc: &BindGroupLayoutDescriptor<'static>,
        ressource: D::Ressource,
        fake: bool,
        label: S,
    ) -> Self {
        Self::init(
            device,
            queue,
            viewer_desc,
            models_desc,
            ressource,
            fake,
            true,
            false,
            label,
        )
    }

    fn init<S: AsRef<str>>(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: &BindGroupLayoutDescriptor<'static>,
        models_desc: &BindGroupLayoutDescriptor<'static>,
        ressource: D::Ressource,
        fake: bool,
        wireframe: bool,
        outliner: bool,
        label: S,
    ) -> Self {
        let index_buffer = create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(D::indices().as_slice()),
            wgpu::BufferUsages::INDEX,
            format!("{} index buffer", label.as_ref()).as_str(),
        );
        let vertex_buffer = create_buffer_with_data(
            device.as_ref(),
            bytemuck::cast_slice(D::raw_vertices().as_slice()),
            wgpu::BufferUsages::VERTEX,
            format!("{} vertex buffer", label.as_ref()).as_str(),
        );

        let vertex_module = if fake {
            D::fake_vertex_module(&device).unwrap_or_else(|| D::vertex_module(&device))
        } else if outliner {
            D::outline_vertex_module(&device).unwrap_or_else(|| D::vertex_module(&device))
        } else {
            D::vertex_module(&device)
        };

        let fragment_module = if fake {
            D::fake_fragment_module(&device).unwrap_or_else(|| D::fragment_module(&device))
        } else if outliner {
            D::outline_fragment_module(&device).unwrap_or_else(|| D::fragment_module(&device))
        } else {
            D::fragment_module(&device)
        };

        let primitive_topology = if wireframe {
            match D::primitive_topology() {
                PrimitiveTopology::TriangleList => PrimitiveTopology::LineList,
                PrimitiveTopology::TriangleStrip => PrimitiveTopology::LineStrip,
                pt => pt,
            }
        } else {
            D::primitive_topology()
        };
        let label_string = label.as_ref().to_string();

        let pipeline = Self::create_pipeline(
            &device,
            viewer_desc,
            models_desc,
            vertex_module,
            fragment_module,
            primitive_topology,
            fake,
            outliner,
            label,
        );
        let instances = DynamicBindGroup::new(
            device.clone(),
            queue,
            format!("{label_string} instances").as_str(),
        );

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
            ressource,
            device,
            label: label_string,
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
                wgpu::BufferUsages::INDEX,
                format!("{} index buffer", self.label).as_str(),
            );
        }
        if let Some(vertices) = instances.get(0).and_then(D::custom_raw_vertices) {
            self.vertex_buffer = create_buffer_with_data(
                self.device.as_ref(),
                bytemuck::cast_slice(vertices.as_slice()),
                wgpu::BufferUsages::VERTEX,
                format!("{} vertex buffer", self.label).as_str(),
            );
        }
    }

    fn create_pipeline<S: AsRef<str>>(
        device: &Device,
        viewer_bind_group_layout_desc: &wgpu::BindGroupLayoutDescriptor<'static>,
        models_bind_group_layout_desc: &wgpu::BindGroupLayoutDescriptor<'static>,
        vertex_module: ShaderModule,
        fragment_module: ShaderModule,
        primitive_topology: PrimitiveTopology,
        fake: bool,
        outliner: bool,
        label: S,
    ) -> RenderPipeline {
        let viewer_bind_group_layout =
            device.create_bind_group_layout(&viewer_bind_group_layout_desc);
        let models_bind_group_layout =
            device.create_bind_group_layout(&models_bind_group_layout_desc);

        // gather the ressources, [instance, additional ressources]
        let instance_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
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
        let blend_state = if fake {
            wgpu::BlendState::REPLACE
        } else {
            wgpu::BlendState::ALPHA_BLENDING
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
                label: Some(label.as_ref()),
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

        let depth_compare = if D::depth_test() {
            wgpu::CompareFunction::Less
        } else {
            wgpu::CompareFunction::Always
        };
        let targets = &[wgpu::ColorTargetState {
            format,
            blend: Some(blend_state),
            write_mask: wgpu::ColorWrites::ALL,
        }];
        let strip_index_format = match primitive_topology {
            PrimitiveTopology::LineStrip | PrimitiveTopology::TriangleStrip => {
                Some(wgpu::IndexFormat::Uint16)
            }
            _ => None,
        };

        let cull_mode = if outliner {
            Some(wgpu::Face::Front)
        } else {
            None
        };

        let primitive = wgpu::PrimitiveState {
            topology: primitive_topology,
            strip_index_format,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode,
            ..Default::default()
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_module,
                entry_point: "main",
                buffers: &[D::Ressource::vertex_buffer_desc().unwrap_or_else(D::Vertex::desc)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_module,
                entry_point: "main",
                targets,
            }),
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: !fake,
            },
            label: Some(label.as_ref()),
            multiview: None,
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
        if self.nb_instances > 0 {
            let pipeline = &self.pipeline;
            render_pass.set_pipeline(pipeline);
            let vbo = if let Some(ref vbo) = self.ressource.vertex_buffer() {
                vbo.slice(..)
            } else {
                self.vertex_buffer.slice(..)
            };
            render_pass.set_vertex_buffer(0, vbo);
            let ibo = if let Some(ref ibo) = self.ressource.index_buffer() {
                ibo.slice(..)
            } else {
                self.index_buffer.slice(..)
            };
            render_pass.set_index_buffer(ibo, wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, viewer_bind_group, &[]);
            render_pass.set_bind_group(1, model_bind_group, &[]);
            render_pass.set_bind_group(2, self.instances.get_bindgroup(), &[]);
            if let Some(ref additional_bind_group) = self.additional_bind_group {
                render_pass.set_bind_group(3, additional_bind_group, &[]);
            }

            log::trace!("Drawing {}..", self.label);
            render_pass.draw_indexed(0..self.nb_indices, 0, 0..self.nb_instances);
            log::trace!("..Done");
        }
    }
}
