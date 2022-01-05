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
//! This module handles the 2D view

//use crate::design::{DesignNotification, DesignNotificationContent, Nucl, StrandBuilder};
use crate::{utils::camera2d::FitRectangle, DrawArea, Duration, PhySize, WindowEvent};
use ensnano_design::Nucl;
use ensnano_interactor::{
    application::{AppId, Application, Notification},
    operation::*,
    ActionMode, DesignOperation, PhantomElement, Selection, SelectionMode, StrandBuilder,
    StrandBuildingStatus,
};
use iced_wgpu::wgpu;
use iced_winit::winit;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

use crate::utils::camera2d as camera;
mod controller;
mod data;
mod flattypes;
mod view;
use camera::{Camera, Globals};
use controller::Controller;
use data::Data;
pub use data::DesignReader;
use flattypes::*;
use std::time::Instant;
use view::View;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;
type CameraPtr = Rc<RefCell<Camera>>;

/// A Flatscene handles one design at a time
pub struct FlatScene<S: AppState> {
    /// Handle the data to send to the GPU
    view: Vec<ViewPtr>,
    /// Handle the data representing the design
    data: Vec<DataPtr>,
    /// Handle the inputs
    controller: Vec<Controller<S>>,
    /// The area on which the flatscene is displayed
    area: DrawArea,
    /// The size of the window on which the flatscene is displayed
    window_size: PhySize,
    /// The identifer of the design being drawn
    selected_design: usize,
    device: Rc<Device>,
    queue: Rc<Queue>,
    last_update: Instant,
    splited: bool,
    old_state: S,
    requests: Arc<Mutex<dyn Requests>>,
}

