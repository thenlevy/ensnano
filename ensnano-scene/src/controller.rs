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
use super::view::HandleColors;
use super::{
    camera, ultraviolet, Duration, ElementSelector, HandleDir, SceneElement, Stereography, ViewPtr,
    WidgetRotationMode as RotationMode,
};
use crate::{PhySize, PhysicalPosition, WindowEvent};
use ensnano_design::grid::{GridId, GridObject, GridPosition, HelixGridPosition};
use ensnano_design::{
    BezierPathId, BezierPlaneId, BezierVertex, BezierVertexId, Nucl, SurfaceInfo, SurfacePoint,
};
use ensnano_interactor::consts::*;
use ensnano_interactor::Selection;
use ensnano_utils::winit::event::*;
use std::cell::RefCell;
use std::ops::Deref;
use ultraviolet::{Rotor3, Vec2, Vec3};

use super::AppState;

use camera::{CameraController, FiniteVec3};

mod automata;
pub use automata::WidgetTarget;
use automata::{EventContext, NormalState, State, Transition};

/// The effect that draging the mouse have
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClickMode {
    TranslateCam,
    #[allow(dead_code)]
    RotateCam,
}

use std::rc::Rc;
type DataPtr = Rc<RefCell<dyn Data>>;

/// An object handling input and notification for the scene.
pub struct Controller<S: AppState> {
    /// A pointer to the View
    view: ViewPtr,
    /// A pointer to the data
    data: DataPtr,
    /// The event that modify the camera are forwarded to the camera_controller
    camera_controller: CameraController,
    /// The size of the window
    window_size: PhySize,
    /// The size of the drawing area
    area_size: PhySize,
    /// The current modifiers
    current_modifiers: ModifiersState,
    /// The effect that dragging the mouse has
    click_mode: ClickMode,
    state: State<S>,
    stereography: Option<Stereography>,
    /// The origin of the two points bezier curve being created.
    bezier_curve_origin: Option<HelixGridPosition>,
}

#[derive(Clone, Debug)]
pub enum Consequence {
    CameraMoved,
    CameraTranslated(f64, f64),
    XoverAtempt(Nucl, Nucl, usize, bool),
    QuickXoverAttempt {
        nucl: Nucl,
        doubled: bool,
    },
    Translation(HandleDir, f64, f64, WidgetTarget),
    MovementEnded,
    Rotation(f64, f64, WidgetTarget),
    InitRotation(RotationMode, f64, f64, WidgetTarget),
    InitTranslation(f64, f64, WidgetTarget),
    Swing(f64, f64),
    Tilt(f64, f64),
    Nothing,
    ToggleWidget,
    BuildEnded,
    Building(isize),
    Undo,
    Redo,
    Candidate(Option<super::SceneElement>),
    PivotElement(Option<super::SceneElement>),
    ElementSelected(Option<super::SceneElement>, bool),
    MoveFreeXover(Option<super::SceneElement>, Vec3),
    EndFreeXover,
    BuildHelix {
        design_id: u32,
        grid_id: GridId,
        position: isize,
        length: usize,
        x: isize,
        y: isize,
    },
    PasteCandidate(Option<super::SceneElement>),
    Paste(Option<super::SceneElement>),
    DoubleClick(Option<super::SceneElement>),
    InitBuild(Vec<Nucl>),
    ObjectTranslated {
        object: GridObject,
        grid: GridId,
        x: isize,
        y: isize,
    },
    HelixSelected(usize),
    PivotCenter,
    CheckXovers,
    AlignWithStereo,
    /// Appen a vertex to a bezier path
    CreateBezierVertex {
        /// The position of the created vertex
        vertex: BezierVertex,
        /// The identifier of the path to which the vertex is beign appenend. If this is None, a
        /// new path is being created
        path: Option<BezierPathId>,
    },
    MoveBezierVertex {
        x: f32,
        y: f32,
        path_id: BezierPathId,
        vertex_id: usize,
    },
    ReleaseBezierVertex,
    MoveBezierCorner {
        plane_id: BezierPlaneId,
        moving_corner: Vec2,
        original_corner_position: Vec2,
        fixed_corner_position: Vec2,
    },
    ReleaseBezierCorner,
    ReleaseBezierTengent,
    MoveBezierTengent {
        vertex_id: BezierVertexId,
        tengent_in: bool,
        full_symetry_other: bool,
        new_vector: Vec2,
    },
    ReverseSurfaceDirection,
    SetRevolutionAxisPosition(f32),
}

