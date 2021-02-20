use super::*;
use iced_wgpu::wgpu;
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::{Mat2, Vec2};
use wgpu::RenderPipeline;

pub struct InsertionDrawer {
    new_instances: Option<Vec<InsertionInstance>>,
    vertices: Vertices,
    instances: DynamicBindGroup,
    pipeline: RenderPipeline,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InsertionVertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InsertionInstance {
    pub position: Vec2,
    pub depth: f32,
    pub orientation: Mat2,
    pub color: [f32; 4],
}

type Vertices = lyon::tessellation::VertexBuffers<InsertionVertex, u16>;

fn make_vertices() -> Vertices {
    let mut vertices = Vertices::new();
    let mut builder = Path::builder();
    let origin = Point::new(0., 0.);
    let left = Point::new(-1., 1.);
    let right = Point::new(1., 1.);

    builder.begin(origin);
    builder.cubic_bezier_to(left, right, origin);
    let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

    builder.end(false);
    let path = builder.build();
    stroke_tess
        .tessellate_path(
            &path,
            &tessellation::StrokeOptions::tolerance(0.01)
                .with_line_cap(tessellation::LineCap::Round)
                .with_end_cap(tessellation::LineCap::Round)
                .with_start_cap(tessellation::LineCap::Round)
                .with_line_join(tessellation::LineJoin::Round),
            &mut tessellation::BuffersBuilder::new(&mut vertices, InsertionVertexBuilder),
        )
        .expect("Error durring tessellation");
    vertices
}

fn insertion_pipeline(
    device: &Device,
    globals: &wgpu::BindGroupLayout,
    insertions: &wgpu::BindGroupLayout,
    depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>,
) -> wgpu::RenderPipeline {
    let vs_module = &device.create_shader_module(wgpu::include_spirv!("insertion.vert.spv"));
    let fs_module = &device.create_shader_module(wgpu::include_spirv!("strand.frag.spv"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[globals, insertions],
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
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<InsertionVertex>() as u64,
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

struct InsertionVertexBuilder;

impl StrokeVertexConstructor<InsertionVertex> for InsertionVertexBuilder {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> InsertionVertex {
        InsertionVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
        }
    }
}
