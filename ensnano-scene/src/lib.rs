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
use ensnano_design::{grid::HelixGridPosition, ultraviolet, BezierVertexId};
use ensnano_interactor::{
    graphics::RenderingMode, NewBezierTengentVector, UnrootedRevolutionSurfaceDescriptor,
};
use ensnano_utils::{wgpu, winit};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ultraviolet::{Mat4, Rotor3, Vec3};

use camera::FiniteVec3;
use ensnano_design::{grid::GridPosition, group_attributes::GroupPivot, Nucl};
use ensnano_interactor::{
    application::{AppId, Application, Camera3D, Notification},
    graphics::DrawArea,
    operation::*,
    ActionMode, CenterOfSelection, CheckXoversParameter, DesignOperation, Selection, SelectionMode,
    StrandBuilder, WidgetBasis,
};
use ensnano_utils::{instance, PhySize};
use instance::Instance;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;
use winit::event::WindowEvent;

/// Computation of the view and projection matrix.
mod camera;
/// Display of the scene
mod view;
pub use view::{DrawOptions, FogParameters, GridInstance};
use view::{
    DrawType, HandleDir, HandleOrientation, HandlesDescriptor, LetterInstance,
    RotationMode as WidgetRotationMode, RotationWidgetDescriptor, RotationWidgetOrientation,
    Stereography, View, ViewUpdate,
};
/// Handling of inputs and notifications
mod controller;
use controller::{Consequence, Controller, WidgetTarget};
/// Handling of designs and internal data
mod data;
pub use controller::ClickMode;
use data::Data;
pub use data::{DesignReader, HBond, HalfHBond, SurfaceInfo, SurfacePoint};
mod element_selector;
use element_selector::{ElementSelector, SceneElement};
mod maths_3d;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr<R> = Rc<RefCell<Data<R>>>;
use std::convert::TryInto;

const PNG_SIZE: u32 = 256 * 10;

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
    scene_kind: SceneKind,
    current_camera: Arc<(Camera3D, f32)>,
}

