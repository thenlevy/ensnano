use iced_wgpu::wgpu;
use wgpu::{include_spirv, Device, PrimitiveTopology};

use super::instances_drawer::Instanciable;
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};

#[derive(Debug, Clone)]
pub struct GridDisc {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub color: u32,
    pub model_id: u32,
    pub radius: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GridDiscRaw {
    model_matrix: Mat4,
    color: Vec4,
    radius: f32,
    model_id: u32,
    _padding: [u32; 2],
}

unsafe impl bytemuck::Zeroable for GridDiscRaw {}
unsafe impl bytemuck::Pod for GridDiscRaw {}

const NB_EDGE: usize = 50;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GridDiscVertex {
    position: Vec3,
    color: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GridDiscVertexRaw {
    position: Vec3,
    color: Vec4,
}

unsafe impl bytemuck::Zeroable for GridDiscVertexRaw {}
unsafe impl bytemuck::Pod for GridDiscVertexRaw {}

impl super::instances_drawer::Vertexable for GridDiscVertex {
    type RawType = GridDiscVertexRaw;

    fn to_raw(&self) -> GridDiscVertexRaw {
        GridDiscVertexRaw {
            position: self.position,
            color: crate::utils::instance::Instance::color_from_au32(self.color),
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<GridDiscVertexRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

impl Instanciable for GridDisc {
    type Vertex = GridDiscVertex;
    type RawInstance = GridDiscRaw;
    type Ressource = ();
    fn vertices() -> Vec<GridDiscVertex> {
        let color = 0xFF_FF_FF_FF; // we will multiply by the instance's color in the fragment shader
        let mut ret = vec![GridDiscVertex {
            position: Vec3::zero(),
            color,
        }];
        ret.reserve(NB_EDGE);
        for i in 0..(NB_EDGE + 1) {
            let theta = i as f32 / NB_EDGE as f32 * 2. * std::f32::consts::PI;
            let position = Vec3::new(0., theta.cos(), theta.sin());
            ret.push(GridDiscVertex { position, color });
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

    fn to_raw_instance(&self) -> GridDiscRaw {
        GridDiscRaw {
            model_matrix: Mat4::from_translation(self.position)
                * self.orientation.into_matrix().into_homogeneous(),
            color: crate::utils::instance::Instance::color_from_au32(self.color),
            radius: self.radius,
            model_id: self.model_id,
            _padding: [0, 0],
        }
    }
}