enum TransistionConsequence {
    Nothing,
    InitCameraMovement {
        translation: bool,
        nucl: Option<Nucl>,
    },
    EndCameraMovement,
    InitFreeXover(Nucl, usize, Vec3),
    StopRotatingPivot,
    StartRotatingPivot,
}

impl<S: AppState> Controller<S> {
    pub(super) fn new(
        view: ViewPtr,
        data: DataPtr,
        window_size: PhySize,
        area_size: PhySize,
    ) -> Self {
        let camera_controller = {
            let view = view.borrow();
            CameraController::new(4.0, view.get_camera(), view.get_projection())
        };
        Self {
            view,
            data,
            camera_controller,
            window_size,
            area_size,
            current_modifiers: ModifiersState::empty(),
            click_mode: ClickMode::TranslateCam,
            state: automata::initial_state(),
            stereography: None,
            bezier_curve_origin: None,
        }
    }

    pub fn set_setreography(&mut self, stereography: Option<Stereography>) {
        self.stereography = stereography;
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        log::info!("New modifiers {:?}", modifiers);
        self.current_modifiers = modifiers;
        if !modifiers.shift() {
            self.bezier_curve_origin = None;
        }
    }

    /// Replace the camera by a new one.
    pub fn teleport_camera(&mut self, position: Vec3, rotation: Rotor3) {
        self.camera_controller.teleport_camera(position, rotation);
        self.end_movement();
    }

    pub fn set_surface_point(&mut self, info: SurfaceInfo) {
        self.camera_controller.set_surface_point(info);
        self.end_movement();
    }

    pub fn reverse_surface_direction(&mut self) {
        self.camera_controller
            .reverse_surface_direction(self.data.borrow().deref());
        self.end_movement();
    }

    pub fn align_horizon(&mut self) {
        let angle = self.camera_controller.horizon_angle();
        self.camera_controller.tilt_camera(angle);
    }

    pub fn set_camera_position(&mut self, position: Vec3) {
        self.camera_controller.set_camera_position(position);
        self.end_movement();
    }

    /// Keep the camera orientation and make it face a given point.
    pub fn center_camera(&mut self, center: Vec3) {
        self.camera_controller.center_camera(center)
    }

    pub fn check_timers(&mut self) -> Consequence {
        log::debug!("Checking timers");
        let transition = self.state.borrow_mut().check_timers(self);
        if let Some(state) = transition.new_state {
            log::info!("3D controller state: {}", state.display());
            let csq = self.state.borrow().transition_from(self);
            self.transition_consequence(csq);
            self.state = RefCell::new(state);
            let csq = self.state.borrow().transition_to(self);
            self.transition_consequence(csq);
        }
        transition.consequences
    }

    fn handles_color_system(&self) -> HandleColors {
        self.state
            .borrow()
            .handles_color_system()
            .unwrap_or(if self.current_modifiers.shift() {
                HandleColors::Cym
            } else {
                HandleColors::Rgb
            })
    }

