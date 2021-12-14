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
    camera, Duration, ElementSelector, HandleDir, SceneElement, ViewPtr,
    WidgetRotationMode as RotationMode,
};
use crate::consts::*;
use crate::{PhySize, PhysicalPosition, WindowEvent};
use ensnano_design::Nucl;
use iced_winit::winit::event::*;
use std::cell::RefCell;
use ultraviolet::{Rotor3, Vec3};

use super::AppState;

use camera::{CameraController, FiniteVec3};

mod automata;
pub use automata::WidgetTarget;
use automata::{NormalState, State, Transition};

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
}

pub enum Consequence {
    CameraMoved,
    CameraTranslated(f64, f64),
    XoverAtempt(Nucl, Nucl, usize),
    Translation(HandleDir, f64, f64, WidgetTarget),
    MovementEnded,
    Rotation(f64, f64, WidgetTarget),
    InitRotation(RotationMode, f64, f64, WidgetTarget),
    InitTranslation(f64, f64, WidgetTarget),
    Swing(f64, f64),
    Nothing,
    ToggleWidget,
    BuildEnded,
    Building(isize),
    Undo,
    Redo,
    Candidate(Option<super::SceneElement>),
    PivotElement(Option<super::SceneElement>),
    ElementSelected(Option<super::SceneElement>, bool),
    InitFreeXover(Nucl, usize, Vec3),
    MoveFreeXover(Option<super::SceneElement>, Vec3),
    EndFreeXover,
    BuildHelix {
        design_id: u32,
        grid_id: usize,
        position: isize,
        length: usize,
        x: isize,
        y: isize,
    },
    PasteCandidate(Option<super::SceneElement>),
    Paste(Option<super::SceneElement>),
    DoubleClick(Option<super::SceneElement>),
    InitBuild(Nucl),
    HelixTranslated {
        helix: usize,
        grid: usize,
        x: isize,
        y: isize,
    },
    HelixSelected(usize),
}

enum TransistionConsequence {
    Nothing,
    InitMovement,
    EndMovement,
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
            CameraController::new(
                4.0,
                BASE_SCROLL_SENSITIVITY,
                view.get_camera(),
                view.get_projection(),
            )
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
        }
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.current_modifiers = modifiers;
    }

    /// Replace the camera by a new one.
    pub fn teleport_camera(&mut self, position: Vec3, rotation: Rotor3) {
        self.camera_controller.teleport_camera(position, rotation);
        self.end_movement();
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
        let transition = self.state.borrow_mut().check_timers(&self);
        if let Some(state) = transition.new_state {
            log::info!("3D controller state: {}", state.display());
            let csq = self.state.borrow().transition_from(&self);
            self.transition_consequence(csq);
            self.state = RefCell::new(state);
            let csq = self.state.borrow().transition_to(&self);
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
            self.camera_controller
                .process_scroll(delta, mouse_x as f32, mouse_y as f32);
            Transition::consequence(Consequence::CameraMoved)
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
                VirtualKeyCode::Space if *state == ElementState::Pressed => {
                    Consequence::ToggleWidget
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
            self.state
                .borrow_mut()
                .input(event, position, &self, pixel_reader, app_state)
        };

        if let Some(state) = transition.new_state {
            log::info!("3D controller state: {}", state.display());
            let csq = self.state.borrow().transition_from(&self);
            self.transition_consequence(csq);
            self.state = RefCell::new(state);
            let csq = self.state.borrow().transition_to(&self);
            self.transition_consequence(csq);
        }
        transition.consequences
    }

    fn transition_consequence(&mut self, csq: TransistionConsequence) {
        match csq {
            TransistionConsequence::Nothing => (),
            TransistionConsequence::InitMovement => self.init_movement(),
            TransistionConsequence::EndMovement => self.end_movement(),
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
        self.camera_controller.update_camera(dt, self.click_mode);
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

    fn init_movement(&mut self) {
        self.camera_controller.init_movement();
    }

    fn end_movement(&mut self) {
        self.camera_controller.end_movement();
    }

    pub fn change_sensitivity(&mut self, sensitivity: f32) {
        self.camera_controller.sensitivity = 10f32.powf(sensitivity / 10.) * BASE_SCROLL_SENSITIVITY
    }

    pub fn set_camera_target(&mut self, target: Vec3, up: Vec3, pivot: Option<Vec3>) {
        self.camera_controller
            .look_at_orientation(target, up, pivot);
        self.shift_cam();
    }

    pub fn translate_camera(&mut self, dx: f64, dy: f64) {
        self.camera_controller.process_mouse(dx, dy)
    }

    pub fn rotate_camera(&mut self, xz: f32, yz: f32, xy: f32, pivot: Option<Vec3>) {
        self.camera_controller.rotate_camera(xz, yz, pivot);
        self.camera_controller.tilt_camera(xy);
        self.shift_cam();
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
    fn get_grid_helix(&self, grid_id: usize, x: isize, y: isize) -> Option<u32>;
    fn notify_rotating_pivot(&mut self);
    fn stop_rotating_pivot(&mut self);
    fn update_handle_colors(&mut self, colors: HandleColors);
}
