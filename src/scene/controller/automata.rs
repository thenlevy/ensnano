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
use super::*;
use ensnano_interactor::ActionMode;
use std::borrow::Cow;
use std::cell::RefCell;
use std::time::Instant;

use super::AppState;

pub(super) type State<S> = RefCell<Box<dyn ControllerState<S>>>;

pub(super) fn initial_state<S: AppState>() -> State<S> {
    RefCell::new(Box::new(NormalState {
        mouse_position: PhysicalPosition::new(-1., -1.),
    }))
}

pub(super) struct Transition<S: AppState> {
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

pub(super) trait ControllerState<S: AppState> {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        pixel_reader: &mut ElementSelector,
        app_state: &S,
    ) -> Transition<S>;

    #[allow(dead_code)]
    fn display(&self) -> Cow<'static, str>;

    fn transition_from(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn transition_to(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn check_timers(&mut self, _controller: &Controller<S>) -> Transition<S> {
        Transition::nothing()
    }

    fn handles_color_system(&self) -> Option<HandleColors> {
        None
    }
}

pub struct NormalState {
    pub mouse_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for NormalState {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        pixel_reader: &mut ElementSelector,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } if app_state.is_pasting() => {
                self.mouse_position = position;
                let element = pixel_reader.set_selected_id(position);
                Transition::consequence(Consequence::PasteCandidate(element))
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let element = pixel_reader.set_selected_id(position);
                if let Some(SceneElement::Grid(d_id, _)) = element {
                    let mouse_x = position.x / controller.area_size.width as f64;
                    let mouse_y = position.y / controller.area_size.height as f64;
                    let candidate = if let Some(intersection) = controller
                        .view
                        .borrow()
                        .grid_intersection(mouse_x as f32, mouse_y as f32)
                    {
                        Some(SceneElement::GridCircle(
                            d_id,
                            intersection.grid_id,
                            intersection.x,
                            intersection.y,
                        ))
                    } else {
                        element
                    };
                    Transition::consequence(Consequence::Candidate(candidate))
                } else {
                    Transition::consequence(Consequence::Candidate(element))
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if controller.current_modifiers.alt() => Transition {
                new_state: Some(Box::new(TranslatingCamera {
                    mouse_position: self.mouse_position,
                    clicked_position: self.mouse_position,
                    button_pressed: MouseButton::Left,
                })),
                consequences: Consequence::Nothing,
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if app_state.is_pasting() => {
                let element = pixel_reader.set_selected_id(position);
                Transition {
                    new_state: Some(Box::new(Pasting {
                        clicked_position: position,
                        element,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let element = pixel_reader.set_selected_id(position);
                log::info!("Clicked on {:?}", element);
                match element {
                    Some(SceneElement::GridCircle(d_id, g_id, x, y)) => {
                        if let ActionMode::BuildHelix {
                            position: position_helix,
                            length,
                        } = app_state.get_action_mode().0
                        {
                            Transition {
                                new_state: Some(Box::new(BuildingHelix {
                                    position_helix,
                                    length_helix: length,
                                    x_helix: x,
                                    y_helix: y,
                                    grid_id: g_id,
                                    design_id: d_id,
                                    clicked_position: position,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            Transition {
                                new_state: Some(Box::new(Selecting {
                                    element,
                                    clicked_position: position,
                                    mouse_position: position,
                                    click_date: Instant::now(),
                                    adding: controller.current_modifiers.shift()
                                        | ctrl(&controller.current_modifiers),
                                })),
                                consequences: Consequence::Nothing,
                            }
                        }
                    }
                    Some(SceneElement::Grid(d_id, _)) => {
                        let mouse_x = position.x / controller.area_size.width as f64;
                        let mouse_y = position.y / controller.area_size.height as f64;
                        let grid_intersection = controller
                            .view
                            .borrow()
                            .grid_intersection(mouse_x as f32, mouse_y as f32);
                        if let ActionMode::BuildHelix {
                            position: helix_position,
                            length,
                        } = app_state.get_action_mode().0
                        {
                            if let Some(intersection) = grid_intersection {
                                if let Some(helix) = controller.data.borrow().get_grid_helix(
                                    intersection.grid_id,
                                    intersection.x,
                                    intersection.y,
                                ) {
                                    Transition {
                                        new_state: Some(Box::new(TranslatingHelix {
                                            grid_id: intersection.grid_id,
                                            helix_id: helix as usize,
                                            x: intersection.x,
                                            y: intersection.y,
                                        })),
                                        consequences: Consequence::HelixSelected(helix as usize),
                                    }
                                } else {
                                    Transition {
                                        new_state: Some(Box::new(BuildingHelix {
                                            position_helix: helix_position,
                                            length_helix: length,
                                            x_helix: intersection.x,
                                            y_helix: intersection.y,
                                            grid_id: intersection.grid_id,
                                            design_id: d_id,
                                            clicked_position: position,
                                        })),
                                        consequences: Consequence::Nothing,
                                    }
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(Selecting {
                                        element,
                                        clicked_position: position,
                                        mouse_position: position,
                                        click_date: Instant::now(),
                                        adding: controller.current_modifiers.shift()
                                            | ctrl(&controller.current_modifiers),
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        } else {
                            let clicked_element;
                            let helix;
                            if let Some(intersection) = grid_intersection.as_ref() {
                                clicked_element = Some(SceneElement::GridCircle(
                                    d_id,
                                    intersection.grid_id,
                                    intersection.x,
                                    intersection.y,
                                ));
                                helix = controller.data.borrow().get_grid_helix(
                                    intersection.grid_id,
                                    intersection.x,
                                    intersection.y,
                                );
                            } else {
                                clicked_element = element;
                                helix = None;
                            };
                            if let Some(h_id) = helix {
                                // if helix is Some, intersection is also Some
                                let intersection = grid_intersection.unwrap();
                                Transition {
                                    new_state: Some(Box::new(TranslatingHelix {
                                        grid_id: intersection.grid_id,
                                        helix_id: h_id as usize,
                                        x: intersection.x,
                                        y: intersection.y,
                                    })),
                                    consequences: Consequence::HelixSelected(h_id as usize),
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(Selecting {
                                        element: clicked_element,
                                        clicked_position: position,
                                        mouse_position: position,
                                        click_date: Instant::now(),
                                        adding: controller.current_modifiers.shift()
                                            | ctrl(&controller.current_modifiers),
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        }
                    }
                    Some(SceneElement::WidgetElement(widget_id)) => {
                        let mouse_x = position.x / controller.area_size.width as f64;
                        let mouse_y = position.y / controller.area_size.height as f64;
                        let translation_target = if controller.current_modifiers.shift() {
                            WidgetTarget::Pivot
                        } else {
                            WidgetTarget::Object
                        };
                        match widget_id {
                            UP_HANDLE_ID | DIR_HANDLE_ID | RIGHT_HANDLE_ID => Transition {
                                new_state: Some(Box::new(TranslatingWidget {
                                    direction: HandleDir::from_widget_id(widget_id),
                                    translation_target,
                                })),
                                consequences: Consequence::InitTranslation(
                                    mouse_x,
                                    mouse_y,
                                    translation_target,
                                ),
                            },
                            RIGHT_CIRCLE_ID | FRONT_CIRCLE_ID | UP_CIRCLE_ID => {
                                let target = if controller.current_modifiers.shift() {
                                    WidgetTarget::Pivot
                                } else {
                                    WidgetTarget::Object
                                };

                                Transition {
                                    new_state: Some(Box::new(RotatingWidget { target })),
                                    consequences: Consequence::InitRotation(
                                        RotationMode::from_widget_id(widget_id),
                                        mouse_x,
                                        mouse_y,
                                        target,
                                    ),
                                }
                            }
                            _ => {
                                println!("WARNING UNEXPECTED WIDGET ID");
                                Transition::nothing()
                            }
                        }
                    }
                    _ => Transition {
                        new_state: Some(Box::new(Selecting {
                            element,
                            clicked_position: position,
                            mouse_position: position,
                            click_date: Instant::now(),
                            adding: controller.current_modifiers.shift()
                                | ctrl(&controller.current_modifiers),
                        })),
                        consequences: Consequence::Nothing,
                    },
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } if ctrl(&controller.current_modifiers) => Transition {
                new_state: Some(Box::new(RotatingCamera {
                    clicked_position: position,
                    button_pressed: MouseButton::Middle,
                })),
                consequences: Consequence::Nothing,
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => Transition {
                new_state: Some(Box::new(TranslatingCamera {
                    mouse_position: self.mouse_position,
                    clicked_position: self.mouse_position,
                    button_pressed: MouseButton::Middle,
                })),
                consequences: Consequence::Nothing,
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => Transition {
                new_state: Some(Box::new(SettingPivot {
                    mouse_position: position,
                    clicked_position: position,
                })),
                consequences: Consequence::Nothing,
            },
            _ => Transition::nothing(),
        }
    }

    fn display(&self) -> Cow<'static, str> {
        "Normal".into()
    }
}

struct TranslatingCamera {
    mouse_position: PhysicalPosition<f64>,
    clicked_position: PhysicalPosition<f64>,
    button_pressed: MouseButton,
}

impl<S: AppState> ControllerState<S> for TranslatingCamera {
    fn display(&self) -> Cow<'static, str> {
        "Translating Camera".into()
    }

    fn transition_to(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::InitMovement
    }

    fn transition_from(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::EndMovement
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
            } if *button == self.button_pressed => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: self.mouse_position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_dx =
                    (position.x - self.clicked_position.x) / controller.area_size.width as f64;
                let mouse_dy =
                    (position.y - self.clicked_position.y) / controller.area_size.height as f64;
                self.mouse_position = position;
                Transition::consequence(Consequence::CameraTranslated(mouse_dx, mouse_dy))
            }
            _ => Transition::nothing(),
        }
    }
}

struct SettingPivot {
    mouse_position: PhysicalPosition<f64>,
    clicked_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for SettingPivot {
    fn display(&self) -> Cow<'static, str> {
        "Setting Pivot".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        pixel_reader: &mut ElementSelector,
        _app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(position, self.clicked_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(RotatingCamera {
                            clicked_position: self.clicked_position,
                            button_pressed: MouseButton::Right,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    self.mouse_position = position;
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Right,
                ..
            } => {
                let element = match pixel_reader.set_selected_id(self.mouse_position) {
                    Some(SceneElement::Grid(d_id, g_id)) => {
                        // for grids we take the precise grid position on which the user clicked.
                        let mouse_x = self.mouse_position.x / controller.area_size.width as f64;
                        let mouse_y = self.mouse_position.y / controller.area_size.height as f64;
                        if let Some(intersection) = controller
                            .view
                            .borrow()
                            .specific_grid_intersection(mouse_x as f32, mouse_y as f32, g_id)
                        {
                            Some(SceneElement::GridCircle(
                                d_id,
                                intersection.grid_id,
                                intersection.x,
                                intersection.y,
                            ))
                        } else {
                            Some(SceneElement::Grid(d_id, g_id))
                        }
                    }
                    element => element,
                };
                log::debug!("Pivot element {:?}", element);
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: position,
                    })),
                    consequences: Consequence::PivotElement(element),
                }
            }
            _ => Transition::nothing(),
        }
    }
}

struct RotatingCamera {
    clicked_position: PhysicalPosition<f64>,
    button_pressed: MouseButton,
}

impl<S: AppState> ControllerState<S> for RotatingCamera {
    fn display(&self) -> Cow<'static, str> {
        "Rotating Camera".into()
    }

    fn transition_to(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::InitMovement
    }

    fn transition_from(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::EndMovement
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
            WindowEvent::CursorMoved { .. } => {
                let mouse_dx =
                    (position.x - self.clicked_position.x) / controller.area_size.width as f64;
                let mouse_dy =
                    (position.y - self.clicked_position.y) / controller.area_size.height as f64;
                Transition::consequence(Consequence::Swing(mouse_dx, mouse_dy))
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } if *button == self.button_pressed => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::Nothing,
            },
            _ => Transition::nothing(),
        }
    }
}

struct Selecting {
    mouse_position: PhysicalPosition<f64>,
    clicked_position: PhysicalPosition<f64>,
    element: Option<SceneElement>,
    click_date: Instant,
    adding: bool,
}

impl<S: AppState> ControllerState<S> for Selecting {
    fn display(&self) -> Cow<'static, str> {
        "Selecting".into()
    }

    fn check_timers(&mut self, controller: &Controller<S>) -> Transition<S> {
        let now = Instant::now();
        if (now - self.click_date).as_millis() > 250 {
            if let Some((nucl, d_id)) = controller
                .data
                .borrow()
                .element_to_nucl(&self.element, true)
            {
                let position_nucl = controller
                    .data
                    .borrow()
                    .get_nucl_position(nucl, d_id)
                    .expect("position nucl");
                let mouse_x = self.mouse_position.x / controller.area_size.width as f64;
                let mouse_y = self.mouse_position.y / controller.area_size.height as f64;
                let projected_pos =
                    controller
                        .camera_controller
                        .get_projection(position_nucl, mouse_x, mouse_y);

                Transition {
                    new_state: Some(Box::new(Xovering {
                        source_element: self.element,
                        source_position: position_nucl,
                    })),
                    consequences: Consequence::InitFreeXover(nucl, d_id, projected_pos),
                }
            } else {
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
        } else {
            Transition::nothing()
        }
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
            WindowEvent::CursorMoved { .. } => {
                if position_difference(position, self.clicked_position) > 5. {
                    if let Some(nucl) = controller.data.borrow().can_start_builder(self.element) {
                        Transition {
                            new_state: Some(Box::new(BuildingStrand)),
                            consequences: Consequence::InitBuild(nucl),
                        }
                    } else {
                        Transition {
                            new_state: Some(Box::new(NormalState {
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                } else {
                    self.mouse_position = position;
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                let now = Instant::now();
                Transition {
                    new_state: Some(Box::new(WaitDoubleClick {
                        click_date: now,
                        element: self.element.clone(),
                        mouse_position: position,
                        clicked_position: self.clicked_position,
                    })),
                    consequences: Consequence::ElementSelected(self.element, self.adding),
                }
            }
            _ => Transition::nothing(),
        }
    }
}

struct WaitDoubleClick {
    click_date: Instant,
    element: Option<SceneElement>,
    mouse_position: PhysicalPosition<f64>,
    clicked_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for WaitDoubleClick {
    fn check_timers(&mut self, _controller: &Controller<S>) -> Transition<S> {
        let now = Instant::now();
        if (now - self.click_date).as_millis() > 250 {
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: self.mouse_position,
                })),
                consequences: Consequence::Nothing,
            }
        } else {
            Transition::nothing()
        }
    }

    fn display(&self) -> Cow<'static, str> {
        "Waiting Double Click".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        _controller: &Controller<S>,
        _pixel_reader: &mut ElementSelector,
        _app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: self.mouse_position,
                })),
                consequences: Consequence::DoubleClick(self.element.clone()),
            },
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(position, self.clicked_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            _ => Transition::nothing(),
        }
    }
}

struct TranslatingWidget {
    direction: HandleDir,
    translation_target: WidgetTarget,
}

/// What is being affected by the translation
#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub enum WidgetTarget {
    /// The selected elements
    Object,
    /// The selection's pivot
    Pivot,
}

impl<S: AppState> ControllerState<S> for TranslatingWidget {
    fn display(&self) -> Cow<'static, str> {
        "Translating widget".into()
    }

    fn handles_color_system(&self) -> Option<HandleColors> {
        match self.translation_target {
            WidgetTarget::Pivot => Some(HandleColors::Cym),
            WidgetTarget::Object => Some(HandleColors::Rgb),
        }
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
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                Transition::consequence(Consequence::Translation(
                    self.direction,
                    mouse_x,
                    mouse_y,
                    self.translation_target,
                ))
            }
            _ => Transition::nothing(),
        }
    }
}

struct TranslatingHelix {
    helix_id: usize,
    grid_id: usize,
    x: isize,
    y: isize,
}

impl<S: AppState> ControllerState<S> for TranslatingHelix {
    fn display(&self) -> Cow<'static, str> {
        format!(
            "Translating helix {} on grid {}",
            self.helix_id, self.grid_id
        )
        .into()
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
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                if let Some(intersection) = controller.view.borrow().specific_grid_intersection(
                    mouse_x as f32,
                    mouse_y as f32,
                    self.grid_id,
                ) {
                    if intersection.x != self.x || intersection.y != self.y {
                        self.x = intersection.x;
                        self.y = intersection.y;
                        Transition::consequence(Consequence::HelixTranslated {
                            helix: self.helix_id,
                            grid: self.grid_id,
                            x: intersection.x,
                            y: intersection.y,
                        })
                    } else {
                        Transition::nothing()
                    }
                } else {
                    Transition::nothing()
                }
            }
            _ => Transition::nothing(),
        }
    }
}

struct RotatingWidget {
    target: WidgetTarget,
}

impl<S: AppState> ControllerState<S> for RotatingWidget {
    fn display(&self) -> Cow<'static, str> {
        "Rotating widget".into()
    }

