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
use crate::dialog::Filters;

use super::{dialog, messages, MainState, State, TransitionMessage, YesNo};

use dialog::PathInput;
use ensnano_exports::ExportType;
use std::path::Path;

pub(super) struct Quit {
    step: QuitStep,
}

enum QuitStep {
    Init {
        /// None if there is no need to save
        /// Some(Some(path)) if there is a need to save at a known path
        /// Some(None) if there is a need to save at an unkonwn path
        need_save: Option<Option<PathBuf>>,
    },
    Quitting,
}

impl Quit {
    fn quitting() -> Self {
        Self {
            step: QuitStep::Quitting,
        }
    }

    pub fn quit(need_save: Option<Option<PathBuf>>) -> Box<Self> {
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

fn init_quit(need_save: Option<Option<PathBuf>>) -> Box<dyn State> {
    if let Some(path) = need_save {
        let quitting = Box::new(Quit::quitting());
        Box::new(YesNo::new(
            messages::SAVE_BEFORE_EXIT,
            save_before_quit(path),
            quitting,
        ))
    } else {
        Box::new(Quit::quitting())
    }
}

fn save_before_quit(path: Option<PathBuf>) -> Box<dyn State> {
    let on_success = Box::new(Quit::quitting());
    let on_error = Box::new(super::NormalState);
    if let Some(path) = path {
        Box::new(SaveWithPath {
            path,
            on_error,
            on_success,
        })
    } else {
        Box::new(SaveAs::new(on_success, on_error))
    }
}

pub(super) struct Load {
    step: LoadStep,
    load_type: LoadType,
}

impl Load {
    pub(super) fn known_path(path: PathBuf) -> Self {
        Self {
            step: LoadStep::GotPath(path),
            load_type: LoadType::Design,
        }
    }

    pub(super) fn init_reolad(
        need_save: Option<Option<PathBuf>>,
        path_to_load: PathBuf,
    ) -> Box<dyn State> {
        if let Some(save_path) = need_save {
            let yes = save_before_known_path(save_path, path_to_load.clone());
            let no = Box::new(Load::known_path(path_to_load));
            Box::new(YesNo::new(messages::SAVE_BEFORE_RELOAD, yes, no))
        } else {
            Box::new(Load::known_path(path_to_load))
        }
    }
}

use std::path::PathBuf;
enum LoadStep {
    Init { need_save: Option<Option<PathBuf>> },
    AskPath { path_input: Option<PathInput> },
    GotPath(PathBuf),
}

#[derive(Copy, Clone)]
pub(super) enum LoadType {
    Design,
    Object3D,
    SvgPath,
}

impl Load {
    fn ask_path(load_type: LoadType) -> Box<Self> {
        Box::new(Self {
            step: LoadStep::AskPath { path_input: None },
            load_type,
        })
    }

    pub fn load(need_save: Option<Option<PathBuf>>, load_type: LoadType) -> Box<Self> {
        Box::new(Self {
            step: LoadStep::Init { need_save },
            load_type,
        })
    }
}

impl State for Load {
    fn make_progress(self: Box<Self>, state: &mut dyn MainState) -> Box<dyn State> {
        match self.step {
            LoadStep::Init { need_save } => init_load(need_save, self.load_type),
            LoadStep::AskPath { path_input } => ask_path(
                path_input,
                state.get_current_design_directory(),
                self.load_type,
            ),
            LoadStep::GotPath(path) => match self.load_type {
                LoadType::Design => load_design(path, state),
                LoadType::Object3D => load_3d_object(path, state),
                LoadType::SvgPath => load_svg(path, state),
            },
        }
    }
}

fn init_load(path_to_save: Option<Option<PathBuf>>, load_type: LoadType) -> Box<dyn State> {
    if let Some(path_to_save) = path_to_save {
        let yes = save_before_load(path_to_save, load_type);
        let no = Load::ask_path(load_type);
        Box::new(YesNo::new(messages::SAVE_BEFORE_LOAD, yes, no))
    } else {
        Load::ask_path(load_type)
    }
}

fn save_before_load(path_to_save: Option<PathBuf>, load_type: LoadType) -> Box<dyn State> {
    let on_success = Load::ask_path(load_type);
    let on_error = Box::new(super::NormalState);
    if let Some(path) = path_to_save {
        Box::new(SaveWithPath {
            path,
            on_error,
            on_success,
        })
    } else {
        Box::new(SaveAs::new(on_success, on_error))
    }
}

fn save_before_known_path(path_to_save: Option<PathBuf>, path_to_load: PathBuf) -> Box<dyn State> {
    let on_success = Box::new(Load::known_path(path_to_load));
    let on_error = Box::new(NormalState);
    if let Some(path) = path_to_save {
        Box::new(SaveWithPath {
            path,
            on_success,
            on_error,
        })
    } else {
        Box::new(SaveAs::new(on_success, on_error))
    }
}

fn ask_path<P: AsRef<Path>>(
    path_input: Option<PathInput>,
    starting_directory: Option<P>,
    load_type: LoadType,
) -> Box<dyn State> {
    if let Some(path_input) = path_input {
        if let Some(result) = path_input.get() {
            if let Some(path) = result {
                Box::new(Load {
                    step: LoadStep::GotPath(path),
                    load_type,
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
                load_type,
            })
        }
    } else {
        let filters = match load_type {
            LoadType::Object3D => messages::OBJECT3D_FILTERS,
            LoadType::Design => messages::DESIGN_LOAD_FILTER,
            LoadType::SvgPath => messages::SVG_FILTERS,
        };
        let path_input = dialog::load(starting_directory, filters);
        Box::new(Load {
            step: LoadStep::AskPath {
                path_input: Some(path_input),
            },
            load_type,
        })
    }
}

fn load_design(path: PathBuf, state: &mut dyn MainState) -> Box<dyn State> {
    if let Err(err) = state.load_design(path) {
        TransitionMessage::new(
            format!("Error when loading design:\n{err}"),
            rfd::MessageLevel::Error,
            Box::new(super::NormalState),
        )
    } else {
        Box::new(super::NormalState)
    }
}

fn load_3d_object(path: PathBuf, state: &mut dyn MainState) -> Box<dyn State> {
    state.load_3d_object(path);
    Box::new(super::NormalState)
}

fn load_svg(path: PathBuf, state: &mut dyn MainState) -> Box<dyn State> {
    state.load_svg(path);
    Box::new(super::NormalState)
}

pub(super) struct NewDesign {
    step: NewStep,
}

enum NewStep {
    Init { need_save: Option<Option<PathBuf>> },
    MakeNewDesign,
}

impl NewDesign {
    pub fn init(need_save: Option<Option<PathBuf>>) -> Self {
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
                if let Some(path) = need_save {
                    init_new_design(path)
                } else {
                    new_design(main_state)
                }
            }
            NewStep::MakeNewDesign => new_design(main_state),
        }
    }
}

fn init_new_design(path_to_save: Option<PathBuf>) -> Box<dyn State> {
    let yes = save_before_new(path_to_save);
    let no = NewDesign::make_new_design();
    Box::new(YesNo::new(messages::SAVE_BEFORE_NEW, yes, no))
}

fn new_design(main_state: &mut dyn MainState) -> Box<dyn State> {
    main_state.new_design();
    Box::new(super::NormalState)
}

fn save_before_new(path_to_save: Option<PathBuf>) -> Box<dyn State> {
    let on_success = NewDesign::make_new_design();
    let on_error = Box::new(super::NormalState);
    if let Some(path) = path_to_save {
        Box::new(SaveWithPath {
            on_success,
            on_error,
            path,
        })
    } else {
        Box::new(SaveAs::new(on_success, on_error))
    }
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
            let getter = dialog::get_file_to_write(
                &messages::DESIGN_WRITE_FILTER,
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

pub(super) struct Exporting {
    file_getter: Option<PathInput>,
    on_success: Box<dyn State>,
    on_error: Box<dyn State>,
    export_type: ExportType,
}

impl Exporting {
    pub(super) fn new(
        on_success: Box<dyn State>,
        on_error: Box<dyn State>,
        export_type: ExportType,
    ) -> Self {
        Self {
            file_getter: None,
            on_success,
            on_error,
            export_type,
        }
    }
}

impl State for Exporting {
    fn make_progress(mut self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        if let Some(ref getter) = self.file_getter {
            if let Some(path_opt) = getter.get() {
                if let Some(ref path) = path_opt {
                    match main_state.export(path, self.export_type) {
                        Err(err) => TransitionMessage::new(
                            messages::failed_to_save_msg(&err),
                            rfd::MessageLevel::Error,
                            self.on_error,
                        ),
                        Ok(success) => TransitionMessage::new(
                            success.message(),
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
            let candidate_name = main_state.get_current_file_name().map(|p| {
                let mut ret = p.to_owned();
                ret.set_extension(export_extenstion(self.export_type));
                ret
            });
            let getter = dialog::get_file_to_write(
                export_filters(self.export_type),
                main_state.get_current_design_directory(),
                candidate_name,
            );
            self.file_getter = Some(getter);
            self
        }
    }
}

fn export_extenstion(export_type: ExportType) -> &'static str {
    match export_type {
        ExportType::Oxdna => messages::OXDNA_CONFIG_EXTENSTION,
        ExportType::Pdb => "pdb",
        ExportType::Cadnano => "json",
        ExportType::Cando => "cndo",
    }
}

fn export_filters(export_type: ExportType) -> &'static Filters {
    match export_type {
        ExportType::Oxdna => &messages::OXDNA_CONFIG_FILTERS,
        ExportType::Pdb => &messages::PDB_FILTER,
        ExportType::Cadnano => &messages::CADNANO_FILTER,
        ExportType::Cando => todo!(),
    }
}