impl<S: AppState> FlatScene<S> {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhySize,
        area: DrawArea,
        requests: Arc<Mutex<dyn Requests>>,
        initial_state: S,
    ) -> Self {
        let mut ret = Self {
            view: Vec::new(),
            data: Vec::new(),
            controller: Vec::new(),
            area,
            window_size,
            selected_design: 0,
            device,
            queue,
            last_update: Instant::now(),
            splited: false,
            old_state: initial_state.clone(),
            requests: requests.clone(),
        };
        ret.add_design(initial_state.get_design_reader(), requests);
        ret
    }

    /// Add a design to the scene. This creates a new `View`, a new `Data` and a new `Controller`
    fn add_design(&mut self, reader: S::Reader, requests: Arc<Mutex<dyn Requests>>) {
        let height = if self.splited {
            self.area.size.height as f32 / 2.
        } else {
            self.area.size.height as f32
        };
        let globals_top = Globals::default([self.area.size.width as f32, height]);
        let globals_bottom = Globals::default([self.area.size.width as f32, height]);

        let camera_top = Rc::new(RefCell::new(Camera::new(globals_top, false)));
        let camera_bottom = Rc::new(RefCell::new(Camera::new(globals_bottom, true)));
        camera_top
            .borrow_mut()
            .init_fit(FitRectangle::INITIAL_RECTANGLE);
        camera_bottom
            .borrow_mut()
            .init_fit(FitRectangle::INITIAL_RECTANGLE);
        let view = Rc::new(RefCell::new(View::new(
            self.device.clone(),
            self.queue.clone(),
            self.area,
            camera_top.clone(),
            camera_bottom.clone(),
            self.splited,
        )));
        let data = Rc::new(RefCell::new(Data::new(view.clone(), reader, 0, requests)));
        //data.borrow_mut().perform_update();
        // TODO is this update necessary ?
        let controller = Controller::new(
            view.clone(),
            data.clone(),
            self.window_size,
            self.area.size,
            camera_top,
            camera_bottom,
            self.splited,
        );
        if self.view.len() > 0 {
            self.view[0] = view;
            self.data[0] = data;
            self.controller[0] = controller;
        } else {
            self.view.push(view);
            self.data.push(data);
            self.controller.push(controller);
        }
    }

    /// Draw the view of the currently selected design
    fn draw_view(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        if let Some(view) = self.view.get(self.selected_design) {
            view.borrow_mut().draw(encoder, target, self.area);
        }
    }

    /// This function must be called when the drawing area of the flatscene is modified
    fn resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.window_size = window_size;
        self.area = area;
        for view in self.view.iter() {
            view.borrow_mut().resize(area);
        }
        for controller in self.controller.iter_mut() {
            controller.resize(window_size, area.size);
        }
    }

    /// Handle an input that happend while the cursor was on the flatscene drawing area
    fn input(
        &mut self,
        event: &WindowEvent,
        cursor_position: PhysicalPosition<f64>,
        app_state: &S,
    ) {
        if let Some(controller) = self.controller.get_mut(self.selected_design) {
            let consequence = controller.input(event, cursor_position, app_state);
            self.read_consequence(consequence, Some(app_state));
        }
    }

    fn read_consequence(&mut self, consequence: controller::Consequence, new_state: Option<&S>) {
        let app_state = new_state.unwrap_or(&self.old_state);
        use controller::Consequence;
        match consequence {
            Consequence::Xover(nucl1, nucl2) => {
                let (prime5_id, prime3_id) =
                    self.data[self.selected_design].borrow().xover(nucl1, nucl2);
                self.requests
                    .lock()
                    .unwrap()
                    .update_opperation(Arc::new(Xover {
                        prime3_id,
                        prime5_id,
                        undo: false,
                        design_id: self.selected_design,
                    }))
            }
            Consequence::Cut(nucl) => {
                let strand_id = self.data[self.selected_design].borrow().get_strand_id(nucl);
                if let Some(strand_id) = strand_id {
                    log::info!("cutting {:?}", nucl);
                    let nucl = nucl.to_real();
                    self.requests
                        .lock()
                        .unwrap()
                        .update_opperation(Arc::new(Cut {
                            nucl,
                            strand_id,
                            design_id: self.selected_design,
                        }))
                }
            }
            Consequence::FreeEnd(free_end) => {
                self.requests.lock().unwrap().suspend_op();
                let candidates = free_end
                    .as_ref()
                    .map(|fe| {
                        fe.candidates
                            .iter()
                            .map(|c| Selection::Nucleotide(0, c.to_real()))
                            .collect()
                    })
                    .unwrap_or(Vec::new());
                self.data[self.selected_design]
                    .borrow_mut()
                    .set_free_end(free_end);
                self.requests.lock().unwrap().new_candidates(candidates);
            }
            Consequence::CutFreeEnd(nucl, free_end) => {
                let strand_id = self.data[self.selected_design].borrow().get_strand_id(nucl);
                if let Some(strand_id) = strand_id {
                    log::info!("cutting {:?}", nucl);
                    let nucl = nucl.to_real();
                    self.requests
                        .lock()
                        .unwrap()
                        .update_opperation(Arc::new(Cut {
                            nucl,
                            strand_id,
                            design_id: self.selected_design,
                        }))
                }
                self.data[self.selected_design]
                    .borrow_mut()
                    .set_free_end(free_end);
            }
            Consequence::CutCross(from, to) => {
                if from.helix != to.helix {
                    // CrossCut with source and target on the same helix are forbidden
                    let op_var = self.data[self.selected_design].borrow().cut_cross(from, to);
                    if let Some((source_id, target_id, target_3prime)) = op_var {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_opperation(Arc::new(CrossCut {
                                source_id,
                                target_id,
                                target_3prime,
                                nucl: to.to_real(),
                                design_id: self.selected_design,
                            }))
                    }
                }
            }
            Consequence::NewCandidate(candidate) if app_state.is_pasting() => self
                .requests
                .lock()
                .unwrap()
                .set_paste_candidate(candidate.map(|n| n.to_real())),
            Consequence::NewCandidate(candidate) => {
                let phantom = candidate.map(|n| PhantomElement {
                    position: n.position as i32,
                    helix_id: n.helix.real as u32,
                    forward: n.forward,
                    bound: false,
                    design_id: self.selected_design as u32,
                });
                let candidate = if let Some(selection) = phantom.and_then(|p| {
                    self.data[self.selected_design]
                        .borrow()
                        .phantom_to_selection(p, app_state.get_selection_mode())
                }) {
                    Some(selection)
                } else {
                    phantom.map(|p| Selection::Phantom(p))
                };
                self.requests
                    .lock()
                    .unwrap()
                    .new_candidates(candidate.iter().cloned().collect())
            }
            Consequence::Built => {
                self.requests.lock().unwrap().suspend_op();
            }
            Consequence::FlipVisibility(helix, apply_to_other) => self.data[self.selected_design]
                .borrow_mut()
                .flip_visibility(helix, apply_to_other),
            Consequence::FlipGroup(helix) => self.data[self.selected_design]
                .borrow_mut()
                .flip_group(helix),
            Consequence::FollowingSuggestion(nucl, double) => {
                let nucl2 = self.data[self.selected_design]
                    .borrow()
                    .get_best_suggestion(nucl)
                    .or(self.data[self.selected_design]
                        .borrow()
                        .can_make_auto_xover(nucl));
                if let Some(nucl2) = nucl2 {
                    self.attempt_xover(nucl, nucl2);
                    if double {
                        self.attempt_xover(nucl.prime3(), nucl2.prime5());
                    }
                }
            }
            Consequence::Centering(nucl, bottom) => {
                self.view[self.selected_design]
                    .borrow_mut()
                    .center_nucl(nucl, bottom);
                let nucl = nucl.to_real();
                self.requests
                    .lock()
                    .unwrap()
                    .request_centering_on_nucl(nucl, self.selected_design)
            }
            Consequence::DrawingSelection(c1, c2) => self.view[self.selected_design]
                .borrow_mut()
                .update_rectangle(c1, c2),
            Consequence::ReleasedSelection(selection) => {
                self.view[self.selected_design]
                    .borrow_mut()
                    .clear_rectangle();
                //self.data[self.selected_design].borrow().get_helices_in_rect(c1, c2, camera);
                if let Some(selection) = selection {
                    self.requests.lock().unwrap().new_selection(selection);
                }
            }
            Consequence::PasteRequest(nucl) => {
                self.requests
                    .lock()
                    .unwrap()
                    .attempt_paste(nucl.map(|n| n.to_real()));
            }
            Consequence::AddClick(click, add) => {
                let mut new_selection = app_state.get_selection().to_vec();
                self.data[self.selected_design].borrow_mut().add_selection(
                    click,
                    add,
                    &mut new_selection,
                    app_state.get_selection_mode(),
                );
                self.requests.lock().unwrap().new_selection(new_selection);
            }
            Consequence::SelectionChanged(selection) => {
                self.requests.lock().unwrap().new_selection(selection);
            }
            Consequence::ClearSelection => {
                self.requests.lock().unwrap().new_selection(vec![]);
            }
            Consequence::DoubleClick(click) => {
                let selection = self.data[self.selected_design]
                    .borrow()
                    .double_click_to_selection(click);
                if let Some(selection) = selection {
                    self.requests
                        .lock()
                        .unwrap()
                        .request_center_selection(selection, AppId::FlatScene)
                }
            }
            Consequence::Helix2DMvmtEnded => self.requests.lock().unwrap().suspend_op(),
            Consequence::Snap {
                pivots,
                translation,
            } => {
                let pivots = pivots.into_iter().map(|n| n.to_real()).collect();
                self.requests.lock().unwrap().apply_design_operation(
                    DesignOperation::SnapHelices {
                        pivots,
                        translation,
                    },
                );
            }
            Consequence::Rotation {
                helices,
                center,
                angle,
            } => {
                let helices = helices.into_iter().map(|fh| fh.real).collect();
                self.requests.lock().unwrap().apply_design_operation(
                    DesignOperation::RotateHelices {
                        helices,
                        center,
                        angle,
                    },
                )
            }
            Consequence::InitBuilding(nucl) => {
                let mut nucls = ensnano_interactor::extract_nucls_and_xover_ends(
                    app_state.get_selection(),
                    &app_state.get_design_reader(),
                );
                let nucl = nucl.to_real();

                if let Some(idx) = (0..nucls.len()).find(|i| nucls[*i] == nucl) {
                    // the nucleotide we start building on should be the first in the vec
                    nucls.swap(idx, 0);
                } else {
                    // If we start building on a non selected nucleotide, we ignore the selection
                    nucls = vec![nucl];
                }
                self.requests
                    .lock()
                    .unwrap()
                    .apply_design_operation(DesignOperation::RequestStrandBuilders { nucls });
            }
            Consequence::MoveBuilders(n) => {
                self.requests
                    .lock()
                    .unwrap()
                    .apply_design_operation(DesignOperation::MoveBuilders(n));
                self.requests.lock().unwrap().new_candidates(vec![]);
            }
            Consequence::NewHelixCandidate(flat_helix) => self
                .requests
                .lock()
                .unwrap()
                .new_candidates(vec![Selection::Helix(
                    self.selected_design as u32,
                    flat_helix.real as u32,
                )]),
            _ => (),
        }
    }

    fn check_timers(&mut self) {
        let consequence = self.controller[self.selected_design].check_timers();
        self.read_consequence(consequence, None);
    }

    fn attempt_xover(&self, nucl1: FlatNucl, nucl2: FlatNucl) {
        let source = nucl1.to_real();
        let target = nucl2.to_real();
        self.requests
            .lock()
            .unwrap()
            .xover_request(source, target, self.selected_design);
    }

    /// Ask the view if it has been modified since the last drawing
    fn needs_redraw_(&mut self, new_state: S) -> bool {
        self.check_timers();
        if let Some(view) = self.view.get(self.selected_design) {
            self.data[self.selected_design]
                .borrow_mut()
                .perform_update(&new_state, &self.old_state);
            self.old_state = new_state;
            let ret = view.borrow().needs_redraw();
            if ret {
                log::debug!("Flatscene requests redraw");
            }
            ret
        } else {
            false
        }
    }

    fn toggle_split_from_btn(&mut self) {
        self.splited ^= true;
        for c in self.controller.iter_mut() {
            c.set_splited(self.splited, true);
        }

        for v in self.view.iter_mut() {
            v.borrow_mut().set_splited(self.splited);
        }
    }

    fn split_and_center(&mut self, n1: FlatNucl, n2: FlatNucl) {
        self.splited = true;
        for v in self.view.iter_mut() {
            v.borrow_mut().set_splited(self.splited);
        }
        for c in self.controller.iter_mut() {
            c.set_splited(self.splited, false);
        }
        self.view[self.selected_design]
            .borrow_mut()
            .center_split(n1, n2);
    }
}

