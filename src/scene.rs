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
use iced_wgpu::wgpu;
use iced_winit::winit;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ultraviolet::{Mat4, Rotor3, Vec3};

use crate::scene::camera::FiniteVec3;
use crate::utils;
use crate::{DrawArea, PhySize, WindowEvent};
use ensnano_design::{group_attributes::GroupPivot, Nucl};
use ensnano_interactor::{
    application::{AppId, Application, Notification},
    operation::*,
    ActionMode, CenterOfSelection, DesignOperation, Selection, SelectionMode, StrandBuilder,
    WidgetBasis,
};
use instance::Instance;
use utils::instance;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

/// Computation of the view and projection matrix.
mod camera;
/// Display of the scene
mod view;
use view::{
    DrawType, HandleDir, HandleOrientation, HandlesDescriptor, LetterInstance,
    RotationMode as WidgetRotationMode, RotationWidgetDescriptor, RotationWidgetOrientation, View,
    ViewUpdate,
};
pub use view::{FogParameters, GridInstance};
/// Handling of inputs and notifications
mod controller;
use controller::{Consequence, Controller, WidgetTarget};
/// Handling of designs and internal data
mod data;
pub use controller::ClickMode;
use data::Data;
pub use data::DesignReader;
mod element_selector;
use element_selector::{ElementSelector, SceneElement};
mod maths_3d;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr<R> = Rc<RefCell<Data<R>>>;
use std::convert::TryInto;

/// A structure responsible of the 3D display of the designs
pub struct Scene<S: AppState> {
    /// The update to be performed before next frame
    update: SceneUpdate,
    /// The Object that handles the drawing to the 3d texture
    view: ViewPtr,
    /// The Object thant handles the designs data
    data: DataPtr<S::DesignReader>,
    /// The Object that handles input and notifications
    controller: Controller<S>,
    /// The limits of the area on which the scene is displayed
    area: DrawArea,
    element_selector: ElementSelector,
    older_state: S,
    requests: Arc<Mutex<dyn Requests>>,
}

