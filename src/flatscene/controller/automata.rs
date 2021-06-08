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
use crate::design::StrandBuilder;
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
                Transition {
                    new_state: Some(Box::new(AddClick {
                        mouse_position: self.mouse_position,
                        click_result,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
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
                        if controller.pasting =>
                    {
                        Transition {
                            new_state: Some(Box::new(Pasting {
                                nucl: None,
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nucl(nucl) if controller.pasting => Transition {
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
                            let stick = if let ActionMode::Build(b) = controller.action_mode {
                                b
                            } else {
                                false
                            };
                            if let Some(builder) = controller.data.borrow().get_builder(nucl, stick)
                            {
                                if builder.created_de_novo() {
                                    Transition {
                                        new_state: Some(Box::new(Building {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            builder,
                                            can_attach: false,
                                        })),
                                        consequences: Consequence::NewCandidate(None),
                                    }
                                } else {
                                    Transition {
                                        new_state: Some(Box::new(InitBuilding {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            builder,
                                            end: controller.data.borrow().is_strand_end(nucl),
                                        })),
                                        consequences: Consequence::NewCandidate(None),
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
                                    new_state: Some(Box::new(InitCutting {
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
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
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
                    ClickResult::Nucl(nucl) if controller.data.borrow().is_suggested(&nucl) => {
                        Transition {
                            new_state: Some(Box::new(CenteringSuggestion {
                                nucl,
                                mouse_position: self.mouse_position,
                                bottom: controller.is_bottom(position.y),
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    _ => Transition::nothing(),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let pivot_opt = if let ClickResult::Nucl(nucl) =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y))
                {
                    Some(nucl)
                } else {
                    None
                };
                Transition::consequence(Consequence::NewCandidate(pivot_opt))
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
        controller.data.borrow_mut().set_selected_helices(vec![]);
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
                controller.data.borrow_mut().end_movement();
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
                let (x, y) = controller
                    .get_camera(position.y)
                    .borrow()
                    .screen_to_world(position.x as f32, position.y as f32);
                /*controller
                .data
                .borrow_mut()
                .translate_helix(Vec2::new(mouse_dx, mouse_dy));*/
                for pivot in self.translation_pivots.iter() {
                    controller
                        .data
                        .borrow_mut()
                        .snap_helix(*pivot, Vec2::new(x, y) - self.world_clicked);
                }
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
                controller
                    .get_camera(self.clicked_position_screen.y)
                    .borrow_mut()
                    .process_mouse(mouse_dx, mouse_dy);
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
            } if controller.modifiers.shift() => {
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
                    ClickResult::CircleWidget { .. } => {
                        // Clicked on an other circle
                        Transition {
                            new_state: Some(Box::new(AddClickPivots {
                                translation_pivots: self.translation_pivots.clone(),
                                rotation_pivots: self.rotation_pivots.clone(),
                                mouse_position: self.mouse_position,
                                click_result,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    click_result => Transition {
                        new_state: Some(Box::new(AddClick {
                            click_result,
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    },
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in ReleasedPivot state"
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
                            let stick = if let ActionMode::Build(b) = controller.action_mode {
                                b
                            } else {
                                false
                            };
                            if let Some(builder) = controller.data.borrow().get_builder(nucl, stick)
                            {
                                Transition {
                                    new_state: Some(Box::new(InitBuilding {
                                        mouse_position: self.mouse_position,
                                        nucl,
                                        builder,
                                        end: controller.data.borrow().is_strand_end(nucl),
                                    })),
                                    consequences: Consequence::NewCandidate(None),
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(DraggingSelection {
                                        mouse_position: self.mouse_position,
                                        fixed_corner: self.mouse_position,
                                        adding: controller.modifiers.shift(),
                                    })),
                                    consequences: Consequence::NewCandidate(None),
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
                            new_state: Some(Box::new(Translating {
                                translation_pivots: self.translation_pivots.clone(),
                                world_clicked: clicked.into(),
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot } => {
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
                    let (x, y) = controller.get_camera(position.y).borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    let click_result = controller.data.borrow().get_click(
                        x,
                        y,
                        &controller.get_camera(position.y),
                    );
                    Transition {
                        new_state: Some(Box::new(AddClick {
                            click_result,
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
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
                let pivot_opt = if let ClickResult::Nucl(nucl) =
                    controller
                        .data
                        .borrow()
                        .get_click(x, y, &controller.get_camera(position.y))
                {
                    Some(nucl)
                } else {
                    None
                };
                Transition::consequence(Consequence::NewCandidate(pivot_opt))
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
    selecting: bool,
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
            selecting: true,
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
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Released,
                ..
            } => {
                if self.selecting {
                    let (x, y) = controller.get_camera(position.y).borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    let click_result = controller.data.borrow().get_click(
                        x,
                        y,
                        &controller.get_camera(position.y),
                    );
                    let selection = if controller.modifiers.shift() {
                        controller.data.borrow_mut().add_helix_selection(
                            click_result.clone(),
                            &controller.get_camera(position.y),
                            app_state,
                        )
                    } else {
                        controller.data.borrow_mut().set_helix_selection(
                            click_result.clone(),
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
                    self.selecting = false;
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
                if !self.selecting {
                    for i in 0..self.rotation_pivots.len() {
                        controller.data.borrow_mut().rotate_helix(
                            self.translation_pivots[i].helix,
                            self.pivot_center,
                            angle,
                        );
                    }
                }
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

struct InitCutting {
    mouse_position: PhysicalPosition<f64>,
    nucl: FlatNucl,
}

impl<S: AppState> ControllerState<S> for InitCutting {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("Init Cutting")
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
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Cut(self.nucl),
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
                println!("from {:?} to {:?}", self.from, self.to);
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

struct InitBuilding {
    mouse_position: PhysicalPosition<f64>,
    builder: StrandBuilder,
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
                if let Some(attachement) = controller.data.borrow().attachable_neighbour(self.nucl)
                {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Xover(self.nucl, attachement),
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Cut(self.nucl),
                    }
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
                    ClickResult::Nucl(FlatNucl {
                        helix,
                        position,
                        forward,
                    }) if helix == self.nucl.helix && forward == self.nucl.forward => {
                        if position != self.nucl.position {
                            self.builder.move_to(position);
                            controller.data.borrow_mut().notify_update();
                            Transition {
                                new_state: Some(Box::new(Building {
                                    mouse_position: self.mouse_position,
                                    builder: self.builder.clone(),
                                    nucl: self.nucl,
                                    can_attach: true,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            Transition::nothing()
                        }
                    }
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_cross_to(self.nucl, nucl) =>
                    {
                        self.builder.reset();
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.nucl,
                                to: nucl,
                                strand_id: self.builder.get_strand_id(),
                                from3prime: self.end.expect("from3prime"),
                                cut: false,
                            })),
                            consequences: Consequence::FreeEnd(self.end.map(|b| FreeEnd {
                                strand_id: self.builder.get_strand_id(),
                                point: Vec2::new(x, y),
                                prime3: b,
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
                                    strand_id: self.builder.get_strand_id(),
                                })),
                                consequences: Consequence::FreeEnd(Some(FreeEnd {
                                    strand_id: self.builder.get_strand_id(),
                                    point: Vec2::new(x, y),
                                    prime3,
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
                                    strand_id: self.builder.get_strand_id(),
                                })),
                                consequences: Consequence::CutFreeEnd(
                                    self.nucl,
                                    Some(FreeEnd {
                                        strand_id: self.builder.get_strand_id(),
                                        point: Vec2::new(x, y),
                                        prime3,
                                    }),
                                ),
                            }
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
                            })),
                        }
                    }
                    _ => Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                        strand_id: self.strand_id,
                        point: Vec2::new(x, y),
                        prime3: self.prime3,
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

struct Building {
    mouse_position: PhysicalPosition<f64>,
    builder: StrandBuilder,
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
                    consequences: Consequence::Built(Box::new(self.builder.clone())),
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
                        self.builder.move_to(position);
                        controller.data.borrow_mut().notify_update();
                        Transition::consequence(Consequence::FreeEnd(None))
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
                    if let Some(builder) = controller.data.borrow().get_builder(self.nucl, false) {
                        if builder.created_de_novo() {
                            Transition {
                                new_state: Some(Box::new(Building {
                                    mouse_position: self.mouse_position,
                                    nucl: self.nucl,
                                    builder,
                                    can_attach: false,
                                })),
                                consequences: Consequence::NewCandidate(None),
                            }
                        } else {
                            Transition {
                                new_state: Some(Box::new(InitBuilding {
                                    mouse_position: self.mouse_position,
                                    nucl: self.nucl,
                                    builder,
                                    end: controller.data.borrow().is_strand_end(self.nucl),
                                })),
                                consequences: Consequence::NewCandidate(None),
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
        "Waiting Double Click".to_owned()
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

struct AddClickPivots {
    mouse_position: PhysicalPosition<f64>,
    translation_pivots: Vec<FlatNucl>,
    rotation_pivots: Vec<Vec2>,
    click_result: ClickResult,
}

impl<S: AppState> ControllerState<S> for AddClickPivots {
    fn transition_from(&self, _controller: &Controller<S>) {
        ()
    }

    fn transition_to(&self, _controller: &Controller<S>) {
        ()
    }

    fn display(&self) -> String {
        String::from("AddClickPivots")
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
