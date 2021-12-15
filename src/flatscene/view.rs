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
use super::data::{
    FlatTorsion, FreeEnd, GpuVertex, Helix, HelixModel, Shift, Strand, StrandVertex,
};
use super::{CameraPtr, FlatIdx, FlatNucl};
use crate::utils::bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use crate::utils::texture::Texture;
use crate::utils::Ndc;
use crate::{DrawArea, PhySize};
use ensnano_design::Nucl;
use iced_wgpu::wgpu;
use std::rc::Rc;
use wgpu::{Device, Queue, RenderPipeline};

mod helix_view;
use helix_view::{HelixView, StrandView};
mod background;
mod insertion;
mod rectangle;
use super::FlatSelection;
use crate::consts::SAMPLE_COUNT;
use crate::utils::{chars2d as chars, circles2d as circles};
use ahash::RandomState;
use background::Background;
use chars::CharDrawer;
pub use chars::CharInstance;
pub use circles::CircleInstance;
use circles::{CircleDrawer, CircleKind};
use iced_winit::winit::dpi::PhysicalPosition;
use insertion::InsertionDrawer;
pub use insertion::InsertionInstance;
use rectangle::Rectangle;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

const SHOW_SUGGESTION: bool = false;

pub struct View {
    device: Rc<Device>,
    queue: Rc<Queue>,
    depth_texture: Texture,
    helices: Vec<Helix>,
    helices_view: Vec<HelixView>,
    helices_background: Vec<HelixView>,
    strands: Vec<StrandView>,
    pasted_strands: Vec<StrandView>,
    helices_model: Vec<HelixModel>,
    models: DynamicBindGroup,
    globals_top: UniformBindGroup,
    globals_bottom: UniformBindGroup,
    helices_pipeline: RenderPipeline,
    strand_pipeline: RenderPipeline,
    camera_top: CameraPtr,
    camera_bottom: CameraPtr,
    splited: bool,
    was_updated: bool,
    area_size: PhySize,
    free_end: Option<FreeEnd>,
    background: Background,
    circle_drawer_top: CircleDrawer,
    circle_drawer_bottom: CircleDrawer,
    nucl_highlighter_top: CircleDrawer,
    nucl_highlighter_bottom: CircleDrawer,
    rotation_widget: CircleDrawer,
    insertion_drawer: InsertionDrawer,
    char_drawers_top: HashMap<char, CharDrawer>,
    char_drawers_bottom: HashMap<char, CharDrawer>,
    char_map_top: HashMap<char, Vec<CharInstance>>,
    char_map_bottom: HashMap<char, Vec<CharInstance>>,
    show_sec: bool,
    suggestions: Vec<(FlatNucl, FlatNucl)>,
    suggestions_view: Vec<StrandView>,
    selected_strands: Vec<StrandView>,
    candidate_strands: Vec<StrandView>,
    selected_helices: Vec<FlatIdx>,
    candidate_helices: Vec<FlatIdx>,
    candidate_nucl: Vec<FlatNucl>,
    selected_nucl: Vec<FlatNucl>,
    suggestion_candidate: Option<(FlatNucl, FlatNucl)>,
    torsions: HashMap<(FlatNucl, FlatNucl), FlatTorsion>,
    show_torsion: bool,
    rectangle: Rectangle,
    groups: Arc<BTreeMap<usize, bool>>,
    basis_map: Arc<HashMap<Nucl, char, RandomState>>,
    edition_info: Option<EditionInfo>,
    hovered_nucl: Option<FlatNucl>,
}

pub struct EditionInfo {
    pub nt_length: usize,
    pub nm_length: f32,
    pub nucl: FlatNucl,
}

impl View {
    pub(super) fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        area: DrawArea,
        camera_top: CameraPtr,
        camera_bottom: CameraPtr,
        splited: bool,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(device.as_ref(), &area.size, SAMPLE_COUNT);
        let models = DynamicBindGroup::new(device.clone(), queue.clone());
        let globals_top = UniformBindGroup::new(
            device.clone(),
            queue.clone(),
            camera_top.borrow().get_globals(),
        );
        let globals_bottom = UniformBindGroup::new(
            device.clone(),
            queue.clone(),
            camera_bottom.borrow().get_globals(),
        );

