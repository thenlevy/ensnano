//! This module handles the 2D view

use crate::design::{Design, DesignNotification, DesignNotificationContent, Nucl};
use crate::mediator;
use crate::{DrawArea, Duration, PhySize, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit;
use mediator::{
    ActionMode, Application, CrossCut, Cut, Mediator, Notification, RawHelixCreation, RmStrand,
    Selection, StrandConstruction, Xover,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

use crate::utils::camera2d as camera;
use crate::utils::PhantomElement;
mod controller;
mod data;
mod flattypes;
mod view;
use camera::{Camera, Globals};
use controller::Controller;
use data::Data;
use flattypes::*;
use view::View;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;
type CameraPtr = Rc<RefCell<Camera>>;

/// A Flatscene handles one design at a time
pub struct FlatScene {
    /// Handle the data to send to the GPU
    view: Vec<ViewPtr>,
    /// Handle the data representing the design
    data: Vec<DataPtr>,
    /// Handle the inputs
    controller: Vec<Controller>,
    /// The area on which the flatscene is displayed
    area: DrawArea,
    /// The size of the window on which the flatscene is displayed
    window_size: PhySize,
    /// The identifer of the design being drawn
    selected_design: usize,
    device: Rc<Device>,
    queue: Rc<Queue>,
    mediator: Arc<Mutex<Mediator>>,
}

impl FlatScene {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhySize,
        area: DrawArea,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Self {
        Self {
            view: Vec::new(),
            data: Vec::new(),
            controller: Vec::new(),
            area,
            window_size,
            selected_design: 0,
            device,
            queue,
            mediator,
        }
    }

    /// Add a design to the scene. This creates a new `View`, a new `Data` and a new `Controller`
    fn add_design(&mut self, design: Arc<RwLock<Design>>) {
        let globals = Globals {
            resolution: [self.area.size.width as f32, self.area.size.height as f32],
            scroll_offset: [-1., -1.],
            zoom: 80.,
            _padding: 0.,
        };
        let camera = Rc::new(RefCell::new(Camera::new(globals)));
        let view = Rc::new(RefCell::new(View::new(
            self.device.clone(),
            self.queue.clone(),
            self.area,
            camera.clone(),
        )));
        let data = Rc::new(RefCell::new(Data::new(view.clone(), design, 0)));
        let mut controller = Controller::new(
            view.clone(),
            data.clone(),
            self.window_size,
            self.area.size,
            camera,
            self.mediator.clone(),
        );
        controller.fit();
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
            self.data[self.selected_design]
                .borrow_mut()
                .perform_update();
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

    /// Change the action beign performed by the user
    fn change_action_mode(&mut self, action_mode: ActionMode) {
        if let Some(controller) = self.controller.get_mut(self.selected_design) {
            controller.set_action_mode(action_mode)
        }
    }

    /// Handle an input that happend while the cursor was on the flatscene drawing area
    fn input(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>) {
        if let Some(controller) = self.controller.get_mut(self.selected_design) {
            let consequence = controller.input(event, cursor_position);
            use controller::Consequence;
            match consequence {
                Consequence::Xover(nucl1, nucl2) => {
                    let (prime5_id, prime3_id) =
                        self.data[self.selected_design].borrow().xover(nucl1, nucl2);
                    let strand_5prime = self.data[self.selected_design]
                        .borrow()
                        .get_strand(prime5_id)
                        .unwrap();
                    let strand_3prime = self.data[self.selected_design]
                        .borrow()
                        .get_strand(prime3_id)
                        .unwrap();
                    self.mediator
                        .lock()
                        .unwrap()
                        .update_opperation(Arc::new(Xover {
                            strand_3prime,
                            strand_5prime,
                            prime3_id,
                            prime5_id,
                            undo: false,
                            design_id: self.selected_design,
                        }))
                }
                Consequence::Cut(nucl) => {
                    let strand_id = self.data[self.selected_design].borrow().get_strand_id(nucl);
                    if let Some(strand_id) = strand_id {
                        println!("cutting");
                        let strand = self.data[self.selected_design]
                            .borrow()
                            .get_strand(strand_id)
                            .unwrap();
                        let nucl = nucl.to_real();
                        self.mediator
                            .lock()
                            .unwrap()
                            .update_opperation(Arc::new(Cut {
                                nucl,
                                strand_id,
                                strand,
                                undo: false,
                                design_id: self.selected_design,
                            }))
                    }
                }
                Consequence::FreeEnd(free_end) => self.data[self.selected_design]
                    .borrow_mut()
                    .set_free_end(free_end),
                Consequence::CutFreeEnd(nucl, free_end) => {
                    let strand_id = self.data[self.selected_design].borrow().get_strand_id(nucl);
                    if let Some(strand_id) = strand_id {
                        println!("cutting");
                        let strand = self.data[self.selected_design]
                            .borrow()
                            .get_strand(strand_id)
                            .unwrap();
                        let nucl = nucl.to_real();
                        self.mediator
                            .lock()
                            .unwrap()
                            .update_opperation(Arc::new(Cut {
                                nucl,
                                strand_id,
                                strand,
                                undo: false,
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
                            let source_strand = self.data[self.selected_design]
                                .borrow()
                                .get_strand(source_id)
                                .unwrap();
                            let target_strand = self.data[self.selected_design]
                                .borrow()
                                .get_strand(target_id)
                                .unwrap();
                            self.mediator
                                .lock()
                                .unwrap()
                                .update_opperation(Arc::new(CrossCut {
                                    source_strand,
                                    target_strand,
                                    source_id,
                                    target_id,
                                    target_3prime,
                                    nucl: to.to_real(),
                                    undo: false,
                                    design_id: self.selected_design,
                                }))
                        }
                    }
                }
                Consequence::NewCandidate(candidate) => {
                    let phantom = candidate.map(|n| PhantomElement {
                        position: n.position as i32,
                        helix_id: n.helix.real as u32,
                        forward: n.forward,
                        bound: false,
                        design_id: self.selected_design as u32,
                    });
                    let other = candidate.and_then(|candidate| {
                        self.data[self.selected_design]
                            .borrow()
                            .get_best_suggestion(candidate)
                    });
                    self.view[self.selected_design]
                        .borrow_mut()
                        .set_candidate_suggestion(candidate, other);
                    let candidate = if let Some(selection) = phantom.and_then(|p| {
                        self.data[self.selected_design]
                            .borrow()
                            .phantom_to_selection(p)
                    }) {
                        Some(selection)
                    } else {
                        phantom.map(|p| Selection::Phantom(p))
                    };
                    self.mediator
                        .lock()
                        .unwrap()
                        .set_candidate(phantom, candidate)
                }
                Consequence::RmStrand(nucl) => {
                    let strand_id = self.data[self.selected_design].borrow().get_strand_id(nucl);
                    if let Some(strand_id) = strand_id {
                        println!("removing strand");
                        let strand = self.data[self.selected_design]
                            .borrow()
                            .get_strand(strand_id)
                            .unwrap();
                        self.mediator
                            .lock()
                            .unwrap()
                            .update_opperation(Arc::new(RmStrand {
                                strand,
                                strand_id,
                                undo: false,
                                design_id: self.selected_design,
                            }))
                    }
                }
                Consequence::RmHelix(h_id) => {
                    let helix = self.data[self.selected_design]
                        .borrow_mut()
                        .can_delete_helix(h_id);
                    if let Some((helix, helix_id)) = helix {
                        self.mediator.lock().unwrap().update_opperation(Arc::new(
                            RawHelixCreation {
                                helix,
                                helix_id,
                                design_id: self.selected_design,
                                delete: true,
                            },
                        ))
                    }
                }
                Consequence::Built(builder) => {
                    let color = builder.get_strand_color();
                    self.mediator
                        .lock()
                        .unwrap()
                        .update_opperation(Arc::new(StrandConstruction {
                            redo: Some(color),
                            color,
                            builder,
                        }));
                }
                Consequence::FlipVisibility(helix, apply_to_other) => self.data
                    [self.selected_design]
                    .borrow_mut()
                    .flip_visibility(helix, apply_to_other),
                Consequence::FlipGroup(helix) => self.data[self.selected_design]
                    .borrow_mut()
                    .flip_group(helix),
                Consequence::FollowingSuggestion(nucl, double) => {
                    let nucl2 = self.data[self.selected_design]
                        .borrow()
                        .get_best_suggestion(nucl);
                    if let Some(nucl2) = nucl2 {
                        self.attempt_xover(nucl, nucl2);
                        if double {
                            self.attempt_xover(nucl.prime3(), nucl2.prime5());
                        }
                    }
                }
                Consequence::Centering(nucl) => {
                    let nucl = nucl.to_real();
                    self.mediator
                        .lock()
                        .unwrap()
                        .request_centering(nucl, self.selected_design)
                }
                Consequence::Select(nucl) => {
                    let selection = self.data[self.selected_design]
                        .borrow()
                        .get_selection(nucl, self.selected_design as u32);
                    self.mediator
                        .lock()
                        .unwrap()
                        .notify_unique_selection(selection);
                }
                Consequence::DrawingSelection(c1, c2) => self.view[self.selected_design]
                    .borrow_mut()
                    .update_rectangle(c1, c2),
                Consequence::ReleasedSelection(_, _) => {
                    self.view[self.selected_design]
                        .borrow_mut()
                        .clear_rectangle();
                    //self.data[self.selected_design].borrow().get_helices_in_rect(c1, c2, camera);
                    self.mediator.lock().unwrap().notify_multiple_selection(
                        self.data[self.selected_design].borrow().selection.clone(),
                    );
                }
                Consequence::PasteRequest(nucl) => {
                    self.mediator.lock().unwrap().attempt_paste(nucl.to_real());
                }
                Consequence::AddClick(click) => {
                    self.data[self.selected_design]
                        .borrow_mut()
                        .add_selection(click);
                    self.mediator.lock().unwrap().notify_multiple_selection(
                        self.data[self.selected_design].borrow().selection.clone(),
                    );
                }
                _ => (),
            }
        }
    }

    fn attempt_xover(&self, nucl1: FlatNucl, nucl2: FlatNucl) {
        let source = nucl1.to_real();
        let target = nucl2.to_real();
        self.mediator
            .lock()
            .unwrap()
            .xover_request(source, target, self.selected_design);
    }

    /// Ask the view if it has been modified since the last drawing
    fn needs_redraw_(&self) -> bool {
        if let Some(view) = self.view.get(self.selected_design) {
            self.data[self.selected_design]
                .borrow_mut()
                .perform_update();
            view.borrow().needs_redraw()
        } else {
            false
        }
    }
}

impl Application for FlatScene {
    fn on_notify(&mut self, notification: Notification) {
        #[allow(clippy::single_match)] // we will implement for notification in the future
        match notification {
            Notification::NewDesign(design) => self.add_design(design),
            Notification::NewActionMode(am) => self.change_action_mode(am),
            Notification::DesignNotification(DesignNotification { design_id, content }) => {
                self.data[design_id].borrow_mut().notify_update();
                if let DesignNotificationContent::ViewNeedReset = content {
                    self.data[design_id].borrow_mut().notify_reset();
                }
            }
            Notification::FitRequest => self.controller[self.selected_design].fit(),
            Notification::Selection3D(selection) => {
                self.needs_redraw(Duration::from_nanos(1));
                if selection.len() <= 1 {
                    self.data[self.selected_design]
                        .borrow_mut()
                        .set_selection(*selection.get(0).unwrap_or(&Selection::Nothing));
                    self.data[self.selected_design].borrow_mut().notify_update();
                    self.view[self.selected_design]
                        .borrow_mut()
                        .center_selection();
                }
            }
            Notification::Save(d_id) => self.data[d_id].borrow_mut().save_isometry(),
            Notification::ToggleText(b) => {
                self.view[self.selected_design].borrow_mut().set_show_sec(b)
            }
            Notification::ShowTorsion(b) => {
                for v in self.view.iter() {
                    v.borrow_mut().set_show_torsion(b);
                }
            }
            Notification::Pasting(b) => {
                for c in self.controller.iter_mut() {
                    c.set_pasting(b)
                }
            }
            Notification::CameraTarget(_) => (),
            Notification::NewSelectionMode(selection_mode) => {
                for data in self.data.iter() {
                    data.borrow_mut().change_selection_mode(selection_mode);
                }
            }
            Notification::AppNotification(_) => (),
            Notification::NewSensitivity(_) => (),
            Notification::ClearDesigns => (),
            Notification::NewCandidate(_) => (), //TODO this can prove usefull in the future
            Notification::Centering(_, _) => (),
            Notification::CameraRotation(_, _) => (),
        }
    }

    fn on_resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.resize(window_size, area)
    }

    fn on_event(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>) {
        self.input(event, cursor_position)
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

    fn needs_redraw(&mut self, _: Duration) -> bool {
        self.needs_redraw_()
    }
}
