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

use super::{dialog, Action, Arc, MainState, Mediator, Mutex, State, TransitionMessage, YesNo};

use dialog::{yes_no_dialog, PathInput, YesNoQuestion};

#[derive(Default)]
pub(super) struct Quit {
    step: QuitStep,
}

enum QuitStep {
    Init,
    Quitting,
}

impl Default for QuitStep {
    fn default() -> Self {
        Self::Init
    }
}

impl Quit {
    fn quitting() -> Self {
        Self {
            step: QuitStep::Quitting,
        }
    }
}

impl State for Quit {
    fn make_progress(
        self: Box<Self>,
        pending_action: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        match self.step {
            QuitStep::Init => init_quit(),
            QuitStep::Quitting => {
                pending_action.exit_control_flow();
                Box::new(super::NormalState)
            }
        }
    }
}

fn init_quit() -> Box<dyn State> {
    let quitting = Box::new(Quit::quitting());
    Box::new(YesNo::new(
        "Do you want to save your design before exiting the program ?".into(),
        save_before_quit(),
        quitting,
    ))
}

fn save_before_quit() -> Box<dyn State> {
    let on_success = Box::new(Quit::quitting());
    let on_error = Box::new(super::NormalState);
    Box::new(Save::new(on_success, on_error))
}

#[derive(Default)]
pub(super) struct Load {
    step: LoadStep,
}

use std::path::PathBuf;
enum LoadStep {
    Init,
    AskPath { path_input: Option<PathInput> },
    GotPath(PathBuf),
}

impl Default for LoadStep {
    fn default() -> Self {
        Self::Init
    }
}

impl Load {
    fn ask_path() -> Box<Self> {
        Box::new(Self {
            step: LoadStep::AskPath { path_input: None },
        })
    }
}

impl State for Load {
    fn make_progress(
        self: Box<Self>,
        state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        match self.step {
            LoadStep::Init => init_load(),
            LoadStep::AskPath { path_input } => ask_path(path_input),
            LoadStep::GotPath(path) => load(path, state),
        }
    }
}

fn init_load() -> Box<dyn State> {
    let yes = save_before_load();
    let no = Load::ask_path();
    Box::new(YesNo::new(
        "Do you want to save the current design beore loading a new one?".into(),
        yes,
        no,
    ))
}

fn save_before_load() -> Box<dyn State> {
    let on_success = Load::ask_path();
    let on_error = Box::new(super::NormalState);
    Box::new(Save::new(on_success, on_error))
}

fn ask_path(path_input: Option<PathInput>) -> Box<dyn State> {
    if let Some(path_input) = path_input {
        if let Some(result) = path_input.get() {
            if let Some(path) = result {
                Box::new(Load {
                    step: LoadStep::GotPath(path),
                })
            } else {
                TransitionMessage::new(
                    "Did not recieve any file to load".into(),
                    rfd::MessageLevel::Error,
                    Box::new(super::NormalState),
                )
            }
        } else {
            Box::new(Load {
                step: LoadStep::AskPath {
                    path_input: Some(path_input),
                },
            })
        }
    } else {
        let path_input = dialog::load();
        Box::new(Load {
            step: LoadStep::AskPath {
                path_input: Some(path_input),
            },
        })
    }
}

fn load(path: PathBuf, state: &mut dyn MainState) -> Box<dyn State> {
    if let Err(err) = state.load_design(path) {
        TransitionMessage::new(
            format!("Error when loading design: {}", err.0),
            rfd::MessageLevel::Error,
            Box::new(super::NormalState),
        )
    } else {
        Box::new(super::NormalState)
    }
}

pub(super) struct NewDesign {
    step: NewStep,
}

enum NewStep {
    Init,
    MakeNewDesign,
}

impl NewDesign {
    fn make_new_design() -> Box<dyn State> {
        Box::new(Self {
            step: NewStep::MakeNewDesign,
        })
    }
}

impl State for NewDesign {
    fn make_progress(
        self: Box<Self>,
        main_state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        match self.step {
            NewStep::Init => init_new_design(),
            NewStep::MakeNewDesign => new_design(main_state),
        }
    }
}

fn init_new_design() -> Box<dyn State> {
    let yes = save_before_new();
    let no = NewDesign::make_new_design();
    Box::new(YesNo::new(
        "Do you want to save the current design before creating a new one?".into(),
        yes,
        no,
    ))
}

fn new_design(main_state: &mut dyn MainState) -> Box<dyn State> {
    main_state.new_design();
    Box::new(super::NormalState)
}

fn save_before_new() -> Box<dyn State> {
    let on_success = NewDesign::make_new_design();
    let on_error = Box::new(super::NormalState);
    Box::new(Save::new(on_success, on_error))
}

pub(super) struct Save {
    file_getter: Option<PathInput>,
    on_success: Box<dyn State>,
    on_error: Box<dyn State>,
}

impl Save {
    pub(super) fn new(on_success: Box<dyn State>, on_error: Box<dyn State>) -> Self {
        Self {
            file_getter: None,
            on_success,
            on_error,
        }
    }
}

impl State for Save {
    fn make_progress(
        mut self: Box<Self>,
        main_state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        if let Some(ref getter) = self.file_getter {
            if let Some(path_opt) = getter.get() {
                if let Some(ref path) = path_opt {
                    if let Err(err) = mediator.lock().unwrap().save_design(path) {
                        TransitionMessage::new(
                            format!("Failed to save: {:?}", err),
                            rfd::MessageLevel::Error,
                            self.on_error,
                        )
                    } else {
                        TransitionMessage::new(
                            "Saved successfully".to_string(),
                            rfd::MessageLevel::Info,
                            self.on_success,
                        )
                    }
                } else {
                    TransitionMessage::new(
                        "Error, did not recieve any file".to_string(),
                        rfd::MessageLevel::Error,
                        self.on_error,
                    )
                }
            } else {
                self
            }
        } else {
            let getter = dialog::save("json");
            self.file_getter = Some(getter);
            self
        }
    }
}