impl<S: AppState> Scene<S> {
    /// Create a new scene.
    /// # Argument
    ///
    /// * `device` a reference to a `Device` object. This can be seen as a socket to the GPU
    ///
    /// * `queue` the command queue of `device`.
    ///
    /// * `window_size` the *Physical* size of the window in which the application is displayed
    ///
    /// * `area` the limits, in *physical* size of the area on which the scene is displayed
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhySize,
        area: DrawArea,
        requests: Arc<Mutex<dyn Requests>>,
        encoder: &mut wgpu::CommandEncoder,
        inital_state: S,
    ) -> Self {
        let update = SceneUpdate::new();
        let view: ViewPtr = Rc::new(RefCell::new(View::new(
            window_size,
            area.size,
            device.clone(),
            queue.clone(),
            encoder,
        )));
        let data: DataPtr<S::DesignReader> = Rc::new(RefCell::new(Data::new(
            inital_state.get_design_reader(),
            view.clone(),
        )));
        let controller: Controller<S> =
            Controller::new(view.clone(), data.clone(), window_size, area.size);
        let element_selector = ElementSelector::new(
            device,
            queue,
            controller.get_window_size(),
            view.clone(),
            area,
        );
        Self {
            view,
            data,
            update,
            controller,
            area,
            requests,
            element_selector,
            older_state: inital_state,
        }
    }

    /*
    /// Add a design to be rendered.
    fn add_design(&mut self, design: Arc<RwLock<Design>>) {
        self.data.borrow_mut().add_design(design)
    }*/

    /// Remove all designs
    fn clear_design(&mut self) {
        self.data.borrow_mut().clear_designs()
    }

    /// Input an event to the scene. The controller parse the event and return the consequence that
    /// the event must have.
    fn input(
        &mut self,
        event: &WindowEvent,
        cursor_position: PhysicalPosition<f64>,
        app_state: &S,
    ) {
        let consequence = self.controller.input(
            event,
            cursor_position,
            &mut self.element_selector,
            app_state,
        );
        self.read_consequence(consequence, app_state);
    }

    fn check_timers(&mut self, app_state: &S) {
        let consequence = self.controller.check_timers();
        self.read_consequence(consequence, app_state);
    }

    fn read_consequence(&mut self, consequence: Consequence, app_state: &S) {
        match consequence {
            Consequence::Nothing => (),
            Consequence::CameraMoved => self.notify(SceneNotification::CameraMoved),
            Consequence::CameraTranslated(dx, dy) => {
                let mut pivot: Option<FiniteVec3> = self
                    .data
                    .borrow()
                    .get_pivot_position()
                    .and_then(|p| p.try_into().ok());
                if pivot.is_none() {
                    self.data.borrow_mut().try_update_pivot_position(app_state);
                    pivot = self
                        .data
                        .borrow()
                        .get_pivot_position()
                        .and_then(|p| p.try_into().ok());
                }
                self.controller.set_pivot_point(pivot);
                self.controller.translate_camera(dx, dy);
                self.notify(SceneNotification::CameraMoved);
            }
            Consequence::XoverAtempt(source, target, d_id) => {
                self.attempt_xover(source, target, d_id);
                self.data.borrow_mut().end_free_xover();
            }
            Consequence::Translation(dir, x_coord, y_coord, target) => {
                let translation = self.view.borrow().compute_translation_handle(
                    x_coord as f32,
                    y_coord as f32,
                    dir,
                );
                if let Some(t) = translation {
                    match target {
                        WidgetTarget::Object => {
                            self.translate_selected_design(t, app_state);
                            if app_state.get_current_group_id().is_none() {
                                self.translate_group_pivot(t)
                            }
                        }
                        WidgetTarget::Pivot => self.translate_group_pivot(t),
                    }
                }
            }
            Consequence::HelixTranslated { helix, grid, x, y } => {
                log::info!("Moving helix {} to grid {} ({} {})", helix, grid, x, y);
                self.requests
                    .lock()
                    .unwrap()
                    .apply_design_operation(DesignOperation::AttachHelix { helix, grid, x, y });
            }
            Consequence::MovementEnded => {
                self.requests.lock().unwrap().suspend_op();
                self.data.borrow_mut().notify_handle_movement();
                self.view.borrow_mut().end_movement();
            }
            Consequence::HelixSelected(h_id) => self
                .requests
                .lock()
                .unwrap()
                .set_selection(vec![Selection::Helix(0, h_id as u32)], None),
            Consequence::InitRotation(mode, x, y, target) => {
                self.view
                    .borrow_mut()
                    .init_rotation(mode, x as f32, y as f32);
                if let Some(pivot) = self.view.borrow().get_group_pivot() {
                    self.requests.lock().unwrap().set_current_group_pivot(pivot);
                    if target == WidgetTarget::Pivot {
                        if let WidgetBasis::World = app_state.get_widget_basis() {
                            self.requests.lock().unwrap().toggle_widget_basis()
                        }
                    }
                }
            }
            Consequence::InitTranslation(x, y, _target) => {
                self.view.borrow_mut().init_translation(x as f32, y as f32);
                if let Some(pivot) = self.view.borrow().get_group_pivot() {
                    self.requests.lock().unwrap().set_current_group_pivot(pivot)
                }
            }
            Consequence::Rotation(x, y, target) => {
                let rotation = self.view.borrow().compute_rotation(x as f32, y as f32);
                if let Some((rotation, origin, positive)) = rotation {
                    if rotation.bv.mag() > 1e-3 {
                        match target {
                            WidgetTarget::Object => {
                                self.rotate_selected_desgin(rotation, origin, positive, app_state);
                                if app_state.get_current_group_id().is_none() {
                                    self.requests.lock().unwrap().rotate_group_pivot(rotation)
                                }
                            }
                            WidgetTarget::Pivot => {
                                self.requests.lock().unwrap().rotate_group_pivot(rotation)
                            }
                        }
                    }
                    self.data.borrow_mut().notify_handle_movement();
                } else {
                    log::warn!("Warning rotiation was None")
                }
            }
            Consequence::Swing(x, y) => {
                let mut pivot: Option<FiniteVec3> = self
                    .data
                    .borrow()
                    .get_pivot_position()
                    .and_then(|p| p.try_into().ok());
                if pivot.is_none() {
                    self.data.borrow_mut().try_update_pivot_position(app_state);
                    pivot = self
                        .data
                        .borrow()
                        .get_pivot_position()
                        .and_then(|p| p.try_into().ok());
                }
                self.controller.set_pivot_point(pivot);
                self.controller.swing(-x, -y);
                self.notify(SceneNotification::CameraMoved);
            }
            Consequence::ToggleWidget => self.requests.lock().unwrap().toggle_widget_basis(),
            Consequence::BuildEnded => self.requests.lock().unwrap().suspend_op(),
            Consequence::Undo => self.requests.lock().unwrap().undo(),
            Consequence::Redo => self.requests.lock().unwrap().redo(),
            Consequence::Building(position) => {
                self.requests
                    .lock()
                    .unwrap()
                    .update_builder_position(position);
            }
            Consequence::Candidate(element) => self.set_candidate(element, app_state),
            Consequence::PivotElement(element) => {
                self.data.borrow_mut().set_pivot_element(element, app_state);
                let pivot = self.data.borrow().get_pivot_position();
                self.view.borrow_mut().update(ViewUpdate::FogCenter(pivot));
            }
            Consequence::ElementSelected(element, adding) => {
                if adding {
                    self.add_selection(element, app_state.get_selection(), app_state)
                } else {
                    self.select(element, app_state)
                }
            }
            Consequence::InitFreeXover(nucl, d_id, position) => {
                self.data.borrow_mut().init_free_xover(nucl, position, d_id)
            }
            Consequence::MoveFreeXover(element, position) => self
                .data
                .borrow_mut()
                .update_free_xover_target(element, position),
            Consequence::EndFreeXover => self.data.borrow_mut().end_free_xover(),
            Consequence::BuildHelix {
                grid_id,
                design_id,
                length,
                position,
                x,
                y,
            } => {
                self.requests
                    .lock()
                    .unwrap()
                    .update_opperation(Arc::new(GridHelixCreation {
                        grid_id,
                        design_id: design_id as usize,
                        x,
                        y,
                        length,
                        position,
                    }));
                self.select(Some(SceneElement::Grid(design_id, grid_id)), app_state);
            }
            Consequence::PasteCandidate(element) => self.pasting_candidate(element),
            Consequence::Paste(element) => self.attempt_paste(element),
            Consequence::DoubleClick(element) => {
                let selection = self.data.borrow().to_selection(element, app_state);
                if let Some(selection) = selection {
                    self.requests
                        .lock()
                        .unwrap()
                        .request_center_selection(selection, AppId::Scene);
                }
            }
            Consequence::InitBuild(nucl) => self.requests.lock().unwrap().apply_design_operation(
                DesignOperation::RequestStrandBuilders { nucls: vec![nucl] },
            ),
        };
    }

    /// If a nucleotide is selected, and the clicked_pixel corresponds to an other nucleotide,
    /// request a cross-over between the two nucleotides.
    fn attempt_xover(&mut self, source: Nucl, target: Nucl, design_id: usize) {
        self.requests
            .lock()
            .unwrap()
            .xover_request(source, target, design_id)
    }

    fn element_center(&mut self, _app_state: &S) -> Option<SceneElement> {
        let clicked_pixel = PhysicalPosition::new(
            self.area.size.width as f64 / 2.,
            self.area.size.height as f64 / 2.,
        );
        let grid = self
            .view
            .borrow()
            .grid_intersection(0.5, 0.5)
            .map(|g| SceneElement::Grid(g.design_id as u32, g.grid_id));

        grid.or_else(move || self.element_selector.set_selected_id(clicked_pixel))
    }

    fn select(&mut self, element: Option<SceneElement>, app_state: &S) {
        let (selection, center_of_selection) =
            self.data.borrow_mut().set_selection(element, app_state);
        if let Some(selection) = selection {
            self.requests
                .lock()
                .unwrap()
                .set_selection(vec![selection], center_of_selection);
        }
    }

    fn add_selection(
        &mut self,
        element: Option<SceneElement>,
        current_selection: &[Selection],
        app_state: &S,
    ) {
        let selection =
            self.data
                .borrow_mut()
                .add_to_selection(element, current_selection, app_state);
        if let Some((selection, center_of_selection)) = selection {
            self.requests
                .lock()
                .unwrap()
                .set_selection(selection, center_of_selection);
        }
    }

    fn attempt_paste(&mut self, element: Option<SceneElement>) {
        let nucl = self.data.borrow().element_to_nucl(&element, false);
        self.requests
            .lock()
            .unwrap()
            .attempt_paste(nucl.map(|n| n.0));
    }

    fn pasting_candidate(&self, element: Option<SceneElement>) {
        let nucl = self.data.borrow().element_to_nucl(&element, false);
        self.requests
            .lock()
            .unwrap()
            .set_paste_candidate(nucl.map(|n| n.0));
    }

    fn set_candidate(&mut self, element: Option<SceneElement>, app_state: &S) {
        let new_candidates = self.data.borrow_mut().set_candidate(element, app_state);
        let widget = if let Some(SceneElement::WidgetElement(widget_id)) = element {
            Some(widget_id)
        } else {
            None
        };
        self.view.borrow_mut().set_widget_candidate(widget);
        let selection = if let Some(c) = new_candidates {
            vec![c]
        } else {
            vec![]
        };
        self.requests.lock().unwrap().set_candidate(selection);
    }

    fn translate_selected_design(&mut self, translation: Vec3, app_state: &S) {
        let rotor = self.data.borrow().get_widget_basis(app_state);
        self.view.borrow_mut().translate_widgets(translation);
        if rotor.is_none() {
            return;
        }
        let rotor = rotor.unwrap();
        let right = Vec3::unit_x().rotated_by(rotor);
        let top = Vec3::unit_y().rotated_by(rotor);
        let dir = Vec3::unit_z().rotated_by(rotor);

        let reader = app_state.get_design_reader();
        let helices = ensnano_interactor::set_of_helices_containing_selection(
            app_state.get_selection(),
            &reader,
        );
        let grids = ensnano_interactor::set_of_grids_containing_selection(
            app_state.get_selection(),
            &reader,
        );
        let at_most_one_grid = grids.as_ref().map(|g| g.len() <= 1).unwrap_or(false);

        let group_id = app_state.get_current_group_id();

        let translation_op: Arc<dyn Operation> =
            if let Some(helices) = helices.filter(|_| at_most_one_grid) {
                Arc::new(HelixTranslation {
                    design_id: 0,
                    helices,
                    right: Vec3::unit_x().rotated_by(rotor),
                    top: Vec3::unit_y().rotated_by(rotor),
                    dir: Vec3::unit_z().rotated_by(rotor),
                    x: translation.dot(right),
                    y: translation.dot(top),
                    z: translation.dot(dir),
                    snap: true,
                    group_id,
                    replace: false,
                })
            } else if let Some(grids) = grids {
                Arc::new(GridTranslation {
                    design_id: 0,
                    grid_ids: grids,
                    right: Vec3::unit_x().rotated_by(rotor),
                    top: Vec3::unit_y().rotated_by(rotor),
                    dir: Vec3::unit_z().rotated_by(rotor),
                    x: translation.dot(right),
                    y: translation.dot(top),
                    z: translation.dot(dir),
                    group_id,
                    replace: false,
                })
            } else {
                return;
            };

        self.requests
            .lock()
            .unwrap()
            .update_opperation(translation_op);
    }

    fn translate_group_pivot(&mut self, translation: Vec3) {
        self.view.borrow_mut().translate_widgets(translation);
        self.requests
            .lock()
            .unwrap()
            .translate_group_pivot(translation);
    }

    fn rotate_selected_desgin(
        &mut self,
        rotation: Rotor3,
        origin: Vec3,
        positive: bool,
        app_state: &S,
    ) {
        log::debug!(
            "Rotation {:?}, positive {}",
            rotation.into_angle_plane(),
            positive
        );
        let (mut angle, mut plane) = rotation.into_angle_plane();
        if !positive {
            angle *= -1.;
            plane *= -1.;
        }
        let grids = ensnano_interactor::set_of_grids_containing_selection(
            app_state.get_selection(),
            &app_state.get_design_reader(),
        );
        let helices = ensnano_interactor::set_of_helices_containing_selection(
            app_state.get_selection(),
            &app_state.get_design_reader(),
        );
        log::debug!("rotating grids {:?}", grids);
        let group_id = app_state.get_current_group_id();
        let rotation: Arc<dyn Operation> = if let Some(grid_ids) = grids.filter(|v| v.len() > 0) {
            Arc::new(GridRotation {
                grid_ids,
                angle,
                plane,
                origin,
                design_id: 0,
                group_id,
                replace: false,
            })
        } else {
            match self.data.borrow().get_selected_element(app_state) {
                Selection::Helix(d_id, h_id) => Arc::new(HelixRotation {
                    helices: helices.unwrap_or(vec![h_id as usize]),
                    angle,
                    plane,
                    origin,
                    design_id: d_id as usize,
                    group_id,
                    replace: false,
                }),
                Selection::Grid(d_id, g_id) => {
                    let grid_id = g_id as usize;
                    Arc::new(GridRotation {
                        grid_ids: vec![grid_id],
                        angle,
                        plane,
                        origin,
                        design_id: d_id as usize,
                        group_id,
                        replace: false,
                    })
                }
                _ => return,
            }
        };

        self.requests.lock().unwrap().update_opperation(rotation);
    }

    /// Adapt the camera, position, orientation and pivot point to a design so that the design fits
    /// the scene, and the pivot point of the camera is the center of the design.
    fn fit_design(&mut self) {
        let camera_position = self.data.borrow().get_fitting_camera_position();
        if let Some(position) = camera_position {
            let pivot_point = self.data.borrow().get_middle_point(0);
            self.notify(SceneNotification::NewCameraPosition(position));
            self.controller.set_pivot_point(pivot_point.try_into().ok());
        }
    }

    fn need_redraw(&mut self, dt: Duration, new_state: S) -> bool {
        self.check_timers(&new_state);
        if self.controller.camera_is_moving() {
            self.notify(SceneNotification::CameraMoved);
        }
        self.controller.update_data();
        if self.update.need_update {
            self.perform_update(dt, &new_state);
        }
        self.data
            .borrow_mut()
            .update_design(new_state.get_design_reader());
        self.data
            .borrow_mut()
            .update_view(&new_state, &self.older_state);
        self.older_state = new_state;
        let ret = self.view.borrow().need_redraw();
        if ret {
            log::debug!("Scene requests redraw");
        }
        ret
    }

    /// Draw the scene
    fn draw_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _app_state: &S,
    ) {
        self.view
            .borrow_mut()
            .draw(encoder, target, DrawType::Scene, self.area);
    }

    fn perform_update(&mut self, dt: Duration, _app_state: &S) {
        if self.update.camera_update {
            self.controller.update_camera(dt);
            self.view.borrow_mut().update(ViewUpdate::Camera);
            self.update.camera_update = false;
        }
        self.update.need_update = false;
    }

    /*
    fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.data.borrow_mut().change_selection_mode(selection_mode);
        self.update_handle();
    }

    fn change_action_mode(&mut self, action_mode: ActionMode) {
        self.data.borrow_mut().change_action_mode(action_mode);
        self.update_handle();
    }*/

    fn change_sensitivity(&mut self, sensitivity: f32) {
        self.controller.change_sensitivity(sensitivity)
    }

    fn set_camera_target(&mut self, target: Vec3, up: Vec3, app_state: &S) {
        let pivot = self
            .data
            .borrow()
            .get_selected_position()
            .filter(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
        let pivot = pivot
            .or_else(|| {
                let element_center = self.element_center(app_state);
                self.data
                    .borrow_mut()
                    .set_selection(element_center, app_state);
                self.data.borrow().get_selected_position()
            })
            .filter(|r| r.x.is_finite() && r.y.is_finite() && r.z.is_finite())
            .or(Some(Vec3::zero()));
        self.controller.set_camera_target(target, up, pivot);
        self.fit_design();
    }

    fn request_camera_rotation(&mut self, xz: f32, yz: f32, xy: f32, app_state: &S) {
        let pivot = self
            .data
            .borrow()
            .get_pivot_position()
            .or(self.data.borrow().get_selected_position())
            .filter(|r| !r.x.is_nan() && !r.y.is_nan() && !r.z.is_nan());
        let pivot = pivot.or_else(|| {
            let element_center = self.element_center(app_state);
            self.data
                .borrow_mut()
                .set_selection(element_center, app_state);
            self.data
                .borrow()
                .get_selected_position()
                .filter(|r| !r.x.is_nan() && !r.y.is_nan() && !r.z.is_nan())
        });
        log::info!("pivot {:?}", pivot);
        self.controller.rotate_camera(xz, yz, xy, pivot);
    }
}

