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
use super::Instanciable;
use ultraviolet::{Vec3, Vec4};

const OBJ_VERTEX_ARRAY: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: Vec3,
    color: Vec4,
    normal: Vec3,
}

impl ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &OBJ_VERTEX_ARRAY,
        }
    }
}

pub struct GltfMesh {
    vertices: Vec<ModelVertex>,
    indices: Vec<u32>,
}

fn load_gltf(path: &'static str) -> Result<GltfMesh, ErrGltf> {
    let (doc, datas, _) = gltf::import(path).unwrap();
    let mesh_data = doc.meshes().next().ok_or(ErrGltf::NoMeshes)?;

    let primitive = mesh_data.primitives().next().ok_or(ErrGltf::NoPrimitive)?;
    let reader = primitive.reader(|b| Some(&datas.get(b.index())?.0[..b.length()]));

    let vertex_positions = {
        let position_iter = reader.read_positions().ok_or(ErrGltf::NoPosition)?;
        position_iter.map(Vec3::from)
    };
    let vertex_normals = {
        let normals_iter = reader.read_normals().ok_or(ErrGltf::NoNormal)?;
        normals_iter.map(Vec3::from)
    };
    let vertex_colors = {
        let color_iter = reader.read_colors(0).ok_or(ErrGltf::NoColor)?;
        color_iter.into_rgba_f32().map(Vec4::from)
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

pub enum ErrGltf {
    NoPrimitive,
    NoMeshes,
    NoColor,
    NoNormal,
    NoPosition,
}

impl Instanciable for GltfMesh {}