    pub fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        pixel_reader: &mut ElementSelector,
        app_state: &S,
    ) -> Consequence {
        let transition = if let WindowEvent::Focused(false) = event {
            self.camera_controller.stop_camera_movement();
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: PhysicalPosition::new(-1., -1.),
                })),
                consequences: Consequence::Nothing,
            }
        } else if let WindowEvent::MouseWheel { delta, .. } = event {
            let mouse_x = position.x / self.area_size.width as f64;
            let mouse_y = position.y / self.area_size.height as f64;
            if ctrl(&self.current_modifiers) {
                self.camera_controller.update_stereographic_zoom(delta);
                Transition::consequence(Consequence::CameraMoved)
            /*} else if self.current_modifiers.shift() {
            self.state.borrow_mut().notify_scroll();
            let element = pixel_reader.set_selected_id(position);
            if let Some(builder) = app_state.get_strand_builders().get(0) {
                let init_position = builder.get_moving_end_nucl().position;
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y.signum() as isize,
                    MouseScrollDelta::PixelDelta(pos) => pos.y.signum() as isize,
                };
                Transition::consequence(Consequence::Building(init_position + delta))
            } else if let Some(nucl) = self
                .data
                .borrow()
                .can_start_builder(self.state.borrow().element_being_selected())
            {
                Transition::init_building(vec![nucl], false)
            } else if let Selection::Nucleotide(_, nucl) =
                self.data.borrow().element_to_selection(&element)
            {
                Transition::init_building(vec![nucl], false)
            } else if let Selection::Xover(_, xover_id) =
                self.data.borrow().element_to_selection(&element)
            {
                if let Some((n1, n2)) =
                    app_state.get_design_reader().get_xover_with_id(xover_id)
                {
                    Transition::init_building(vec![n1, n2], false)
                } else {
                    self.camera_controller.process_scroll(
                        delta,
                        mouse_x as f32,
                        mouse_y as f32,
                    );
                    Transition::consequence(Consequence::CameraMoved)
                }
            } else {
                self.camera_controller
                    .process_scroll(delta, mouse_x as f32, mouse_y as f32);
                Transition::consequence(Consequence::CameraMoved)
            }

                * The above code was used to move the current strand builder with the mouse
                * wheel
                */
            } else {
                self.camera_controller.process_scroll(
                    delta,
                    mouse_x as f32,
                    mouse_y as f32,
                    app_state.get_scroll_sensitivity(),
                );
                Transition::consequence(Consequence::CameraMoved)
            }
        } else if let WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state,
                    virtual_keycode: Some(key),
                    ..
                },
            ..
        } = event
        {
            let csq = match *key {
                VirtualKeyCode::A if *state == ElementState::Pressed => {
                    Consequence::AlignWithStereo
                }
                VirtualKeyCode::C if *state == ElementState::Pressed => Consequence::CheckXovers,
                VirtualKeyCode::Z
                    if ctrl(&self.current_modifiers) && *state == ElementState::Pressed =>
                {
                    Consequence::Undo
                }
                VirtualKeyCode::R
                    if ctrl(&self.current_modifiers) && *state == ElementState::Pressed =>
                {
                    Consequence::Redo
                }
                VirtualKeyCode::Q => Consequence::PivotCenter,
                VirtualKeyCode::Space if *state == ElementState::Pressed => {
                    Consequence::ToggleWidget
                }
                VirtualKeyCode::W if *state == ElementState::Pressed => {
                    Consequence::ReverseSurfaceDirection
                }
                _ => {
                    if self.camera_controller.process_keyboard(*key, *state) {
                        Consequence::CameraMoved
                    } else {
                        Consequence::Nothing
                    }
                }
            };
            Transition::consequence(csq)
        } else {
            self.state.borrow_mut().input(
                event,
                EventContext::new(self, app_state, pixel_reader, position),
            )
        };

        if let Some(mut state) = transition.new_state {
            state.give_context(EventContext::new(self, app_state, pixel_reader, position));
            log::info!("3D controller state: {}", state.display());
            let csq = self.state.borrow().transition_from(self);
            self.transition_consequence(csq);
            self.state = RefCell::new(state);
            let csq = self.state.borrow().transition_to(self);
            self.transition_consequence(csq);
        }
        transition.consequences
    }

    fn transition_consequence(&mut self, csq: TransistionConsequence) {
        match csq {
            TransistionConsequence::Nothing => (),
            TransistionConsequence::InitCameraMovement { translation, nucl } => {
                if let Some(info) = nucl
                    .and_then(|n| self.data.borrow().get_surface_info_nucl(n))
                    .filter(|_| self.current_modifiers.shift())
                {
                    self.camera_controller.set_surface_point_if_unset(info);
                }
                self.init_movement(translation && self.current_modifiers.shift())
            }
            TransistionConsequence::EndCameraMovement => self.end_movement(),
            TransistionConsequence::InitFreeXover(nucl, d_id, position) => {
                self.data.borrow_mut().init_free_xover(nucl, position, d_id)
            }
            TransistionConsequence::StartRotatingPivot => {
                self.data.borrow_mut().notify_rotating_pivot()
            }
            TransistionConsequence::StopRotatingPivot => {
                self.data.borrow_mut().stop_rotating_pivot()
            }
        }
    }

    /// True if the camera is moving and its position must be updated before next frame
    pub fn camera_is_moving(&self) -> bool {
        self.camera_controller.is_moving()
    }

    /// Set the pivot point of the camera
    pub fn set_pivot_point(&mut self, point: Option<FiniteVec3>) {
        self.camera_controller.set_pivot_point(point)
    }

    /// Swing the camera arround its pivot point
    pub fn swing(&mut self, x: f64, y: f64) {
        self.camera_controller.swing(x, y);
    }

    /// Moves the camera according to its speed and the time elapsed since previous frame
    pub fn update_camera(&mut self, dt: Duration) {
        self.camera_controller.update_camera(
            dt,
            self.click_mode,
            &self.current_modifiers,
            self.data.borrow().deref(),
        );
        self.data
            .borrow_mut()
            .notify_camera_movement(&self.camera_controller);
    }

    /// Handles a resizing of the window and/or drawing area
    pub fn resize(&mut self, window_size: PhySize, area_size: PhySize) {
        self.window_size = window_size;
        self.area_size = area_size;
        self.camera_controller.resize(area_size);
        // the view needs the window size to build a depth texture
        self.view
            .borrow_mut()
            .update(super::view::ViewUpdate::Size(window_size));
    }

    pub fn get_window_size(&self) -> PhySize {
        self.window_size
    }

    fn init_movement(&mut self, along_surface: bool) {
        self.camera_controller.init_movement(along_surface);
        if !ctrl(&self.current_modifiers) {
            self.camera_controller
                .init_constrained_rotation(!self.current_modifiers.alt())
        }
    }

    fn end_movement(&mut self) {
        self.camera_controller.end_movement();
    }

    pub fn set_camera_target(&mut self, target: Vec3, up: Vec3, pivot: Option<Vec3>) {
        self.camera_controller.init_movement(false);
        self.camera_controller
            .look_at_orientation(target, up, pivot);
        self.shift_cam();
    }

    pub fn translate_camera(&mut self, dx: f64, dy: f64) {
        self.camera_controller.process_mouse(dx, dy)
    }

    pub fn rotate_camera(&mut self, xz: f32, yz: f32, xy: f32, pivot: Option<Vec3>) {
        self.camera_controller.init_movement(false);
        self.camera_controller.rotate_camera(xz, yz, pivot);
        self.camera_controller.tilt_camera(xy);
        self.shift_cam();
    }

    pub fn continuous_tilt(&mut self, angle: f32) {
        self.camera_controller.continuous_tilt(angle);
    }

    fn shift_cam(&mut self) {
        self.camera_controller.shift()
    }

    pub fn stop_camera_movement(&mut self) {
        self.camera_controller.stop_camera_movement()
    }

    pub fn update_data(&mut self) {
        self.update_handle_colors();
    }

    fn update_handle_colors(&self) {
        self.data
            .borrow_mut()
            .update_handle_colors(self.handles_color_system());
    }

    pub fn is_building_bezier_curve(&self) -> bool {
        self.current_modifiers.shift()
    }

    /// Set the origin or destination of the two point bezier helix being built.
    ///
    /// If an origin was set, `point` is treated as a destianation and the pair
    /// `(origin, destination)` is returned. Otherwise, `point` is treated as an origin and `None`
    /// is returned.
    pub fn add_bezier_point(
        &mut self,
        point: HelixGridPosition,
    ) -> Option<(HelixGridPosition, HelixGridPosition)> {
        if let Some(position) = self.bezier_curve_origin.take() {
            Some((position, point))
        } else {
            self.bezier_curve_origin = Some(point);
            None
        }
    }

    pub fn get_icon(&self) -> Option<ensnano_interactor::CursorIcon> {
        self.state.borrow().cursor()
    }
}

