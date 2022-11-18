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

use super::dragging_state::ClickInfo;
use super::*;
use std::time::Instant;

/// The limit between "near" and "far" distances.
const FAR_AWAY: f64 = 5.0;

/// Holding the mouse button for this duration will trigger OptionalTransition in some states.
const LONG_HOLDING_TIME: std::time::Duration = std::time::Duration::from_millis(350);

/// A possible transition that will be triggered by a certain event in any PointAndClicking state.
///
///
/// If `None`, the controller's automata will transition to `NormalState` when the event occur.
///
/// The state is produced in a function and not stored by the object because Box<dyn> cannot be
/// cloned.
trait OptionalTransition<S: AppState>:
    Fn(ClickInfo) -> Option<Box<dyn ControllerState<S>>> + 'static
{
}
impl<S: AppState, F: 'static> OptionalTransition<S> for F where
    F: Fn(ClickInfo) -> Option<Box<dyn ControllerState<S>>>
{
}

enum OptionalTransitionPtr<S: AppState> {
    Owned(Box<dyn OptionalTransition<S>>),
    Borrowed(&'static dyn OptionalTransition<S>),
}

impl<S: AppState> Default for OptionalTransitionPtr<S> {
    fn default() -> Self {
        Self::Borrowed(&back_to_normal_state)
    }
}

impl<S: AppState> OptionalTransitionPtr<S> {
    fn double_clicking(element: Option<SceneElement>) -> Self {
        let now = Instant::now();
        Self::Owned(Box::new(move |info| {
            Some(Box::new(PointAndClicking::double_clicking(
                info.clicked_position,
                now,
                element,
            )))
        }))
    }
}

impl<S: AppState> std::ops::Deref for OptionalTransitionPtr<S> {
    type Target = dyn OptionalTransition<S> + 'static;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(x) => x,
            Self::Borrowed(x) => x,
        }
    }
}

/// A function that maps a context (i.e. a pair &Controller, &mut ElementSelector) to an
/// OptionalTransition.
///
/// This is usefull when the context has an influence on weither a certain event should trigger an
/// OptionalTransition.
trait ContextDependentTransition<S: AppState>:
    for<'a, 'b> Fn(&'b mut EventContext<'a, S>, ClickInfo) -> Box<dyn OptionalTransition<S>>
{
}

enum ContextDependentTransitionPtr<S: AppState> {
    Owned(Box<dyn ContextDependentTransition<S>>),
    Borrowed(&'static dyn ContextDependentTransition<S>),
}

impl<S: AppState> std::ops::Deref for ContextDependentTransitionPtr<S> {
    type Target = dyn ContextDependentTransition<S> + 'static;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(x) => x,
            Self::Borrowed(x) => x,
        }
    }
}

impl<S: AppState, F: 'static> ContextDependentTransition<S> for F where
    F: for<'a, 'b> Fn(&'b mut EventContext<'a, S>, ClickInfo) -> Box<dyn OptionalTransition<S>>
{
}