    fn handles_color_system(&self) -> Option<HandleColors> {
        match self.target {
            WidgetTarget::Pivot => Some(HandleColors::Cym),
            WidgetTarget::Object => Some(HandleColors::Rgb),
        }
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
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                Transition::consequence(Consequence::Rotation(mouse_x, mouse_y, self.target))
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_to(&self, controller: &Controller<S>) -> TransistionConsequence {
        if self.target == WidgetTarget::Pivot {
            controller.data.borrow_mut().notify_rotating_pivot();
        }
        TransistionConsequence::Nothing
    }

    fn transition_from(&self, controller: &Controller<S>) -> TransistionConsequence {
        if self.target == WidgetTarget::Pivot {
            controller.data.borrow_mut().stop_rotating_pivot();
        }
        TransistionConsequence::Nothing
    }
}

struct BuildingStrand;
impl<S: AppState> ControllerState<S> for BuildingStrand {
    fn display(&self) -> Cow<'static, str> {
        "Building Strand".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        _pixel_reader: &mut ElementSelector,
        app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::BuildEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                if let Some(builder) = app_state.get_strand_builders().get(0) {
                    let mouse_x = position.x / controller.area_size.width as f64;
                    let mouse_y = position.y / controller.area_size.height as f64;
                    let position = controller.view.borrow().compute_projection_axis(
                        &builder.axis,
                        mouse_x,
                        mouse_y,
                    );
                    let consequence = if let Some(position) = position {
                        Consequence::Building(position)
                    } else {
                        Consequence::Nothing
                    };
                    Transition::consequence(consequence)
                } else {
                    Transition::nothing()
                }
            }
            _ => Transition::nothing(),
        }
    }
}

