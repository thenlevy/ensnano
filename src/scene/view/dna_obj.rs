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
//! This modules defines the meshes that are used to draw DNA.

use super::instances_drawer::{Instanciable, Vertexable};
use crate::consts::*;
use iced_wgpu::wgpu;
use std::f32::consts::PI;
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};

/// The vertex type for the meshes used to draw DNA.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DnaVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

pub trait DnaObject:
    Instanciable<Ressource = (), Vertex = DnaVertex, RawInstance = RawDnaInstance>
{
}

const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
impl Vertexable for DnaVertex {
    type RawType = DnaVertex;
    fn to_raw(&self) -> DnaVertex {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<DnaVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTR_ARRAY,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RawDnaInstance {
    pub model: Mat4,
    pub color: Vec4,
    pub scale: Vec3,
    pub id: u32,
    pub inversed_model: Mat4,
}

pub struct SphereInstance {
    /// The position in space
    pub position: Vec3,
    pub color: Vec4,
    pub id: u32,
    pub radius: f32,
}

impl Instanciable for SphereInstance {
    type Vertex = DnaVertex;
    type RawInstance = RawDnaInstance;
    type Ressource = ();

    fn vertices() -> Vec<DnaVertex> {
        let mut vertices = Vec::new();

        let stack_step = PI / NB_STACK_SPHERE as f32;
        let sector_step = 2. * PI / NB_SECTOR_SPHERE as f32;
        for i in 0..=NB_STACK_SPHERE {
            // 0..=x means that x is included
            let stack_angle = PI / 2. - (i as f32) * stack_step;
            let radius = SPHERE_RADIUS;
            let xy = radius * stack_angle.cos();
            let z = radius * stack_angle.sin();

            for j in 0..=NB_SECTOR_SPHERE {
                let sector_angle = j as f32 * sector_step;

                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();
                let position = [x, y, z];
                let normal = [x, y, z];

                vertices.push(DnaVertex { position, normal })
            }
        }
        vertices
    }

    fn indices() -> Vec<u16> {
        let mut indices = Vec::new();

        for i in 0..NB_STACK_SPHERE {
            let mut k1: u16 = i * (NB_SECTOR_SPHERE + 1); // begining of ith stack
            let mut k2: u16 = k1 + NB_SECTOR_SPHERE + 1; // begining of (i + 1)th stack

            for _ in 0..NB_SECTOR_SPHERE {
                if i > 0 {
                    indices.push(k1);
                    indices.push(k2);
                    indices.push(k1 + 1);
                }

                if i < NB_STACK_SPHERE - 1 {
                    indices.push(k1 + 1);
                    indices.push(k2);
                    indices.push(k2 + 1);
                }
                k1 += 1;
                k2 += 1;
            }
        }
        indices
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }

    fn to_raw_instance(&self) -> RawDnaInstance {
        let model = Mat4::from_translation(self.position);
        RawDnaInstance {
            model,
            color: self.color,
            scale: Vec3::new(self.radius, self.radius, self.radius),
            id: self.id,
            inversed_model: model.inversed(),
        }
    }

    fn vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("dna_obj.vert.spv"))
    }

    fn fragment_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("dna_obj.frag.spv"))
    }

    fn fake_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_fake.frag.spv")))
    }

    fn outline_vertex_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_outline.vert.spv")))
    }

    fn outline_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_outline.frag.spv")))
    }
}

impl DnaObject for SphereInstance {}

pub struct TubeInstance {
    pub position: Vec3,
    pub rotor: Rotor3,
    pub color: Vec4,
    pub id: u32,
    pub radius: f32,
    pub length: f32,
}

impl TubeInstance {
    pub fn with_radius(self, radius: f32) -> Self {
        Self { radius, ..self }
    }
}

impl Instanciable for TubeInstance {
    type Vertex = DnaVertex;
    type RawInstance = RawDnaInstance;
    type Ressource = ();

    fn vertices() -> Vec<DnaVertex> {
        let radius = BOUND_RADIUS;
        (0..(2 * NB_RAY_TUBE))
            .map(|i| {
                let point = i / 2;
                let side = if i % 2 == 0 { -1. } else { 1. };
                let theta = (point as f32) * 2. * PI / NB_RAY_TUBE as f32;
                let position = [
                    side * BOUND_LENGTH / 2.,
                    theta.sin() * radius,
                    theta.cos() * radius,
                ];

                let normal = [0., theta.sin(), theta.cos()];
                DnaVertex { position, normal }
            })
            .collect()
    }

