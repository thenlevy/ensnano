use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};

const TEXTURE_SIZE: u32 = 512;
use crate::consts::*;

use iced_wgpu::wgpu;
use wgpu::util::DeviceExt;
use wgpu::{Device, Sampler, Texture, TextureView};

#[derive(Clone, Copy)]
pub struct Vertex {
    position: [f32; 2],
    normal: [f32; 2],
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

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
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ],
        }
    }
}

type Vertices = lyon::tessellation::VertexBuffers<Vertex, u16>;

pub struct SquareTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl SquareTexture {
    pub fn new(device: &Device, encoder: &mut wgpu::CommandEncoder) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            label: Some("square texture"),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        fill_square_texture(&view, device, encoder);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

fn fill_square_texture(target: &TextureView, device: &Device, encoder: &mut wgpu::CommandEncoder) {
    let pipeline = pipeline(device);
    let vertices = square_texture_vertices();
    let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices.vertices),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices.indices),
        usage: wgpu::BufferUsage::INDEX,
    });
    let clear_color = wgpu::Color {
        r: 0.,
        g: 0.,
        b: 1.,
        a: 0.4, // this will be usefull to discard fragments that are not on the grid
    };

    let texture_size = iced_winit::winit::dpi::PhysicalSize {
        width: TEXTURE_SIZE,
        height: TEXTURE_SIZE,
    };

    let msaa_texture = if SAMPLE_COUNT > 1 {
        Some(crate::utils::texture::Texture::create_msaa_texture(
            device,
            &texture_size,
            SAMPLE_COUNT,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ))
    } else {
        None
    };

    let attachment = if msaa_texture.is_some() {
        msaa_texture.as_ref().unwrap()
    } else {
        target
    };

    let resolve_target = if msaa_texture.is_some() {
        Some(target)
    } else {
        None
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment,
            resolve_target,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });

    render_pass.set_viewport(
        0f32,
        0f32,
        TEXTURE_SIZE as f32,
        TEXTURE_SIZE as f32,
        0.0,
        1.0,
    );

    render_pass.set_pipeline(&pipeline);

    render_pass.set_index_buffer(ibo.slice(..));
    render_pass.set_vertex_buffer(0, vbo.slice(..));
    render_pass.draw_indexed(0..vertices.indices.len() as u32, 0, 0..1);
}

fn square_texture_vertices() -> Vertices {
    let mut vertices = Vertices::new();
    let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

    let mut builder = Path::builder();

    builder.begin(Point::new(-1., -1.));
    builder.line_to(Point::new(-1., 1.));
    builder.line_to(Point::new(1., 1.));
    builder.line_to(Point::new(1., -1.));
    builder.end(true);
    let path = builder.build();

    stroke_tess
        .tessellate_path(
            &path,
            &tessellation::StrokeOptions::default(),
            &mut tessellation::BuffersBuilder::new(&mut vertices, Custom),
        )
        .expect("error durring tessellation");
    vertices
}

pub struct HonneyTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl HonneyTexture {
    pub fn new(device: &Device, encoder: &mut wgpu::CommandEncoder) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            label: Some("honneycomb texture"),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        fill_honneycomb_texture(&view, device, encoder);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

fn fill_honneycomb_texture(
    target: &TextureView,
    device: &Device,
    encoder: &mut wgpu::CommandEncoder,
) {
    let pipeline = pipeline(device);
    let vertices = honeycomb_texture_vertices();
    let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices.vertices),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices.indices),
        usage: wgpu::BufferUsage::INDEX,
    });
    let clear_color = wgpu::Color {
        r: 0.,
        g: 0.,
        b: 1.,
        a: 0.4, // this will be usefull to discard fragments that are not on the grid
    };

    let texture_size = iced_winit::winit::dpi::PhysicalSize {
        width: TEXTURE_SIZE,
        height: TEXTURE_SIZE,
    };

    let msaa_texture = if SAMPLE_COUNT > 1 {
        Some(crate::utils::texture::Texture::create_msaa_texture(
            device,
            &texture_size,
            SAMPLE_COUNT,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ))
    } else {
        None
    };

    let attachment = if msaa_texture.is_some() {
        msaa_texture.as_ref().unwrap()
    } else {
        target
    };

    let resolve_target = if msaa_texture.is_some() {
        Some(target)
    } else {
        None
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment,
            resolve_target,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });

    render_pass.set_viewport(
        0f32,
        0f32,
        TEXTURE_SIZE as f32,
        TEXTURE_SIZE as f32,
        0.0,
        1.0,
    );

    render_pass.set_pipeline(&pipeline);

    render_pass.set_index_buffer(ibo.slice(..));
    render_pass.set_vertex_buffer(0, vbo.slice(..));
    render_pass.draw_indexed(0..vertices.indices.len() as u32, 0, 0..1);
}

fn honeycomb_texture_vertices() -> Vertices {
    let mut vertices = Vertices::new();
    let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

    let mut builder = Path::builder();

    builder.begin(Point::new(1., -1.));
    builder.line_to(Point::new(1., -1. / 3.));
    builder.line_to(Point::new(-1., 1. / 3.));
    builder.line_to(Point::new(-1., 1.));
    builder.end(false);
    let path = builder.build();

    stroke_tess
        .tessellate_path(
            &path,
            &tessellation::StrokeOptions::default(),
            &mut tessellation::BuffersBuilder::new(&mut vertices, Custom),
        )
        .expect("error durring tessellation");
    vertices
}

struct Custom;

impl StrokeVertexConstructor<Vertex> for Custom {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        Vertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
        }
    }
}

fn pipeline(device: &Device) -> wgpu::RenderPipeline {
    let vs_module = &device.create_shader_module(wgpu::include_spirv!("texture.vert.spv"));
    let fs_module = &device.create_shader_module(wgpu::include_spirv!("texture.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[],
        push_constant_ranges: &[],
        label: None,
    });

    let desc = wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            ..Default::default()
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2],
            }],
        },
        sample_count: SAMPLE_COUNT,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&desc)
}
