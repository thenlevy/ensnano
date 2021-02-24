use super::*;
use iced_wgpu::wgpu;
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::{Mat2, Vec2};
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, Buffer, DepthStencilStateDescriptor, RenderPass, RenderPipeline};

pub struct InsertionDrawer {
    new_instances: Option<Vec<InsertionInstance>>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instances: DynamicBindGroup,
    pipeline: RenderPipeline,
    number_indices: usize,
    number_instances: usize,
}

impl InsertionDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        globals: &BindGroupLayout,
        depth_stencil_state: Option<DepthStencilStateDescriptor>,
    ) -> Self {
        let instances = DynamicBindGroup::new(device.clone(), queue.clone());
        let pipeline = insertion_pipeline(
            device.as_ref(),
            globals,
            instances.get_layout(),
            depth_stencil_state,
        );
        let vertices = make_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices.vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices.indices),
            usage: wgpu::BufferUsage::INDEX,
        });
        let number_indices = vertices.indices.len();

        let new_instances = Some(vec![InsertionInstance {
            position: Vec2::zero(),
            orientation: Mat2::identity(),
            _pading: 0,
            depth: 500.,
            color: [0., 0., 0., 1.],
        }]);
        Self {
            new_instances,
            instances,
            index_buffer,
            vertex_buffer,
            pipeline,
            number_indices,
            number_instances: 0,
        }
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        self.update_instances();
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(1, self.instances.get_bindgroup(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..));
        render_pass.draw_indexed(
            0..self.number_indices as u32,
            0,
            0..self.number_instances as u32,
        );
    }

    pub fn new_instances(&mut self, instances: Vec<InsertionInstance>) {
        self.new_instances = Some(instances)
    }

    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().cloned().collect();
            self.instances.update(instances_data.as_slice());
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InsertionVertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
}

unsafe impl bytemuck::Zeroable for InsertionVertex {}
unsafe impl bytemuck::Pod for InsertionVertex {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct InsertionInstance {
    pub position: Vec2,
    pub depth: f32,
    pub _pading: u32,
    pub orientation: Mat2,
    pub color: [f32; 4],
}

unsafe impl bytemuck::Zeroable for InsertionInstance {}
unsafe impl bytemuck::Pod for InsertionInstance {}

impl InsertionInstance {
    pub fn new(position: Vec2, depth: f32, orientation: ultraviolet::Rotor2, color: u32) -> Self {
        Self {
            position,
            depth,
            _pading: 0,
            orientation: orientation.into_matrix(),
            color: crate::utils::instance::Instance::color_from_u32(color).into(),
        }
    }
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