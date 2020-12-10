use super::data::{FreeEnd, GpuVertex, Helix, HelixModel, Strand, StrandVertex};
use super::CameraPtr;
use crate::utils::bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use crate::utils::texture::Texture;
use crate::{DrawArea, PhySize};
use iced_wgpu::wgpu;
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{Device, Queue, RenderPipeline};

mod helix_view;
use helix_view::{HelixView, StrandView};
mod background;
use crate::utils::{chars2d as chars, circles2d as circles};
use background::Background;
use chars::CharDrawer;
pub use chars::CharInstance;
pub use circles::CircleInstance;
use circles::{CircleDrawer, CircleKind};

use crate::consts::SAMPLE_COUNT;

pub struct View {
    device: Rc<Device>,
    queue: Rc<Queue>,
    depth_texture: Texture,
    helices: Vec<Helix>,
    helices_view: Vec<HelixView>,
    helices_background: Vec<HelixView>,
    strands: Vec<StrandView>,
    helices_model: Vec<HelixModel>,
    models: DynamicBindGroup,
    globals: UniformBindGroup,
    helices_pipeline: RenderPipeline,
    strand_pipeline: RenderPipeline,
    camera: CameraPtr,
    was_updated: bool,
    area_size: PhySize,
    free_end: Option<FreeEnd>,
    background: Background,
    circle_drawer: CircleDrawer,
    rotation_widget: CircleDrawer,
    char_drawers: HashMap<char, CharDrawer>,
    char_map: HashMap<char, Vec<CharInstance>>,
}

impl View {
    pub(super) fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        area: DrawArea,
        camera: CameraPtr,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(device.as_ref(), &area.size, SAMPLE_COUNT);
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

