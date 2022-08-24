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
use super::wgpu;
use ensnano_design::{External3DObject, External3DObjectId};
use ensnano_interactor::consts;
use ensnano_utils::{create_buffer_with_data, obj_loader::*, texture::Texture, TEXTURE_FORMAT};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::rc::Rc;
use std::{collections::BTreeMap, path::Path};
use wgpu::{BindGroupLayoutDescriptor, Device};

pub struct Object3DDrawer {
    gltf_drawers: BTreeMap<External3DObjectId, GltfDrawer>,
    stl_drawers: BTreeMap<External3DObjectId, StlDrawer>,
    device: Rc<Device>,
}

impl Object3DDrawer {
    pub fn new(device: Rc<Device>) -> Self {
        Self {
            gltf_drawers: Default::default(),
            stl_drawers: Default::default(),
            device,
        }
    }
}

#[derive(Debug)]
pub struct ExternalObjects {
    pub path_base: PathBuf,
    pub objects: Vec<(External3DObjectId, External3DObject)>,
}

impl Object3DDrawer {
    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
    ) {
        for d in self.gltf_drawers.values_mut() {
            d.draw(render_pass, viewer_bind_group)
        }
        for d in self.stl_drawers.values_mut() {
            d.draw(render_pass, viewer_bind_group)
        }
    }

    pub fn update_objects(
        &mut self,
        objects: ExternalObjects,
        bg_desc: &BindGroupLayoutDescriptor,
    ) {
        for (obj_id, object) in objects.objects.into_iter() {
            if !self.stl_drawers.contains_key(&obj_id) && !self.gltf_drawers.contains_key(&obj_id) {
                self.add_object(obj_id, object, &objects.path_base, bg_desc);
            }
        }
    }

    fn update_object(&mut self, _id: External3DObjectId, _object: External3DObject) {
        //TODO update object attributes
    }

    fn add_object(
        &mut self,
        id: External3DObjectId,
        object: External3DObject,
        base_path: &PathBuf,
        bg_desc: &BindGroupLayoutDescriptor,
    ) {
        let path = object.get_path_to_source_file(base_path);
        println!("{:?}", path);
        if path.extension() == Some(OsStr::new("stl")) {
            let mut drawer = StlDrawer::new(self.device.as_ref(), bg_desc);
            drawer.add_stl(self.device.as_ref(), path);
            self.stl_drawers.insert(id, drawer);
        } else if path.extension() == Some(OsStr::new("gltf")) {
            let mut drawer = GltfDrawer::new(self.device.as_ref(), bg_desc);
            drawer.add_gltf(self.device.as_ref(), path);
            self.gltf_drawers.insert(id, drawer);
        }
    }
}

pub struct GltfDrawer {
    vbos: Vec<wgpu::Buffer>,
    ibos: Vec<wgpu::Buffer>,
    nb_idx: Vec<u32>,
    render_pipeline: wgpu::RenderPipeline,
}

impl GltfDrawer {
    pub fn new(
        device: &wgpu::Device,
        view_bg_layout_desc: &wgpu::BindGroupLayoutDescriptor,
    ) -> Self {
        let primitive_topology = wgpu::PrimitiveTopology::TriangleStrip;
        let render_pipeline =
            build_render_pipeline(device, view_bg_layout_desc, primitive_topology);

        Self {
            render_pipeline,
            vbos: vec![],
            ibos: vec![],
            nb_idx: vec![],
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, viewer_bind_group, &[]);
        for i in 0..self.vbos.len() {
            render_pass.set_vertex_buffer(0, self.vbos[i].slice(..));
            render_pass.set_index_buffer(self.ibos[i].slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.nb_idx[i], 0, 0..1);
        }
    }

    pub fn add_gltf<P: AsRef<Path>>(&mut self, device: &wgpu::Device, path: P) {
        match load_gltf(path) {
            Ok(file) => {
                for mesh in file.meshes {
                    self.nb_idx.push(mesh.indices.len() as u32);
                    self.vbos.push(create_buffer_with_data(
                        device,
                        bytemuck::cast_slice(mesh.vertices.as_slice()),
                        wgpu::BufferUsages::VERTEX,
                        "gltf vertex",
                    ));
                    self.ibos.push(create_buffer_with_data(
                        device,
                        bytemuck::cast_slice(mesh.indices.as_slice()),
                        wgpu::BufferUsages::INDEX,
                        "gltf index",
                    ));
                }
            }
            Err(err) => {
                log::error!("Could not read gltf file: {:?}", err);
            }
        }
    }
}

pub struct StlDrawer {
    vbos: Vec<wgpu::Buffer>,
    nb_idx: Vec<u32>,
    render_pipeline: wgpu::RenderPipeline,
}

impl StlDrawer {
    pub fn new(
        device: &wgpu::Device,
        view_bg_layout_desc: &wgpu::BindGroupLayoutDescriptor,
    ) -> Self {
        let primitive_topology = wgpu::PrimitiveTopology::TriangleList;
        let render_pipeline =
            build_render_pipeline(device, view_bg_layout_desc, primitive_topology);

        Self {
            render_pipeline,
            vbos: vec![],
            nb_idx: vec![],
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, viewer_bind_group, &[]);
        for i in 0..self.vbos.len() {
            render_pass.set_vertex_buffer(0, self.vbos[i].slice(..));
            render_pass.draw(0..self.nb_idx[i], 0..1);
        }
    }

    pub fn add_stl<P: AsRef<Path>>(&mut self, device: &wgpu::Device, path: P) {
        match load_stl(path) {
            Ok(mesh) => {
                self.nb_idx.push(mesh.vertices.len() as u32);
                self.vbos.push(create_buffer_with_data(
                    device,
                    bytemuck::cast_slice(mesh.vertices.as_slice()),
                    wgpu::BufferUsages::VERTEX,
                    "std vertex",
                ));
            }
            Err(err) => {
                log::error!("Could not read stl file: {:?}", err);
            }
        }
    }
}

fn build_render_pipeline(
    device: &wgpu::Device,
    view_bg_layout_desc: &wgpu::BindGroupLayoutDescriptor,
    primitive_topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    let viewer_bg_layout = device.create_bind_group_layout(view_bg_layout_desc);

    let vertex_module = device.create_shader_module(&wgpu::include_spirv!("gltf_obj.vert.spv"));
    let fragment_module = device.create_shader_module(&wgpu::include_spirv!("gltf_obj.frag.spv"));
    let format = TEXTURE_FORMAT;
    let blend_state = wgpu::BlendState::ALPHA_BLENDING;
    let sample_count = consts::SAMPLE_COUNT;

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Gltf Drawer"),
        bind_group_layouts: &[&viewer_bg_layout],
        push_constant_ranges: &[],
    });

    let depth_compare = wgpu::CompareFunction::Less;

    let strip_index_format = match primitive_topology {
        wgpu::PrimitiveTopology::LineStrip | wgpu::PrimitiveTopology::TriangleStrip => {
            Some(wgpu::IndexFormat::Uint32)
        }
        _ => None,
    };

    let primitive = wgpu::PrimitiveState {
        topology: primitive_topology,
        strip_index_format,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        ..Default::default()
    };

    let targets = &[wgpu::ColorTargetState {
        format,
        blend: Some(blend_state),
        write_mask: wgpu::ColorWrites::ALL,
    }];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_module,
            entry_point: "main",
            buffers: &[ModelVertex::desc()],
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
            alpha_to_coverage_enabled: true,
        },
        label: Some("Gltf drawer pipeline"),
        multiview: None,
    })
}
