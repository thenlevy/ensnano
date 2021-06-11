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
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use ultraviolet::{Mat4, Rotor3, Vec3};

use crate::design::StrandBuilder;
use crate::{design, mediator, utils};
use crate::{DrawArea, PhySize, WindowEvent};
use ensnano_design::{grid::Hyperboloid, Nucl};
use instance::Instance;
use mediator::{
    ActionMode, AppId, Application, CreateGrid, GridHelixCreation, GridRotation, GridTranslation,
    HelixRotation, HelixTranslation, MediatorPtr, NewHyperboloid, Notification, Operation,
    Selection, SelectionMode, StrandConstruction,
};
use utils::instance;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

/// Computation of the view and projection matrix.
mod camera;
/// Display of the scene
mod view;
use ensnano_design::grid::GridTypeDescr;
use view::{
    DrawType, HandleDir, HandleOrientation, HandlesDescriptor, LetterInstance,
    RotationMode as WidgetRotationMode, RotationWidgetDescriptor, RotationWidgetOrientation, View,
    ViewUpdate,
};
pub use view::{FogParameters, GridInstance};
/// Handling of inputs and notifications
mod controller;
use controller::{Consequence, Controller};
/// Handling of designs and internal data
mod data;
pub use controller::ClickMode;
use data::Data;
pub use data::DesignReader;
use design::{Design, DesignNotification, DesignNotificationContent};
mod element_selector;
use element_selector::{ElementSelector, SceneElement};
mod maths_3d;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr<R: DesignReader> = Rc<RefCell<Data<R>>>;

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
        let data: DataPtr<S::DesignReader> = Rc::new(RefCell::new(Data::new(view.clone())));
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
                self.controller.translate_camera(dx, dy);
                self.notify(SceneNotification::CameraMoved);
            }
            Consequence::XoverAtempt(source, target, d_id) => {
                self.attempt_xover(source, target, d_id);
                self.data.borrow_mut().end_free_xover();
            }
            Consequence::Translation(dir, x_coord, y_coord) => {
                let translation = self.view.borrow().compute_translation_handle(
                    x_coord as f32,
                    y_coord as f32,
                    dir,
                );
                if let Some(t) = translation {
                    self.translate_selected_design(t, app_state);
                }
            }
            Consequence::MovementEnded => {
                self.requests.lock().unwrap().suspend_op();
                self.update_handle(app_state);
            }
            Consequence::InitRotation(x, y) => {
                self.view.borrow_mut().init_rotation(x as f32, y as f32)
            }
            Consequence::InitTranslation(x, y) => {
                self.view.borrow_mut().init_translation(x as f32, y as f32)
            }
            Consequence::Rotation(mode, x, y) => {
                let rotation = self
                    .view
                    .borrow()
                    .compute_rotation(x as f32, y as f32, mode);
                if let Some((rotation, origin, positive)) = rotation {
                    if rotation.bv.mag() > 1e-3 {
                        self.rotate_selected_desgin(rotation, origin, positive, app_state)
                    }
                } else {
                    println!("Warning rotiation was None")
                }
            }
            Consequence::Swing(x, y) => {
                let mut pivot = self.data.borrow().get_pivot_position();
                if pivot.is_none() {
                    self.data.borrow_mut().try_update_pivot_position(app_state);
                    pivot = self.data.borrow().get_pivot_position();
                }
                self.controller.set_pivot_point(pivot);
                self.controller.swing(-x, -y);
                self.notify(SceneNotification::CameraMoved);
            }
            Consequence::ToggleWidget => self.data.borrow_mut().toggle_widget_basis(false),
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
                self.view.borrow_mut().update(ViewUpdate::Camera);
                self.requests.lock().unwrap().suspend_op();
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
        };
    }

    pub fn make_new_grid(&self, grid_type: GridTypeDescr) {
        let camera = self.view.borrow().get_camera();
        let position = camera.borrow().position + 10_f32 * camera.borrow().direction();
        let orientation = camera.borrow().rotor.reversed()
            * Rotor3::from_rotation_xz(std::f32::consts::FRAC_PI_2);
        self.requests
            .lock()
            .unwrap()
            .update_opperation(Arc::new(CreateGrid {
                design_id: 0,
                position,
                orientation,
                grid_type,
                delete: false,
            }));
        self.requests.lock().unwrap().suspend_op();
    }

    pub fn make_hyperboloid(&self, hyperboloid: Hyperboloid) {
        let camera = self.view.borrow().get_camera();
        let position = camera.borrow().position + 40_f32 * camera.borrow().direction();
        let orientation = camera.borrow().rotor.reversed()
            * Rotor3::from_rotation_xz(std::f32::consts::FRAC_PI_2);
        self.requests
            .lock()
            .unwrap()
            .update_opperation(Arc::new(NewHyperboloid {
                design_id: 0,
                position,
                orientation,
                hyperboloid,
                delete: false,
            }));
        self.data.borrow_mut().set_pivot_position(position);
        self.requests.lock().unwrap().suspend_op();
    }

    /// If a nucleotide is selected, and the clicked_pixel corresponds to an other nucleotide,
    /// request a cross-over between the two nucleotides.
    fn attempt_xover(&mut self, source: Nucl, target: Nucl, design_id: usize) {
        self.requests
            .lock()
            .unwrap()
            .xover_request(source, target, design_id)
    }

    fn element_center(&mut self, app_state: &S) -> Option<SceneElement> {
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
        let selection = self.data.borrow_mut().set_selection(element, app_state);
        if let Some(selection) = selection {
            self.requests.lock().unwrap().set_selection(vec![selection]);
        }
        self.update_handle(app_state);
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
        if let Some(selection) = selection {
            self.requests.lock().unwrap().set_selection(selection);
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

        let translation_op: Arc<dyn Operation> =
            match self.data.borrow().get_selected_element(app_state) {
                Selection::Grid(d_id, g_id) => Arc::new(GridTranslation {
                    design_id: d_id as usize,
                    grid_id: g_id as usize,
                    right: Vec3::unit_x().rotated_by(rotor),
                    top: Vec3::unit_y().rotated_by(rotor),
                    dir: Vec3::unit_z().rotated_by(rotor),
                    x: translation.dot(right),
                    y: translation.dot(top),
                    z: translation.dot(dir),
                }),
                Selection::Helix(d_id, h_id) => Arc::new(HelixTranslation {
                    design_id: d_id as usize,
                    helix_id: h_id as usize,
                    right: Vec3::unit_x().rotated_by(rotor),
                    top: Vec3::unit_y().rotated_by(rotor),
                    dir: Vec3::unit_z().rotated_by(rotor),
                    x: translation.dot(right),
                    y: translation.dot(top),
                    z: translation.dot(dir),
                    snap: true,
                }),
                _ => return,
            };

        self.requests
            .lock()
            .unwrap()
            .update_opperation(translation_op);
    }

    fn rotate_selected_desgin(
        &mut self,
        rotation: Rotor3,
        origin: Vec3,
        positive: bool,
        app_state: &S,
    ) {
        let (mut angle, mut plane) = rotation.into_angle_plane();
        if !positive {
            angle *= -1.;
            plane *= -1.;
        }
        let rotation: Arc<dyn Operation> = match self.data.borrow().get_selected_element(app_state)
        {
            Selection::Helix(d_id, h_id) => {
                let helix_id = h_id as usize;
                Arc::new(HelixRotation {
                    helix_id,
                    angle,
                    plane,
                    origin,
                    design_id: d_id as usize,
                })
            }
            Selection::Grid(d_id, g_id) => {
                let grid_id = g_id as usize;
                Arc::new(GridRotation {
                    grid_id,
                    angle,
                    plane,
                    origin,
                    design_id: d_id as usize,
                })
            }
            _ => return,
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
            self.controller.set_pivot_point(Some(pivot_point));
            self.controller.set_pivot_point(None);
        }
    }

    fn need_redraw(&mut self, dt: Duration, new_state: S) -> bool {
        self.check_timers(&new_state);
        if self.controller.camera_is_moving() {
            self.notify(SceneNotification::CameraMoved);
        }
        if self.update.need_update {
            self.perform_update(dt, &new_state);
        }
        self.data
            .borrow_mut()
            .update_view(&new_state, &self.older_state);
        self.older_state = new_state;
        self.view.borrow().need_redraw()
    }

    /// Draw the scene
    fn draw_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        app_state: &S,
    ) {
        self.view
            .borrow_mut()
            .draw(encoder, target, DrawType::Scene, self.area);
    }

    fn update_handle(&mut self, app_state: &S) {
        let origin = self.data.borrow().get_selected_position();
        let orientation = self.data.borrow().get_widget_basis(app_state);
        let handle_descr = if app_state.get_action_mode().wants_handle() {
            origin
                .clone()
                .zip(orientation.clone())
                .map(|(origin, orientation)| HandlesDescriptor {
                    origin,
                    orientation: HandleOrientation::Rotor(orientation),
                    size: 0.25,
                })
        } else {
            None
        };
        self.view
            .borrow_mut()
            .update(ViewUpdate::Handles(handle_descr));
        let only_right = !self.data.borrow().selection_can_rotate_freely(app_state);
        let rotation_widget_descr = if app_state.get_action_mode().wants_rotation() {
            origin
                .clone()
                .zip(orientation.clone())
                .map(|(origin, orientation)| RotationWidgetDescriptor {
                    origin,
                    orientation: RotationWidgetOrientation::Rotor(orientation),
                    size: 0.2,
                    only_right,
                })
        } else {
            None
        };
        self.view
            .borrow_mut()
            .update(ViewUpdate::RotationWidget(rotation_widget_descr));
    }

    fn perform_update(&mut self, dt: Duration, app_state: &S) {
        if self.update.camera_update {
            self.controller.update_camera(dt);
            self.view.borrow_mut().update(ViewUpdate::Camera);
            self.update.camera_update = false;
            self.update_handle(app_state)
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
        let pivot = self.data.borrow().get_selected_position();
        let pivot = pivot.or_else(|| {
            let element_center = self.element_center(app_state);
            self.data
                .borrow_mut()
                .set_selection(element_center, app_state);
            self.data.borrow().get_selected_position()
        });
        self.controller.set_camera_target(target, up, pivot);
        self.fit_design();
    }

    fn request_camera_rotation(&mut self, xz: f32, yz: f32, xy: f32, app_state: &S) {
        let pivot = self.data.borrow().get_selected_position();
        let pivot = pivot.or_else(|| {
            let element_center = self.element_center(app_state);
            self.data
                .borrow_mut()
                .set_selection(element_center, app_state);
            self.data.borrow().get_selected_position()
        });
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
        match notification {
            Notification::DesignNotification(notification) => {
                //self.handle_design_notification(notification)
            }
            Notification::AppNotification(_) => (),
            Notification::NewDesign(_) => (),
            Notification::ClearDesigns => self.clear_design(),
            Notification::ToggleText(value) => self.view.borrow_mut().set_draw_letter(value),
            Notification::FitRequest => self.fit_design(),
            Notification::NewActionMode(am) => {
                ()
                //self.change_action_mode(am),
            }
            Notification::NewSelectionMode(sm) => {
                //self.change_selection_mode(sm),
                ()
            }
            Notification::NewSensitivity(x) => self.change_sensitivity(x),
            Notification::NewCandidate(candidate, app_id) => (),
            Notification::Selection3D(selection, app_id) => (),
            Notification::Save(_) => (),
            Notification::CameraTarget((target, up)) => {
                self.set_camera_target(target, up, &self.older_state);
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::CameraRotation(xz, yz, xy) => {
                self.request_camera_rotation(xz, yz, xy, &self.older_state);
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::Centering(nucl, design_id) => {
                let mut selected = false;
                if let Some(position) = self.data.borrow().get_nucl_position(nucl, design_id) {
                    self.controller.center_camera(position);
                    selected = true;
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
                    self.notify(SceneNotification::CameraMoved);
                }
            }
            Notification::ShowTorsion(_) => (),
            Notification::Pasting(b) => self.controller.pasting = b,
            Notification::ModifersChanged(modifiers) => self.controller.update_modifiers(modifiers),
            Notification::Split2d => (),
            Notification::Redim2dHelices(_) => (),
            Notification::ToggleWidget(b) => {
                self.data.borrow_mut().toggle_widget_basis(b);
                self.update_handle(&self.older_state);
            }
            Notification::RenderingMode(mode) => self.view.borrow_mut().rendering_mode(mode),
            Notification::Background3D(bg) => self.view.borrow_mut().background3d(bg),
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
        self.draw_view(encoder, target, &self.older_state)
    }

    fn needs_redraw(&mut self, dt: Duration, state: S) -> bool {
        self.need_redraw(dt, state)
    }
}

impl<S: AppState> Scene<S> {
    /*
    fn handle_design_notification(&mut self, notification: DesignNotification) {
        let _design_id = notification.design_id;
        match notification.content {
            DesignNotificationContent::ModelChanged(_) => {
                self.update.need_update = true;
                self.data.borrow_mut().notify_matrices_update();
            }
            DesignNotificationContent::InstanceChanged => {
                self.data.borrow_mut().notify_instance_update()
            }
            DesignNotificationContent::ViewNeedReset => {
                self.data.borrow_mut().notify_instance_update();
                self.data.borrow_mut().set_selection(None);
            }
        }
    }*/
}

pub trait AppState {
    type DesignReader: DesignReader;
    fn get_selection(&self) -> &[Selection];
    fn get_candidates(&self) -> &[Selection];
    fn selection_was_updated(&self, other: &Self) -> bool;
    fn candidates_set_was_updated(&self, other: &Self) -> bool;
    fn design_was_modified(&self, other: &Self) -> bool;
    fn design_model_matrix_was_updated(&self, other: &Self) -> bool;
    fn get_selection_mode(&self) -> SelectionMode;
    fn get_action_mode(&self) -> ActionMode;
    fn get_design_reader(&self) -> Self::DesignReader;
    fn get_strand_builders(&self) -> Vec<StrandBuilder>;
}

pub trait Requests {
    fn update_opperation(&mut self, op: Arc<dyn Operation>);
    fn set_candidate(&mut self, candidates: Vec<Selection>);
    fn set_paste_candidate(&mut self, nucl: Option<Nucl>);
    fn set_selection(&mut self, selection: Vec<Selection>);
    fn attempt_paste(&mut self, nucl: Option<Nucl>);
    fn xover_request(&mut self, source: Nucl, target: Nucl, design_id: usize);
    fn suspend_op(&mut self);
    fn request_center_selection(&mut self, selection: Selection, app_id: AppId);
    fn undo(&mut self);
    fn redo(&mut self);
    fn update_builder_position(&mut self, position: isize);
}
