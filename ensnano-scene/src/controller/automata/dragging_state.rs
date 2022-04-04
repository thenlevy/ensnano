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

use crate::element_selector;

use super::*;
use ensnano_design::Axis;

pub(super) struct DraggedCursor<'a, S: AppState> {
    /// The current cursor position
    position: PhysicalPosition<f64>,
    /// The *normalized* difference between the current cursor position and the position of the
    /// cursor when the mouse button was pressed
    delta_position: PhysicalPosition<f64>,
    /// The current cursor position normalized by the size of the scene's area.
    normalized_position: PhysicalPosition<f64>,
    controller: &'a Controller<S>,
    pixel_reader: &'a mut ElementSelector,
    app_state: &'a S,
}

impl<'a, S: AppState> DraggedCursor<'a, S> {
    fn get_projection_on_plane(&self, plane_origin: Vec3) -> Vec3 {
        self.controller.camera_controller.get_projection(
            plane_origin,
            self.normalized_position.x,
            self.normalized_position.y,
            self.controller.stereography.as_ref(),
        )
    }

    pub fn get_element_under_cursor(&mut self) -> Option<SceneElement> {
        self.pixel_reader.set_selected_id(self.position)
    }

    pub(super) fn from_click_cursor(
        clicked_position: PhysicalPosition<f64>,
        current_position: PhysicalPosition<f64>,
        controller: &'a Controller<S>,
        pixel_reader: &'a mut ElementSelector,
        app_state: &'a S,
    ) -> Self {
        let normalized_x = current_position.x / controller.area_size.width as f64;
        let normalized_y = current_position.y / controller.area_size.height as f64;

        let mouse_dx =
            (current_position.x - clicked_position.x) / controller.area_size.width as f64;
        let mouse_dy =
            (current_position.y - clicked_position.y) / controller.area_size.height as f64;

        Self {
            position: current_position,
            delta_position: PhysicalPosition {
                x: mouse_dx,
                y: mouse_dy,
            },
            normalized_position: PhysicalPosition {
                x: normalized_x,
                y: normalized_y,
            },
            controller,
            pixel_reader,
            app_state,
        }
    }

    fn get_clicked_position(&self) -> PhysicalPosition<f64> {
        PhysicalPosition {
            x: self.position.x - self.delta_position.x * self.controller.area_size.width as f64,
            y: self.position.y - self.delta_position.y * self.controller.area_size.height as f64,
        }
    }

    /// If self is over a possible cross-over origin, return it.
    pub fn get_xover_origin(&mut self) -> Option<XoverOrigin> {
        let element = self.get_element_under_cursor();
        let (nucl, d_id) = self
            .controller
            .data
            .borrow()
            .element_to_nucl(&element, true)?;
        let position = self
            .controller
            .data
            .borrow()
            .get_nucl_position(nucl, d_id)?;
        Some(XoverOrigin {
            scene_element: element,
            nucl,
            position,
        })
    }

    fn get_projection_on_axis(&self, axis: Axis<'_>) -> Option<isize> {
        self.controller.view.borrow().compute_projection_axis(
            axis,
            self.normalized_position.x,
            self.normalized_position.y,
            None,
            self.controller.stereography.is_some(),
        )
    }

    fn get_new_build_position(&mut self) -> Option<isize> {
        let builder = self.app_state.get_strand_builders().get(0)?;
        let element = self.get_element_under_cursor();
        let nucl_under_cursor = self
            .controller
            .data
            .borrow()
            .element_to_nucl(&element, false);

        nucl_under_cursor
            .map(|n| n.0.position)
            .or_else(|| self.get_projection_on_axis(builder.get_axis()))
    }
}

#[derive(Clone, Copy)]
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

    pub fn to_dragging_cursor<'a, S: AppState>(
        self,
        controller: &'a Controller<S>,
        pixel_reader: &'a mut ElementSelector,
        app_state: &'a S,
    ) -> DraggedCursor<'a, S> {
        DraggedCursor::from_click_cursor(
            self.clicked_position,
            self.current_position,
            controller,
            pixel_reader,
            app_state,
        )
    }
}

