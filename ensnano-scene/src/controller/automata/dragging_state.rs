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

//! Defines states in which the user is "dragging" something.
//!
//! In this context dragging means that the user is holding one of the mouse button while moving
//! the cursor.
//! In such a state, cursor movement all cursor movement have similar consequences shuch has moving
//! the camera or moving an object.

use super::*;

pub(super) struct DraggedCursor {
    /// The current cursor position
    position: PhysicalPosition<f64>,
    /// The *normalized* difference between the current cursor position and the position of the
    /// cursor when the mouse button was pressed
    delta_position: PhysicalPosition<f64>,
}

pub(super) struct ClickInfo {
    pub button: MouseButton,
    pub clicked_position: PhysicalPosition<f64>,
    pub current_position: PhysicalPosition<f64>,
}

impl ClickInfo {
    pub fn new(button: MouseButton, clicked_position: PhysicalPosition<f64>) -> Self {
        Self {
            button,
            clicked_position,
            current_position: clicked_position,
        }
    }
}

/// A object maping cursor movement to their consequences
pub(super) trait DraggingTransitionTable {
    /// The consequences of moving the cursor
    fn on_cursor_moved(&mut self, cursor: DraggedCursor) -> Option<Consequence>;
    fn on_button_released(&self) -> Option<Consequence>;
    /// A description of the state that the controller automata is in
    fn description() -> &'static str;
    /// If not None, the cursor icon that should be used when the controller's automata is in this
    /// state
    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        None
    }
    /// `true` iff being in this state means that the controller's camera is in transitory state
    fn continuously_moving_camera() -> bool;
}

/// A state in which the user is holding a mouse button while moving the cursor.
pub(super) struct DraggingState<Table: DraggingTransitionTable> {
    current_cursor_position: PhysicalPosition<f64>,
    /// The position of the cursor when the mouse button was pressed
    clicked_position: PhysicalPosition<f64>,
    /// The button that was pressed to enter this state
    clicked_button: MouseButton,
    /// A object maping cursor movement to transitions in the controller's automata.
    transition_table: Table,
}

impl<Table: DraggingTransitionTable> DraggingState<Table> {
    /// Register the cursor movement and return an up-to-date DraggedCursor.
    fn move_cursor<S: AppState>(
        &mut self,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
    ) -> DraggedCursor {
        self.current_cursor_position = position;
        let mouse_dx = (position.x - self.clicked_position.x) / controller.area_size.width as f64;
        let mouse_dy = (position.y - self.clicked_position.y) / controller.area_size.height as f64;
        DraggedCursor {
            position: self.current_cursor_position,
            delta_position: PhysicalPosition {
                x: mouse_dx,
                y: mouse_dy,
            },
        }
    }
}

macro_rules! dragging_state_constructor {
    ($contructor_name: ident, $type: tt) => {
        pub(super) fn $contructor_name(click: ClickInfo) -> DraggingState<$type> {
            DraggingState {
                current_cursor_position: click.current_position,
                clicked_button: click.button,
                clicked_position: click.clicked_position,
                transition_table: $type,
            }
        }
    };
}

impl<S: AppState, Table: DraggingTransitionTable> ControllerState<S> for DraggingState<Table> {
    fn display(&self) -> Cow<'static, str> {
        Table::description().into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _pixel_reader: &mut ElementSelector,
        _app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if *button == self.clicked_button => {
                let consequences = self
                    .transition_table
                    .on_button_released()
                    .unwrap_or(Consequence::Nothing);
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.current_cursor_position,
                    })),
                    consequences,
                }
            }

            WindowEvent::CursorMoved { .. } => {
                let cursor = self.move_cursor(position, controller);
                let consequences = self
                    .transition_table
                    .on_cursor_moved(cursor)
                    .unwrap_or(Consequence::Nothing);
                Transition::consequence(consequences)
            }
            _ => Transition::nothing(),
        }
    }

    fn cursor(&self) -> Option<ensnano_interactor::CursorIcon> {
        Table::cursor()
    }

    fn transition_to(&self, _controller: &Controller<S>) -> TransistionConsequence {
        if Table::continuously_moving_camera() {
            TransistionConsequence::InitMovement
        } else {
            TransistionConsequence::Nothing
        }
    }

    fn transition_from(&self, _controller: &Controller<S>) -> TransistionConsequence {
        if Table::continuously_moving_camera() {
            TransistionConsequence::EndMovement
        } else {
            TransistionConsequence::Nothing
        }
    }
}

/// The user is moving the camera.
///
/// Cursor movements translate the camera
pub(super) struct TranslatingCamera;

impl DraggingTransitionTable for TranslatingCamera {
    fn description() -> &'static str {
        "Translating Camera"
    }

    fn on_cursor_moved(&mut self, cursor: DraggedCursor) -> Option<Consequence> {
        Some(Consequence::CameraTranslated(
            cursor.delta_position.x,
            cursor.delta_position.y,
        ))
    }

    fn on_button_released(&self) -> Option<Consequence> {
        Some(Consequence::MovementEnded)
    }

    fn continuously_moving_camera() -> bool {
        true
    }

    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::AllScroll)
    }
}

dragging_state_constructor!(translating_camera, TranslatingCamera);

/// The user is rotating the camera
///
/// Cursor movements rotate the camera
pub(super) struct RotatingCamera;

impl DraggingTransitionTable for RotatingCamera {
    fn description() -> &'static str {
        "Rotating Camera"
    }

    fn on_cursor_moved(&mut self, cursor: DraggedCursor) -> Option<Consequence> {
        Some(Consequence::Swing(
            cursor.delta_position.x,
            cursor.delta_position.y,
        ))
    }

    fn on_button_released(&self) -> Option<Consequence> {
        Some(Consequence::MovementEnded)
    }

    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::AllScroll)
    }

    fn continuously_moving_camera() -> bool {
        true
    }
}

dragging_state_constructor! {rotating_camera, RotatingCamera}

/// The user is tilting the camera
///
/// Cursor movements tilt the camera
pub(super) struct TiltingCamera;

impl DraggingTransitionTable for TiltingCamera {
    fn description() -> &'static str {
        "Tilting Camera"
    }

    fn on_cursor_moved(&mut self, cursor: DraggedCursor) -> Option<Consequence> {
        Some(Consequence::Tilt(
            cursor.delta_position.x,
            cursor.delta_position.y,
        ))
    }

    fn on_button_released(&self) -> Option<Consequence> {
        Some(Consequence::MovementEnded)
    }

    fn continuously_moving_camera() -> bool {
        true
    }

    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::ColResize)
    }
}

dragging_state_constructor! {tilting_camera, TiltingCamera}
