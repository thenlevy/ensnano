use ab_glyph::{point, Font, FontRef, Glyph};
use iced_wgpu::wgpu;
use std::rc::Rc;
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Device, Extent3d, Queue, Sampler, Texture,
    TextureView,
};

use crate::consts::SAMPLE_COUNT;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.25, -0.5, 0.0],
        tex_coords: [0., 1.0],
    }, // A
    Vertex {
        position: [-0.25, 0., 0.0],
        tex_coords: [0., 0.],
    }, // B
    Vertex {
        position: [0.25, -0.5, 0.0],
        tex_coords: [1.0, 1.0],
    }, // C
    Vertex {
        position: [0.25, 0., 0.0],
        tex_coords: [1.0, 0.],
    }, // D
];

const INDICES: &[u16] = &[0, 1, 2, 3];

pub struct Letter {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub sampler: Sampler,
    pub bind_group: BindGroup,
    pub size: Extent3d,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub bind_group_layout: BindGroupLayout,
    pub char_size: [f32 ; 2],
}

const MIP_LEVEL_COUNT: u32 = 6;

impl Letter {
    pub fn new(character: char, device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let size = Extent3d {
            width: 1 << (MIP_LEVEL_COUNT + 3),
            height: 1 << (MIP_LEVEL_COUNT + 3),
            depth: 1,
        };

        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            // All textures are stored as 3d, we represent our 2d texture
            // by setting depth to 1.
            size,
            mip_level_count: MIP_LEVEL_COUNT, // We'll talk about this a little later
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // SAMPLED tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: Some("diffuse_texture"),
        });

        let mut char_size = [512f32, 512f32];

        for mip_level in 0..MIP_LEVEL_COUNT {
            let size = Extent3d {
                width: 1 << (MIP_LEVEL_COUNT + 3 - mip_level),
                height: 1 << (MIP_LEVEL_COUNT + 3 - mip_level),
                depth: 1,
            };
            let mut pixels = vec![0u8; (size.width * size.height * 4) as usize];

            let font = FontRef::try_from_slice(include_bytes!("../../font/MonospaceBold.ttf"))
                .expect("Could not read font");
            let q_glyph: Glyph = font
                .glyph_id(character)
                .with_scale_and_position(size.width as f32 * 1.5, point(0.0, 0.0));

            if let Some(q) = font.outline_glyph(q_glyph) {
                q.draw(|x, y, c| {
                    for i in 0..4 {
                        pixels[((x + y * size.width) * 4 + i) as usize] = (c * 255.) as u8;
                    }
                });
                if mip_level == 0 {
                    char_size[0] = q.px_bounds().width();
                    char_size[1] = q.px_bounds().height();
                }
            }

            queue.write_texture(
                // Tells wgpu where to copy the pixel data
                wgpu::TextureCopyView {
                    texture: &diffuse_texture,
                    mip_level,
                    origin: wgpu::Origin3d::ZERO,
                },
                &pixels,
                // The layout of the texture
                wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: 4 * size.width,
                    rows_per_image: size.height,
                },
                size,
            );
        }

        let diffuse_texture_view =
            diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: true,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            min_binding_size: None,
                            dynamic: false,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
                        usage: wgpu::BufferUsage::UNIFORM,
                        contents: bytemuck::cast_slice(&char_size),
                        label: None,
                    }).slice(..))
                }
            ],
            label: Some("diffuse_bind_group"),
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });

        Self {
            size,
            texture: diffuse_texture,
            bind_group: diffuse_bind_group,
            sampler: diffuse_sampler,
            texture_view: diffuse_texture_view,
            vertex_buffer,
            index_buffer,
            bind_group_layout: texture_bind_group_layout,
            char_size,
        }
    }
}