/// A state in which the user is clicking on an object.
///
/// The controller's automata between the moment the button is pressed and the moment it is
/// released.
pub(super) struct PointAndClicking<S: AppState> {
    /// The position of the cursor when the mouse button was pressed
    clicked_position: PhysicalPosition<f64>,
    /// The button that was pressed
    pressed_button: MouseButton,
    /// The consequences of releasing of clicking of the object initially pointed by the cursor
    release_consequences: Consequence,
    /// An `OptionalTransition` triggered by releasing the button that was pressed to enter the
    /// state
    release_transition: OptionalTransitionPtr<S>,
    /// An `OptionalTransition` triggered by moving the cursor far away from
    /// `self.clicked_position`.
    away_state: OptionalTransitionPtr<S>,
    /// If Some(_), a function that will update `self.away_state` when the cursor position
    /// changes.
    away_state_maker: Option<ContextDependentTransitionPtr<S>>,
    /// If Some(_), an `OptionalTransition` triggered when the cursor has been held for a long
    /// time.
    long_hold_state: Option<OptionalTransitionPtr<S>>,
    /// If Some(_), a function that will update `self.long_hold_state` when the cursor position
    /// changes.
    long_hold_state_maker: Option<ContextDependentTransitionPtr<S>>,
    /// A description of the current state of the controller's automata
    description: &'static str,
    clicked_date: std::time::Instant,
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
    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        mut context: EventContext<'a, S>,
    ) -> Transition<S> {
        let position = context.cursor_position;
        match event {
            WindowEvent::CursorMoved { .. } => {
                if let Some(transition_maker) = self.away_state_maker.as_ref() {
                    self.away_state = OptionalTransitionPtr::Owned(transition_maker(
                        &mut context,
                        self.get_click_info(position),
                    ))
                }
                if position_difference(position, self.clicked_position) > FAR_AWAY {
                    let new_state =
                        (self.away_state)(self.get_click_info(position)).or_else(|| {
                            Some(Box::new(NormalState {
                                mouse_position: position,
                            }))
                        });
                    Transition {
                        new_state,
                        consequences: Consequence::Nothing,
                    }
                } else {
                    if let Some(transition_maker) = self.long_hold_state_maker.as_ref() {
                        self.long_hold_state = Some(OptionalTransitionPtr::Owned(transition_maker(
                            &mut context,
                            self.get_click_info(position),
                        )))
                    }
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } if *button == self.pressed_button => {
                let new_state =
                    (self.release_transition)(self.get_click_info(position)).or_else(|| {
                        Some(Box::new(NormalState {
                            mouse_position: position,
                        }))
                    });

                Transition {
                    new_state,
                    consequences: self.release_consequences.clone(),
                }
            }
            _ => Transition::nothing(),
        }
    }

    fn display(&self) -> Cow<'static, str> {
        self.description.into()
    }

    fn check_timers(&mut self, controller: &Controller<S>) -> Transition<S> {
        if let Some(transition) = self.long_hold_state.as_ref() {
            log::info!("Some long hold state");
            let now = Instant::now();
            if (now - self.clicked_date) > LONG_HOLDING_TIME
                || other_ctrl(&controller.current_modifiers)
            {
                if let Some(new_state) = transition(self.get_click_info(self.clicked_position)) {
                    return Transition {
                        new_state: Some(new_state),
                        consequences: Consequence::Nothing,
                    };
                }
            } else {
                log::debug!("No transition");
            }
        }
        Transition::nothing()
    }

    fn give_context<'a>(&mut self, mut context: EventContext<'a, S>) {
        if let Some(transition_maker) = self.long_hold_state_maker.as_ref() {
            let position = context.cursor_position;
            self.long_hold_state = Some(OptionalTransitionPtr::Owned(transition_maker(
                &mut context,
                self.get_click_info(position),
            )))
        }
    }
}

impl<S: AppState> PointAndClicking<S> {
    /// A state in which the user is setting the pivot arrond which camera translation occur.
    ///
    /// If the cursor is moved away from it's initial position, the controller's automata
    /// transition to "Rotating Camera" state
    pub(super) fn setting_pivot(
        clicked_position: PhysicalPosition<f64>,
        pivot_elment: Option<SceneElement>,
        tilt: bool,
    ) -> Self {
        let away_state = if tilt {
            OptionalTransitionPtr::Borrowed(&tilt_camera)
        } else {
            OptionalTransitionPtr::Borrowed(&rotating_camera)
        };
        Self {
            away_state,
            away_state_maker: None,
            release_transition: Default::default(),
            clicked_position,
            description: "Setting Pivot",
            pressed_button: MouseButton::Right,
            release_consequences: Consequence::PivotElement(pivot_elment),
            long_hold_state: None,
            long_hold_state_maker: None,
            clicked_date: std::time::Instant::now(),
        }
    }
}

fn rotating_camera<S: AppState>(click: ClickInfo) -> Option<Box<dyn ControllerState<S>>> {
    Some(Box::new(dragging_state::rotating_camera(click)))
}

fn tilt_camera<S: AppState>(click: ClickInfo) -> Option<Box<dyn ControllerState<S>>> {
    Some(Box::new(dragging_state::tilting_camera(click)))
}

fn back_to_normal_state<S: AppState>(click: ClickInfo) -> Option<Box<dyn ControllerState<S>>> {
    Some(Box::new(NormalState {
        mouse_position: click.current_position,
    }))
}

fn leaving_selection<'a, S: AppState>(
    context: &'a EventContext<'a, S>,
    element: Option<SceneElement>,
) -> Box<dyn OptionalTransition<S>> {
    if let Some(SceneElement::BezierVertex { path_id, vertex_id }) = element {
        Box::new(move |click_info| {
            Some(Box::new(dragging_state::moving_bezier_vertex(
                click_info,
                MovingBezierVertex::Existing { vertex_id, path_id },
            )))
        })
    } else {
        let nucl = context.can_start_builder(element);
        Box::new(move |click_info| build_strand(click_info, nucl))
    }
}