        let depth_stencil_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState::IGNORE,
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: Default::default(),
        });

        let helices_pipeline = helices_pipeline_descr(
            &device,
            globals_top.get_layout(), // the layout is the same for both globals
            models.get_layout(),
            depth_stencil_state.clone(),
        );
        let strand_pipeline = strand_pipeline_descr(
            &device,
            globals_top.get_layout(),
            depth_stencil_state.clone(),
        );

        let background = Background::new(&device, globals_top.get_layout(), &depth_stencil_state);
        let circle_drawer_top = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals_top.get_layout(),
            CircleKind::FullCircle,
        );
        let circle_drawer_bottom = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals_top.get_layout(),
            CircleKind::FullCircle,
        );
        let nucl_highlighter_top = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals_top.get_layout(),
            CircleKind::FullCircle,
        );
        let nucl_highlighter_bottom = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals_top.get_layout(),
            CircleKind::FullCircle,
        );
        let rotation_widget = CircleDrawer::new(
            device.clone(),
            queue.clone(),
            globals_top.get_layout(),
            CircleKind::RotationWidget,
        );
        let rectangle = Rectangle::new(&device, queue.clone());
        let chars = [
            'A', 'T', 'G', 'C', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', 'n', 't',
            'm', '.', '/', ' ', '(', ')',
        ];
        let mut char_drawers_top = HashMap::new();
        let mut char_map_top = HashMap::new();
        let mut char_drawers_bottom = HashMap::new();
        let mut char_map_bottom = HashMap::new();
        for c in chars.iter() {
            char_drawers_top.insert(
                *c,
                CharDrawer::new(device.clone(), queue.clone(), globals_top.get_layout(), *c),
            );
            char_drawers_bottom.insert(
                *c,
                CharDrawer::new(device.clone(), queue.clone(), globals_top.get_layout(), *c),
            );
            char_map_top.insert(*c, Vec::new());
            char_map_bottom.insert(*c, Vec::new());
        }

        let insertion_drawer = InsertionDrawer::new(
            device.clone(),
            queue.clone(),
            globals_top.get_layout(),
            depth_stencil_state.clone(),
        );

        Self {
            device,
            queue,
            depth_texture,
            helices: Vec::new(),
            helices_view: Vec::new(),
            strands: Vec::new(),
            pasted_strands: Vec::new(),
            helices_model: Vec::new(),
            helices_background: Vec::new(),
            models,
            globals_top,
            globals_bottom,
            helices_pipeline,
            strand_pipeline,
            camera_top,
            camera_bottom,
            splited,
            was_updated: false,
            area_size: area.size,
            free_end: None,
            background,
            circle_drawer_top,
            circle_drawer_bottom,
            nucl_highlighter_top,
            nucl_highlighter_bottom,
            rotation_widget,
            char_drawers_top,
            char_map_top,
            char_drawers_bottom,
            char_map_bottom,
            show_sec: false,
            suggestions: vec![],
            suggestions_view: vec![],
            selected_strands: vec![],
            candidate_strands: vec![],
            selected_helices: vec![],
            candidate_helices: vec![],
            suggestion_candidate: None,
            torsions: HashMap::new(),
            show_torsion: false,
            rectangle,
            insertion_drawer,
            groups: Default::default(),
            basis_map: Default::default(),
            edition_info: Default::default(),
            selected_nucl: vec![],
            candidate_nucl: vec![],
            hovered_nucl: None,
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

    pub fn set_splited(&mut self, splited: bool) {
        self.was_updated = true;
        self.splited = splited;
    }

    pub fn update_strand_building_info(&mut self, info: Option<EditionInfo>) {
        if info.as_ref().map(|i| i.nucl) != self.edition_info.as_ref().map(|i| i.nucl) {
            self.was_updated = true;
        }
        self.edition_info = info;
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
        let other_cam = if self.splited {
            &self.camera_bottom
        } else {
            &self.camera_top
        };
        self.strands.iter_mut().last().unwrap().update(
            &strand,
            helices,
            &self.free_end,
            &self.camera_top,
            other_cam,
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
        self.strands.truncate(strands.len());
        for (i, s) in self.strands.iter_mut().enumerate() {
            let other_cam = if self.splited {
                &self.camera_bottom
            } else {
                &self.camera_top
            };
            if i < strands.len() {
                s.update(
                    &strands[i],
                    helices,
                    &self.free_end,
                    &self.camera_top,
                    other_cam,
                );
            }
        }
        for strand in strands.iter().skip(self.strands.len()) {
            self.add_strand(strand, helices)
        }
        let mut insertions = Vec::new();
        for s in strands.iter() {
            for i in s.get_insertions(helices) {
                insertions.push(i);
            }
        }
        self.insertion_drawer.new_instances(insertions);
        self.was_updated = true;
    }

    pub fn update_selection(&mut self, strands: &[Strand], helices: &[Helix]) {
        self.selected_strands.clear();
        for s in strands.iter() {
            let mut strand_view = StrandView::new(self.device.clone(), self.queue.clone());
            strand_view.update(s, helices, &None, &self.camera_top, &self.camera_bottom);
            self.selected_strands.push(strand_view);
        }
        self.was_updated = true;
    }

    pub fn update_candidate(&mut self, strands: &[Strand], helices: &[Helix]) {
        self.candidate_strands.clear();
        for s in strands.iter() {
            let mut strand_view = StrandView::new(self.device.clone(), self.queue.clone());
            strand_view.update(s, helices, &None, &self.camera_top, &self.camera_bottom);
            self.candidate_strands.push(strand_view);
        }
        self.was_updated = true;
    }

    pub fn set_candidate_nucls(&mut self, nucls: Vec<FlatNucl>) {
        self.candidate_nucl = nucls;
    }

    pub fn set_selected_nucls(&mut self, nucls: Vec<FlatNucl>) {
        self.selected_nucl = nucls;
    }

    pub fn update_pasted_strand(&mut self, strand: &[Strand], helices: &[Helix]) {
        self.pasted_strands = strand
            .iter()
            .map(|strand| {
                let mut pasted_strand = StrandView::new(self.device.clone(), self.queue.clone());
                pasted_strand.update(
                    strand,
                    helices,
                    &None,
                    &self.camera_top,
                    &self.camera_bottom,
                );
                pasted_strand
            })
            .collect();
    }

    pub fn set_free_end(&mut self, free_end: Option<FreeEnd>) {
        self.free_end = free_end;
    }

    pub fn needs_redraw(&self) -> bool {
        if self.splited {
            self.camera_top.borrow().was_updated()
                | self.was_updated
                | self.camera_bottom.borrow().was_updated()
        } else {
            self.camera_top.borrow().was_updated() | self.was_updated
        }
    }

    pub fn set_selected_helices(&mut self, selection: Vec<FlatIdx>) {
        self.selected_helices = selection;
    }

    pub fn set_candidate_helices(&mut self, selection: Vec<FlatIdx>) {
        self.candidate_helices = selection;
    }

    pub fn center_selection(&mut self, selection: FlatSelection) -> Option<(FlatNucl, FlatNucl)> {
        self.camera_top.borrow_mut().zoom_closer();
        self.was_updated = true;
        match selection {
            FlatSelection::Bound(_, n1, n2) => {
                self.helices[n1.helix].make_visible(n1.position, self.camera_top.clone());
                let world_pos_1 = self.helices[n1.helix].get_nucl_position(&n1, Shift::No);
                let world_pos_2 = self.helices[n2.helix].get_nucl_position(&n2, Shift::No);
                let screen_pos_1 = self
                    .camera_top
                    .borrow()
                    .world_to_norm_screen(world_pos_1.x, world_pos_1.y);
                let screen_pos_2 = self
                    .camera_top
                    .borrow()
                    .world_to_norm_screen(world_pos_2.x, world_pos_2.y);
                if (screen_pos_1.0 - screen_pos_2.0) * (screen_pos_1.0 - screen_pos_2.0)
                    + (screen_pos_1.1 - screen_pos_2.1) * (screen_pos_1.1 - screen_pos_2.1)
                    > 0.25
                {
                    // Center the topmost nucleotide on the top camera
                    if screen_pos_1.1 < screen_pos_2.1 {
                        Some((n1, n2))
                    } else {
                        Some((n2, n1))
                    }
                } else {
                    None
                }
            }
            FlatSelection::Nucleotide(
                _,
                FlatNucl {
                    helix, position, ..
                },
            ) => {
                self.helices[helix].make_visible(position, self.camera_top.clone());
                None
            }
            _ => None,
        }
    }

    pub fn center_split(&mut self, n1: FlatNucl, n2: FlatNucl) {
        let zoom = self.camera_top.borrow().get_globals().zoom;
        self.camera_bottom.borrow_mut().set_zoom(zoom);
        self.helices[n1.helix].make_visible(n1.position, self.camera_top.clone());
        self.helices[n2.helix].make_visible(n2.position, self.camera_bottom.clone());
    }

    /// Center the top camera on a nucleotide
    pub fn center_nucl(&mut self, nucl: FlatNucl, bottom: bool) {
        let helix = nucl.helix;
        let position = self.helices[helix].get_pivot(nucl.position);
        if bottom {
            self.camera_bottom.borrow_mut().set_center(position);
        } else {
            self.camera_top.borrow_mut().set_center(position);
        }
    }

    pub fn update_rectangle(&mut self, c1: PhysicalPosition<f64>, c2: PhysicalPosition<f64>) {
        if self.splited {
            if (c1.y < self.area_size.height as f64 / 2.)
                != (c2.y < self.area_size.height as f64 / 2.)
            {
                self.rectangle.update_corners(None);
            } else {
                self.rectangle.update_corners(Some([
                    Ndc::from_physical(c1, self.area_size),
                    Ndc::from_physical(c2, self.area_size),
                ]));
            }
        } else {
            self.rectangle.update_corners(Some([
                Ndc::from_physical(c1, self.area_size),
                Ndc::from_physical(c2, self.area_size),
            ]));
        }
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
        if let Some(globals) = self.camera_top.borrow_mut().update() {
            log::debug!("new camera globals: {:?}", globals);
            self.globals_top.update(globals);
            need_new_circles = true;
        }
        if let Some(globals) = self.camera_bottom.borrow_mut().update() {
            self.globals_bottom.update(globals);
            need_new_circles = true;
        }
        if need_new_circles || self.was_updated {
            let instances_top = self.generate_circle_instances(&self.camera_top);
            let instances_bottom = self.generate_circle_instances(&self.camera_bottom);
            if SHOW_SUGGESTION {
                self.view_suggestion();
            }
            self.circle_drawer_top.new_instances(Rc::new(instances_top));
            self.circle_drawer_bottom
                .new_instances(Rc::new(instances_bottom));
            self.generate_char_instances();
            let nucleotide_highliting = Rc::new(self.generate_nucl_highlighting());
            self.nucl_highlighter_top
                .new_instances(nucleotide_highliting.clone());
            self.nucl_highlighter_bottom
                .new_instances(nucleotide_highliting);
        }

        let clear_color = wgpu::Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 0.,
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

        let bottom = false;
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
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
        if self.splited {
            render_pass.set_viewport(
                0.,
                0.,
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
                0.,
                1.,
            );
            render_pass.set_scissor_rect(0, 0, self.area_size.width, self.area_size.height / 2);
        }
        render_pass.set_bind_group(0, self.globals_top.get_bindgroup(), &[]);
        render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
        self.background.draw(&mut render_pass);

        render_pass.set_pipeline(&self.helices_pipeline);

        for background in self.helices_background.iter() {
            background.draw(&mut render_pass);
        }
        for helix in self.helices_view.iter() {
            helix.draw(&mut render_pass);
        }
        self.rotation_widget.draw(&mut render_pass);
        drop(render_pass);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
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
        if self.splited {
            render_pass.set_viewport(
                0.,
                0.,
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
                0.,
                1.,
            );
            render_pass.set_scissor_rect(0, 0, self.area_size.width, self.area_size.height / 2);
        }
        render_pass.set_bind_group(0, self.globals_top.get_bindgroup(), &[]);
        render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
        self.circle_drawer_top.draw(&mut render_pass);
        for drawer in self.char_drawers_top.values_mut() {
            drawer.draw(&mut render_pass);
        }
        self.insertion_drawer.draw(&mut render_pass);
        render_pass.set_pipeline(&self.strand_pipeline);
        for strand in self.strands.iter() {
            strand.draw(&mut render_pass, bottom);
        }
        for strand in self.pasted_strands.iter() {
            strand.draw(&mut render_pass, bottom);
        }
        for suggestion in self.suggestions_view.iter() {
            suggestion.draw(&mut render_pass, bottom);
        }
        for highlight in self.selected_strands.iter() {
            highlight.draw(&mut render_pass, bottom);
        }
        for highlight in self.candidate_strands.iter() {
            highlight.draw(&mut render_pass, bottom);
        }
        render_pass.set_pipeline(&self.helices_pipeline);
        self.nucl_highlighter_top.draw(&mut render_pass);
        drop(render_pass);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
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
        if self.splited {
            render_pass.set_viewport(
                0.,
                0.,
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
                0.,
                1.,
            );
            render_pass.set_scissor_rect(0, 0, self.area_size.width, self.area_size.height / 2);
        }
        render_pass.set_bind_group(0, self.globals_top.get_bindgroup(), &[]);
        render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
        self.background.draw_border(&mut render_pass);

        render_pass.set_pipeline(&self.strand_pipeline);
        for strand in self.strands.iter() {
            strand.draw_split(&mut render_pass, bottom);
        }
        for strand in self.pasted_strands.iter() {
            strand.draw_split(&mut render_pass, bottom);
        }
        for suggestion in self.suggestions_view.iter() {
            suggestion.draw_split(&mut render_pass, bottom);
        }
        for highlight in self.selected_strands.iter() {
            highlight.draw_split(&mut render_pass, bottom);
        }
        for highlight in self.candidate_strands.iter() {
            highlight.draw_split(&mut render_pass, bottom);
        }

        drop(render_pass);
        if self.splited {
            let bottom = true;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: attachment,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
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
                0.,
                self.area_size.height as f32 / 2.,
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
                0.,
                1.,
            );
            render_pass.set_scissor_rect(
                0,
                self.area_size.height / 2,
                self.area_size.width,
                self.area_size.height / 2,
            );
            render_pass.set_bind_group(0, self.globals_bottom.get_bindgroup(), &[]);
            render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
            self.background.draw(&mut render_pass);

            render_pass.set_pipeline(&self.helices_pipeline);

            for background in self.helices_background.iter() {
                background.draw(&mut render_pass);
            }
            for helix in self.helices_view.iter() {
                helix.draw(&mut render_pass);
            }
            self.rotation_widget.draw(&mut render_pass);
            drop(render_pass);
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: attachment,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
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
                0.,
                self.area_size.height as f32 / 2.,
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
                0.,
                1.,
            );
            render_pass.set_scissor_rect(
                0,
                self.area_size.height / 2,
                self.area_size.width,
                self.area_size.height / 2,
            );
            render_pass.set_bind_group(0, self.globals_bottom.get_bindgroup(), &[]);
            render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
            self.circle_drawer_bottom.draw(&mut render_pass);
            for drawer in self.char_drawers_bottom.values_mut() {
                drawer.draw(&mut render_pass);
            }
            self.insertion_drawer.draw(&mut render_pass);
            render_pass.set_pipeline(&self.strand_pipeline);
            for strand in self.strands.iter() {
                strand.draw(&mut render_pass, bottom);
            }
            for strand in self.pasted_strands.iter() {
                strand.draw(&mut render_pass, bottom);
            }
            for suggestion in self.suggestions_view.iter() {
                suggestion.draw(&mut render_pass, bottom);
            }
            for highlight in self.selected_strands.iter() {
                highlight.draw(&mut render_pass, bottom);
            }
            for highlight in self.candidate_strands.iter() {
                highlight.draw(&mut render_pass, bottom);
            }
            render_pass.set_pipeline(&self.helices_pipeline);
            self.nucl_highlighter_bottom.draw(&mut render_pass);
            drop(render_pass);
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: attachment,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
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
                0.,
                self.area_size.height as f32 / 2.,
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
                0.,
                1.,
            );
            render_pass.set_scissor_rect(
                0,
                self.area_size.height / 2,
                self.area_size.width,
                self.area_size.height / 2,
            );
            render_pass.set_bind_group(0, self.globals_bottom.get_bindgroup(), &[]);
            render_pass.set_bind_group(1, self.models.get_bindgroup(), &[]);
            self.background.draw_border(&mut render_pass);

            render_pass.set_pipeline(&self.strand_pipeline);
            for strand in self.strands.iter() {
                strand.draw_split(&mut render_pass, bottom);
            }
            for strand in self.pasted_strands.iter() {
                strand.draw_split(&mut render_pass, bottom);
            }
            for suggestion in self.suggestions_view.iter() {
                suggestion.draw_split(&mut render_pass, bottom);
            }
            for highlight in self.selected_strands.iter() {
                highlight.draw_split(&mut render_pass, bottom);
            }
            for highlight in self.candidate_strands.iter() {
                highlight.draw_split(&mut render_pass, bottom);
            }
        }
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
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
    fn generate_circle_instances(&self, camera: &CameraPtr) -> Vec<CircleInstance> {
        let mut ret = Vec::new();
        self.collect_helices_circles(&mut ret, camera);
        self.collect_suggestions(&mut ret);
        if self.show_torsion {
            self.collect_torsion_indications(&mut ret);
        }
        ret
    }

    fn generate_nucl_highlighting(&self) -> Vec<CircleInstance> {
        let mut ret = Vec::new();
        self.collect_nucl_highlight(&mut ret);
        ret
    }

    /// Add the helices circles to the list of circle instances
    fn collect_helices_circles(&self, circles: &mut Vec<CircleInstance>, camera: &CameraPtr) {
        for h in self.helices.iter() {
            if let Some(circle) = h.get_circle(camera, self.groups.as_ref()) {
                circles.push(circle);
            }
            for circle in h.handle_circles() {
                circles.push(circle)
            }
        }
        for h_id in self
            .selected_helices
            .iter()
            .filter(|h| !self.candidate_helices.contains(h))
        {
            if let Some(mut circle) = self
                .helices
                .get(h_id.0)
                .and_then(|h| h.get_circle(camera, self.groups.as_ref()))
            {
                circle.set_radius(circle.radius * 1.4);
                circle.set_color(0xFF_FF0000);
                circles.push(circle);
            }
        }

        for h_id in self.candidate_helices.iter() {
            if let Some(mut circle) = self
                .helices
                .get(h_id.0)
                .and_then(|h| h.get_circle(camera, self.groups.as_ref()))
            {
                circle.set_radius(circle.radius * 1.4);
                circle.set_color(0xFF_00FF00);
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

    /// Collect the candidate/selection circles
    fn collect_nucl_highlight(&self, circles: &mut Vec<CircleInstance>) {
        for n in self.candidate_nucl.iter() {
            let candidate_color = crate::consts::CANDIDATE_COLOR;
            if let Some(h1) = self.helices.get(n.helix.flat.0) {
                let mut c = h1.get_circle_nucl(n.position, n.forward, candidate_color);
                c.set_radius(std::f32::consts::FRAC_1_SQRT_2);
                circles.push(c)
            } else {
                log::error!("Could not get flat helix {}", n.helix.flat.0);
            }
        }

        for n in self.selected_nucl.iter() {
            let selected_color = crate::consts::SELECTED_COLOR;
            if let Some(h1) = self.helices.get(n.helix.flat.0) {
                let mut c = h1.get_circle_nucl(n.position, n.forward, selected_color);
                c.set_radius(std::f32::consts::FRAC_1_SQRT_2);
                circles.push(c)
            } else {
                log::error!("Could not get flat helix {}", n.helix.flat.0);
            }
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

    pub fn set_hovered_nucl(&mut self, hovered_nucl: Option<FlatNucl>) {
        self.was_updated |= hovered_nucl != self.hovered_nucl;
        self.hovered_nucl = hovered_nucl;
    }

    pub fn set_candidate_suggestion(
        &mut self,
        candidate: Option<FlatNucl>,
        other: Option<FlatNucl>,
    ) {
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
        for v in self.char_map_top.values_mut() {
            v.clear();
        }
        for v in self.char_map_bottom.values_mut() {
            v.clear();
        }

        for h in self.helices.iter() {
            h.add_char_instances(
                &self.camera_top,
                &mut self.char_map_top,
                &self.char_drawers_top,
                self.groups.as_ref(),
                self.basis_map.as_ref(),
                self.show_sec,
                &self.edition_info,
                &self.hovered_nucl,
            );
            h.add_char_instances(
                &self.camera_bottom,
                &mut self.char_map_bottom,
                &self.char_drawers_bottom,
                self.groups.as_ref(),
                self.basis_map.as_ref(),
                self.show_sec,
                &self.edition_info,
                &self.hovered_nucl,
            )
        }

        for (c, v) in self.char_map_top.iter() {
            self.char_drawers_top
                .get_mut(c)
                .unwrap()
                .new_instances(Rc::new(v.clone()))
        }
        for (c, v) in self.char_map_bottom.iter() {
            self.char_drawers_bottom
                .get_mut(c)
                .unwrap()
                .new_instances(Rc::new(v.clone()))
        }
    }

    pub fn set_wheels(&mut self, wheels: Vec<CircleInstance>) {
        self.was_updated = true;
        self.rotation_widget.new_instances(Rc::new(wheels));
    }

    pub fn update_maps(
        &mut self,
        groups: Arc<BTreeMap<usize, bool>>,
        basis_map: Arc<HashMap<Nucl, char, RandomState>>,
    ) {
        self.was_updated = true;
        self.groups = groups;
        self.basis_map = basis_map;
    }
}

fn helices_pipeline_descr(
    device: &Device,
    globals_layout: &wgpu::BindGroupLayout,
    models_layout: &wgpu::BindGroupLayout,
    depth_stencil: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    let vs_module = &device.create_shader_module(&wgpu::include_spirv!("view/grid.vert.spv"));
    let fs_module = &device.create_shader_module(&wgpu::include_spirv!("view/grid.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[globals_layout, models_layout],
        push_constant_ranges: &[],
        label: None,
    });
    let color_targets = &[wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::ALL,
    }];
    let primitive_state = wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        ..Default::default()
    };

    let desc = wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main",
            targets: color_targets,
        }),
        primitive: primitive_state,
        depth_stencil,
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32, 3 => Uint32],
            }],
        },
        multisample: wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        label: None,
    };

    device.create_render_pipeline(&desc)
}

fn strand_pipeline_descr(
    device: &Device,
    globals: &wgpu::BindGroupLayout,
    depth_stencil: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    let vs_module = &device.create_shader_module(&wgpu::include_spirv!("view/strand.vert.spv"));
    let fs_module = &device.create_shader_module(&wgpu::include_spirv!("view/strand.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[globals],
        push_constant_ranges: &[],
        label: None,
    });
    let color_targets = &[wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    }];

    let primitive_state = wgpu::PrimitiveState {
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
    };

    let desc = wgpu::RenderPipelineDescriptor {
        primitive: primitive_state,
        layout: Some(&pipeline_layout),
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main",
            targets: color_targets,
        }),
        depth_stencil,
        vertex: wgpu::VertexState {
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<StrandVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32, 4 => Float32],
            }],
            module: &vs_module,
            entry_point: "main",
        },
        multisample: wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
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

impl ToString for EditionInfo {
    fn to_string(&self) -> String {
        format!("{}nt/{:.1}nm", self.nt_length, self.nm_length)
    }
}
