use super::*;
use std::borrow::Cow;
use std::cell::RefCell;

pub(super) type State = RefCell<Box<dyn ControllerState>>;

pub(super) fn initial_state(pasting: bool) -> State {
    RefCell::new(Box::new(NormalState {
        pasting,
        mouse_position: PhysicalPosition::new(-1., -1.),
    }))
}

pub(super) struct Transition {
    pub new_state: Option<Box<dyn ControllerState>>,
    pub consequences: Consequence,
}

impl Transition {
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

pub(super) trait ControllerState {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        pixel_reader: &mut ElementSelector,
    ) -> Transition;

    #[allow(dead_code)]
    fn display(&self) -> Cow<'static, str>;

    fn transition_from(&self, _controller: &Controller) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn transition_to(&self, _controller: &Controller) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn check_timers(&mut self) -> Transition {
        Transition::nothing()
    }
}

pub struct NormalState {
    pub mouse_position: PhysicalPosition<f64>,
    pub pasting: bool,
}

impl ControllerState for NormalState {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        pixel_reader: &mut ElementSelector,
    ) -> Transition {
        match event {
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let element = pixel_reader.set_selected_id(position);
                Transition::consequence(Consequence::Candidate(element))
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
            } => {
                let element = pixel_reader.set_selected_id(position);
                match element {
                    Some(SceneElement::WidgetElement(widget_id)) => {
                        let mouse_x = position.x / controller.area_size.width as f64;
                        let mouse_y = position.y / controller.area_size.height as f64;
                        match widget_id {
                            UP_HANDLE_ID | DIR_HANDLE_ID | RIGHT_HANDLE_ID => Transition {
                                new_state: Some(Box::new(TranslatingWidget {
                                    direction: HandleDir::from_widget_id(widget_id),
                                })),
                                consequences: Consequence::InitTranslation(mouse_x, mouse_y),
                            },
                            RIGHT_CIRCLE_ID | FRONT_CIRCLE_ID | UP_CIRCLE_ID => Transition {
                                new_state: Some(Box::new(RotatingWidget {
                                    rotation_mode: RotationMode::from_widget_id(widget_id),
                                })),
                                consequences: Consequence::InitRotation(mouse_x, mouse_y),
                            },
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

impl ControllerState for TranslatingCamera {
    fn display(&self) -> Cow<'static, str> {
        "Translating Camera".into()
    }

    fn transition_to(&self, _controller: &Controller) -> TransistionConsequence {
        TransistionConsequence::InitMovement
    }

    fn transition_from(&self, _controller: &Controller) -> TransistionConsequence {
        TransistionConsequence::EndMovement
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        _pixel_reader: &mut ElementSelector,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if *button == self.button_pressed => Transition {
                new_state: Some(Box::new(NormalState {
                    pasting: controller.pasting,
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

impl ControllerState for SettingPivot {
    fn display(&self) -> Cow<'static, str> {
        "Setting Pivot".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        pixel_reader: &mut ElementSelector,
    ) -> Transition {
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
                let element = pixel_reader.set_selected_id(self.mouse_position);
                Transition {
                    new_state: Some(Box::new(NormalState {
                        pasting: controller.pasting,
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

impl ControllerState for RotatingCamera {
    fn display(&self) -> Cow<'static, str> {
        "Rotating Camera".into()
    }

    fn transition_to(&self, _controller: &Controller) -> TransistionConsequence {
        TransistionConsequence::InitMovement
    }

    fn transition_from(&self, _controller: &Controller) -> TransistionConsequence {
        TransistionConsequence::EndMovement
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        _pixel_reader: &mut ElementSelector,
    ) -> Transition {
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
                    pasting: controller.pasting,
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
}

impl ControllerState for Selecting {
    fn display(&self) -> Cow<'static, str> {
        "Selecting".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        _pixel_reader: &mut ElementSelector,
    ) -> Transition {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(position, self.clicked_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                            pasting: controller.pasting,
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
                button: MouseButton::Left,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    pasting: controller.pasting,
                    mouse_position: position,
                })),
                consequences: Consequence::ElementSelected(self.element),
            },
            _ => Transition::nothing(),
        }
    }
}

struct TranslatingWidget {
    direction: HandleDir,
}

impl ControllerState for TranslatingWidget {
    fn display(&self) -> Cow<'static, str> {
        "Translating widget".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        _pixel_reader: &mut ElementSelector,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    pasting: controller.pasting,
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                Transition::consequence(Consequence::Translation(self.direction, mouse_x, mouse_y))
            }
            _ => Transition::nothing(),
        }
    }
}

struct RotatingWidget {
    rotation_mode: RotationMode,
}

impl ControllerState for RotatingWidget {
    fn display(&self) -> Cow<'static, str> {
        "Rotating widget".into()
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
        _pixel_reader: &mut ElementSelector,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    pasting: controller.pasting,
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                Transition::consequence(Consequence::Rotation(self.rotation_mode, mouse_x, mouse_y))
            }
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