impl<S: AppState> Application for FlatScene<S> {
    type AppState = S;
    fn on_notify(&mut self, notification: Notification) {
        match notification {
            Notification::FitRequest => self.controller[self.selected_design].fit(),
            Notification::Save(d_id) => self.data[d_id].borrow_mut().save_isometry(),
            Notification::ToggleText(b) => {
                self.view[self.selected_design].borrow_mut().set_show_sec(b)
            }
            Notification::ShowTorsion(b) => {
                for v in self.view.iter() {
                    v.borrow_mut().set_show_torsion(b);
                }
            }
            Notification::CameraTarget(_) => (),
            Notification::NewSensitivity(_) => (),
            Notification::ClearDesigns => (),
            Notification::Centering(_, _) => (),
            Notification::CenterSelection(selection, app_id) => {
                log::info!("2D view centering selection {:?}", selection);
                let flat_selection = self.data[self.selected_design]
                    .borrow()
                    .convert_to_flat(selection);
                let flat_selection_bonds = self.data[self.selected_design]
                    .borrow()
                    .xover_to_nuclpair(flat_selection);
                if app_id != AppId::FlatScene {
                    let xover = self.view[self.selected_design]
                        .borrow_mut()
                        .center_selection(flat_selection_bonds);
                    if let Some((n1, n2)) = xover {
                        self.split_and_center(n1, n2);
                    }
                }
            }
            Notification::CameraRotation(_, _, _) => (),
            Notification::ModifersChanged(modifiers) => {
                for c in self.controller.iter_mut() {
                    c.update_modifiers(modifiers.clone())
                }
            }
            Notification::Split2d => self.toggle_split_from_btn(),
            Notification::Redim2dHelices(b) => {
                let selection = if b {
                    None
                } else {
                    Some(self.old_state.get_selection())
                };
                self.data[self.selected_design]
                    .borrow_mut()
                    .redim_helices(selection)
            }
            Notification::RenderingMode(_) => (),
            Notification::Background3D(_) => (),
            Notification::Fog(_) => (),
            Notification::WindowFocusLost => (),
            Notification::TeleportCamera(_, _) => (),
            Notification::FlipSplitViews => self.controller[0].flip_split_views(),
        }
    }

