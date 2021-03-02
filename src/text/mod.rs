//! This module provides utilities for drawing text in the applications
use fontdue::Font;
use iced_wgpu::wgpu;
use std::rc::Rc;
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Device, Extent3d, Queue, Sampler, Texture,
    TextureView,
};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float2, 1 => Float2];
impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &VERTEX_ATTR_ARRAY,
        }
    }
}

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
    pub advance: f32,
    pub height: f32,
}

const MAX_SIZE: u32 = 9;
const MIN_SIZE: u32 = 1;
const MIP_LEVEL_COUNT: u32 = MAX_SIZE - MIN_SIZE + 1;

impl Letter {
    pub fn new(character: char, device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let size = Extent3d {
            width: 1 << MAX_SIZE,
            height: 1 << MAX_SIZE,
            depth: 1,
        };

        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            // All textures are stored as 3d, we represent our 2d texture
            // by setting depth to 1.
            size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: Some("diffuse_texture"),
        });

        let font: &[u8] = if character.is_ascii_alphabetic() {
            include_bytes!("../../font/DejaVuSansMono.ttf")
        } else {
            include_bytes!("../../font/Inconsolata-Regular.ttf")
        };
        let font = Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();
        let (metrics, _) = font.rasterize(character, size.height as f32);

        let min_x = metrics.xmin as f32 / size.width as f32;
        let max_x = min_x + metrics.width as f32 / size.width as f32;

        let min_y = metrics.ymin as f32 / size.height as f32;
        let max_y = min_y + metrics.height as f32 / size.height as f32;

        let vertices: &[Vertex] = &[
            Vertex {
                position: [min_x, max_y],
                tex_coords: [0., metrics.height as f32 / size.height as f32],
            },
            Vertex {
                position: [min_x, min_y],
                tex_coords: [0., 0.],
            },
            Vertex {
                position: [max_x, max_y],
                tex_coords: [
                    metrics.width as f32 / size.width as f32,
                    metrics.height as f32 / size.height as f32,
                ],
            },
            Vertex {
                position: [max_x, min_y],
                tex_coords: [metrics.width as f32 / size.width as f32, 0.],
            },
        ];

        let advance = metrics.advance_width / size.width as f32;
        let height = metrics.height as f32 / size.height as f32;
        let mut last_pixels = None;

        for mip_level in 0..MIP_LEVEL_COUNT {
            let size = Extent3d {
                width: 1 << (MAX_SIZE - mip_level),
                height: 1 << (MAX_SIZE - mip_level),
                depth: 1,
            };
            let mut pixels = vec![0u8; (size.width * size.height * 4) as usize];

            if let Some(ref previous) = last_pixels {
                for x in 0..size.width as usize {
                    for y in 0..size.height as usize {
                        // We use 4 bytes per pixel because we use BgraUnormSrgb format
                        let coverage =
                            get_average_pixel_value(previous, x, y, 2 * size.width as usize);
                        for i in 0..4 {
                            pixels[4 * (y * size.width as usize + x) + i] = coverage
                        }
                    }
                }
            } else {
                let (metrics, bitmap) = font.rasterize(character, size.height as f32);

                for x in 0..metrics.width {
                    for y in 0..metrics.height {
                        // We use 4 bytes per pixel because we use BgraUnormSrgb format
                        for i in 0..4 {
                            pixels[4 * (y * size.width as usize + x) + i] =
                                bitmap[y * metrics.width + x];
                        }
                    }
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

            last_pixels = Some(pixels);
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
            ],
            label: Some("diffuse_bind_group"),
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
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
            advance,
            height,
        }
    }
}

fn get_average_pixel_value(pixels: &Vec<u8>, x: usize, y: usize, width: usize) -> u8 {
    let get = |x, y| pixels[4 * (y * width + x)] as u16;
    let sum = get(2 * x, 2 * y)
        + get(2 * x + 1, 2 * y)
        + get(2 * x + 1, 2 * y)
        + get(2 * x + 1, 2 * y + 1);
    (sum / 4) as u8
}
