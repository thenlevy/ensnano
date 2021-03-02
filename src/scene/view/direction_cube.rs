use super::instances_drawer::{Instanciable, RessourceProvider, Vertexable};
use iced_wgpu::wgpu;
use std::rc::Rc;
use ultraviolet::{Vec2, Vec3};
use wgpu::{Device, Queue};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SkyBox {
    size: f32,
}
impl SkyBox {
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

unsafe impl bytemuck::Zeroable for SkyBox {}
unsafe impl bytemuck::Pod for SkyBox {}

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
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectionCube {
    dist: f32,
}

impl DirectionCube {
    pub fn new(dist: f32) -> Self {
        Self { dist }
    }
}

unsafe impl bytemuck::Zeroable for DirectionCube {}
unsafe impl bytemuck::Pod for DirectionCube {}

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
#[derive(Debug, Clone, Copy)]
pub struct CubeVertex {
    position: Vec3,
    texture_position: Vec2,
}

unsafe impl bytemuck::Zeroable for CubeVertex {}
unsafe impl bytemuck::Pod for CubeVertex {}

const CUBE_VERTEX_ARRAY: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float3, 1 => Float2];
impl Vertexable for CubeVertex {
    type RawType = CubeVertex;

    fn to_raw(&self) -> CubeVertex {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<CubeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
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
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: true,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Uint,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler {
                    comparison: false,
                    filtering: false,
                },
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
        let rgba = diffuse_image.as_rgba8().unwrap();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * dimensions.0,
                rows_per_image: dimensions.1,
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
