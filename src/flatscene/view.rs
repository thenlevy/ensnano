use super::data::{GpuVertex, Helix, HelixModel, Strand, StrandVertex};
use super::CameraPtr;
use crate::utils::bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use crate::utils::texture::Texture;
use crate::{DrawArea, PhySize};
use iced_wgpu::wgpu;
use std::rc::Rc;
use wgpu::{Device, Queue, RenderPipeline};

mod helix_view;
use helix_view::{HelixView, StrandView};

pub struct View {
    device: Rc<Device>,
    queue: Rc<Queue>,
    depth_texture: Texture,
    helices: Vec<HelixView>,
    strands: Vec<StrandView>,
    helices_model: Vec<HelixModel>,
    models: DynamicBindGroup,
    globals: UniformBindGroup,
    helices_pipeline: RenderPipeline,
    strand_pipeline: RenderPipeline,
    camera: CameraPtr,
}

impl View {
    pub(super) fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhySize,
        camera: CameraPtr,
    ) -> Self {
        let depth_texture = Texture::create_depth_texture(device.clone().as_ref(), &window_size);
        let models = DynamicBindGroup::new(device.clone(), queue.clone());
        let globals =
            UniformBindGroup::new(device.clone(), queue.clone(), camera.borrow().get_globals());

        let depth_stencil_state = Some(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor::IGNORE,
                back: wgpu::StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        });

        let helices_pipeline = helices_pipeline_descr(
            &device,
            globals.get_layout(),
            models.get_layout(),
            depth_stencil_state.clone(),
        );
        let strand_pipeline =
            strand_pipeline_descr(&device, globals.get_layout(), depth_stencil_state.clone());

        Self {
            device: device.clone(),
            queue: queue.clone(),
            depth_texture,
            helices: Vec::new(),
            strands: Vec::new(),
            helices_model: Vec::new(),
            models,
            globals,
            helices_pipeline,
            strand_pipeline,
            camera,
        }
    }

    pub fn add_helix(&mut self, helix: Helix) {
        let id_helix = self.helices.len() as u32;
        self.helices.push(HelixView::new(
            self.device.clone(),
            self.queue.clone(),
            id_helix,
        ));
        self.helices[id_helix as usize].update(&helix);
        self.helices_model.push(helix.model());
        self.models.update(self.helices_model.as_slice());
    }

    pub fn add_strand(&mut self, strand: Strand, helices: &Vec<Helix>) {
        self.strands
            .push(StrandView::new(self.device.clone(), self.queue.clone()));
        self.strands
            .iter_mut()
            .last()
            .unwrap()
            .update(&strand, helices);
    }

    pub fn needs_redraw(&self) -> bool {
        self.camera.borrow().was_updated()
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        area: DrawArea,
    ) {
        if let Some(globals) = self.camera.borrow_mut().update() {
            self.globals.update(globals);
        }
        let clear_color = wgpu::Color {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        };
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: true,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: true,
                }),
            }),
        });
        render_pass.set_viewport(
            area.position.x as f32,
            area.position.y as f32,
            area.size.width as f32,
            area.size.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            area.position.x,
            area.position.y,
            area.size.width,
            area.size.height,
        );
        render_pass.set_bind_group(0, self.globals.get_bindgroup(), &[]);
        render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
        render_pass.set_pipeline(&self.helices_pipeline);

        for helix in self.helices.iter() {
            helix.draw(&mut render_pass);
        }
        render_pass.set_pipeline(&self.strand_pipeline);
        for strand in self.strands.iter() {
            strand.draw(&mut render_pass);
        }
    }
}

fn helices_pipeline_descr(
    device: &Device,
    globals_layout: &wgpu::BindGroupLayout,
    models_layout: &wgpu::BindGroupLayout,
    depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>,
) -> wgpu::RenderPipeline {
    let vs_module = &device.create_shader_module(wgpu::include_spirv!("view/grid.vert.spv"));
    let fs_module = &device.create_shader_module(wgpu::include_spirv!("view/grid.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[globals_layout, models_layout],
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
        depth_stencil_state,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 8,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 16,
                        format: wgpu::VertexFormat::Uint,
                        shader_location: 2,
                    },
                ],
            }],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&desc)
}

fn strand_pipeline_descr(
    device: &Device,
    globals: &wgpu::BindGroupLayout,
    depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>,
) -> wgpu::RenderPipeline {
    let vs_module = &device.create_shader_module(wgpu::include_spirv!("view/strand.vert.spv"));
    let fs_module = &device.create_shader_module(wgpu::include_spirv!("view/strand.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[globals],
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
        depth_stencil_state,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<StrandVertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 8,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 16,
                        format: wgpu::VertexFormat::Float4,
                        shader_location: 2,
                    },
                ],
            }],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&desc)
}
