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
use super::instances_drawer::{Instanciable, RessourceProvider, Vertexable};
use ensnano_design::ultraviolet::{Vec2, Vec3};
use ensnano_utils::wgpu;
use std::convert::TryInto;
use std::rc::Rc;
use wgpu::{Device, Queue};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyBox {
    size: f32,
}
impl SkyBox {
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl Instanciable for SkyBox {
    type RawInstance = SkyBox;
    type Ressource = ();
    type Vertex = CubeVertex;

    fn to_raw_instance(&self) -> SkyBox {
        *self
    }

    fn vertices() -> Vec<CubeVertex> {
        DirectionCube::vertices()
    }

    fn indices() -> Vec<u16> {
        DirectionCube::indices()
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }

    fn vertex_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("skybox.vert.spv"))
    }

    fn fragment_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("skybox.frag.spv"))
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirectionCube {
    dist: f32,
}

impl DirectionCube {
    pub fn new(dist: f32) -> Self {
        Self { dist }
    }
}

impl Instanciable for DirectionCube {
    type RawInstance = DirectionCube;
    type Ressource = DirectionTexture;
    type Vertex = CubeVertex;

    fn to_raw_instance(&self) -> DirectionCube {
        *self
    }

    fn vertices() -> Vec<CubeVertex> {
        vec![
            //front
            CubeVertex {
                position: Vec3::new(-1., 1., 1.),
                texture_position: Vec2::new(0., 0.),
            },
            CubeVertex {
                position: Vec3::new(1., 1., 1.),
                texture_position: Vec2::new(1. / 3., 0.),
            },
            CubeVertex {
                position: Vec3::new(-1., -1., 1.),
                texture_position: Vec2::new(0., 0.5),
            },
            CubeVertex {
                position: Vec3::new(1., -1., 1.),
                texture_position: Vec2::new(1. / 3., 0.5),
            },
            // back
            CubeVertex {
                position: Vec3::new(1., 1., -1.),
                texture_position: Vec2::new(1. / 3., 0.),
            },
            CubeVertex {
                position: Vec3::new(-1., 1., -1.),
                texture_position: Vec2::new(2. / 3., 0.),
            },
            CubeVertex {
                position: Vec3::new(1., -1., -1.),
                texture_position: Vec2::new(1. / 3., 0.5),
            },
            CubeVertex {
                position: Vec3::new(-1., -1., -1.),
                texture_position: Vec2::new(2. / 3., 0.5),
            },
            // left
            CubeVertex {
                position: Vec3::new(-1., 1., -1.),
                texture_position: Vec2::new(2. / 3., 0.),
            },
            CubeVertex {
                position: Vec3::new(-1., 1., 1.),
                texture_position: Vec2::new(1., 0.),
            },
            CubeVertex {
                position: Vec3::new(-1., -1., -1.),
                texture_position: Vec2::new(2. / 3., 0.5),
            },
            CubeVertex {
                position: Vec3::new(-1., -1., 1.),
                texture_position: Vec2::new(1., 0.5),
            },
            // right
            CubeVertex {
                position: Vec3::new(1., 1., 1.),
                texture_position: Vec2::new(0., 0.5),
            },
            CubeVertex {
                position: Vec3::new(1., 1., -1.),
                texture_position: Vec2::new(1. / 3., 0.5),
            },
            CubeVertex {
                position: Vec3::new(1., -1., 1.),
                texture_position: Vec2::new(0., 1.),
            },
            CubeVertex {
                position: Vec3::new(1., -1., -1.),
                texture_position: Vec2::new(1. / 3., 1.),
            },
            // top
            CubeVertex {
                position: Vec3::new(-1., 1., -1.),
                texture_position: Vec2::new(1. / 3., 0.5),
            },
            CubeVertex {
                position: Vec3::new(1., 1., -1.),
                texture_position: Vec2::new(2. / 3., 0.5),
            },
            CubeVertex {
                position: Vec3::new(-1., 1., 1.),
                texture_position: Vec2::new(1. / 3., 1.),
            },
            CubeVertex {
                position: Vec3::new(1., 1., 1.),
                texture_position: Vec2::new(2. / 3., 1.),
            },
            // bottom
            CubeVertex {
                position: Vec3::new(-1., -1., 1.),
                texture_position: Vec2::new(2. / 3., 0.5),
            },
            CubeVertex {
                position: Vec3::new(1., -1., 1.),
                texture_position: Vec2::new(1., 0.5),
            },
            CubeVertex {
                position: Vec3::new(-1., -1., -1.),
                texture_position: Vec2::new(2. / 3., 1.),
            },
            CubeVertex {
                position: Vec3::new(1., -1., -1.),
                texture_position: Vec2::new(1., 1.),
            },
        ]
    }

    fn indices() -> Vec<u16> {
        vec![
            0, 1, 2, 1, 2, 3, 4, 5, 6, 5, 6, 7, 8, 9, 10, 9, 10, 11, 12, 13, 14, 13, 14, 15, 16,
            17, 18, 17, 18, 19, 20, 21, 22, 21, 22, 23,
        ]
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }

    fn vertex_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("direction_cube.vert.spv"))
    }

    fn fragment_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&wgpu::include_spirv!("direction_cube.frag.spv"))
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CubeVertex {
    position: Vec3,
    texture_position: Vec2,
}

const CUBE_VERTEX_ARRAY: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
impl Vertexable for CubeVertex {
    type RawType = CubeVertex;

    fn to_raw(&self) -> CubeVertex {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<CubeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &CUBE_VERTEX_ARRAY,
        }
    }
}

pub struct DirectionTexture {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl RessourceProvider for DirectionTexture {
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
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ]
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
}

impl DirectionTexture {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let diffuse_bytes = include_bytes!("../../../icons/direction_cube.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let dimensions = diffuse_image.dimensions();
        let bgra = diffuse_image.into_bgra8();

        use image::GenericImageView;

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ensnano_utils::TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: Default::default(),
            },
            &bgra,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: (4 * dimensions.0).try_into().ok(),
                rows_per_image: dimensions.1.try_into().ok(),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            texture_view: view,
            sampler,
        }
    }
}
