//! The `Controller` struct handles the event that happens on the drawing area of the scene.
//!
//! The `Controller` is internally implemented as a finite automata that transitions when a event
//! happens. In addition to the transistion in the automat, a `Consequence` is returned to the
//! scene, that describes the consequences that the input must have on the view or the data held by
//! the scene.
use super::{CameraPtr, DataPtr, PhySize, PhysicalPosition, ViewPtr, WindowEvent};
use iced_winit::winit::event::*;
use ultraviolet::Vec2;

pub struct Controller {
    #[allow(dead_code)]
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    last_left_clicked_position: Option<PhysicalPosition<f64>>,
    last_right_clicked_position: Option<PhysicalPosition<f64>>,
    mouse_position: PhysicalPosition<f64>,
    window_size: PhySize,
    area_size: PhySize,
    camera: CameraPtr,
    state: State,
}

pub enum Consequence {
    #[allow(dead_code)]
    GlobalsChanged,
    Nothing,
    MovementEnded,
    Clicked(f32, f32),
    Translated(f32, f32),
    Rotated(Vec2, f32),
}

impl Controller {
    pub fn new(
        view: ViewPtr,
        data: DataPtr,
        window_size: PhySize,
        area_size: PhySize,
        camera: CameraPtr,
    ) -> Self {
        Self {
            view,
            data,
            window_size,
            area_size,
            last_left_clicked_position: None,
            last_right_clicked_position: None,
            mouse_position: PhysicalPosition::new(-1., -1.),
            camera,
            state: State::Normal,
        }
    }

    pub fn resize(&mut self, window_size: PhySize, area_size: PhySize) {
        self.area_size = area_size;
        self.window_size = window_size;
        self.camera
            .borrow_mut()
            .resize(area_size.width as f32, area_size.height as f32);
    }

    pub fn input(&mut self, event: &WindowEvent, position: PhysicalPosition<f64>) -> Consequence {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                if *state == ElementState::Released {
                    self.last_left_clicked_position = None;
                    self.camera.borrow_mut().end_movement();
                    Consequence::MovementEnded
                } else {
                    self.last_left_clicked_position = Some(self.mouse_position);
                    let (x, y) = self.camera.borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    Consequence::Clicked(x, y)
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if let Some(clicked_position) = self.last_left_clicked_position {
                    match self.state {
                        State::Normal => {
                            let mouse_dx =
                                (position.x - clicked_position.x) / self.area_size.width as f64;
                            let mouse_dy =
                                (position.y - clicked_position.y) / self.area_size.height as f64;
                            self.camera
                                .borrow_mut()
                                .process_mouse(mouse_dx as f32, mouse_dy as f32);
                            Consequence::Nothing
                        }
                        State::Translating => {
                            let (mouse_dx, mouse_dy) = {
                                let (x, y) = self
                                    .camera
                                    .borrow()
                                    .screen_to_world(position.x as f32, position.y as f32);
                                let (old_x, old_y) = self.camera.borrow().screen_to_world(
                                    clicked_position.x as f32,
                                    clicked_position.y as f32,
                                );
                                (x - old_x, y - old_y)
                            };

                            Consequence::Translated(mouse_dx as f32, mouse_dy as f32)
                        }
                        State::Rotating(pivot) => {
                            let angle = {
                                let (x, y) = self
                                    .camera
                                    .borrow()
                                    .screen_to_world(position.x as f32, position.y as f32);
                                let (old_x, old_y) = self.camera.borrow().screen_to_world(
                                    clicked_position.x as f32,
                                    clicked_position.y as f32,
                                );
                                (y - pivot.y).atan2(x - pivot.x)
                                    - (old_y - pivot.y).atan2(old_x - pivot.x)
                            };
                            Consequence::Rotated(pivot, angle)
                        }
                    }
                } else {
                    Consequence::Nothing
                }
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match *key {
                VirtualKeyCode::Up => {
                    self.camera.borrow_mut().zoom_in();
                    Consequence::Nothing
                }
                VirtualKeyCode::Down => {
                    self.camera.borrow_mut().zoom_out();
                    Consequence::Nothing
                }
                _ => Consequence::Nothing,
            },
            _ => Consequence::Nothing,
        }
    }

