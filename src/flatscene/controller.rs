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
//! The `Controller` struct handles the event that happens on the drawing area of the scene.
//!
//! The `Controller` is internally implemented as a finite automata that transitions when a event
//! happens. In addition to the transistion in the automat, a `Consequence` is returned to the
//! scene, that describes the consequences that the input must have on the view or the data held by
//! the scene.
use self::automata::ReleasedPivot;

use super::data::{ClickResult, FreeEnd};
use super::{
    ActionMode, AppState, CameraPtr, DataPtr, FlatHelix, FlatNucl, PhySize, PhysicalPosition,
    Selection, ViewPtr, WindowEvent,
};

use iced_winit::winit::event::*;
use std::cell::RefCell;
use ultraviolet::Vec2;

mod automata;
use automata::{ControllerState, NormalState, Transition};

pub struct Controller<S: AppState> {
    #[allow(dead_code)]
    view: ViewPtr,
    data: DataPtr,
    #[allow(dead_code)]
    window_size: PhySize,
    area_size: PhySize,
    camera_top: CameraPtr,
    camera_bottom: CameraPtr,
    splited: bool,
    state: RefCell<Box<dyn ControllerState<S>>>,
    action_mode: ActionMode,
    modifiers: ModifiersState,
}

#[derive(Debug)]
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
    NewHelixCandidate(FlatHelix),
    RmStrand(FlatNucl),
    RmHelix(FlatHelix),
    FlipVisibility(FlatHelix, bool),
    Built,
    FlipGroup(FlatHelix),
    FollowingSuggestion(FlatNucl, bool),
    Centering(FlatNucl, bool),
    DrawingSelection(PhysicalPosition<f64>, PhysicalPosition<f64>),
    ReleasedSelection(Option<Vec<Selection>>),
    PasteRequest(Option<FlatNucl>),
    AddClick(ClickResult, bool),
    SelectionChanged(Vec<Selection>),
    ClearSelection,
    DoubleClick(ClickResult),
    MoveBuilders(isize),
    InitBuilding(FlatNucl),
    Helix2DMvmtEnded,
    Snap {
        pivots: Vec<FlatNucl>,
        translation: Vec2,
    },
    Rotation {
        helices: Vec<FlatHelix>,
        center: Vec2,
        angle: f32,
    },
}

impl<S: AppState> Controller<S> {
    pub fn new(
        view: ViewPtr,
        data: DataPtr,
        window_size: PhySize,
        area_size: PhySize,
        camera_top: CameraPtr,
        camera_bottom: CameraPtr,
        splited: bool,
    ) -> Self {
        Self {
            view,
            data,
            window_size,
            area_size,
            camera_top,
            camera_bottom,
            state: RefCell::new(Box::new(NormalState {
                mouse_position: PhysicalPosition::new(-1., -1.),
            })),
            splited,
            action_mode: ActionMode::Normal,
            modifiers: ModifiersState::empty(),
        }
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn resize(&mut self, window_size: PhySize, area_size: PhySize) {
        self.area_size = area_size;
        self.window_size = window_size;
        self.update_globals();
    }

    pub fn set_splited(&mut self, splited: bool) {
        self.splited = splited;
        self.update_globals();
    }

    fn update_globals(&mut self) {
        if self.splited {
            self.camera_top.borrow_mut().resize(
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
            );
            self.camera_bottom.borrow_mut().resize(
                self.area_size.width as f32,
                self.area_size.height as f32 / 2.,
            );
        } else {
            self.camera_top
                .borrow_mut()
                .resize(self.area_size.width as f32, self.area_size.height as f32);
        }
    }

    pub fn get_camera(&self, y: f64) -> CameraPtr {
        if self.splited {
            if y > self.area_size.height as f64 / 2. {
                self.camera_bottom.clone()
            } else {
                self.camera_top.clone()
            }
        } else {
            self.camera_top.clone()
        }
    }

    pub fn fit(&mut self) {
        let rectangle = self.data.borrow().get_fit_rectangle();
        self.camera_top.borrow_mut().fit(rectangle);
        self.camera_bottom.borrow_mut().fit(rectangle);
    }

    pub fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        app_state: &S,
    ) -> Consequence {
        let transition = if let WindowEvent::Focused(false) = event {
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: PhysicalPosition::new(-1., -1.),
                })),
                consequences: Consequence::Nothing,
            }
        } else {
            self.state
                .borrow_mut()
                .input(event, position, self, app_state)
        };

        if let Some(state) = transition.new_state {
            log::info!("2D automata state: {}", state.display());
            self.state.borrow().transition_from(&self);
            self.state = RefCell::new(state);
            self.state.borrow().transition_to(&self);
        }
        transition.consequences
    }

    pub fn select_pivots(&mut self, translation_pivots: Vec<FlatNucl>, rotation_pivots: Vec<Vec2>) {
        let transition = Transition {
            new_state: Some(Box::new(ReleasedPivot {
                translation_pivots,
                rotation_pivots,
                mouse_position: PhysicalPosition::new(-1., -1.),
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
                    ..
                },
            ..
        } = event
        {
            match *key {
                // ZOOMING in and out is temporarilly disabled because of split view
                /*
                VirtualKeyCode::Up => {
                    self.camera.borrow_mut().zoom_in();
                }
                VirtualKeyCode::Down => {
                    self.camera.borrow_mut().zoom_out();
                }
                */
                VirtualKeyCode::J => {
                    self.data.borrow_mut().move_helix_backward();
                }
                VirtualKeyCode::K => {
                    self.data.borrow_mut().move_helix_forward();
                }
                _ => (),
            }
        }
    }

    fn end_movement(&self) {
        self.camera_top.borrow_mut().end_movement();
        self.camera_bottom.borrow_mut().end_movement();
    }

    fn get_height(&self) -> u32 {
        if self.splited {
            self.area_size.height / 2
        } else {
            self.area_size.height
        }
    }

    fn is_bottom(&self, y: f64) -> bool {
        if self.splited {
            y > self.area_size.height as f64 / 2.
        } else {
            false
        }
    }

    pub fn check_timers(&mut self) -> Consequence {
        let transition = self.state.borrow_mut().check_timers(&self);
        if let Some(state) = transition.new_state {
            println!("{}", state.display());
            self.state.borrow().transition_from(&self);
            self.state = RefCell::new(state);
            self.state.borrow().transition_to(&self);
        }
        transition.consequences
    }
}

fn ctrl(modifiers: &ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.logo()
    } else {
        modifiers.ctrl()
    }
}