    fn indices() -> Vec<u16> {
        let mut indices: Vec<_> = (0u16..(2 * NB_RAY_TUBE as u16)).collect();
        indices.push(0);
        indices.push(1);
        indices
    }

    fn vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("dna_obj.vert.spv"))
    }

    fn fragment_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("dna_obj.frag.spv"))
    }

    fn fake_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_fake.frag.spv")))
    }

    fn outline_vertex_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_outline.vert.spv")))
    }

    fn outline_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_outline.frag.spv")))
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleStrip
    }

    fn to_raw_instance(&self) -> RawDnaInstance {
        let model =
            Mat4::from_translation(self.position) * self.rotor.into_matrix().into_homogeneous();
        RawDnaInstance {
            model,
            color: self.color,
            scale: Vec3::new(self.length, self.radius, self.radius),
            id: self.id,
            inversed_model: model.inversed(),
        }
    }
}

impl DnaObject for TubeInstance {}

pub struct ConeInstance {
    pub position: Vec3,
    pub rotor: Rotor3,
    pub color: Vec4,
    pub id: u32,
    pub radius: f32,
    pub length: f32,
}

impl Instanciable for ConeInstance {
    type Vertex = DnaVertex;
    type RawInstance = RawDnaInstance;
    type Ressource = ();

    fn vertices() -> Vec<DnaVertex> {
        let radius = 1.;
        let mut ret: Vec<DnaVertex> = (0..(2 * NB_RAY_TUBE))
            .map(|i| {
                let point = i / 2 + i % 2;
                let side = if i % 2 == 0 { 0. } else { 1. };
                let height = if i % 2 == 0 { radius } else { 0. };
                let theta = (point as f32) * 2. * PI / NB_RAY_TUBE as f32;
                let position = [side, theta.sin() * height, theta.cos() * height];
                use std::f32::consts::FRAC_1_SQRT_2;

                let normal = [
                    FRAC_1_SQRT_2,
                    FRAC_1_SQRT_2 * theta.sin(),
                    FRAC_1_SQRT_2 * theta.cos(),
                ];
                DnaVertex { position, normal }
            })
            .collect();

        for i in 0..(2 * NB_RAY_TUBE) {
            let point = i / 2 + i % 2;
            let height = if i % 2 == 0 { radius } else { 0. };
            let theta = (point as f32) * 2. * PI / NB_RAY_TUBE as f32;
            let position = [0., theta.sin() * height, theta.cos() * height];
            let normal = [-1., 0., 0.];
            ret.push(DnaVertex { position, normal });
        }

        ret
    }

    fn indices() -> Vec<u16> {
        let nb_point = 2 * NB_RAY_TUBE as u16;
        let mut ret = Vec::with_capacity(3 * nb_point as usize);
        for i in 0..nb_point {
            ret.push((2 * i) % nb_point);
            ret.push((2 * i + 1) % nb_point);
            ret.push((2 * i + 2) % nb_point);
            ret.push((2 * i) % nb_point + nb_point);
            ret.push((2 * i + 1) % nb_point + nb_point);
            ret.push((2 * i + 2) % nb_point + nb_point);
        }
        ret
    }

    fn vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("dna_obj.vert.spv"))
    }

    fn fragment_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("dna_obj.frag.spv"))
    }

    fn fake_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_fake.frag.spv")))
    }

    fn outline_vertex_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_outline.vert.spv")))
    }

    fn outline_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(&wgpu::include_spirv!("dna_obj_outline.frag.spv")))
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }

    fn to_raw_instance(&self) -> RawDnaInstance {
        let model =
            Mat4::from_translation(self.position) * self.rotor.into_matrix().into_homogeneous();
        RawDnaInstance {
            model,
            color: self.color,
            scale: Vec3::new(self.length, self.radius, self.radius),
            id: self.id,
            inversed_model: model.inversed(),
        }
    }
}

impl DnaObject for ConeInstance {}
