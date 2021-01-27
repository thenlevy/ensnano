use super::data::{FlatTorsion, FreeEnd, GpuVertex, Helix, HelixModel, Strand, StrandVertex};
use super::{CameraPtr, FlatIdx, FlatNucl};
use crate::utils::bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use crate::utils::texture::Texture;
use crate::utils::Ndc;
use crate::{DrawArea, PhySize};
use iced_wgpu::wgpu;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use wgpu::{Device, Queue, RenderPipeline};

mod helix_view;
use helix_view::{HelixView, StrandView};
mod background;
mod rectangle;
use super::FlatSelection;
use crate::utils::{chars2d as chars, circles2d as circles};
use background::Background;
use chars::CharDrawer;
pub use chars::CharInstance;
pub use circles::CircleInstance;
use circles::{CircleDrawer, CircleKind};
use rectangle::Rectangle;

use crate::consts::SAMPLE_COUNT;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize};

const SHOW_SUGGESTION: bool = false;

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
    selection: FlatSelection,
    show_sec: bool,
    suggestions: Vec<(FlatNucl, FlatNucl)>,
    suggestions_view: Vec<StrandView>,
    suggestion_candidate: Option<(FlatNucl, FlatNucl)>,
    torsions: HashMap<(FlatNucl, FlatNucl), FlatTorsion>,
    show_torsion: bool,
    rectangle: Rectangle,
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
        let rectangle = Rectangle::new(&device, queue.clone());
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
            selection: FlatSelection::Nothing,
            show_sec: false,
            suggestions: vec![],
            suggestions_view: vec![],
            suggestion_candidate: None,
            torsions: HashMap::new(),
            show_torsion: false,
            rectangle,
        }
    }

    pub fn set_show_sec(&mut self, show_sec: bool) {
        self.show_sec = show_sec;
        self.was_updated = true;
    }

    pub fn set_show_torsion(&mut self, show: bool) {
        self.show_torsion = show;
        self.was_updated = true;
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

    pub fn rm_helices(&mut self, helices: BTreeSet<FlatIdx>) {
        if self.helices.len() == 0 {
            // self was already reseted
            return;
        }
        for h in helices.iter().rev() {
            self.helices.remove(h.0);
            self.helices_background.remove(h.0);
            self.helices_view.remove(h.0);
            self.helices_model.remove(h.0);
        }
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<(FlatNucl, FlatNucl)>) {
        self.suggestions = suggestions;
    }

    pub fn set_torsions(&mut self, torsions: HashMap<(FlatNucl, FlatNucl), FlatTorsion>) {
        self.torsions = torsions
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
        self.strands.iter_mut().last().unwrap().update(
            &strand,
            helices,
            &self.free_end,
            &self.selection,
        );
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
            s.update(&strands[i], helices, &self.free_end, &self.selection);
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

    pub fn set_selection(&mut self, selection: FlatSelection) {
        self.selection = selection;
    }

    pub fn center_selection(&mut self) {
        match self.selection {
            FlatSelection::Bound(
                _,
                FlatNucl {
                    helix, position, ..
                },
                _,
            ) => {
                self.helices[helix].make_visible(position, self.camera.clone());
            }
            _ => (),
        }
    }

    /// Center the camera on a nucleotide
    #[allow(dead_code)] // not used for now but might be useful in the future
    pub fn center_nucl(&mut self, nucl: FlatNucl) {
        let helix = nucl.helix;
        let position = self.helices[helix].get_pivot(nucl.position);
        self.camera.borrow_mut().set_center(position);
    }

    pub fn update_rectangle(&mut self, c1: PhysicalPosition<f64>, c2: PhysicalPosition<f64>) {
        self.rectangle.update_corners(Some([Ndc::from_physical(c1, self.area_size), Ndc::from_physical(c2, self.area_size)]));
        self.was_updated = true;
    }

    pub fn clear_rectangle(&mut self) {
        self.rectangle.update_corners(None);
        self.was_updated = true;
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
            if SHOW_SUGGESTION {
                self.view_suggestion();
            }
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
        for suggestion in self.suggestions_view.iter() {
            suggestion.draw(&mut render_pass);
        }

        self.circle_drawer.draw(&mut render_pass);
        self.rotation_widget.draw(&mut render_pass);
        for drawer in self.char_drawers.values_mut() {
            drawer.draw(&mut render_pass);
        }
        drop(render_pass);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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
        self.rectangle.draw(&mut render_pass);
        self.was_updated = false;
    }

    /// Return all the circles that must be displayed to represent the flatscene.
    ///
    /// Currently these circles are:
    ///  * Helices circles
    ///  * Cross-over suggestions
    ///  * Torsion indications
    fn generate_circle_instances(&self) -> Vec<CircleInstance> {
        let mut ret = Vec::new();
        self.collect_helices_circles(&mut ret);
        self.collect_suggestions(&mut ret);
        if self.show_torsion {
            self.collect_torsion_indications(&mut ret);
        }
        ret
    }

    /// Add the helices circles to the list of circle instances
    fn collect_helices_circles(&self, circles: &mut Vec<CircleInstance>) {
        for h in self.helices.iter() {
            if let Some(circle) = h.get_circle(&self.camera) {
                circles.push(circle);
            }
        }
    }

    /// Collect the cross-over suggestions
    fn collect_suggestions(&self, circles: &mut Vec<CircleInstance>) {
        let mut last_blue = None;
        let mut k = 1000;
        for (n1, n2) in self.suggestions.iter() {
            // Don't change the color if the value of n1 hasn't change, so that all suggested
            // cross-overs for n1 appears with the same color
            if last_blue != Some(n1) {
                k += 1;
                last_blue = Some(n1);
            }
            let color = {
                let hue = (k as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
                let saturation = (k as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.6;
                let value = (k as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.3;
                let hsv = color_space::Hsv::new(hue, saturation, value);
                let rgb = color_space::Rgb::from(hsv);
                (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
            };
            let h1 = &self.helices[n1.helix];
            let h2 = &self.helices[n2.helix];
            circles.push(h1.get_circle_nucl(n1.position, n1.forward, color));
            circles.push(h2.get_circle_nucl(n2.position, n2.forward, color));
        }
    }

    /// Collect the torsion indications.
    /// The radius and color of the circles depends on the strangth amplitude.
    fn collect_torsion_indications(&self, circles: &mut Vec<CircleInstance>) {
        for ((n0, n1), torsion) in self.torsions.iter() {
            let multiplier = ((torsion.strength_prime5 - torsion.strength_prime3).abs() / 200.)
                .max(0.08)
                .min(1.);
            let color = torsion_color(torsion.strength_prime5 - torsion.strength_prime3);
            let h0 = &self.helices[n0.helix];
            let mut circle = h0.get_circle_nucl(n0.position, n0.forward, color);
            circle.radius *= multiplier;
            if let Some(friend) = torsion.friend {
                // The circle center should be placed between the two friend cross-overs
                let circle2 = h0.get_circle_nucl(friend.0.position, n0.forward, color);
                circle.center = (circle.center + circle2.center) / 2.;
            }
            circles.push(circle);
            let h1 = &self.helices[n1.helix];
            let mut circle = h1.get_circle_nucl(n1.position, n1.forward, color);
            circle.radius *= multiplier;
            if let Some(friend) = torsion.friend {
                // The circle center should be placed between the two friend cross-overs
                let circle2 = h1.get_circle_nucl(friend.1.position, n1.forward, color);
                circle.center = (circle.center + circle2.center) / 2.;
            }
            circles.push(circle);
        }
    }

    fn view_suggestion(&mut self) {
        self.suggestions_view.clear();
        for (n1, n2) in self.suggestions.iter() {
            let mut view = StrandView::new(self.device.clone(), self.queue.clone());
            view.set_indication(*n1, *n2, &self.helices);
            self.suggestions_view.push(view);
        }
    }

    pub fn set_candidate(&mut self, candidate: Option<FlatNucl>, other: Option<FlatNucl>) {
        self.suggestions_view.clear();
        self.was_updated |= self.suggestion_candidate != candidate.zip(other);
        if let Some((n1, n2)) = candidate.zip(other) {
            let mut view = StrandView::new(self.device.clone(), self.queue.clone());
            view.set_indication(n1, n2, &self.helices);
            self.suggestions_view.push(view);
        }
        self.suggestion_candidate = candidate.zip(other);
    }

    fn generate_char_instances(&mut self) {
        for v in self.char_map.values_mut() {
            v.clear();
        }

        for h in self.helices.iter() {
            h.add_char_instances(
                &self.camera,
                &mut self.char_map,
                &self.char_drawers,
                self.show_sec,
            )
        }

        for (c, v) in self.char_map.iter() {
            self.char_drawers
                .get_mut(c)
                .unwrap()
                .new_instances(Rc::new(v.clone()))
        }
    }

    pub fn set_wheels(&mut self, wheels: Vec<CircleInstance>) {
        self.was_updated = true;
        self.rotation_widget.new_instances(Rc::new(wheels));
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

fn torsion_color(strength: f32) -> u32 {
    const RED_HUE: f32 = 0.;
    const BLUE_HUE: f32 = 240.;
    const MAX_STRENGTH: f32 = 200.;
    let hue = if strength > 0. { RED_HUE } else { BLUE_HUE };
    //println!("strength {}", strength);
    let sat = (strength / MAX_STRENGTH).min(1.).max(-1.);
    let val = (strength / MAX_STRENGTH).min(1.).max(-1.);
    let hsv = color_space::Hsv::new(hue as f64, sat.abs() as f64, val.abs() as f64);
    let rgb = color_space::Rgb::from(hsv);
    (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
}
