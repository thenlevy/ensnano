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
use iced_wgpu::wgpu;
use ultraviolet::{Vec2, Vec3, Vec4};
use wgpu::{include_spirv, Device};

use super::instances_drawer::{Instanciable, RessourceProvider, Vertexable};
use crate::text::Letter;

#[derive(Debug, Clone)]
pub struct LetterInstance {
    pub position: Vec3,
    pub color: Vec4,
    pub design_id: u32,
    pub scale: f32,
    pub shift: Vec3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RawLetter {
    pub position: Vec3,
    pub design_id: u32,
    pub color: Vec4,
    pub shift: Vec3,
    pub scale: f32,
}

impl RessourceProvider for Letter {
    fn ressources_layout() -> &'static [wgpu::BindGroupLayoutEntry] {
        &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler {
                    comparison: false,
                    filtering: true,
                },
                count: None,
            },
        ]
    }

    /// This methods allows the ressource tho provide the vertex buffer. If the return value is
    /// Some, it takes priority over the Instanciable's vertices.
    fn vertex_buffer_desc() -> Option<wgpu::VertexBufferLayout<'static>>
    where
        Self: Sized,
    {
        Some(crate::text::Vertex::desc())
    }

    fn ressources(&self) -> Vec<wgpu::BindGroupEntry> {
        vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
        ]
    }

    fn vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        Some(&self.vertex_buffer)
    }

    fn index_buffer(&self) -> Option<&wgpu::Buffer> {
        Some(&self.index_buffer)
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LetterVertex {
    pub position: Vec2,
}

impl Vertexable for LetterVertex {
    type RawType = LetterVertex;

    fn to_raw(&self) -> Self {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LetterVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        }
    }
}

impl Instanciable for LetterInstance {
    type Ressource = Letter;
    type Vertex = LetterVertex;
    type RawInstance = RawLetter;

    fn to_raw_instance(&self) -> RawLetter {
        RawLetter {
            position: self.position,
            color: self.color,
            design_id: self.design_id,
            scale: self.scale,
            shift: self.shift,
        }
    }

    fn vertices() -> Vec<LetterVertex> {
        vec![
            LetterVertex {
                position: Vec2::new(0f32, 0f32),
            },
            LetterVertex {
                position: Vec2::new(0f32, 1f32),
            },
            LetterVertex {
                position: Vec2::new(1f32, 0f32),
            },
            LetterVertex {
                position: Vec2::new(1f32, 1f32),
            },
        ]
    }

    fn indices() -> Vec<u16> {
        vec![0, 1, 2, 3]
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleStrip
    }

    fn vertex_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&include_spirv!("letter.vert.spv"))
    }

    fn fragment_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&include_spirv!("letter.frag.spv"))
    }

    fn alpha_to_coverage_enabled() -> bool {
        true
    }
}