struct Xovering {
    source_element: Option<SceneElement>,
    source_position: Vec3,
}

impl<S: AppState> ControllerState<S> for Xovering {
    fn display(&self) -> Cow<'static, str> {
        "Building Strand".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller<S>,
        pixel_reader: &mut ElementSelector,
        _app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                let element = pixel_reader.set_selected_id(position);
                if let Some((source, target, design_id)) = controller
                    .data
                    .borrow()
                    .attempt_xover(&self.source_element, &element)
                {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: position,
                        })),
                        consequences: Consequence::XoverAtempt(source, target, design_id),
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: position,
                        })),
                        consequences: Consequence::EndFreeXover,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                let element = pixel_reader.set_selected_id(position);
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                let projected_pos = controller.camera_controller.get_projection(
                    self.source_position,
                    mouse_x,
                    mouse_y,
                );
                Transition::consequence(Consequence::MoveFreeXover(element, projected_pos))
            }
            _ => Transition::nothing(),
        }
    }
}

struct BuildingHelix {
    design_id: u32,
    grid_id: usize,
    x_helix: isize,
    y_helix: isize,
    length_helix: usize,
    position_helix: isize,
    clicked_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for BuildingHelix {
    fn display(&self) -> Cow<'static, str> {
        "Building Helix".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        _controller: &Controller<S>,
        _pixel_reader: &mut ElementSelector,
        _app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(self.clicked_position, position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::BuildHelix {
                    design_id: self.design_id,
                    grid_id: self.grid_id,
                    length: self.length_helix,
                    x: self.x_helix,
                    y: self.y_helix,
                    position: self.position_helix,
                },
            },
            _ => Transition::nothing(),
        }
    }
}

struct Pasting {
    clicked_position: PhysicalPosition<f64>,
    element: Option<SceneElement>,
}

impl<S: AppState> ControllerState<S> for Pasting {
    fn display(&self) -> Cow<'static, str> {
        "Pasting".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        _controller: &Controller<S>,
        _pixel_reader: &mut ElementSelector,
        _app_state: &S,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(self.clicked_position, position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::Paste(self.element),
            },
            _ => Transition::nothing(),
        }
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
