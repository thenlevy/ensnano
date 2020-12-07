//! This modules defines the meshes that are used to draw DNA.

use super::instances_drawer::{Instanciable, Vertexable};
use crate::consts::*;
use iced_wgpu::wgpu;
use std::f32::consts::PI;
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};

/// The vertex type for the meshes used to draw DNA.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DnaVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

unsafe impl bytemuck::Pod for DnaVertex {}
unsafe impl bytemuck::Zeroable for DnaVertex {}

impl Vertexable for DnaVertex {
    type RawType = DnaVertex;
    fn to_raw(&self) -> DnaVertex {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<DnaVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                // Normal
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct RawDnaInstance {
    pub model: Mat4,
    pub color: Vec4,
    pub scale: Vec3,
    pub id: u32,
}

unsafe impl bytemuck::Zeroable for RawDnaInstance {}
unsafe impl bytemuck::Pod for RawDnaInstance {}

pub struct SphereInstance {
    /// The position in space
    pub position: Vec3,
    /// The rotation of the instance
    pub rotor: Rotor3,
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
        RawDnaInstance {
            model: Mat4::from_translation(self.position)
                * self.rotor.into_matrix().into_homogeneous(),
            color: self.color,
            scale: Vec3::new(self.radius, self.radius, self.radius),
            id: self.id,
        }
    }

    fn vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::include_spirv!("dna_obj.vert.spv"))
    }

    fn fragment_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::include_spirv!("dna_obj.frag.spv"))
    }

    fn fake_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(wgpu::include_spirv!("dna_obj_fake.frag.spv")))
    }
}

pub struct TubeInstance {
    pub position: Vec3,
    pub rotor: Rotor3,
    pub color: Vec4,
    pub id: u32,
    pub radius: f32,
    pub length: f32,
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
        device.create_shader_module(wgpu::include_spirv!("dna_obj.vert.spv"))
    }

    fn fragment_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::include_spirv!("dna_obj.frag.spv"))
    }

    fn fake_fragment_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
        Some(device.create_shader_module(wgpu::include_spirv!("dna_obj_fake.frag.spv")))
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleStrip
    }

    fn to_raw_instance(&self) -> RawDnaInstance {
        RawDnaInstance {
            model: Mat4::from_translation(self.position)
                * self.rotor.into_matrix().into_homogeneous(),
            color: self.color,
            scale: Vec3::new(self.length, self.radius, self.radius),
            id: self.id,
        }
    }
}