/// A object maping cursor movement to their consequences
pub(super) trait DraggingTransitionTable {
    /// The consequences of moving the cursor
    fn on_cursor_moved<S: AppState>(&mut self, cursor: DraggedCursor<'_, S>)
        -> Option<Consequence>;
    fn on_button_released(&self) -> Option<Consequence>;
    /// A description of the state that the controller automata is in
    fn description() -> &'static str;
    /// If not None, the cursor icon that should be used when the controller's automata is in this
    /// state
    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        None
    }
    fn on_enterring(&self) -> TransistionConsequence;
    fn on_leaving(&self) -> TransistionConsequence;
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
    fn move_cursor<'a, S: AppState>(
        &mut self,
        position: PhysicalPosition<f64>,
        controller: &'a Controller<S>,
        pixel_reader: &'a mut ElementSelector,
        app_state: &'a S,
    ) -> DraggedCursor<'a, S> {
        self.current_cursor_position = position;
        DraggedCursor::from_click_cursor(
            self.clicked_position,
            position,
            controller,
            pixel_reader,
            app_state,
        )
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
        pixel_reader: &mut ElementSelector,
        app_state: &S,
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
                let cursor = self.move_cursor(position, controller, pixel_reader, app_state);
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
        self.transition_table.on_enterring()
    }

    fn transition_from(&self, _controller: &Controller<S>) -> TransistionConsequence {
        self.transition_table.on_leaving()
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

    fn on_cursor_moved<S: AppState>(
        &mut self,
        cursor: DraggedCursor<'_, S>,
    ) -> Option<Consequence> {
        Some(Consequence::CameraTranslated(
            cursor.delta_position.x,
            cursor.delta_position.y,
        ))
    }

    fn on_button_released(&self) -> Option<Consequence> {
        Some(Consequence::MovementEnded)
    }

    fn on_enterring(&self) -> TransistionConsequence {
        TransistionConsequence::InitCameraMovement
    }

    fn on_leaving(&self) -> TransistionConsequence {
        TransistionConsequence::EndCameraMovement
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

    fn on_cursor_moved<S: AppState>(
        &mut self,
        cursor: DraggedCursor<'_, S>,
    ) -> Option<Consequence> {
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

    fn on_enterring(&self) -> TransistionConsequence {
        TransistionConsequence::InitCameraMovement
    }

    fn on_leaving(&self) -> TransistionConsequence {
        TransistionConsequence::EndCameraMovement
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

    fn on_cursor_moved<S: AppState>(
        &mut self,
        cursor: DraggedCursor<'_, S>,
    ) -> Option<Consequence> {
        Some(Consequence::Tilt(
            cursor.delta_position.x,
            cursor.delta_position.y,
        ))
    }

    fn on_button_released(&self) -> Option<Consequence> {
        Some(Consequence::MovementEnded)
    }

    fn on_enterring(&self) -> TransistionConsequence {
        TransistionConsequence::InitCameraMovement
    }

    fn on_leaving(&self) -> TransistionConsequence {
        TransistionConsequence::EndCameraMovement
    }

    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::ColResize)
    }
}

dragging_state_constructor! {tilting_camera, TiltingCamera}

/// The user is making a cross-over
///
/// Cursor movement set the xover target
pub(super) struct MakingXover {
    /// The origin of the cross-over beeing made
    origin: XoverOrigin,
    /// The element that is currently under the cursor
    target_element: Option<SceneElement>,
    /// The xover that will be attempted when releasing the button
    current_xover: Option<(Nucl, Nucl, usize)>,
    /// Weither the attempted xover should be automatically optimized
    magic_xover: bool,
}

impl DraggingTransitionTable for MakingXover {
    fn description() -> &'static str {
        "Making Xover"
    }

    fn on_cursor_moved<S: AppState>(
        &mut self,
        mut cursor: DraggedCursor<'_, S>,
    ) -> Option<Consequence> {
        let element = cursor.get_element_under_cursor();
        self.target_element = element.clone();
        let projected_position = cursor.get_projection_on_plane(self.origin.position);
        self.current_xover = cursor
            .controller
            .data
            .borrow()
            .attempt_xover(&self.origin.scene_element, &self.target_element);
        self.magic_xover = cursor.controller.current_modifiers.shift();
        Some(Consequence::MoveFreeXover(element, projected_position))
    }

    fn on_button_released(&self) -> Option<Consequence> {
        if let Some((source, target, design_id)) = self.current_xover.clone() {
            Some(Consequence::XoverAtempt(
                source,
                target,
                design_id,
                self.magic_xover,
            ))
        } else {
            Some(Consequence::EndFreeXover)
        }
    }

    fn on_enterring(&self) -> TransistionConsequence {
        TransistionConsequence::InitFreeXover(self.origin.nucl, 0, self.origin.position)
    }

    fn on_leaving(&self) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn cursor() -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::Grabbing)
    }
}

/// The element that was clicked on to enter the MakingXover state
#[derive(Clone)]
pub(super) struct XoverOrigin {
    scene_element: Option<SceneElement>,
    position: Vec3,
    nucl: Nucl,
}

pub(super) fn making_xover(
    click_info: ClickInfo,
    origin: XoverOrigin,
) -> DraggingState<MakingXover> {
    let transition_table = MakingXover {
        magic_xover: false,
        target_element: None,
        current_xover: None,
        origin,
    };

    DraggingState {
        current_cursor_position: click_info.current_position,
        clicked_button: click_info.button,
        clicked_position: click_info.clicked_position,
        transition_table,
    }
}

/// The user is moving strand builders
pub(super) struct BuildingStrands {
    to_initialize: Option<Vec<Nucl>>,
}

impl DraggingTransitionTable for BuildingStrands {
    fn description() -> &'static str {
        "Moving strands builders"
    }

    fn on_leaving(&self) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn on_enterring(&self) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn on_cursor_moved<S: AppState>(
        &mut self,
        mut cursor: DraggedCursor<'_, S>,
    ) -> Option<Consequence> {
        if let Some(nucls) = self.to_initialize.take() {
            Some(Consequence::InitBuild(nucls))
        } else {
            cursor
                .get_new_build_position()
                .map(|p| Consequence::Building(p))
        }
    }

    fn on_button_released(&self) -> Option<Consequence> {
        Some(Consequence::BuildEnded)
    }
}

pub(super) fn building_strands(
    click_info: ClickInfo,
    nucls: Vec<Nucl>,
) -> DraggingState<BuildingStrands> {
    let transition_table = BuildingStrands {
        to_initialize: Some(nucls),
    };

    DraggingState {
        current_cursor_position: click_info.current_position,
        clicked_position: click_info.current_position,
        clicked_button: click_info.button,
        transition_table,
    }
}
