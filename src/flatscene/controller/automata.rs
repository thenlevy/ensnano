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
use super::super::data::ClickResult;
use super::super::view::CircleInstance;
use super::super::{FlatHelix, FlatNucl};
use super::*;
use std::time::Instant;

const WHEEL_RADIUS: f32 = 1.5;
use crate::consts::*;

pub struct Transition<S: AppState> {
    pub new_state: Option<Box<dyn ControllerState<S>>>,
    pub consequences: Consequence,
}

impl<S: AppState> Transition<S> {
    pub fn nothing() -> Self {
        Self {
            new_state: None,
            consequences: Consequence::Nothing,
        }
    }

    pub fn consequence(consequences: Consequence) -> Self {
        Self {
            new_state: None,
            consequences,
        }
    }
}

pub trait ControllerState<S: AppState> {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        state: &S,
    ) -> Transition<S>;

    #[allow(dead_code)]
    fn display(&self) -> String;

    fn transition_from(&self, controller: &Controller<S>) -> ();

    fn transition_to(&self, controller: &Controller<S>) -> ();

    fn check_timers(&mut self, _controller: &Controller<S>) -> Transition<S> {
        Transition::nothing()
    }
}

pub struct NormalState {
    pub mouse_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for NormalState {
    fn display(&self) -> String {
        String::from("Normal state")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } if controller.modifiers.alt() => Transition {
                new_state: Some(Box::new(MovingCamera {
                    mouse_position: self.mouse_position,
                    clicked_position_screen: self.mouse_position,
                    translation_pivots: vec![],
                    rotation_pivots: vec![],
                    clicked_button: MouseButton::Left,
                })),
                consequences: Consequence::Nothing,
            },
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                if let ClickResult::Nucl(nucl) = click_result {
                    if controller.data.borrow().can_make_auto_xover(nucl).is_some() {
                        Transition {
                            new_state: Some(Box::new(FollowingSuggestion {
                                nucl,
                                mouse_position: self.mouse_position,
                                double: false,
                                button: MouseButton::Right,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    } else {
                        Transition {
                            new_state: Some(Box::new(Cutting {
                                mouse_position: self.mouse_position,
                                nucl,
                                whole_strand: false,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                log::debug!(
                    "On left button pressed, pasting = {}",
                    app_state.is_pasting()
                );
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in normal mode"
                );*/
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                match click_result {
                    ClickResult::CircleWidget { .. } | ClickResult::Nothing
                        if app_state.is_pasting() =>
                    {
                        Transition {
                            new_state: Some(Box::new(Pasting {
                                nucl: None,
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nucl(nucl) if app_state.is_pasting() => Transition {
                        new_state: Some(Box::new(Pasting {
                            nucl: Some(nucl),
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    },
                    ClickResult::HelixHandle { h_id, handle } => Transition {
                        new_state: Some(Box::new(TranslatingHandle::new(h_id, handle, position))),
                        consequences: Consequence::Nothing,
                    },
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().is_suggested(&nucl)
                            && ctrl(&controller.modifiers) =>
                    {
                        Transition {
                            new_state: Some(Box::new(FollowingSuggestion {
                                nucl,
                                mouse_position: self.mouse_position,
                                double: controller.modifiers.shift(),
                                button: MouseButton::Left,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_make_auto_xover(nucl).is_some() =>
                    {
                        Transition {
                            new_state: Some(Box::new(FollowingSuggestion {
                                nucl,
                                mouse_position: self.mouse_position,
                                double: false,
                                button: MouseButton::Left,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nucl(nucl) => {
                        if controller.action_mode == ActionMode::Cut {
                            Transition {
                                new_state: Some(Box::new(Cutting {
                                    nucl,
                                    mouse_position: self.mouse_position,
                                    whole_strand: controller.modifiers.shift(),
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let _stick = if let ActionMode::Build(b) = controller.action_mode {
                                b
                            } else {
                                false
                            };
                            if controller.data.borrow().can_start_builder_at(nucl) {
                                if !controller.data.borrow().has_nucl(nucl) {
                                    // If the builder is not on an existing strand, we transition
                                    // directly to building state
                                    Transition {
                                        new_state: Some(Box::new(Building {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            can_attach: false,
                                        })),
                                        consequences: Consequence::InitBuilding(nucl),
                                    }
                                } else {
                                    Transition {
                                        new_state: Some(Box::new(InitBuilding {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            end: controller.data.borrow().is_strand_end(nucl),
                                        })),
                                        consequences: Consequence::InitBuilding(nucl),
                                    }
                                }
                            } else if let Some(attachement) =
                                controller.data.borrow().attachable_neighbour(nucl)
                            {
                                Transition {
                                    new_state: Some(Box::new(InitAttachement {
                                        mouse_position: self.mouse_position,
                                        from: nucl,
                                        to: attachement,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            } else if controller.data.borrow().has_nucl(nucl)
                                && controller.data.borrow().is_xover_end(&nucl).is_none()
                            {
                                Transition {
                                    new_state: Some(Box::new(AddOrXover {
                                        mouse_position: self.mouse_position,
                                        nucl,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(DraggingSelection {
                                        mouse_position: self.mouse_position,
                                        fixed_corner: self.mouse_position,
                                        adding: false,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot }
                        if ctrl(&controller.modifiers) =>
                    {
                        Transition {
                            new_state: Some(Box::new(FlipVisibility {
                                mouse_position: self.mouse_position,
                                helix: translation_pivot.helix,
                                apply_to_other: controller.modifiers.alt(),
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot }
                        if controller.modifiers.alt() =>
                    {
                        Transition {
                            new_state: Some(Box::new(FlipGroup {
                                mouse_position: self.mouse_position,
                                helix: translation_pivot.helix,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot } => {
                        if controller.action_mode == ActionMode::Cut {
                            Transition {
                                new_state: Some(Box::new(RmHelix {
                                    mouse_position: self.mouse_position,
                                    helix: translation_pivot.helix,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let clicked =
                                controller.get_camera(position.y).borrow().screen_to_world(
                                    self.mouse_position.x as f32,
                                    self.mouse_position.y as f32,
                                );
                            let selection = if controller.modifiers.shift() {
                                controller.data.borrow_mut().add_helix_selection(
                                    click_result,
                                    &controller.get_camera(position.y),
                                    app_state,
                                )
                            } else {
                                controller.data.borrow_mut().set_helix_selection(
                                    click_result,
                                    &controller.get_camera(position.y),
                                    app_state,
                                )
                            };
                            Transition {
                                new_state: Some(Box::new(Translating {
                                    mouse_position: self.mouse_position,
                                    world_clicked: clicked.into(),
                                    translation_pivots: selection.translation_pivots,
                                })),
                                consequences: Consequence::SelectionChanged(
                                    selection.new_selection,
                                ),
                            }
                        }
                    }
                    ClickResult::Nothing => Transition {
                        new_state: Some(Box::new(DraggingSelection {
                            mouse_position: self.mouse_position,
                            fixed_corner: self.mouse_position,
                            adding: false,
                        })),
                        consequences: Consequence::Nothing,
                    },
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Middle,
                state: ElementState::Pressed,
                ..
            } => Transition {
                new_state: Some(Box::new(MovingCamera {
                    mouse_position: self.mouse_position,
                    clicked_position_screen: self.mouse_position,
                    translation_pivots: vec![],
                    rotation_pivots: vec![],
                    clicked_button: MouseButton::Middle,
                })),
                consequences: Consequence::Nothing,
            },
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let candidate_helix =
                    if let ClickResult::CircleWidget { translation_pivot } = click_result {
                        Some(translation_pivot.helix)
                    } else {
                        None
                    };
                let pivot_opt = if let ClickResult::Nucl(nucl) = click_result {
                    Some(nucl)
                } else {
                    None
                };
                if let Some(helix) = candidate_helix {
                    Transition::consequence(Consequence::NewHelixCandidate(helix))
                } else {
                    Transition::consequence(Consequence::NewCandidate(pivot_opt))
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_to(&self, controller: &Controller<S>) {
        controller.data.borrow_mut().set_free_end(None);
    }

    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }
}

pub struct Translating {
    mouse_position: PhysicalPosition<f64>,
    world_clicked: Vec2,
    translation_pivots: Vec<FlatNucl>,
}

impl<S: AppState> ControllerState<S> for Translating {
    fn display(&self) -> String {
        String::from("Translating state")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                let mut translation_pivots = vec![];
                let mut rotation_pivots = vec![];
                for pivot in self.translation_pivots.iter() {
                    if let Some(rotation_pivot) = controller
                        .data
                        .borrow()
                        .get_rotation_pivot(pivot.helix.flat, &controller.get_camera(position.y))
                    {
                        translation_pivots.push(pivot.clone());
                        rotation_pivots.push(rotation_pivot);
                    }
                }

                if rotation_pivots.len() > 0 {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            translation_pivots,
                            rotation_pivots,
                        })),
                        consequences: Consequence::Helix2DMvmtEnded,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Helix2DMvmtEnded,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(position.x as f32, position.y as f32);
                /*
                for pivot in self.translation_pivots.iter() {
                    controller
                        .data
                        .borrow_mut()
                        .snap_helix(*pivot, Vec2::new(x, y) - self.world_clicked);
                }*/
                Transition::consequence(Consequence::Snap {
                    pivots: self.translation_pivots.clone(),
                    translation: Vec2::new(x, y) - self.world_clicked,
                })
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, controller: &Controller<S>) {
        controller.data.borrow_mut().end_movement()
    }

    fn transition_to(&self, controller: &Controller<S>) {
        let helices = self.translation_pivots.iter().map(|p| p.helix).collect();
        controller.data.borrow_mut().set_selected_helices(helices)
    }
}

pub struct MovingCamera {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_screen: PhysicalPosition<f64>,
    translation_pivots: Vec<FlatNucl>,
    rotation_pivots: Vec<Vec2>,
    clicked_button: MouseButton,
}

impl<S: AppState> ControllerState<S> for MovingCamera {
    fn display(&self) -> String {
        String::from("Moving camera")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if *button == self.clicked_button => {
                if self.rotation_pivots.len() > 0 {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            translation_pivots: self.translation_pivots.clone(),
                            rotation_pivots: self.rotation_pivots.clone(),
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let mouse_dx = (position.x as f32 - self.clicked_position_screen.x as f32)
                    / controller.area_size.width as f32;
                let mouse_dy = (position.y as f32 - self.clicked_position_screen.y as f32)
                    / controller.get_height() as f32;
                let (x, y) = controller
                    .get_camera(self.clicked_position_screen.y)
                    .borrow_mut()
                    .process_mouse(mouse_dx, mouse_dy);
                if let Some(other_camera) = controller
                    .get_other_camera(self.clicked_position_screen.y)
                    .filter(|_| controller.modifiers.shift())
                {
                    other_camera.borrow_mut().translate_by_vec(x, y);
                }

                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, controller: &Controller<S>) {
        controller.end_movement();
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }
}

pub struct ReleasedPivot {
    pub mouse_position: PhysicalPosition<f64>,
    pub translation_pivots: Vec<FlatNucl>,
    pub rotation_pivots: Vec<Vec2>,
}

impl<S: AppState> ControllerState<S> for ReleasedPivot {
    fn transition_to(&self, controller: &Controller<S>) {
        let helices = self.translation_pivots.iter().map(|p| p.helix).collect();
        controller.data.borrow_mut().set_selected_helices(helices);

        let wheels = self
            .rotation_pivots
            .iter()
            .map(|p| CircleInstance::new(*p, WHEEL_RADIUS, -1, CIRCLE2D_GREY))
            .collect();
        controller.view.borrow_mut().set_wheels(wheels);
    }

    fn transition_from(&self, controller: &Controller<S>) {
        controller.view.borrow_mut().set_wheels(vec![]);
    }

    fn display(&self) -> String {
        String::from("Released Pivot")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        if app_state.is_pasting() {
            return Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: self.mouse_position,
                })),
                consequences: Consequence::Nothing,
            };
        }
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } if controller.modifiers.alt() => Transition {
                new_state: Some(Box::new(MovingCamera {
                    mouse_position: self.mouse_position,
                    clicked_position_screen: self.mouse_position,
                    translation_pivots: vec![],
                    rotation_pivots: vec![],
                    clicked_button: MouseButton::Left,
                })),
                consequences: Consequence::Nothing,
            },
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                match click_result {
                    ClickResult::CircleWidget { translation_pivot }
                        if ctrl(&controller.modifiers) =>
                    {
                        Transition {
                            new_state: Some(Box::new(FlipVisibility {
                                mouse_position: self.mouse_position,
                                helix: translation_pivot.helix,
                                apply_to_other: controller.modifiers.alt(),
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { .. } if controller.modifiers.shift() => {
                        Transition {
                            new_state: Some(Box::new(AddCirclePivot {
                                translation_pivots: self.translation_pivots.clone(),
                                rotation_pivots: self.rotation_pivots.clone(),
                                mouse_position: self.mouse_position,
                                click_result,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::HelixHandle { h_id, handle } => Transition {
                        new_state: Some(Box::new(TranslatingHandle::new(h_id, handle, position))),
                        consequences: Consequence::Nothing,
                    },
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_make_auto_xover(nucl).is_some() =>
                    {
                        Transition {
                            new_state: Some(Box::new(FollowingSuggestion {
                                nucl,
                                mouse_position: self.mouse_position,
                                double: false,
                                button: MouseButton::Left,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nucl(nucl) => {
                        if controller.action_mode == ActionMode::Cut {
                            Transition {
                                new_state: Some(Box::new(Cutting {
                                    nucl,
                                    mouse_position: self.mouse_position,
                                    whole_strand: controller.modifiers.shift(),
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let _stick = if let ActionMode::Build(b) = controller.action_mode {
                                b
                            } else {
                                false
                            };
                            if controller.data.borrow().can_start_builder_at(nucl) {
                                if !controller.data.borrow().has_nucl(nucl) {
                                    // If the builder is not on an existing strand, we transition
                                    // directly to building state
                                    Transition {
                                        new_state: Some(Box::new(Building {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            can_attach: false,
                                        })),
                                        consequences: Consequence::InitBuilding(nucl),
                                    }
                                } else {
                                    Transition {
                                        new_state: Some(Box::new(InitBuilding {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            end: controller.data.borrow().is_strand_end(nucl),
                                        })),
                                        consequences: Consequence::InitBuilding(nucl),
                                    }
                                }
                            } else if let Some(attachement) =
                                controller.data.borrow().attachable_neighbour(nucl)
                            {
                                Transition {
                                    new_state: Some(Box::new(InitAttachement {
                                        mouse_position: self.mouse_position,
                                        from: nucl,
                                        to: attachement,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            } else if controller.data.borrow().has_nucl(nucl)
                                && controller.data.borrow().is_xover_end(&nucl).is_none()
                            {
                                Transition {
                                    new_state: Some(Box::new(AddOrXover {
                                        mouse_position: self.mouse_position,
                                        nucl,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(DraggingSelection {
                                        mouse_position: self.mouse_position,
                                        fixed_corner: self.mouse_position,
                                        adding: false,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot }
                        if self.translation_pivots.contains(&translation_pivot) =>
                    {
                        let clicked = controller.get_camera(position.y).borrow().screen_to_world(
                            self.mouse_position.x as f32,
                            self.mouse_position.y as f32,
                        );
                        Transition {
                            new_state: Some(Box::new(InitHelixTranslation {
                                translation_pivots: self.translation_pivots.clone(),
                                world_clicked: clicked.into(),
                                clicked_position_screen: self.mouse_position,
                                mouse_position: self.mouse_position,
                                click_result,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { .. } => {
                        // Clicked on an other circle
                        let clicked = controller.get_camera(position.y).borrow().screen_to_world(
                            self.mouse_position.x as f32,
                            self.mouse_position.y as f32,
                        );
                        let selection = controller.data.borrow_mut().set_helix_selection(
                            click_result,
                            &controller.get_camera(position.y),
                            app_state,
                        );
                        Transition {
                            new_state: Some(Box::new(Translating {
                                translation_pivots: selection.translation_pivots,
                                world_clicked: clicked.into(),
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::SelectionChanged(selection.new_selection),
                        }
                    }
                    ClickResult::Nothing => Transition {
                        new_state: Some(Box::new(LeavingPivot {
                            clicked_position_screen: self.mouse_position,
                            mouse_position: self.mouse_position,
                            translation_pivots: self.translation_pivots.clone(),
                            rotation_pivots: self.rotation_pivots.clone(),
                            shift: controller.modifiers.shift(),
                        })),
                        consequences: Consequence::Nothing,
                    },
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                if self.translation_pivots.len() > 0 {
                    Transition {
                        new_state: Some(Box::new(Rotating::new(
                            self.translation_pivots.clone(),
                            self.rotation_pivots.clone(),
                            self.mouse_position,
                            self.mouse_position,
                        ))),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    let (x, y) = controller.get_camera(position.y).borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    let click_result = controller.data.borrow().get_click(
                        x,
                        y,
                        &controller.get_camera(position.y),
                    );
                    if let ClickResult::Nucl(nucl) = click_result {
                        if controller.data.borrow().can_make_auto_xover(nucl).is_some() {
                            Transition {
                                new_state: Some(Box::new(FollowingSuggestion {
                                    nucl,
                                    mouse_position: self.mouse_position,
                                    double: false,
                                    button: MouseButton::Right,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            Transition {
                                new_state: Some(Box::new(Cutting {
                                    mouse_position: self.mouse_position,
                                    nucl,
                                    whole_strand: false,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        }
                    } else {
                        Transition::nothing()
                    }
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Middle,
                state,
                ..
            } => {
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(MovingCamera {
                        mouse_position: self.mouse_position,
                        clicked_position_screen: self.mouse_position,
                        translation_pivots: self.translation_pivots.clone(),
                        rotation_pivots: self.rotation_pivots.clone(),
                        clicked_button: MouseButton::Middle,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let candidate_helix =
                    if let ClickResult::CircleWidget { translation_pivot } = click_result {
                        Some(translation_pivot.helix)
                    } else {
                        None
                    };
                let pivot_opt = if let ClickResult::Nucl(nucl) = click_result {
                    Some(nucl)
                } else {
                    None
                };
                if let Some(helix) = candidate_helix {
                    Transition::consequence(Consequence::NewHelixCandidate(helix))
                } else {
                    Transition::consequence(Consequence::NewCandidate(pivot_opt))
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// This state in entered when use user has clicked after realising a pivot. If the user moves
/// their mouse, go in moving camera mode without unselecting the helix. If the user release their
/// click without moving their mouse, clear selection
pub struct LeavingPivot {
    translation_pivots: Vec<FlatNucl>,
    rotation_pivots: Vec<Vec2>,
    clicked_position_screen: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
    shift: bool,
}

impl<S: AppState> ControllerState<S> for LeavingPivot {
    fn transition_to(&self, controller: &Controller<S>) {
        let wheels = self
            .rotation_pivots
            .iter()
            .map(|p| CircleInstance::new(*p, WHEEL_RADIUS, -1, CIRCLE2D_GREY))
            .collect();
        controller.view.borrow_mut().set_wheels(wheels);
    }

    fn transition_from(&self, controller: &Controller<S>) {
        controller.view.borrow_mut().set_wheels(vec![]);
    }

    fn display(&self) -> String {
        String::from("Leaving Pivot")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in LeavingPivot state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::ClearSelection,
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released right mouse button in ReleasedPivot state"
                );*/
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                if self.translation_pivots.len() > 0 {
                    Transition {
                        new_state: Some(Box::new(Rotating::new(
                            self.translation_pivots.clone(),
                            self.rotation_pivots.clone(),
                            self.mouse_position,
                            self.mouse_position,
                        ))),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(self.clicked_position_screen, self.mouse_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(DraggingSelection {
                            mouse_position: self.mouse_position,
                            fixed_corner: self.clicked_position_screen,
                            adding: self.shift,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

pub struct Rotating {
    translation_pivots: Vec<FlatNucl>,
    rotation_pivots: Vec<Vec2>,
    clicked_position_screen: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
    pivot_center: Vec2,
    cutting: bool,
}

impl Rotating {
    pub fn new(
        translation_pivots: Vec<FlatNucl>,
        rotation_pivots: Vec<Vec2>,
        clicked_position_screen: PhysicalPosition<f64>,
        mouse_position: PhysicalPosition<f64>,
    ) -> Self {
        let mut min_x = rotation_pivots[0].x;
        let mut max_x = rotation_pivots[0].x;
        let mut min_y = rotation_pivots[0].y;
        let mut max_y = rotation_pivots[0].y;
        for p in rotation_pivots.iter() {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        Self {
            translation_pivots,
            rotation_pivots,
            clicked_position_screen,
            mouse_position,
            pivot_center: Vec2::new((min_x + max_x) / 2., (min_y + max_y) / 2.),
            cutting: true,
        }
    }
}

impl<S: AppState> ControllerState<S> for Rotating {
    fn transition_to(&self, controller: &Controller<S>) {
        let helices = self.translation_pivots.iter().map(|p| p.helix).collect();
        controller.data.borrow_mut().set_selected_helices(helices);

        let wheels = self
            .rotation_pivots
            .iter()
            .map(|p| CircleInstance::new(*p, WHEEL_RADIUS, -1, CIRCLE2D_GREY))
            .collect();
        controller.view.borrow_mut().set_wheels(wheels);
    }

    fn transition_from(&self, controller: &Controller<S>) {
        controller.data.borrow_mut().end_movement();
        controller.view.borrow_mut().set_wheels(vec![]);
    }

    fn display(&self) -> String {
        String::from("Rotating")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Released,
                ..
            } => {
                if self.cutting {
                    let (x, y) = controller.get_camera(position.y).borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    let click_result = controller.data.borrow().get_click(
                        x,
                        y,
                        &controller.get_camera(position.y),
                    );
                    let consequences = if let ClickResult::Nucl(nucl) = click_result {
                        if let Some(nucl) = controller.data.borrow().can_make_auto_xover(nucl) {
                            Consequence::FollowingSuggestion(nucl, false)
                        } else if let Some(attachement) =
                            controller.data.borrow().attachable_neighbour(nucl)
                        {
                            Consequence::Xover(nucl, attachement)
                        } else {
                            Consequence::Cut(nucl)
                        }
                    } else {
                        Consequence::Nothing
                    };
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            translation_pivots: self.translation_pivots.clone(),
                            rotation_pivots: self.rotation_pivots.clone(),
                            mouse_position: self.mouse_position,
                        })),
                        consequences,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            translation_pivots: self.translation_pivots.clone(),
                            rotation_pivots: self.rotation_pivots.clone(),
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(self.clicked_position_screen, position) > 5. {
                    self.cutting = false;
                }
                let angle = {
                    let (x, y) = controller
                        .get_camera(self.clicked_position_screen.y)
                        .borrow()
                        .screen_to_world(position.x as f32, position.y as f32);
                    let (old_x, old_y) = controller
                        .get_camera(self.clicked_position_screen.y)
                        .borrow()
                        .screen_to_world(
                            self.clicked_position_screen.x as f32,
                            self.clicked_position_screen.y as f32,
                        );
                    (y - self.pivot_center.y).atan2(x - self.pivot_center.x)
                        - (old_y - self.pivot_center.y).atan2(old_x - self.pivot_center.x)
                };
                if !self.cutting {
                    let helices = self.translation_pivots.iter().map(|n| n.helix).collect();
                    Transition::consequence(Consequence::Rotation {
                        helices,
                        center: self.pivot_center,
                        angle,
                    })
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct AddOrXover {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
}

impl<S: AppState> ControllerState<S> for AddOrXover {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Add or Xover")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));

                Transition {
                    new_state: Some(Box::new(DoubleClicking {
                        mouse_position: self.mouse_position,
                        clicked_time: Instant::now(),
                        click_result,
                        clicked_position: position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                match click_result {
                    ClickResult::Nucl(nucl2) if nucl2 == self.nucl => Transition::nothing(),
                    _ => {
                        let strand_id = controller.data.borrow().get_strand_id(self.nucl).unwrap();
                        Transition {
                            new_state: Some(Box::new(MovingFreeEnd {
                                mouse_position: self.mouse_position,
                                from: self.nucl,
                                prime3: true,
                                strand_id,
                            })),
                            consequences: Consequence::CutFreeEnd(
                                self.nucl,
                                Some(FreeEnd {
                                    strand_id,
                                    point: Vec2::new(x, y),
                                    prime3: true,
                                    candidates: vec![self.nucl],
                                }),
                            ),
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// This state is entered when clicking on a strand extremity that has a neighbouring
/// strand. If the cursor is released on the same nucleotide, the two neighbouring strands
/// are merged.
struct InitAttachement {
    mouse_position: PhysicalPosition<f64>,
    from: FlatNucl,
    to: FlatNucl,
}

impl<S: AppState> ControllerState<S> for InitAttachement {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Init Attachement")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Init Building state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                log::debug!("transition from {:?} to {:?}", self.from, self.to);
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Xover(self.from, self.to),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                match click_result {
                    ClickResult::Nucl(nucl2) if nucl2 == self.from => Transition::nothing(),
                    _ => {
                        let strand_id = controller.data.borrow().get_strand_id(self.from).unwrap();
                        Transition {
                            new_state: Some(Box::new(MovingFreeEnd {
                                mouse_position: self.mouse_position,
                                from: self.from,
                                prime3: controller.data.borrow().is_strand_end(self.from).unwrap(),
                                strand_id,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// The state in which the controller is just after creating strand builders.
/// From there depending on which mouse movement the user make, the controller will transition to
/// an other state. A transition is triggered when the cursor leaves the square in which the user
/// clicked to initiate strand building.
///
/// * If the cursor is moved on a neighbour nucleotide, the controller transition to Building
/// * If the cursor is moved out of the helix, the strand is cut and the controller transition to
/// CrossCut state.
///
/// It is possible to reach this state with no strand builder being active, in this case moving the
/// cursor will have no effet and the controller will transition to NormalState when the left mouse
/// button is released.
struct InitBuilding {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
    end: Option<bool>,
}

impl<S: AppState> ControllerState<S> for InitBuilding {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Init Building")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(DoubleClicking {
                    clicked_time: Instant::now(),
                    click_result: ClickResult::Nucl(self.nucl),
                    mouse_position: self.mouse_position,
                    clicked_position: position,
                })),
                consequences: Consequence::Built,
            },
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if let Some(builder) = app_state.get_strand_builders().get(0) {
                    let (x, y) = controller.get_camera(position.y).borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    let click_result = controller.data.borrow().get_click(
                        x,
                        y,
                        &controller.get_camera(position.y),
                    );
                    match click_result {
                        ClickResult::Nucl(FlatNucl {
                            helix,
                            position,
                            forward,
                        }) if helix == self.nucl.helix && forward == self.nucl.forward => {
                            if position != self.nucl.position {
                                //self.builder.move_to(position);
                                controller.data.borrow_mut().notify_update();
                                Transition {
                                    new_state: Some(Box::new(Building {
                                        mouse_position: self.mouse_position,
                                        nucl: self.nucl,
                                        can_attach: true,
                                    })),
                                    consequences: Consequence::MoveBuilders(position),
                                }
                            } else {
                                Transition::nothing()
                            }
                        }
                        ClickResult::Nucl(nucl)
                            if controller.data.borrow().can_cross_to(self.nucl, nucl) =>
                        {
                            //self.builder.reset();
                            controller.data.borrow_mut().notify_update();
                            Transition {
                                new_state: Some(Box::new(Crossing {
                                    mouse_position: self.mouse_position,
                                    from: self.nucl,
                                    to: nucl,
                                    strand_id: builder.get_strand_id(),
                                    from3prime: self.end.expect("from3prime"),
                                    cut: false,
                                })),
                                consequences: Consequence::FreeEnd(self.end.map(|b| FreeEnd {
                                    strand_id: builder.get_strand_id(),
                                    point: Vec2::new(x, y),
                                    prime3: b,
                                    candidates: vec![self.nucl, nucl],
                                })),
                            }
                        }
                        _ => {
                            if let Some(prime3) = self.end {
                                Transition {
                                    new_state: Some(Box::new(MovingFreeEnd {
                                        mouse_position: self.mouse_position,
                                        from: self.nucl,
                                        prime3,
                                        strand_id: builder.get_strand_id(),
                                    })),
                                    consequences: Consequence::FreeEnd(Some(FreeEnd {
                                        strand_id: builder.get_strand_id(),
                                        point: Vec2::new(x, y),
                                        prime3,
                                        candidates: vec![self.nucl],
                                    })),
                                }
                            } else {
                                let prime3 = controller
                                    .data
                                    .borrow()
                                    .is_xover_end(&self.nucl)
                                    .unwrap_or(true);
                                Transition {
                                    new_state: Some(Box::new(MovingFreeEnd {
                                        mouse_position: self.mouse_position,
                                        from: self.nucl,
                                        prime3,
                                        strand_id: builder.get_strand_id(),
                                    })),
                                    consequences: Consequence::CutFreeEnd(
                                        self.nucl,
                                        Some(FreeEnd {
                                            strand_id: builder.get_strand_id(),
                                            point: Vec2::new(x, y),
                                            prime3,
                                            candidates: vec![self.nucl],
                                        }),
                                    ),
                                }
                            }
                        }
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct MovingFreeEnd {
    mouse_position: PhysicalPosition<f64>,
    from: FlatNucl,
    strand_id: usize,
    prime3: bool,
}

impl<S: AppState> ControllerState<S> for MovingFreeEnd {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Moving Free End")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Moving Free End state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::FreeEnd(None),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                match click_result {
                    ClickResult::Nucl(nucl) if nucl == self.from => Transition::nothing(),
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_cross_to(self.from, nucl) =>
                    {
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.from,
                                to: nucl,
                                from3prime: self.prime3,
                                strand_id: self.strand_id,
                                cut: false,
                            })),
                            consequences: Consequence::FreeEnd(Some(FreeEnd {
                                strand_id: self.strand_id,
                                point: Vec2::new(x, y),
                                prime3: self.prime3,
                                candidates: vec![self.from, nucl],
                            })),
                        }
                    }
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_cut_cross_to(self.from, nucl) =>
                    {
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.from,
                                to: nucl,
                                from3prime: self.prime3,
                                strand_id: self.strand_id,
                                cut: true,
                            })),
                            consequences: Consequence::FreeEnd(Some(FreeEnd {
                                strand_id: self.strand_id,
                                point: Vec2::new(x, y),
                                prime3: self.prime3,
                                candidates: vec![self.from, nucl],
                            })),
                        }
                    }
                    _ => Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                        strand_id: self.strand_id,
                        point: Vec2::new(x, y),
                        prime3: self.prime3,
                        candidates: vec![self.from],
                    }))),
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// Elongating or shortening strands.
struct Building {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
    can_attach: bool,
}

impl<S: AppState> ControllerState<S> for Building {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Building")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Building state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                if self.can_attach {
                    if let Some(attachement) =
                        controller.data.borrow().attachable_neighbour(self.nucl)
                    {
                        return Transition {
                            new_state: Some(Box::new(NormalState {
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Xover(self.nucl, attachement),
                        };
                    }
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Built,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click_unbounded_helix(x, y, self.nucl.helix);
                if nucl != self.nucl {
                    self.can_attach = false;
                }
                match nucl {
                    FlatNucl {
                        helix, position, ..
                    } if helix == self.nucl.helix => {
                        controller.data.borrow_mut().notify_update();
                        Transition::consequence(Consequence::MoveBuilders(position))
                    }
                    _ => Transition::nothing(),
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

pub struct Crossing {
    mouse_position: PhysicalPosition<f64>,
    from: FlatNucl,
    to: FlatNucl,
    from3prime: bool,
    strand_id: usize,
    cut: bool,
}

impl<S: AppState> ControllerState<S> for Crossing {
    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Crossing")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Crossing state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: if self.cut {
                        Consequence::CutCross(self.from, self.to)
                    } else {
                        Consequence::Xover(self.from, self.to)
                    },
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                if nucl != ClickResult::Nucl(self.to) {
                    Transition {
                        new_state: Some(Box::new(MovingFreeEnd {
                            mouse_position: self.mouse_position,
                            from: self.from,
                            prime3: self.from3prime,
                            strand_id: self.strand_id,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                        strand_id: self.strand_id,
                        point: Vec2::new(x, y),
                        prime3: self.from3prime,
                        candidates: vec![self.from, self.to],
                    })))
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct Cutting {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
    whole_strand: bool,
}

impl<S: AppState> ControllerState<S> for Cutting {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Cutting")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Released,
                ..
            } => {
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let attachement = if let ClickResult::Nucl(nucl) = nucl {
                    Some(nucl).zip(controller.data.borrow().attachable_neighbour(nucl))
                } else {
                    None
                };
                if let Some(attachement) = attachement {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Xover(attachement.0, attachement.1),
                    }
                } else {
                    let consequences = if nucl == ClickResult::Nucl(self.nucl) {
                        if self.whole_strand {
                            Consequence::RmStrand(self.nucl)
                        } else {
                            Consequence::Cut(self.nucl)
                        }
                    } else {
                        Consequence::Nothing
                    };
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct RmHelix {
    mouse_position: PhysicalPosition<f64>,
    helix: FlatHelix,
}

impl<S: AppState> ControllerState<S> for RmHelix {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("RmHelix")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if let ClickResult::CircleWidget { translation_pivot } = nucl {
                    if translation_pivot.helix == self.helix {
                        Consequence::RmHelix(self.helix)
                    } else {
                        Consequence::Nothing
                    }
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct FlipGroup {
    mouse_position: PhysicalPosition<f64>,
    helix: FlatHelix,
}

impl<S: AppState> ControllerState<S> for FlipGroup {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("FlipGroup")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if let ClickResult::CircleWidget { translation_pivot } = nucl {
                    if translation_pivot.helix == self.helix {
                        Consequence::FlipGroup(self.helix)
                    } else {
                        Consequence::Nothing
                    }
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct FlipVisibility {
    mouse_position: PhysicalPosition<f64>,
    helix: FlatHelix,
    apply_to_other: bool,
}

impl<S: AppState> ControllerState<S> for FlipVisibility {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("RmHelix")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if let ClickResult::CircleWidget { translation_pivot } = nucl {
                    if translation_pivot.helix == self.helix {
                        Consequence::FlipVisibility(self.helix, self.apply_to_other)
                    } else {
                        Consequence::Nothing
                    }
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct FollowingSuggestion {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
    double: bool,
    button: MouseButton,
}

impl<S: AppState> ControllerState<S> for FollowingSuggestion {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Following Suggestion")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if button == &self.button => {
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if let ClickResult::Nucl(nucl) = nucl {
                    if nucl == self.nucl {
                        Consequence::FollowingSuggestion(self.nucl, self.double)
                    } else {
                        Consequence::Nothing
                    }
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                if position_difference(self.mouse_position, position) >= 3. {
                    if controller.data.borrow().can_start_builder_at(self.nucl) {
                        if !controller.data.borrow().has_nucl(self.nucl) {
                            Transition {
                                new_state: Some(Box::new(Building {
                                    mouse_position: self.mouse_position,
                                    nucl: self.nucl,
                                    can_attach: false,
                                })),
                                consequences: Consequence::InitBuilding(self.nucl),
                            }
                        } else {
                            Transition {
                                new_state: Some(Box::new(InitBuilding {
                                    mouse_position: self.mouse_position,
                                    nucl: self.nucl,
                                    end: controller.data.borrow().is_strand_end(self.nucl),
                                })),
                                consequences: Consequence::InitBuilding(self.nucl),
                            }
                        }
                    } else {
                        Transition::nothing()
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct CenteringSuggestion {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
    bottom: bool,
}

impl<S: AppState> ControllerState<S> for CenteringSuggestion {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("CenteringSuggestion")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if let ClickResult::Nucl(nucl) = nucl {
                    if nucl == self.nucl {
                        let nucl = controller.data.borrow().get_best_suggestion(self.nucl);
                        Consequence::Centering(nucl.unwrap_or(self.nucl), !self.bottom)
                    } else {
                        Consequence::Nothing
                    }
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct Pasting {
    mouse_position: PhysicalPosition<f64>,
    nucl: Option<FlatNucl>,
}

impl<S: AppState> ControllerState<S> for Pasting {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Pasting")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if self.nucl.is_none() {
                    Consequence::PasteRequest(self.nucl)
                } else if nucl == ClickResult::Nucl(self.nucl.unwrap()) {
                    Consequence::PasteRequest(self.nucl)
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// The user is drawing a selection
struct DraggingSelection {
    pub mouse_position: PhysicalPosition<f64>,
    pub fixed_corner: PhysicalPosition<f64>,
    pub adding: bool,
}

impl<S: AppState> ControllerState<S> for DraggingSelection {
    fn display(&self) -> String {
        format!("Dragging Selection {}", self.adding)
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                let valid_rectangle = controller.is_bottom(self.fixed_corner.y)
                    == controller.is_bottom(self.mouse_position.y);
                let corner1_world = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.fixed_corner.x as f32, self.fixed_corner.y as f32);
                let corner2_world = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let rectangle_selection = if valid_rectangle {
                    Some(controller.data.borrow_mut().select_rectangle(
                        corner1_world.into(),
                        corner2_world.into(),
                        &controller.get_camera(position.y),
                        self.adding,
                        app_state,
                    ))
                } else {
                    None
                };
                if let Some(rectangle_selection) = rectangle_selection {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            translation_pivots: rectangle_selection.translation_pivots,
                            rotation_pivots: rectangle_selection.rotation_pivots,
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::ReleasedSelection(Some(
                            rectangle_selection.new_selection,
                        )),
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::ReleasedSelection(None),
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                if position.x < controller.area_size.width as f64
                    && position.x >= 0.
                    && position.y <= controller.area_size.height as f64
                    && position.y >= 0.
                {
                    self.mouse_position = position;
                }
                Transition::consequence(Consequence::DrawingSelection(
                    self.fixed_corner,
                    self.mouse_position,
                ))
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, controller: &Controller<S>) {
        controller.end_movement();
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }
}

struct AddClick {
    mouse_position: PhysicalPosition<f64>,
    click_result: ClickResult,
}

impl<S: AppState> ControllerState<S> for AddClick {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("AddClick")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                if let ClickResult::CircleWidget { .. } = click {
                    let selection = if controller.modifiers.shift() {
                        controller.data.borrow_mut().add_helix_selection(
                            click,
                            &controller.get_camera(position.y),
                            app_state,
                        )
                    } else {
                        controller.data.borrow_mut().set_helix_selection(
                            click,
                            &controller.get_camera(position.y),
                            app_state,
                        )
                    };
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: position,
                            translation_pivots: selection.translation_pivots,
                            rotation_pivots: selection.rotation_pivots,
                        })),
                        consequences: Consequence::SelectionChanged(selection.new_selection),
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(DoubleClicking {
                            mouse_position: self.mouse_position,
                            click_result: self.click_result.clone(),
                            clicked_time: Instant::now(),
                            clicked_position: position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct DoubleClicking {
    clicked_time: Instant,
    click_result: ClickResult,
    mouse_position: PhysicalPosition<f64>,
    clicked_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for DoubleClicking {
    fn check_timers(&mut self, controller: &Controller<S>) -> Transition<S> {
        let now = Instant::now();
        if (now - self.clicked_time).as_millis() > 250 {
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: self.mouse_position,
                })),
                consequences: Consequence::AddClick(
                    self.click_result.clone(),
                    controller.modifiers.shift(),
                ),
            }
        } else {
            Transition::nothing()
        }
    }

    fn display(&self) -> String {
        "Double Clicking".to_owned()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                let consequences = if click == self.click_result {
                    Consequence::DoubleClick(click)
                } else {
                    Consequence::Nothing
                };

                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(self.clicked_position, position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::AddClick(
                            self.click_result.clone(),
                            controller.modifiers.shift(),
                        ),
                    }
                } else {
                    Transition::nothing()
                }
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }
}

struct AddCirclePivot {
    mouse_position: PhysicalPosition<f64>,
    translation_pivots: Vec<FlatNucl>,
    rotation_pivots: Vec<Vec2>,
    click_result: ClickResult,
}

impl<S: AppState> ControllerState<S> for AddCirclePivot {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("AddCirclePivot")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y));
                if click == self.click_result {
                    let selection = if controller.modifiers.shift() {
                        controller.data.borrow_mut().add_helix_selection(
                            click,
                            &controller.get_camera(position.y),
                            app_state,
                        )
                    } else {
                        controller.data.borrow_mut().set_helix_selection(
                            click,
                            &controller.get_camera(position.y),
                            app_state,
                        )
                    };
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            translation_pivots: selection.translation_pivots,
                            rotation_pivots: selection.rotation_pivots,
                        })),
                        consequences: Consequence::SelectionChanged(selection.new_selection),
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            translation_pivots: self.translation_pivots.clone(),
                            rotation_pivots: self.rotation_pivots.clone(),
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct InitHelixTranslation {
    translation_pivots: Vec<FlatNucl>,
    clicked_position_screen: PhysicalPosition<f64>,
    world_clicked: Vec2,
    mouse_position: PhysicalPosition<f64>,
    click_result: ClickResult,
}

impl<S: AppState> ControllerState<S> for InitHelixTranslation {
    fn display(&self) -> String {
        String::from("Init Helix Translation")
    }

    fn transition_to(&self, _controller: &Controller<S>) -> () {}

    fn transition_from(&self, _controller: &Controller<S>) -> () {}

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in LeavingPivot state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let selection = controller.data.borrow_mut().set_helix_selection(
                    self.click_result.clone(),
                    &controller.get_camera(position.y),
                    app_state,
                );
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::SelectionChanged(selection.new_selection),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(self.clicked_position_screen, self.mouse_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(Translating {
                            mouse_position: self.mouse_position,
                            world_clicked: self.world_clicked,
                            translation_pivots: self.translation_pivots.clone(),
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct TranslatingHandle {
    h_id: FlatHelix,
    handle: super::super::data::HelixHandle,
    auto: bool,
    clicked_position_screen: PhysicalPosition<f64>,
}

impl TranslatingHandle {
    fn new(
        h_id: FlatHelix,
        handle: super::super::data::HelixHandle,
        clicked_position_screen: PhysicalPosition<f64>,
    ) -> Self {
        Self {
            h_id,
            handle,
            clicked_position_screen,
            auto: true,
        }
    }
}

impl<S: AppState> ControllerState<S> for TranslatingHandle {
    fn display(&self) -> String {
        String::from("Translating state")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                if self.auto {
                    controller
                        .data
                        .borrow_mut()
                        .auto_redim_helix(self.h_id, self.handle)
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                if position_difference(position, self.clicked_position_screen) > 5. {
                    self.auto = false;
                }
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(position.x as f32, position.y as f32);
                /*controller
                .data
                .borrow_mut()
                .translate_helix(Vec2::new(mouse_dx, mouse_dy));*/
                controller
                    .data
                    .borrow_mut()
                    .move_handle(self.h_id, self.handle, Vec2::new(x, y));
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .get_camera(position.y)
                    .borrow_mut()
                    .process_scroll(delta, position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

fn ctrl(modifiers: &ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.logo()
    } else {
        modifiers.ctrl()
    }
}