        let background = Background::new(&device, globals.get_layout(), &depth_stencil_state);
        let circle_drawer = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals.get_layout(),
            CircleKind::FullCircle,
        );
        let rotation_widget = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals.get_layout(),
            CircleKind::RotationWidget,
        );
        let chars = [
            'A', 'T', 'G', 'C', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-',
        ];
        let mut char_drawers = HashMap::new();
        let mut char_map = HashMap::new();
        for c in chars.iter() {
            char_drawers.insert(
                *c,
                CharDrawer::new(device.clone(), queue.clone(), globals.get_layout(), *c),
            );
            char_map.insert(*c, Vec::new());
        }

        Self {
            device,
            queue,
            depth_texture,
            helices: Vec::new(),
            helices_view: Vec::new(),
            strands: Vec::new(),
            helices_model: Vec::new(),
            helices_background: Vec::new(),
            models,
            globals,
            helices_pipeline,
            strand_pipeline,
            camera,
            was_updated: false,
            area_size: area.size,
            free_end: None,
            background,
            circle_drawer,
            rotation_widget,
            char_drawers,
            char_map,
        }
    }

    pub fn resize(&mut self, area: DrawArea) {
        self.depth_texture =
            Texture::create_depth_texture(self.device.clone().as_ref(), &area.size, SAMPLE_COUNT);
        self.area_size = area.size;
        self.was_updated = true;
    }

    fn add_helix(&mut self, helix: &Helix) {
        let id_helix = self.helices_view.len() as u32;
        self.helices_view.push(HelixView::new(
            self.device.clone(),
            self.queue.clone(),
            false,
        ));
        self.helices_background.push(HelixView::new(
            self.device.clone(),
            self.queue.clone(),
            true,
        ));
        self.helices_view[id_helix as usize].update(&helix);
        self.helices_background[id_helix as usize].update(&helix);
        self.helices_model.push(helix.model());
        self.models.update(self.helices_model.as_slice());
    }

    pub fn update_helices(&mut self, helices: &[Helix]) {
        for (i, h) in self.helices_view.iter_mut().enumerate() {
            self.helices_model[i] = helices[i].model();
            self.helices_background[i].update(&helices[i]);
            h.update(&helices[i])
        }
        for helix in helices.iter().skip(self.helices_view.len()) {
            self.add_helix(helix)
        }
        self.models.update(self.helices_model.as_slice());
        self.helices = helices.to_vec();
        self.was_updated = true;
    }

    pub fn add_strand(&mut self, strand: &Strand, helices: &[Helix]) {
        self.strands
            .push(StrandView::new(self.device.clone(), self.queue.clone()));
        self.strands
            .iter_mut()
            .last()
            .unwrap()
            .update(&strand, helices, &self.free_end);
    }

    pub fn reset(&mut self) {
        self.helices.clear();
        self.helices_model.clear();
        self.helices_view.clear();
        self.strands.clear();
        self.helices_background.clear();
    }

    pub fn update_strands(&mut self, strands: &[Strand], helices: &[Helix]) {
        for (i, s) in self.strands.iter_mut().enumerate() {
            s.update(&strands[i], helices, &self.free_end);
        }
        for strand in strands.iter().skip(self.strands.len()) {
            self.add_strand(strand, helices)
        }
        self.was_updated = true;
    }

    pub fn set_free_end(&mut self, free_end: Option<FreeEnd>) {
        self.free_end = free_end;
    }

    pub fn needs_redraw(&self) -> bool {
        self.camera.borrow().was_updated() | self.was_updated
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _area: DrawArea,
    ) {
        let mut need_new_circles = false;
        if let Some(globals) = self.camera.borrow_mut().update() {
            self.globals.update(globals);
            need_new_circles = true;
        }
        if need_new_circles || self.was_updated {
            let instances = self.generate_circle_instances();
            self.circle_drawer.new_instances(Rc::new(instances));
            self.generate_char_instances();
        }

        let clear_color = wgpu::Color {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        };

        let msaa_texture = if SAMPLE_COUNT > 1 {
            Some(crate::utils::texture::Texture::create_msaa_texture(
                self.device.clone().as_ref(),
                &self.area_size,
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
        render_pass.set_bind_group(0, self.globals.get_bindgroup(), &[]);
        render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
        self.background.draw(&mut render_pass);

        render_pass.set_pipeline(&self.helices_pipeline);

        for background in self.helices_background.iter() {
            background.draw(&mut render_pass);
        }
        for helix in self.helices_view.iter() {
            helix.draw(&mut render_pass);
        }

        render_pass.set_pipeline(&self.strand_pipeline);
        for strand in self.strands.iter() {
            strand.draw(&mut render_pass);
        }

        self.circle_drawer.draw(&mut render_pass);
        self.rotation_widget.draw(&mut render_pass);
        for drawer in self.char_drawers.values_mut() {
            drawer.draw(&mut render_pass);
        }
        self.was_updated = false;
    }

    fn generate_circle_instances(&self) -> Vec<CircleInstance> {
        let mut ret = Vec::new();
        for h in self.helices.iter() {
            if let Some(circle) = h.get_circle(&self.camera) {
                ret.push(circle);
            }
        }
        ret
    }

    fn generate_char_instances(&mut self) {
        for v in self.char_map.values_mut() {
            v.clear();
        }

        for h in self.helices.iter() {
            h.add_char_instances(&self.camera, &mut self.char_map, &self.char_drawers)
        }

        for (c, v) in self.char_map.iter() {
            self.char_drawers
                .get_mut(c)
                .unwrap()
                .new_instances(Rc::new(v.clone()))
        }
    }

    pub fn set_wheel(&mut self, wheel: Option<CircleInstance>) {
        self.was_updated = true;
        if let Some(wheel) = wheel {
            self.rotation_widget.new_instances(Rc::new(vec![wheel]));
        } else {
            self.rotation_widget.new_instances(Rc::new(Vec::new()));
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
                attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Uint, 3 => Uint],
            }],
        },
        sample_count: SAMPLE_COUNT,
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
                attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Float4, 3 => Float, 4 => Float],
            }],
        },
        sample_count: SAMPLE_COUNT,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&desc)
}