    pub fn notify_select(&mut self) {
        self.state = State::Translating
    }

    pub fn set_pivot(&mut self, pivot: Vec2) {
        self.state = State::Rotating(pivot)
    }

    pub fn notify_unselect(&mut self) {
        self.state = State::Normal
    }

    pub fn process_keyboard(&self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    virtual_keycode: Some(key),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        {
            match *key {
                VirtualKeyCode::Up => {
                    self.camera.borrow_mut().zoom_in();
                }
                VirtualKeyCode::Down => {
                    self.camera.borrow_mut().zoom_out();
                }
                _ => (),
            }
        }
    }
}

enum State {
    Normal,
    Translating,
    Rotating(Vec2),
}

struct Transition {
    new_state: Option<Box<dyn ControllerState>>,
    consequences: Consequence,
}

impl Transition {
    pub fn nothing() -> Self {
        Self {
            new_state: None,
            consequences: Consequence::Nothing,
        }
    }
}

trait ControllerState {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition;
}

struct NormalState {
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for NormalState {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in normal mode"
                );
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let pivot_opt = controller.data.borrow_mut().request_pivot(x, y);
                if let Some(pivot) = pivot_opt {
                    Transition {
                        new_state: Some(Box::new(Translating {
                            mouse_position: self.mouse_position,
                            clicked_position_world: Vec2::new(x, y),
                            pivot,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(MovingCamera {
                            mouse_position: self.mouse_position,
                            clicked_position_screen: Vec2::new(self.mouse_position.x as f32, self.mouse_position.y as f32),
                            pivot: None,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput{ .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct Translating {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_world: Vec2,
    pivot: Vec2,
}

impl ControllerState for Translating {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in translating mode"
                );
                Transition {
                    new_state: Some(Box::new(ReleasedPivot {
                        mouse_position: self.mouse_position,
                        pivot: self.pivot,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (mouse_dx, mouse_dy) = {
                    let (x, y) = controller
                        .camera
                        .borrow()
                        .screen_to_world(position.x as f32, position.y as f32);
                    (x - self.clicked_position_world.x, y - self.clicked_position_world.y)
                };
                controller
                    .data
                    .borrow_mut()
                    .translate_helix(Vec2::new(mouse_dx, mouse_dy));
                Transition::nothing()
            }
            WindowEvent::KeyboardInput{ .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct MovingCamera {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_screen: Vec2,
    pivot: Option<Vec2>,
}

impl ControllerState for MovingCamera {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in translating mode"
                );
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let mouse_dx =
                    (position.x as f32 - self.clicked_position_screen.x) / controller.area_size.width as f32;
                let mouse_dy =
                    (position.y as f32 - self.clicked_position_screen.y) / controller.area_size.height as f32;
                controller.camera
                    .borrow_mut()
                    .process_mouse(mouse_dx, mouse_dy);
                Transition::nothing()
            }
            WindowEvent::KeyboardInput{ .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct ReleasedPivot {
    mouse_position: PhysicalPosition<f64>,
    pivot: Vec2,
}

impl ControllerState for ReleasedPivot {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in ReleasedPivot state"
                );
                Transition {
                    new_state: Some(Box::new(LeavingPivot {
                        pivot: self.pivot,
                        clicked_position_screen: self.mouse_position,
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released right mouse button in ReleasedPivot state"
                );
                Transition {
                    new_state: Some(Box::new(Rotating {
                        pivot: self.pivot,
                        clicked_position_screen: self.mouse_position,
                        mouse_position: self.mouse_position,
                        button: MouseButton::Right,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput{ .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// This state in entered when use user has 
struct LeavingPivot {
    pivot: Vec2,
    clicked_position_screen: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for LeavingPivot {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        unimplemented!()
    }

}

struct Rotating {
    pivot: Vec2,
    clicked_position_screen: PhysicalPosition<f64>,
    button: MouseButton,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for Rotating {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        unimplemented!()
    }

}