    fn on_resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.resize(window_size, area)
    }

    fn on_event(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>, state: &S) {
        self.input(event, cursor_position, state)
    }

    fn on_redraw_request(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _dt: Duration,
    ) {
        //println!("draw flatscene");
        self.draw_view(encoder, target)
    }

    fn needs_redraw(&mut self, _: Duration, state: S) -> bool {
        let now = Instant::now();
        if (now - self.last_update).as_millis() < 25 {
            false
        } else {
            self.last_update = now;
            self.needs_redraw_(state)
        }
    }

    fn is_splited(&self) -> bool {
        self.splited
    }
}

pub trait AppState: Clone {
    type Reader: DesignReader + ensnano_interactor::DesignReader;
    fn selection_was_updated(&self, other: &Self) -> bool;
    fn candidate_was_updated(&self, other: &Self) -> bool;
    fn get_selection(&self) -> &[Selection];
    fn get_candidates(&self) -> &[Selection];
    fn get_selection_mode(&self) -> SelectionMode;
    fn get_design_reader(&self) -> Self::Reader;
    fn get_strand_builders(&self) -> &[StrandBuilder];
    fn design_was_updated(&self, other: &Self) -> bool;
    fn is_changing_color(&self) -> bool;
    fn is_pasting(&self) -> bool;
    fn get_building_state(&self) -> Option<StrandBuildingStatus>;
}

use ultraviolet::Isometry2;
pub trait Requests {
    fn xover_request(&mut self, source: Nucl, target: Nucl, design_id: usize);
    fn request_center_selection(&mut self, selection: Selection, app_id: AppId);
    fn new_selection(&mut self, selection: Vec<Selection>);
    fn new_candidates(&mut self, candidates: Vec<Selection>);
    fn attempt_paste(&mut self, nucl: Option<Nucl>);
    fn request_centering_on_nucl(&mut self, nucl: Nucl, design_id: usize);
    fn update_opperation(&mut self, operation: Arc<dyn Operation>);
    fn set_isometry(&mut self, helix: usize, isometry: Isometry2);
    fn set_visibility_helix(&mut self, helix: usize, visibility: bool);
    fn flip_group(&mut self, helix: usize);
    fn suspend_op(&mut self);
    fn apply_design_operation(&mut self, op: DesignOperation);
    fn set_paste_candidate(&mut self, candidate: Option<Nucl>);
}
