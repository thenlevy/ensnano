use iced_wgpu::wgpu;
use wgpu::{include_spirv, Device, PrimitiveTopology};

use super::drawable::Vertex;
use super::instances_drawer::Instanciable;
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};

pub struct GridDisc {
    position: Vec3,
    orientation: Rotor3,
    color: u32,
    model_id: u32,
    radius: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GridDiscRaw {
    model_matrix: Mat4,
    color: Vec4,
    radius: f32,
    model_id: u32,
}

unsafe impl bytemuck::Zeroable for GridDiscRaw {}
unsafe impl bytemuck::Pod for GridDiscRaw {}

const NB_EDGE: usize = 50;

impl Instanciable for GridDisc {
    type RawType = GridDiscRaw;
    fn vertices() -> Vec<Vertex> {
        let color = 0xFF_FF_FF_FF; // we will multiply by the instance's color in the fragment shader
        let fake = false;
        let mut ret = vec![Vertex {
            position: Vec3::zero(),
            color,
            fake,
        }];
        ret.reserve(NB_EDGE);
        for i in 0..(NB_EDGE + 1) {
            let theta = i as f32 / NB_EDGE as f32 * 2. * std::f32::consts::PI;
            let position = Vec3::new(0., theta.cos(), theta.sin());
            ret.push(Vertex {
                position,
                color,
                fake,
            });
        }
        ret
    }

    fn indices() -> Vec<u16> {
        let mut ret = Vec::with_capacity(3 * NB_EDGE);
        for i in 0..NB_EDGE {
            ret.push(0);
            ret.push(i as u16 + 1);
            ret.push(i as u16 + 2);
        }
        ret
    }

    fn primitive_topology() -> PrimitiveTopology {
        PrimitiveTopology::TriangleList
    }

    fn vertex_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(include_spirv!("grid_disc.vert.spv"))
    }

    fn fragment_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(include_spirv!("grid_disc.frag.spv"))
    }

    fn to_instance(&self) -> GridDiscRaw {
        GridDiscRaw {
            model_matrix: Mat4::from_translation(self.position)
                * self.orientation.into_matrix().into_homogeneous(),
            color: crate::utils::instance::Instance::color_from_au32(self.color),
            radius: self.radius,
            model_id: self.model_id,
        }
    }
}