#[derive(Debug, Clone, Copy)]
pub enum SceneKind {
    Cartesian,
    Stereographic,
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
        scene_kind: SceneKind,
    ) -> Self {
        let update = SceneUpdate::default();
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
            scene_kind,
            current_camera: Arc::new((
                Default::default(),
                area.size.width as f32 / area.size.height as f32,
            )),
        }
    }

    /// Remove all designs
    fn clear_design(&mut self) {
        self.data.borrow_mut().clear_designs()
    }

    fn is_stereographic(&self) -> bool {
        matches!(self.scene_kind, SceneKind::Stereographic)
    }

    /// Input an event to the scene. The controller parse the event and return the consequence that
    /// the event must have.
    fn input(
        &mut self,
        event: &WindowEvent,
        cursor_position: PhysicalPosition<f64>,
        app_state: &S,
    ) -> Option<ensnano_interactor::CursorIcon> {
        let consequence = self.controller.input(
            event,
            cursor_position,
            &mut self.element_selector,
            app_state,
        );
        self.read_consequence(consequence, app_state);
        self.controller.get_icon()
    }

    fn check_timers(&mut self, app_state: &S) {
        let consequence = self.controller.check_timers();
        self.read_consequence(consequence, app_state);
    }

    fn read_consequence(&mut self, consequence: Consequence, app_state: &S) {
        if !matches!(consequence, Consequence::Nothing) {
            log::info!("Consequence {:?}", consequence);
        }
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
            Consequence::XoverAtempt(source, target, d_id, magic) => {
                self.attempt_xover(source, target, d_id, magic);
                self.data.borrow_mut().end_free_xover();
            }
            Consequence::QuickXoverAttempt { nucl, doubled } => {
                let suggestions = app_state.get_design_reader().get_suggestions();
                let mut pair = suggestions
                    .iter()
                    .find(|(a, b)| *a == nucl || *b == nucl)
                    .cloned();
                if let Some((n1, n2)) = pair {
                    if doubled {
                        pair = suggestions
                            .iter()
                            .find(|(a, b)| *a == n1.prime5() || *b == n1.prime5())
                            .cloned();
                    }
                    self.requests.lock().unwrap().apply_design_operation(
                        DesignOperation::MakeSeveralXovers {
                            xovers: vec![pair.unwrap_or((n1, n2))],
                            doubled,
                        },
                    );
                } else {
                    log::error!("No suggested cross over target for nucl {:?}", nucl)
                }
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
            Consequence::ObjectTranslated { object, grid, x, y } => {
                log::info!("Moving helix {:?} to grid {:?} ({} {})", object, grid, x, y);
                self.requests
                    .lock()
                    .unwrap()
                    .apply_design_operation(DesignOperation::AttachObject { object, grid, x, y });
            }
            Consequence::MovementEnded => {
                self.requests.lock().unwrap().suspend_op();
                self.data.borrow_mut().notify_handle_movement();
                self.view.borrow_mut().end_movement();
            }
            Consequence::HelixSelected(helix_id) => self.requests.lock().unwrap().set_selection(
                vec![Selection::Helix {
                    design_id: 0,
                    helix_id,
                    segment_id: 0,
                }],
                None,
            ),
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
            Consequence::Tilt(x, _) => {
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
                let angle = x as f32 * -std::f32::consts::TAU;
                self.controller.continuous_tilt(angle);
                self.notify(SceneNotification::CameraMoved);
            }
            Consequence::ToggleWidget => {
                self.requests.lock().unwrap().toggle_widget_basis();
            }
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
                if self.controller.is_building_bezier_curve() {
                    let point = HelixGridPosition::from_grid_id_x_y(grid_id, x, y);
                    if let Some((start, end)) = self.controller.add_bezier_point(point) {
                        self.requests.lock().unwrap().apply_design_operation(
                            DesignOperation::AddTwoPointsBezier { start, end },
                        );
                    } else {
                        // This is the first point of the bezier curve, select the corresponding
                        // disc to highlight it.
                        self.select(
                            Some(SceneElement::GridCircle(
                                0,
                                GridPosition {
                                    grid: grid_id,
                                    x,
                                    y,
                                },
                            )),
                            app_state,
                        )
                    }
                } else {
                    // build regular grid helix
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
            Consequence::InitBuild(nucls) => {
                if let Some(xover_id) = nucls.get(0).cloned().and_then(|n| {
                    app_state
                        .get_design_reader()
                        .get_id_of_xover_involving_nucl(n)
                }) {
                    self.requests
                        .lock()
                        .unwrap()
                        .set_selection(vec![Selection::Xover(0, xover_id)], None);
                }
                self.requests
                    .lock()
                    .unwrap()
                    .apply_design_operation(DesignOperation::RequestStrandBuilders { nucls });
            }
            Consequence::PivotCenter => {
                self.data.borrow_mut().set_pivot_position(Vec3::zero());
                self.view
                    .borrow_mut()
                    .update(ViewUpdate::FogCenter(Some(Vec3::zero())));
            }
            Consequence::CheckXovers => {
                let xovers = ensnano_interactor::list_of_xover_ids(
                    app_state.get_selection(),
                    &app_state.get_design_reader(),
                );
                if let Some((_, xovers)) = xovers {
                    self.requests
                        .lock()
                        .unwrap()
                        .apply_design_operation(DesignOperation::CheckXovers { xovers })
                }
            }
            Consequence::AlignWithStereo => {
                if !self.is_stereographic() {
                    let camera = self.data.borrow().get_aligned_camera();
                    self.on_notify(Notification::TeleportCamera(camera));
                }
            }
            Consequence::CreateBezierVertex { vertex, path } => {
                if let Some(path) = path {
                    self.requests.lock().unwrap().apply_design_operation(
                        DesignOperation::AppendVertexToPath {
                            path_id: path,
                            vertex,
                        },
                    )
                } else {
                    self.requests.lock().unwrap().apply_design_operation(
                        DesignOperation::CreateBezierPath {
                            first_vertex: vertex,
                        },
                    )
                }
            }
            Consequence::MoveBezierVertex {
                x,
                y,
                path_id,
                vertex_id,
            } => {
                let mut vertices = vec![BezierVertexId { path_id, vertex_id }];
                if app_state
                    .get_selection()
                    .iter()
                    .any(|s| *s == Selection::BezierVertex(BezierVertexId { path_id, vertex_id }))
                {
                    for v in app_state.get_selection().iter().filter_map(|s| {
                        if let Selection::BezierVertex(v) = s {
                            Some(v)
                        } else {
                            None
                        }
                    }) {
                        vertices.push(*v);
                    }
                }
                self.requests
                    .lock()
                    .unwrap()
                    .update_opperation(Arc::new(TranslateBezierPathVertex { vertices, x, y }))
            }
            Consequence::ReleaseBezierVertex => self.requests.lock().unwrap().suspend_op(),
            Consequence::MoveBezierCorner {
                plane_id,
                original_corner_position,
                fixed_corner_position,
                moving_corner,
            } => self.requests.lock().unwrap().update_opperation(Arc::new(
                TranslateBezierSheetCorner {
                    plane_id,
                    origin_moving_corner: original_corner_position,
                    fixed_corner: fixed_corner_position,
                    moving_corner,
                },
            )),
            Consequence::ReleaseBezierCorner => self.requests.lock().unwrap().suspend_op(),
            Consequence::ReleaseBezierTengent => self.requests.lock().unwrap().suspend_op(),
            Consequence::MoveBezierTengent {
                vertex_id,
                tengent_in,
                full_symetry_other: adjust_other,
                new_vector,
            } => self.requests.lock().unwrap().apply_design_operation(
                DesignOperation::SetVectorOfBezierTengent(NewBezierTengentVector {
                    full_symetry_other_tengent: adjust_other,
                    new_vector,
                    tengent_in,
                    vertex_id,
                }),
            ),
            Consequence::ReverseSurfaceDirection => {
                self.controller.reverse_surface_direction();
                self.notify(SceneNotification::CameraMoved);
            }
            Consequence::SetRevolutionAxisPosition(r) => {
                self.requests
                    .lock()
                    .unwrap()
                    .set_revolution_axis_position(r);
            }
        };
    }

    /// Request a cross-over between two nucleotides.
    fn attempt_xover(&mut self, mut source: Nucl, mut target: Nucl, design_id: usize, magic: bool) {
        if magic {
            if let Some(opt) = self
                .older_state
                .get_design_reader()
                .get_optimal_xover_arround(source, target)
            {
                (source, target) = opt;
            }
        }
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
        if let Some(SceneElement::GridCircle(_, gp)) = element {
            log::info!("Attempt past on {:?}", gp);
            self.requests.lock().unwrap().attempt_paste_on_grid(gp);
        } else {
            let nucl = self.data.borrow().element_to_nucl(&element, false);
            self.requests
                .lock()
                .unwrap()
                .attempt_paste(nucl.map(|n| n.0));
        }
    }

    fn pasting_candidate(&self, element: Option<SceneElement>) {
        if let Some(SceneElement::GridCircle(_, gp)) = element {
            log::info!("Paste candidate on {:?}", gp);
            self.requests.lock().unwrap().paste_candidate_on_grid(gp);
        } else {
            let nucl = self.data.borrow().element_to_nucl(&element, false);
            self.requests
                .lock()
                .unwrap()
                .set_paste_candidate(nucl.map(|n| n.0));
        }
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
        log::debug!("grids {:?}", grids);
        let control_points = ensnano_interactor::extract_control_points(app_state.get_selection());
        let at_most_one_grid = grids.as_ref().map(|g| g.len() <= 1).unwrap_or(false);

        let group_id = app_state.get_current_group_id();

        let translation_op: Arc<dyn Operation> = if !control_points.is_empty() {
            Arc::new(BezierControlPointTranslation {
                design_id: 0,
                control_points,
                right: Vec3::unit_x().rotated_by(rotor),
                top: Vec3::unit_y().rotated_by(rotor),
                dir: Vec3::unit_z().rotated_by(rotor),
                x: translation.dot(right),
                y: translation.dot(top),
                z: translation.dot(dir),
                snap: true,
                group_id,
            })
        } else if let Some(helices) = helices.filter(|_| at_most_one_grid) {
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
        let rotation: Arc<dyn Operation> = if let Some(grid_ids) = grids.filter(|v| !v.is_empty()) {
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
                Selection::Helix {
                    design_id,
                    helix_id,
                    ..
                } => Arc::new(HelixRotation {
                    helices: helices.unwrap_or_else(|| vec![helix_id]),
                    angle,
                    plane,
                    origin,
                    design_id: design_id as usize,
                    group_id,
                    replace: false,
                }),
                Selection::Grid(d_id, g_id) => Arc::new(GridRotation {
                    grid_ids: vec![g_id],
                    angle,
                    plane,
                    origin,
                    design_id: d_id as usize,
                    group_id,
                    replace: false,
                }),
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
            self.perform_update(dt);
        }
        self.data
            .borrow_mut()
            .update_design(new_state.get_design_reader());
        self.data
            .borrow_mut()
            .update_view(&new_state, &self.older_state);
        let mut ret = new_state.draw_options_were_updated(&self.older_state);
        self.older_state = new_state;
        ret |= self.view.borrow().need_redraw();
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
        app_state: &S,
    ) {
        let is_stereographic = matches!(self.scene_kind, SceneKind::Stereographic);
        log::trace!("draw scene");
        self.view.borrow_mut().draw(
            encoder,
            target,
            DrawType::Scene,
            self.area,
            is_stereographic,
            app_state.get_draw_options(),
        );
    }

    fn perform_update(&mut self, dt: Duration) {
        if self.update.camera_update {
            self.controller.update_camera(dt);
            self.view.borrow_mut().update(ViewUpdate::Camera);
            self.update.camera_update = false;
            self.current_camera = Arc::new((
                self.get_camera(),
                self.view.borrow().get_projection().borrow().get_ratio(),
            ))
        }
        self.update.need_update = false;
    }

    fn get_camera(&self) -> Camera3D {
        let view = self.view.borrow();
        let cam = view.get_camera();
        let ret = Camera3D {
            position: cam.borrow().position,
            orientation: cam.borrow().rotor,
            pivot_position: self.data.borrow().get_pivot_position(),
        };
        ret
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
            .or_else(|| Some(Vec3::zero()));
        self.controller.set_camera_target(target, up, pivot);
        self.fit_design();
    }

    fn request_camera_rotation(&mut self, xz: f32, yz: f32, xy: f32, app_state: &S) {
        let pivot = self
            .data
            .borrow()
            .get_pivot_position()
            .or_else(|| self.data.borrow().get_selected_position())
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

    fn create_png_export_texture(
        &self,
        device: &Device,
        size: wgpu::Extent3d,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let desc = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            label: Some("desc"),
        };
        let texture_view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("texture_view_descriptor"),
            format: Some(wgpu::TextureFormat::Bgra8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&texture_view_descriptor);
        (texture, view)
    }

    fn export_png(&self) {
        use chrono::Utc;
        let png_name = Utc::now()
            .format("export_3d_%Y_%m_%d_%H_%M_%S.png")
            .to_string();
        let device = self.element_selector.device.as_ref();
        let queue = self.element_selector.queue.as_ref();
        println!("export to {png_name}");
        use ensnano_utils::BufferDimensions;
        use std::io::Write;

        let ratio = self.view.borrow().get_projection().borrow().get_ratio();
        let width = if ratio < 1. {
            (ratio * PNG_SIZE as f32).floor() as u32
        } else {
            PNG_SIZE
        };
        let height = if ratio < 1. {
            PNG_SIZE
        } else {
            (PNG_SIZE as f32 / ratio).floor() as u32
        };
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let (texture, texture_view) = self.create_png_export_texture(device, size);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("3D Png export"),
        });

        let draw_options = DrawOptions {
            rendering_mode: RenderingMode::Cartoon,
            ..Default::default()
        };

        self.view.borrow_mut().draw(
            &mut encoder,
            &texture_view,
            DrawType::Png { width, height },
            DrawArea {
                position: PhysicalPosition { x: 0, y: 0 },
                size: PhySize { width, height },
            },
            self.is_stereographic(),
            draw_options,
        );

        // create a buffer and fill it with the texture
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let buffer_dimensions =
            BufferDimensions::new(extent.width as usize, extent.height as usize);
        let buf_size = buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height;
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: buf_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("staging_buffer"),
        });
        let buffer_copy_view = wgpu::ImageCopyBuffer {
            buffer: &staging_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: (buffer_dimensions.padded_bytes_per_row as u32)
                    .try_into()
                    .ok(),
                rows_per_image: None,
            },
        };
        let origin = wgpu::Origin3d { x: 0, y: 0, z: 0 };
        let texture_copy_view = wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin,
            aspect: Default::default(),
        };

        encoder.copy_texture_to_buffer(texture_copy_view, buffer_copy_view, extent);
        queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);

        let pixels = async {
            if let Ok(()) = buffer_future.await {
                let pixels_slice = buffer_slice.get_mapped_range();
                let mut pixels = Vec::with_capacity((size.height * size.width) as usize);
                for chunck in pixels_slice.chunks(buffer_dimensions.padded_bytes_per_row) {
                    for chunk in chunck.chunks(4) {
                        // convert Bgra to Rgba
                        pixels.push(chunk[2]);
                        pixels.push(chunk[1]);
                        pixels.push(chunk[0]);
                        pixels.push(chunk[3]);
                    }
                }
                drop(pixels_slice);
                staging_buffer.unmap();
                pixels
            } else {
                panic!("could not read fake texture");
            }
        };
        let pixels = futures::executor::block_on(pixels);
        let mut png_encoder = png::Encoder::new(
            std::fs::File::create(png_name).unwrap(),
            buffer_dimensions.width as u32,
            buffer_dimensions.height as u32,
        );
        png_encoder.set_depth(png::BitDepth::Eight);
        png_encoder.set_color(png::ColorType::Rgba);

        let mut png_writer = png_encoder
            .write_header()
            .unwrap()
            .into_stream_writer_with_size(buffer_dimensions.unpadded_bytes_per_row)
            .unwrap();

        for chunk in pixels.chunks(buffer_dimensions.padded_bytes_per_row) {
            png_writer
                .write_all(&chunk[..buffer_dimensions.unpadded_bytes_per_row])
                .unwrap();
        }
        png_writer.finish().unwrap();
    }
}