/// A structure that stores the element that needs to be updated in a scene
pub struct SceneUpdate {
    pub tube_instances: Option<Vec<Instance>>,
    pub sphere_instances: Option<Vec<Instance>>,
    pub fake_tube_instances: Option<Vec<Instance>>,
    pub fake_sphere_instances: Option<Vec<Instance>>,
    pub selected_tube: Option<Vec<Instance>>,
    pub selected_sphere: Option<Vec<Instance>>,
    pub candidate_spheres: Option<Vec<Instance>>,
    pub candidate_tubes: Option<Vec<Instance>>,
    pub model_matrices: Option<Vec<Mat4>>,
    pub need_update: bool,
    pub camera_update: bool,
}

impl SceneUpdate {
    pub fn new() -> Self {
        Self {
            tube_instances: None,
            sphere_instances: None,
            fake_tube_instances: None,
            fake_sphere_instances: None,
            selected_tube: None,
            selected_sphere: None,
            candidate_spheres: None,
            candidate_tubes: None,
            need_update: false,
            camera_update: false,
            model_matrices: None,
        }
    }
}

/// A notification to be given to the scene
pub enum SceneNotification {
    /// The camera has moved. As a consequence, the projection and view matrix must be
    /// updated.
    CameraMoved,
    /// The camera is replaced by a new one.
    #[allow(dead_code)]
    NewCamera(Vec3, Rotor3),
    /// The drawing area has been modified
    NewSize(PhySize, DrawArea),
    NewCameraPosition(Vec3),
}