fn build_strand<S: AppState>(
    click: ClickInfo,
    nucl: Option<Nucl>,
) -> Option<Box<dyn ControllerState<S>>> {
    let nucls = vec![nucl?];
    Some(Box::new(dragging_state::building_strands(click, nucls)))
}

impl<S: AppState> PointAndClicking<S> {
    /// A state in which the user is selecting an element.
    ///
    /// If the user is clicking on a nucleotide and hold the mouse button for a long time, the
    /// controller's automata transitions to the `MakingXover` state.
    pub(super) fn selecting(
        clicked_position: PhysicalPosition<f64>,
        element: Option<SceneElement>,
        adding: bool,
    ) -> Self {
        Self {
            away_state: Default::default(),
            away_state_maker: Some(ContextDependentTransitionPtr::Owned(Box::new(
                move |context, _| leaving_selection(context, element),
            ))),
            clicked_date: Instant::now(),
            clicked_position,
            description: "Selecting",
            pressed_button: MouseButton::Left,
            release_consequences: Consequence::ElementSelected(element, adding),
            release_transition: OptionalTransitionPtr::double_clicking(element),
            long_hold_state: None,
            long_hold_state_maker: Some(ContextDependentTransitionPtr::Borrowed(
                &making_xover_maker,
            )),
        }
    }

    #[allow(dead_code)] // was used to make it possible to reverse surface direction with a double
                        // click. We may want to use it again in the future.
    pub(super) fn reversing_surface_direction(
        clicked_position: PhysicalPosition<f64>,
        clicked_date: Instant,
    ) -> Self {
        Self {
            away_state: Default::default(),
            away_state_maker: None,
            clicked_date,
            description: "Reversing surface direction",
            pressed_button: MouseButton::Middle,
            release_consequences: Consequence::ReverseSurfaceDirection,
            release_transition: Default::default(),
            long_hold_state: Some(Default::default()),
            clicked_position,
            long_hold_state_maker: None,
        }
    }

    /// A state in which the user may be performing a double click
    ///
    /// If the user clicks on the element a second time in a short (i.e. < `LONG_HOLDING_TIME` )
    /// time interval, this triggers a "double click" consequence.
    fn double_clicking(
        clicked_position: PhysicalPosition<f64>,
        clicked_date: Instant,
        element: Option<SceneElement>,
    ) -> Self {
        Self {
            away_state: Default::default(),
            away_state_maker: None,
            clicked_date,
            description: "Waiting for double click",
            pressed_button: MouseButton::Left,
            release_consequences: Consequence::DoubleClick(element),
            release_transition: Default::default(),
            long_hold_state: Some(Default::default()),
            clicked_position,
            long_hold_state_maker: None,
        }
    }

    /// A state in which the user is pasting something
    pub(super) fn pasting(
        clicked_position: PhysicalPosition<f64>,
        element: Option<SceneElement>,
    ) -> Self {
        Self {
            away_state: Default::default(),
            away_state_maker: None,
            clicked_date: Instant::now(),
            description: "Pasting",
            pressed_button: MouseButton::Left,
            release_consequences: Consequence::Paste(element),
            long_hold_state: None,
            clicked_position,
            long_hold_state_maker: None,
            release_transition: Default::default(),
        }
    }

    pub(super) fn building_helix(state: super::BuildingHelix) -> Self {
        Self {
            away_state: Default::default(),
            away_state_maker: None,
            clicked_date: Instant::now(),
            description: "Building Helix",
            pressed_button: MouseButton::Left,
            release_consequences: Consequence::BuildHelix {
                design_id: state.design_id,
                grid_id: state.grid_id,
                length: state.length_helix,
                x: state.x_helix,
                y: state.y_helix,
                position: state.position_helix,
            },
            long_hold_state: None,
            clicked_position: state.clicked_position,
            long_hold_state_maker: None,
            release_transition: Default::default(),
        }
    }
}

fn making_xover_maker<'a, S: AppState>(
    context: &mut EventContext<'a, S>,
    _click: ClickInfo,
) -> Box<dyn OptionalTransition<S>> {
    let origin = context.get_xover_origin_under_cursor();
    Box::new(move |click: ClickInfo| making_xover(click, &origin))
}

fn making_xover<S: AppState>(
    click: ClickInfo,
    origin: &Option<XoverOrigin>,
) -> Option<Box<dyn ControllerState<S>>> {
    if let Some(source) = origin {
        Some(Box::new(dragging_state::making_xover(
            click,
            source.clone(),
        )))
    } else {
        None
    }
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}
