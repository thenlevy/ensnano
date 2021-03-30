use super::*;
use iced_winit::winit::event::*;
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
                        new_state: Some(Box::new(NormalState {
                            pasting: controller.pasting,
                            mouse_position: position,
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
