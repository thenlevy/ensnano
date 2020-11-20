//! The `Controller` struct handles the event that happens on the drawing area of the scene.
//!
//! The `Controller` is internally implemented as a finite automata that transitions when a event
//! happens. In addition to the transistion in the automat, a `Consequence` is returned to the
//! scene, that describes the consequences that the input must have on the view or the data held by
//! the scene.
use super::data::{FreeEnd, Nucl};
use super::{ActionMode, CameraPtr, DataPtr, PhySize, PhysicalPosition, ViewPtr, WindowEvent};
use iced_winit::winit::event::*;
use std::cell::RefCell;
use ultraviolet::Vec2;

mod automata;
use automata::{ControllerState, NormalState};

pub struct Controller {
    #[allow(dead_code)]
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    window_size: PhySize,
    area_size: PhySize,
    camera: CameraPtr,
    state: RefCell<Box<dyn ControllerState>>,
    action_mode: ActionMode,
}

pub enum Consequence {
    #[allow(dead_code)]
    GlobalsChanged,
    Nothing,
    Xover(Nucl, Nucl),
    Cut(Nucl),
    FreeEnd(Option<FreeEnd>),
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
            action_mode: ActionMode::Normal,
        }
    }

    pub fn resize(&mut self, window_size: PhySize, area_size: PhySize) {
        self.area_size = area_size;
        self.window_size = window_size;
        self.camera
            .borrow_mut()
            .resize(area_size.width as f32, area_size.height as f32);
    }

    pub fn fit(&mut self) {
        let rectangle = self.data.borrow().get_fit_rectangle();
        self.camera.borrow_mut().fit(rectangle);
    }

    pub fn input(&mut self, event: &WindowEvent, position: PhysicalPosition<f64>) -> Consequence {
        let transition = self.state.borrow_mut().input(event, position, self);
        if let Some(state) = transition.new_state {
            self.state.borrow().transition_from(&self);
            self.state = RefCell::new(state);
            self.state.borrow().transition_to(&self);
        }
        transition.consequences
    }

    pub fn set_action_mode(&mut self, action_mode: ActionMode) {
        self.action_mode = action_mode;
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
                VirtualKeyCode::J => {
                    self.data.borrow_mut().move_helix_backward();
                }
                VirtualKeyCode::K => {
                    self.data.borrow_mut().move_helix_forward();
                }
                VirtualKeyCode::X => self.data.borrow_mut().merge_strand(0, 1),
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