fn ctrl(modifiers: &ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.logo()
    } else {
        modifiers.ctrl()
    }
}

pub(super) trait Data {
    fn element_to_nucl(
        &self,
        element: &Option<SceneElement>,
        non_phantom: bool,
    ) -> Option<(Nucl, usize)>;
    fn get_nucl_position(&self, nucl: Nucl, d_id: usize) -> Option<Vec3>;
    fn attempt_xover(
        &self,
        source: &Option<SceneElement>,
        dest: &Option<SceneElement>,
    ) -> Option<(Nucl, Nucl, usize)>;
    fn can_start_builder(&self, element: Option<SceneElement>) -> Option<Nucl>;
    fn get_grid_object(&self, position: GridPosition) -> Option<GridObject>;
    fn notify_rotating_pivot(&mut self);
    fn stop_rotating_pivot(&mut self);
    fn update_handle_colors(&mut self, colors: HandleColors);
    fn element_to_selection(&self, element: &Option<SceneElement>) -> Selection;
    fn init_free_xover(&mut self, nucl: Nucl, position: Vec3, design_id: usize);
    fn get_surface_info(&self, point: SurfacePoint) -> Option<SurfaceInfo>;
    fn get_surface_info_nucl(&self, nucl: Nucl) -> Option<SurfaceInfo>;
    fn notify_camera_movement(&mut self, camera: &CameraController);
}