/// A structure that stores the element that needs to be updated in a scene
#[derive(Default)]
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
        if !self.is_stereographic() {
            self.view.borrow_mut().update(ViewUpdate::Fog(fog))
        }
    }
}

impl<S: AppState> Application for Scene<S> {
    type AppState = S;
    fn on_notify(&mut self, notification: Notification) {
        log::info!("scene notified {:?}", notification);
        let older_state = self.older_state.clone();
        match notification {
            Notification::ClearDesigns => self.clear_design(),
            Notification::ToggleText(value) => self.view.borrow_mut().set_draw_letter(value),
            Notification::FitRequest => self.fit_design(),
            Notification::CameraTarget((target, up)) => {
                self.set_camera_target(target, up, &older_state);
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::TeleportCamera(camera) => {
                self.controller
                    .teleport_camera(camera.position, camera.orientation);
                if let Some(pivot) = camera.pivot_position {
                    self.data.borrow_mut().set_pivot_position(pivot);
                }
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
                    let surface_info = if let Selection::Nucleotide(_, nt) = selection {
                        self.data.borrow().get_surface_info_nucl(nt)
                    } else {
                        None
                    };
                    if let Some(surface_info) = surface_info {
                        self.controller.set_surface_point(surface_info);
                    } else if let Some(position) = self.data.borrow().get_selected_position() {
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
            Notification::Fog(fog) => self.fog_request(fog),
            Notification::WindowFocusLost => self.controller.stop_camera_movement(),
            Notification::NewStereographicCamera(camera_ptr) => {
                if !self.is_stereographic() {
                    self.data
                        .borrow_mut()
                        .update_stereographic_camera(camera_ptr);
                    if self.older_state.follow_stereographic_camera() {
                        let camera = self.data.borrow().get_aligned_camera();
                        self.on_notify(Notification::TeleportCamera(camera));
                    }
                }
            }
            Notification::FlipSplitViews => (),
            Notification::HorizonAligned => {
                self.controller.align_horizon();
                self.notify(SceneNotification::CameraMoved);
            }
            Notification::ScreenShot3D => {
                if !self.is_stereographic() {
                    self.export_png();
                }
            }
        }
    }

    fn on_event(
        &mut self,
        event: &WindowEvent,
        cursor_position: PhysicalPosition<f64>,
        app_state: &S,
    ) -> Option<ensnano_interactor::CursorIcon> {
        self.element_selector
            .set_stereographic(self.is_stereographic());
        if self.is_stereographic() {
            let stereography = self.view.borrow().get_stereography();
            self.controller.set_setreography(Some(stereography));
        } else {
            self.controller.set_setreography(None);
        }
        self.input(event, cursor_position, app_state)
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

    fn get_camera(&self) -> Option<Arc<(Camera3D, f32)>> {
        Some(self.current_camera.clone())
    }

    fn get_current_selection_pivot(&self) -> Option<GroupPivot> {
        self.view.borrow().get_current_pivot()
    }

    fn is_splited(&self) -> bool {
        false
    }
}

pub trait AppState: Clone + 'static {
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
    fn get_check_xover_parameters(&self) -> CheckXoversParameter;
    fn follow_stereographic_camera(&self) -> bool;
    fn get_draw_options(&self) -> DrawOptions;
    fn draw_options_were_updated(&self, other: &Self) -> bool;
    fn get_scroll_sensitivity(&self) -> f32;
    fn show_insertion_representents(&self) -> bool;

    fn insertion_bond_display_was_modified(&self, other: &Self) -> bool {
        self.show_insertion_representents() != other.show_insertion_representents()
    }

    fn show_bezier_paths(&self) -> bool;

    fn get_design_path(&self) -> Option<PathBuf>;

    fn get_selected_bezier_vertex(&self) -> Option<BezierVertexId>;

    fn has_selected_a_bezier_grid(&self) -> bool;

    fn get_revolution_axis_position(&self) -> Option<f64>;
    fn revolution_bezier_updated(&self, other: &Self) -> bool;
    fn get_current_unrooted_surface(&self) -> Option<UnrootedRevolutionSurfaceDescriptor>;
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
    fn paste_candidate_on_grid(&mut self, position: GridPosition);
    fn attempt_paste_on_grid(&mut self, position: GridPosition);
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
    fn set_revolution_axis_position(&mut self, position: f32);
}
