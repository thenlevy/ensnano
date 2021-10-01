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

const OBJ_VERTEX_ARRAY: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32 ; 3],
    normal: [f32 ; 3],
    color: [f32 ; 4],
}

impl ModelVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &OBJ_VERTEX_ARRAY,
        }
    }
}

pub struct GltfFile {
    pub meshes: Vec<GltfMesh>,
}

pub struct GltfMesh {
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
}

fn read_mesh(mesh_data: &gltf::Mesh, datas: &[gltf::buffer::Data]) -> Result<GltfMesh, ErrGltf> {
    let primitive = mesh_data.primitives().next().ok_or(ErrGltf::NoPrimitive)?;
    let reader = primitive.reader(|b| Some(&datas.get(b.index())?.0[..b.length()]));

    let vertex_positions = {
        let position_iter = reader.read_positions().ok_or(ErrGltf::NoPosition)?;
        position_iter
    };
    let vertex_normals = {
        let normals_iter = reader.read_normals().ok_or(ErrGltf::NoNormal)?;
        normals_iter
    };
    let vertex_colors = {
        let color_iter = reader.read_colors(0).ok_or(ErrGltf::NoColor)?;
        color_iter.into_rgba_u8().map(|v| [v[0] as f32 / 255., v[1] as f32 / 255., v[2] as f32 / 255., v[3] as f32 /255.])
    };
    let indices = reader.read_indices().unwrap().into_u32().collect();

    let vertices: Vec<ModelVertex> = vertex_positions
        .zip(vertex_normals.zip(vertex_colors))
        .map(|(position, (normal, color))| ModelVertex {
            position,
            normal,
            color,
        })
        .collect();

    Ok(GltfMesh { vertices, indices })
}

pub fn load_gltf(path: &'static str) -> Result<GltfFile, ErrGltf> {
    let (doc, datas, _) = gltf::import(path).ok().ok_or(ErrGltf::CannotReadFile)?;
    let mesh_data = doc.meshes();
    let mut meshes = Vec::new();
    for m in mesh_data {
        let mesh = read_mesh(&m, &datas)?;
        meshes.push(mesh)
    }
    Ok(GltfFile { meshes })
}

#[derive(Debug)]
pub enum ErrGltf {
    CannotReadFile,
    NoPrimitive,
    NoColor,
    NoNormal,
    NoPosition,
}
