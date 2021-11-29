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

use crate::controller::normal_state::NormalState;

use super::{dialog, messages, MainState, State, TransitionMessage, YesNo};

use dialog::PathInput;
use std::path::Path;

pub(super) struct Quit {
    step: QuitStep,
}

enum QuitStep {
    Init { need_save: bool },
    Quitting,
}

impl Quit {
    fn quitting() -> Self {
        Self {
            step: QuitStep::Quitting,
        }
    }

    pub fn quit(need_save: bool) -> Box<Self> {
        Box::new(Self {
            step: QuitStep::Init { need_save },
        })
    }
}

impl State for Quit {
    fn make_progress(self: Box<Self>, pending_action: &mut dyn MainState) -> Box<dyn State> {
        match self.step {
            QuitStep::Init { need_save } => init_quit(need_save),
            QuitStep::Quitting => {
                pending_action.exit_control_flow();
                Box::new(super::NormalState)
            }
        }
    }
}

fn init_quit(need_save: bool) -> Box<dyn State> {
    if need_save {
        let quitting = Box::new(Quit::quitting());
        Box::new(YesNo::new(
            messages::SAVE_BEFORE_EXIT,
            save_before_quit(),
            quitting,
        ))
    } else {
        Box::new(Quit::quitting())
    }
}

fn save_before_quit() -> Box<dyn State> {
    let on_success = Box::new(Quit::quitting());
    let on_error = Box::new(super::NormalState);
    Box::new(SaveAs::new(on_success, on_error))
}

pub(super) struct Load {
    step: LoadStep,
}

impl Load {
    pub(super) fn known_path(path: PathBuf) -> Self {
        Self {
            step: LoadStep::GotPath(path),
        }
    }

    pub(super) fn init_reolad(need_save: bool, path: PathBuf) -> Box<dyn State> {
        if need_save {
            let yes = save_before_known_path(path.clone());
            let no = Box::new(Load::known_path(path));
            Box::new(YesNo::new(messages::SAVE_BEFORE_RELOAD, yes, no))
        } else {
            Box::new(Load::known_path(path))
        }
    }
}

use std::path::PathBuf;
enum LoadStep {
    Init { need_save: bool },
    AskPath { path_input: Option<PathInput> },
    GotPath(PathBuf),
}

impl Load {
    fn ask_path() -> Box<Self> {
        Box::new(Self {
            step: LoadStep::AskPath { path_input: None },
        })
    }

    pub fn load(need_save: bool) -> Box<Self> {
        Box::new(Self {
            step: LoadStep::Init { need_save },
        })
    }
}

impl State for Load {
    fn make_progress(self: Box<Self>, state: &mut dyn MainState) -> Box<dyn State> {
        match self.step {
            LoadStep::Init { need_save } => init_load(need_save),
            LoadStep::AskPath { path_input } => {
                ask_path(path_input, state.get_current_design_directory())
            }
            LoadStep::GotPath(path) => load(path, state),
        }
    }
}

fn init_load(need_save: bool) -> Box<dyn State> {
    if need_save {
        let yes = save_before_load();
        let no = Load::ask_path();
        Box::new(YesNo::new(messages::SAVE_BEFORE_LOAD, yes, no))
    } else {
        Load::ask_path()
    }
}

fn save_before_load() -> Box<dyn State> {
    let on_success = Load::ask_path();
    let on_error = Box::new(super::NormalState);
    Box::new(SaveAs::new(on_success, on_error))
}

fn save_before_known_path(path: PathBuf) -> Box<dyn State> {
    let on_success = Box::new(Load::known_path(path));
    let on_error = Box::new(NormalState);
    Box::new(SaveAs::new(on_success, on_error))
}

