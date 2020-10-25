//! The `Controller` struct handles the event that happens on the drawing area of the scene.
//!
//! The `Controller` is internally implemented as a finite automata that transitions when a event
//! happens. In addition to the transistion in the automat, a `Consequence` is returned to the
//! scene, that describes the consequences that the input must have on the view or the data held by
//! the scene.
use super::data::Nucl;
use super::{CameraPtr, DataPtr, PhySize, PhysicalPosition, ViewPtr, WindowEvent};
use iced_winit::winit::event::*;
use std::cell::RefCell;
use ultraviolet::Vec2;

pub struct Controller {
    #[allow(dead_code)]
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    window_size: PhySize,
    area_size: PhySize,
    camera: CameraPtr,
    state: RefCell<Box<dyn ControllerState>>,
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
            camera,
            state: RefCell::new(Box::new(NormalState {
                mouse_position: PhysicalPosition::new(-1., -1.),
            })),
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
        let transition = self.state.borrow_mut().input(event, position, self);
        if let Some(state) = transition.new_state {
            self.state = RefCell::new(state)
        }
        transition.consequences
    }

    /*
    pub fn notify_select(&mut self) {
        self.state = State::Translating
    }

    pub fn set_pivot(&mut self, pivot: Vec2) {
        self.state = State::Rotating(pivot)
    }

    pub fn notify_unselect(&mut self) {
        self.state = State::Normal
    }*/

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

    #[allow(dead_code)]
    fn display(&self) -> String;
}

struct NormalState {
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for NormalState {
    fn display(&self) -> String {
        String::from("Normal state")
    }

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
                let pivot_opt = controller.data.borrow().get_click(x, y);
                controller
                    .data
                    .borrow_mut()
                    .set_selected_helix(pivot_opt.map(|p| p.helix));
                if let Some(pivot_nucl) = pivot_opt {
                    Transition {
                        new_state: Some(Box::new(Translating {
                            mouse_position: self.mouse_position,
                            clicked_position_world: Vec2::new(x, y),
                            pivot_nucl,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(MovingCamera {
                            mouse_position: self.mouse_position,
                            clicked_position_screen: self.mouse_position,
                            pivot_nucl: None,
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
            _ => Transition::nothing(),
        }
    }
}

struct Translating {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_world: Vec2,
    pivot_nucl: Nucl,
}

impl ControllerState for Translating {
    fn display(&self) -> String {
        String::from("Translating state")
    }
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
                controller.data.borrow_mut().end_movement();
                Transition {
                    new_state: Some(Box::new(ReleasedPivot {
                        mouse_position: self.mouse_position,
                        pivot_nucl: self.pivot_nucl,
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
                    (
                        x - self.clicked_position_world.x,
                        y - self.clicked_position_world.y,
                    )
                };
                controller
                    .data
                    .borrow_mut()
                    .translate_helix(Vec2::new(mouse_dx, mouse_dy));
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct MovingCamera {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_screen: PhysicalPosition<f64>,
    pivot_nucl: Option<Nucl>,
}

impl ControllerState for MovingCamera {
    fn display(&self) -> String {
        String::from("Moving camera")
    }
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
                controller.camera.borrow_mut().end_movement();
                if let Some(pivot_nucl) = self.pivot_nucl {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            pivot_nucl,
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
                    / controller.area_size.height as f32;
                controller
                    .camera
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
}

struct ReleasedPivot {
    mouse_position: PhysicalPosition<f64>,
    pivot_nucl: Nucl,
}

impl ControllerState for ReleasedPivot {
    fn display(&self) -> String {
        String::from("Released Pivot")
    }
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
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let pivot_opt = controller.data.borrow().get_click(x, y);
                if let Some(pivot) = pivot_opt {
                    controller
                        .data
                        .borrow_mut()
                        .set_selected_helix(Some(pivot.helix));
                    Transition {
                        new_state: Some(Box::new(Translating {
                            pivot_nucl: pivot,
                            clicked_position_world: Vec2::new(x, y),
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(LeavingPivot {
                            pivot_nucl: self.pivot_nucl,
                            clicked_position_screen: self.mouse_position,
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
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
                let pivot_coordinates = controller
                    .data
                    .borrow()
                    .get_pivot_position(self.pivot_nucl.helix, self.pivot_nucl.position)
                    .expect("pivot coordinates");
                Transition {
                    new_state: Some(Box::new(Rotating {
                        pivot_nucl: self.pivot_nucl,
                        pivot_coordinates,
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
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// This state in entered when use user has
struct LeavingPivot {
    pivot_nucl: Nucl,
    clicked_position_screen: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for LeavingPivot {
    fn display(&self) -> String {
        String::from("Leaving Pivot")
    }
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
                    "Pressed mouse button in LeavingPivot state"
                );
                controller.data.borrow_mut().set_selected_helix(None);
                Transition {
                    new_state: Some(Box::new(NormalState {
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
                let pivot_coordinates = controller
                    .data
                    .borrow()
                    .get_pivot_position(self.pivot_nucl.helix, self.pivot_nucl.position)
                    .expect("pivot coordinates");
                Transition {
                    new_state: Some(Box::new(Rotating {
                        pivot_nucl: self.pivot_nucl,
                        pivot_coordinates,
                        clicked_position_screen: self.mouse_position,
                        mouse_position: self.mouse_position,
                        button: MouseButton::Right,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(self.clicked_position_screen, self.mouse_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(MovingCamera {
                            pivot_nucl: Some(self.pivot_nucl),
                            mouse_position: self.mouse_position,
                            clicked_position_screen: self.clicked_position_screen,
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
            _ => Transition::nothing(),
        }
    }
}

struct Rotating {
    pivot_nucl: Nucl,
    pivot_coordinates: Vec2,
    clicked_position_screen: PhysicalPosition<f64>,
    button: MouseButton,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for Rotating {
    fn display(&self) -> String {
        String::from("Rotating")
    }
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
                    "Pressed mouse button in LeavingPivot state"
                );
                controller.data.borrow_mut().end_movement();
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput { button, state, .. } if *button == self.button => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Rotating state"
                );
                Transition {
                    new_state: Some(Box::new(ReleasedPivot {
                        mouse_position: self.mouse_position,
                        pivot_nucl: self.pivot_nucl,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let angle = {
                    let (x, y) = controller
                        .camera
                        .borrow()
                        .screen_to_world(position.x as f32, position.y as f32);
                    let (old_x, old_y) = controller.camera.borrow().screen_to_world(
                        self.clicked_position_screen.x as f32,
                        self.clicked_position_screen.y as f32,
                    );
                    (y - self.pivot_coordinates.y).atan2(x - self.pivot_coordinates.x)
                        - (old_y - self.pivot_coordinates.y).atan2(old_x - self.pivot_coordinates.x)
                };
                controller
                    .data
                    .borrow_mut()
                    .rotate_helix(self.pivot_coordinates, angle);
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}
