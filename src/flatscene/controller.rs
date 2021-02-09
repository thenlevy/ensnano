//! The `Controller` struct handles the event that happens on the drawing area of the scene.
//!
//! The `Controller` is internally implemented as a finite automata that transitions when a event
//! happens. In addition to the transistion in the automat, a `Consequence` is returned to the
//! scene, that describes the consequences that the input must have on the view or the data held by
//! the scene.
use super::data::FreeEnd;
use super::{
    ActionMode, Arc, CameraPtr, DataPtr, FlatHelix, FlatNucl, Mediator, Mutex, PhySize,
    PhysicalPosition, ViewPtr, WindowEvent,
};
use crate::design::StrandBuilder;
use iced_winit::winit::event::*;
use std::cell::RefCell;
use ultraviolet::Vec2;

mod automata;
use automata::{ControllerState, NormalState, Transition};

pub struct Controller {
    #[allow(dead_code)]
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    window_size: PhySize,
    area_size: PhySize,
    pub camera: CameraPtr,
    state: RefCell<Box<dyn ControllerState>>,
    action_mode: ActionMode,
    mediator: Arc<Mutex<Mediator>>,
    pasting: bool,
}

pub enum Consequence {
    #[allow(dead_code)]
    GlobalsChanged,
    Nothing,
    Xover(FlatNucl, FlatNucl),
    Cut(FlatNucl),
    CutCross(FlatNucl, FlatNucl),
    FreeEnd(Option<FreeEnd>),
    CutFreeEnd(FlatNucl, Option<FreeEnd>),
    NewCandidate(Option<FlatNucl>),
    RmStrand(FlatNucl),
    RmHelix(FlatHelix),
    FlipVisibility(FlatHelix, bool),
    Built(Box<StrandBuilder>),
    FlipGroup(FlatHelix),
    FollowingSuggestion(FlatNucl, bool),
    Centering(FlatNucl),
    Select(FlatNucl),
    DrawingSelection(PhysicalPosition<f64>, PhysicalPosition<f64>),
    ReleasedSelection(Vec2, Vec2),
    PasteRequest(FlatNucl),
}

impl Controller {
    pub fn new(
        view: ViewPtr,
        data: DataPtr,
        window_size: PhySize,
        area_size: PhySize,
        camera: CameraPtr,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Self {
        Self {
            view,
            data,
            window_size,
            area_size,
            camera,
            state: RefCell::new(Box::new(NormalState {
                mouse_position: PhysicalPosition::new(-1., -1.),
                pasting: false,
            })),
            action_mode: ActionMode::Normal,
            mediator,
            pasting: false,
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
        let transition = if let WindowEvent::Focused(false) = event {
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: PhysicalPosition::new(-1., -1.),
                    pasting: self.pasting,
                })),
                consequences: Consequence::Nothing,
            }
        } else {
            self.state.borrow_mut().input(event, position, self)
        };

        if let Some(state) = transition.new_state {
            println!("{}", state.display());
            self.state.borrow().transition_from(&self);
            self.state = RefCell::new(state);
            self.state.borrow().transition_to(&self);
        }
        transition.consequences
    }

    pub fn set_pasting(&mut self, pasting: bool) {
        self.pasting = pasting;
        let transition = Transition {
            new_state: Some(Box::new(NormalState {
                mouse_position: PhysicalPosition::new(-1., -1.),
                pasting: self.pasting,
            })),
            consequences: Consequence::Nothing,
        };
        self.state.borrow().transition_from(&self);
        self.state = RefCell::new(transition.new_state.unwrap());
        self.state.borrow().transition_to(&self);
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
                    modifiers,
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
                VirtualKeyCode::Z if modifiers.ctrl() => self.mediator.lock().unwrap().undo(),
                VirtualKeyCode::R if modifiers.ctrl() => self.mediator.lock().unwrap().redo(),
                _ => (),
            }
        }
    }
}
