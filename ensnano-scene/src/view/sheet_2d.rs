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

use super::instances_drawer::*;
use ensnano_design::{ultraviolet, BezierPlaneId};
use ensnano_utils::wgpu;
use ultraviolet::{Mat4, Rotor3, Vec2, Vec3};
use wgpu::{include_spirv, Device};

#[derive(Debug, Clone)]
pub struct Sheet2D {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub graduation_unit: f32,
    pub axis_position: Option<f32>,
    pub plane_id: BezierPlaneId,
}

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sheet2DRaw {
    pub model: Mat4,
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub graduation_unit: f32,
    pub axis_position: f32,
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SheetVertex {
    pub position: Vec2,
}

impl Vertexable for SheetVertex {
    type RawType = SheetVertex;

    fn to_raw(&self) -> Self {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SheetVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        }
    }
}

impl Sheet2D {
    pub fn corners(&self) -> [Vec2; 4] {
        [
            Vec2::new(self.min_x, self.min_y),
            Vec2::new(self.max_x, self.min_y),
            Vec2::new(self.min_x, self.max_y),
            Vec2::new(self.max_x, self.max_y),
        ]
    }

    pub fn space_position_of_point2d(&self, vec: Vec2) -> Vec3 {
        self.position
            + Vec3::unit_z().rotated_by(self.orientation) * vec.x
            + Vec3::unit_y().rotated_by(self.orientation) * vec.y
    }
}

impl Instanciable for Sheet2D {
    type Vertex = SheetVertex;
    type Ressource = ();
    type RawInstance = Sheet2DRaw;

    fn to_raw_instance(&self) -> Self::RawInstance {
        Sheet2DRaw {
            model: Mat4::from_translation(self.position)
                * self.orientation.into_matrix().into_homogeneous(),
            min_x: self.min_x,
            max_x: self.max_x,
            min_y: self.min_y,
            max_y: self.max_y,
            graduation_unit: self.graduation_unit,
            axis_position: self.axis_position.unwrap_or(f32::INFINITY),
            _padding: [0.; 2],
        }
    }

    fn vertices() -> Vec<SheetVertex> {
        vec![
            SheetVertex {
                position: Vec2::new(0f32, 0f32),
            },
            SheetVertex {
                position: Vec2::new(0f32, 1f32),
            },
            SheetVertex {
                position: Vec2::new(1f32, 0f32),
            },
            SheetVertex {
                position: Vec2::new(1f32, 1f32),
            },
        ]
    }

    fn vertex_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&include_spirv!("sheet_2d.vert.spv"))
    }

    fn fragment_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&include_spirv!("sheet_2d.frag.spv"))
    }

    fn indices() -> Vec<u16> {
        vec![0, 1, 2, 3]
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleStrip
    }

    fn alpha_to_coverage_enabled() -> bool {
        true
    }
}
