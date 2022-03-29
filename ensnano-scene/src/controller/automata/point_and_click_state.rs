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

//! Defines the state in which the user is clicking on an object.
//!
//! Such a state is enterred when a mouse button is pressed while the cursor is on some specific
//! object. If the mouse button is released while the cursor is still close to the position on
//! which it was when the button was pressed, the release of the button is treated as a click on
//! the object.
//!
//! If the cursor moves away form this position this causes a transition to either the normal
//! state, or a specific DraggingState.

use super::*;
use super::dragging_state::{DraggingState, ClickInfo};

/// The limit between "near" and "far" distances.
const FAR_AWAY: f64 = 5.0;

/// A state to which the controller automata should transition when the cursor is moved far
/// away from `self.clicked_position`.
/// 
/// If `None`, the controller's automata will transition to `NormalState` when the cursor moves
/// far away from `self.clicked_position`.
///
/// The state is produced in a function and not stored by the object because Box<dyn> cannot be
/// cloned.
trait AwayTransition<S: AppState>: Fn (ClickInfo) -> Option<Box<dyn ControllerState<S>>> + 'static { }
impl<S: AppState, F: 'static> AwayTransition<S> for F where F: Fn (ClickInfo) -> Option<Box<dyn ControllerState<S>>> { }

/// A state in which the user is clicking on an object. 
/// 
/// The controller's automata between the moment the button is pressed and the moment it is
/// released.
struct PointAndClicking<S: AppState> {
    /// The position of the cursor when the mouse button was pressed
    clicked_position: PhysicalPosition<f64>,
    /// The button that was pressed
    pressed_button: MouseButton,
    /// The consequences of releasing of clicking of the object initially pointed by the cursor
    release_consequences:  Consequence,
    /// A state to which the controller automata should transition when the cursor is moved far
    /// away from `self.clicked_position`.
    /// 
    /// If `None`, the controller's automata will transition to `NormalState` when the cursor moves
    /// far away from `self.clicked_position`.
    away_state: &'static dyn AwayTransition<S>,
    /// A description of the current state of the controller's automata
    description: &'static str,
}

impl<S: AppState> PointAndClicking<S> {
    fn get_click_info(&self, position: PhysicalPosition<f64>) -> ClickInfo {
        ClickInfo {
            button: self.pressed_button,
            current_position: position,
            clicked_position: self.clicked_position,
        }
    }
}

impl<S: AppState> ControllerState<S> for PointAndClicking<S> {
    fn input(&mut self, event: &WindowEvent, position: PhysicalPosition<f64>, _controller: &Controller<S>, _pixel_reader: &mut ElementSelector, _app_state: &S) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(position, self.clicked_position) > FAR_AWAY {
                    let new_state = (self.away_state)(self.get_click_info(position)).or_else(|| Some(Box::new(NormalState {
                        mouse_position: position
                    })));
                    Transition {
                        new_state,
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } if *button == self.pressed_button => {
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: position
                    })),
                    consequences: self.release_consequences.clone()
                }
            }
            _ => Transition::nothing(),
        }
    }

    fn display(&self) -> Cow<'static, str> {
        self.description.into()
    }
}

impl<S: AppState> PointAndClicking<S> {
    /// A state in which the user is setting the pivot arrond which camera translation occur.
    ///
    /// If the cursor is moved away from it's initial position, the controller's automata
    /// transition to "Rotating Camera" state
    pub (super) fn setting_pivot(clicked_position: PhysicalPosition<f64>, pivot_elment: Option<SceneElement>) -> Self {
        Self {
            away_state: &rotating_camera,
            clicked_position,
            description: "Setting Pivot",
            pressed_button: MouseButton::Right,
            release_consequences: Consequence::PivotElement(pivot_elment),
        }
    }
}

fn rotating_camera<S: AppState>(click: ClickInfo) -> Option<Box<dyn ControllerState<S>>> {
    Some(Box::new(dragging_state::rotating_camera(click)))
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