impl<S: AppState> Scene<S> {
    /// Send a notificatoin to the scene
    pub fn notify(&mut self, notification: SceneNotification) {
        match notification {
            SceneNotification::NewCamera(position, projection) => {
                self.controller.teleport_camera(position, projection);
                self.update.camera_update = true;
            }
            SceneNotification::NewCameraPosition(position) => {
                self.controller.set_camera_position(position);
                self.update.camera_update = true;
            }
            SceneNotification::CameraMoved => self.update.camera_update = true,
            SceneNotification::NewSize(window_size, area) => {
                self.area = area;
                self.resize(window_size);
            }
        };
        self.update.need_update = true;
    }

    fn resize(&mut self, window_size: PhySize) {
        self.view.borrow_mut().update(ViewUpdate::Size(window_size));
        self.controller.resize(window_size, self.area.size);
        self.update.camera_update = true;
        self.element_selector
            .resize(self.controller.get_window_size(), self.area);
    }

    pub fn fog_request(&mut self, fog: FogParameters) {
        self.view.borrow_mut().update(ViewUpdate::Fog(fog))
    }
}

impl<S: AppState> Application for Scene<S> {
    type AppState = S;
    fn on_notify(&mut self, notification: Notification) {
        let older_state = self.older_state.clone();
        match notification {
            Notification::ClearDesigns => self.clear_design(),
            Notification::ToggleText(value) => self.view.borrow_mut().set_draw_letter(value),
            Notification::FitRequest => self.fit_design(),
            Notification::NewSensitivity(x) => self.change_sensitivity(x),
            Notification::Save(_) => (),
            Notification::CameraTarget((target, up)) => {
                self.set_camera_target(target, up, &older_state);
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::TeleportCamera(position, orientation) => {
                self.controller.teleport_camera(position, orientation);
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::CameraRotation(xz, yz, xy) => {
                self.request_camera_rotation(xz, yz, xy, &older_state);
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::Centering(nucl, design_id) => {
                if let Some(position) = self.data.borrow().get_nucl_position(nucl, design_id) {
                    self.controller.center_camera(position);
                }
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::CenterSelection(selection, app_id) => {
                if app_id != AppId::Scene {
                    self.data
                        .borrow_mut()
                        .notify_selection(vec![selection].as_slice());
                    if let Some(position) = self.data.borrow().get_selected_position() {
                        self.controller.center_camera(position);
                    }
                    let pivot_element = self.data.borrow().selection_to_element(selection);
                    self.data
                        .borrow_mut()
                        .set_pivot_element(pivot_element, &older_state);
                    self.notify(SceneNotification::CameraMoved);
                }
            }
            Notification::ShowTorsion(_) => (),
            Notification::ModifersChanged(modifiers) => self.controller.update_modifiers(modifiers),
            Notification::Split2d => (),
            Notification::Redim2dHelices(_) => (),
            Notification::RenderingMode(mode) => self.view.borrow_mut().rendering_mode(mode),
            Notification::Background3D(bg) => self.view.borrow_mut().background3d(bg),
            Notification::Fog(fog) => self.fog_request(fog),
            Notification::WindowFocusLost => self.controller.stop_camera_movement(),
            Notification::FlipSplitViews => (),
        }
    }

    fn on_event(
        &mut self,
        event: &WindowEvent,
        cursor_position: PhysicalPosition<f64>,
        app_state: &S,
    ) {
        self.input(event, cursor_position, &app_state)
    }

    fn on_resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.notify(SceneNotification::NewSize(window_size, area))
    }

    fn on_redraw_request(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _dt: Duration,
    ) {
        let older_state = self.older_state.clone();
        self.draw_view(encoder, target, &older_state)
    }

    fn needs_redraw(&mut self, dt: Duration, state: S) -> bool {
        self.need_redraw(dt, state)
    }

    fn get_position_for_new_grid(&self) -> Option<(Vec3, Rotor3)> {
        let camera = self.view.borrow().get_camera();
        let position = camera.borrow().position + 10_f32 * camera.borrow().direction();
        let orientation = camera.borrow().rotor.reversed()
            * Rotor3::from_rotation_xz(std::f32::consts::FRAC_PI_2);
        Some((position, orientation))
    }

    fn get_camera(&self) -> Option<(Vec3, Rotor3)> {
        let view = self.view.borrow();
        let cam = view.get_camera();
        let ret = Some((cam.borrow().position, cam.borrow().rotor));
        ret
    }

    fn get_current_selection_pivot(&self) -> Option<GroupPivot> {
        self.view.borrow().get_current_pivot()
    }

    fn is_splited(&self) -> bool {
        false
    }
}

pub trait AppState: Clone {
    type DesignReader: DesignReader;
    fn get_selection(&self) -> &[Selection];
    fn get_candidates(&self) -> &[Selection];
    fn selection_was_updated(&self, other: &Self) -> bool;
    fn candidates_set_was_updated(&self, other: &Self) -> bool;
    fn design_was_modified(&self, other: &Self) -> bool;
    fn design_model_matrix_was_updated(&self, other: &Self) -> bool;
    fn get_selection_mode(&self) -> SelectionMode;
    fn get_action_mode(&self) -> (ActionMode, WidgetBasis);
    fn get_design_reader(&self) -> Self::DesignReader;
    fn get_strand_builders(&self) -> &[StrandBuilder];
    fn get_widget_basis(&self) -> WidgetBasis;
    fn is_changing_color(&self) -> bool;
    fn is_pasting(&self) -> bool;
    fn get_selected_element(&self) -> Option<CenterOfSelection>;
    fn get_current_group_pivot(&self) -> Option<ensnano_design::group_attributes::GroupPivot>;
    fn get_current_group_id(&self) -> Option<ensnano_design::GroupId>;
    fn suggestion_parameters_were_updated(&self, other: &Self) -> bool;
}

pub trait Requests {
    fn update_opperation(&mut self, op: Arc<dyn Operation>);
    fn apply_design_operation(&mut self, op: DesignOperation);
    fn set_candidate(&mut self, candidates: Vec<Selection>);
    fn set_paste_candidate(&mut self, nucl: Option<Nucl>);
    fn set_selection(
        &mut self,
        selection: Vec<Selection>,
        center_of_selection: Option<CenterOfSelection>,
    );
    fn attempt_paste(&mut self, nucl: Option<Nucl>);
    fn xover_request(&mut self, source: Nucl, target: Nucl, design_id: usize);
    fn suspend_op(&mut self);
    fn request_center_selection(&mut self, selection: Selection, app_id: AppId);
    fn undo(&mut self);
    fn redo(&mut self);
    fn update_builder_position(&mut self, position: isize);
    fn toggle_widget_basis(&mut self);
    fn set_current_group_pivot(&mut self, pivot: GroupPivot);
    fn translate_group_pivot(&mut self, translation: Vec3);
    fn rotate_group_pivot(&mut self, rotation: Rotor3);
}