fn ask_path<P: AsRef<Path>>(
    path_input: Option<PathInput>,
    starting_directory: Option<P>,
) -> Box<dyn State> {
    if let Some(path_input) = path_input {
        if let Some(result) = path_input.get() {
            if let Some(path) = result {
                Box::new(Load {
                    step: LoadStep::GotPath(path),
                })
            } else {
                TransitionMessage::new(
                    messages::NO_FILE_RECIEVED_LOAD,
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
        let path_input = dialog::load(starting_directory, messages::DESIGN_FILTERS);
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
    Init { need_save: bool },
    MakeNewDesign,
}

impl NewDesign {
    pub fn init(need_save: bool) -> Self {
        Self {
            step: NewStep::Init { need_save },
        }
    }

    fn make_new_design() -> Box<dyn State> {
        Box::new(Self {
            step: NewStep::MakeNewDesign,
        })
    }
}

impl State for NewDesign {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        match self.step {
            NewStep::Init { need_save } => {
                if need_save {
                    init_new_design()
                } else {
                    new_design(main_state)
                }
            }
            NewStep::MakeNewDesign => new_design(main_state),
        }
    }
}

fn init_new_design() -> Box<dyn State> {
    let yes = save_before_new();
    let no = NewDesign::make_new_design();
    Box::new(YesNo::new(messages::SAVE_BEFORE_NEW, yes, no))
}

fn new_design(main_state: &mut dyn MainState) -> Box<dyn State> {
    main_state.new_design();
    Box::new(super::NormalState)
}

fn save_before_new() -> Box<dyn State> {
    let on_success = NewDesign::make_new_design();
    let on_error = Box::new(super::NormalState);
    Box::new(SaveAs::new(on_success, on_error))
}

pub(super) struct SaveAs {
    file_getter: Option<PathInput>,
    on_success: Box<dyn State>,
    on_error: Box<dyn State>,
}

impl SaveAs {
    pub(super) fn new(on_success: Box<dyn State>, on_error: Box<dyn State>) -> Self {
        Self {
            file_getter: None,
            on_success,
            on_error,
        }
    }
}

impl State for SaveAs {
    fn make_progress(mut self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        if let Some(ref getter) = self.file_getter {
            if let Some(path_opt) = getter.get() {
                if let Some(ref path) = path_opt {
                    if let Err(err) = main_state.save_design(path) {
                        TransitionMessage::new(
                            format!("Failed to save: {:?}", err.0),
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
                        messages::NO_FILE_RECIEVED_SAVE,
                        rfd::MessageLevel::Error,
                        Box::new(super::NormalState),
                    )
                }
            } else {
                self
            }
        } else {
            let getter = dialog::save(
                crate::consts::ENS_EXTENSION,
                main_state.get_current_design_directory(),
                main_state.get_current_file_name(),
            );
            self.file_getter = Some(getter);
            self
        }
    }
}

pub(super) struct SaveWithPath {
    pub path: PathBuf,
    pub on_error: Box<dyn State>,
    pub on_success: Box<dyn State>,
}

impl State for SaveWithPath {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        if let Err(err) = main_state.save_design(&self.path) {
            TransitionMessage::new(
                format!("Failed to save: {:?}", err.0),
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
    }
}

pub(super) struct OxDnaExport {
    file_getter: Option<PathInput>,
    on_success: Box<dyn State>,
    on_error: Box<dyn State>,
}

impl OxDnaExport {
    pub(super) fn new(on_success: Box<dyn State>, on_error: Box<dyn State>) -> Self {
        Self {
            file_getter: None,
            on_success,
            on_error,
        }
    }
}

impl State for OxDnaExport {
    fn make_progress(mut self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        if let Some(ref getter) = self.file_getter {
            if let Some(path_opt) = getter.get() {
                if let Some(ref path) = path_opt {
                    match main_state.oxdna_export(path) {
                        Err(err) => TransitionMessage::new(
                            messages::failed_to_save_msg(&err),
                            rfd::MessageLevel::Error,
                            self.on_error,
                        ),
                        Ok((config, topo)) => TransitionMessage::new(
                            messages::succesfull_oxdna_export_msg(config, topo),
                            rfd::MessageLevel::Info,
                            self.on_success,
                        ),
                    }
                } else {
                    TransitionMessage::new(
                        messages::NO_FILE_RECIEVED_OXDNA,
                        rfd::MessageLevel::Error,
                        self.on_error,
                    )
                }
            } else {
                self
            }
        } else {
            let getter = dialog::get_dir();
            self.file_getter = Some(getter);
            self
        }
    }
}
