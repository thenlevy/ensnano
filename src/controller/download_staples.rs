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

use super::{Arc, MainState, Mutex, NormalState, State, TransitionMessage};

use crate::dialog;
use dialog::{MustAckMessage, PathInput, YesNoQuestion};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct DownloadStaples {
    step: Step,
}

enum Step {
    /// The staple downloading request has just started
    Init,
    /// Asking the user where to write the result
    AskingPath(AskingPath_),
    /// The path was asked, waiting for user to chose it
    PathAsked {
        path_input: dialog::PathInput,
        design_id: usize,
    },
    /// Downloading
    Downloading { design_id: usize, path: PathBuf },
}

impl Default for Step {
    fn default() -> Self {
        Self::Init
    }
}

impl State for DownloadStaples {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        let downloader = main_state.get_staple_downloader();
        match self.step {
            Step::Init => get_design_providing_staples(downloader.as_ref()),
            Step::AskingPath(state) => ask_path(state, main_state.get_current_design_directory()),
            Step::PathAsked {
                path_input,
                design_id,
            } => poll_path(path_input, design_id),
            Step::Downloading { design_id, path } => {
                download_staples(downloader.as_ref(), design_id, path)
            }
        }
    }

    /*
    let result = mediator.download_stapples();
    match result {
        Ok(DownloadStappleOk {
            design_id,
            warnings,
        }) => {
            for warn in warnings {
                requests
                    .keep_proceed
                    .push_back(Action::Warning(warn.dialog()))
            }
            requests
                .keep_proceed
                .push_back(Action::AskStaplesPath { d_id: design_id })
        }
        Err(DownloadStappleError::NoScaffoldSet) => {
            message(
                "No scaffold set. \n
                    Chose a strand and set it as the scaffold by checking the scaffold checkbox\
                    in the status bar"
                    .into(),
                rfd::MessageLevel::Error,
            );
        }
        Err(DownloadStappleError::ScaffoldSequenceNotSet) => {
            message(
                "No sequence uploaded for scaffold. \n
                Upload a sequence for the scaffold by pressing the \"Load scaffold\" button"
                    .into(),
                rfd::MessageLevel::Error,
            );
        }
        Err(DownloadStappleError::SeveralDesignNoneSelected) => {
            message(
                "No design selected, select a design by selecting one of its elements".into(),
                rfd::MessageLevel::Error,
            );
        }
    }
    */
}

fn get_design_providing_staples(downlader: &dyn StaplesDownloader) -> Box<dyn State> {
    let result = downlader.download_staples();
    match result {
        Ok(DownloadStappleOk { warnings }) => AskingPath_ {
            warnings,
            design_id: 0,
            warning_ack: None,
        }
        .to_state(),
        Err(DownloadStappleError::NoScaffoldSet) => {
            let message = "No scaffold set. \n
                    Chose a strand and set it as the scaffold by checking the scaffold checkbox\
                    in the status bar"
                .into();
            TransitionMessage::new(message, rfd::MessageLevel::Error, Box::new(NormalState))
        }
        Err(DownloadStappleError::ScaffoldSequenceNotSet) => {
            let message = "No sequence uploaded for scaffold. \n
                Upload a sequence for the scaffold by pressing the \"Load scaffold\" button"
                .into();
            TransitionMessage::new(message, rfd::MessageLevel::Error, Box::new(NormalState))
        }
        Err(DownloadStappleError::SeveralDesignNoneSelected) => {
            let message =
                "No design selected, select a design by selecting one of its elements".into();
            TransitionMessage::new(message, rfd::MessageLevel::Error, Box::new(NormalState))
        }
    }
}

fn ask_path<P: AsRef<Path>>(
    mut state: AskingPath_,
    starting_diectory: Option<P>,
) -> Box<DownloadStaples> {
    if let Some(must_ack) = state.warning_ack.as_ref() {
        if !must_ack.was_ack() {
            return Box::new(DownloadStaples {
                step: Step::AskingPath(state),
            });
        }
    }
    if let Some(msg) = state.warnings.pop() {
        let must_ack = dialog::blocking_message(msg.into(), rfd::MessageLevel::Warning);
        state.with_ack(must_ack)
    } else {
        let path_input = dialog::save("xlsx", starting_diectory);
        Box::new(DownloadStaples {
            step: Step::PathAsked {
                path_input,
                design_id: state.design_id,
            },
        })
    }
}

struct AskingPath_ {
    warnings: Vec<String>,
    design_id: usize,
    warning_ack: Option<MustAckMessage>,
}

impl AskingPath_ {
    fn to_state(self) -> Box<DownloadStaples> {
        Box::new(DownloadStaples {
            step: Step::AskingPath(self),
        })
    }

    fn with_ack(mut self, ack: MustAckMessage) -> Box<DownloadStaples> {
        self.warning_ack = Some(ack);
        self.to_state()
    }
}

fn poll_path(path_input: PathInput, design_id: usize) -> Box<dyn State> {
    if let Some(result) = path_input.get() {
        if let Some(path) = result {
            Box::new(DownloadStaples {
                step: Step::Downloading { path, design_id },
            })
        } else {
            let message = "No target file recieved".to_string();
            TransitionMessage::new(message, rfd::MessageLevel::Error, Box::new(NormalState))
        }
    } else {
        Box::new(DownloadStaples {
            step: Step::PathAsked {
                path_input,
                design_id,
            },
        })
    }
}

fn download_staples(
    downlader: &dyn StaplesDownloader,
    design_id: usize,
    path: PathBuf,
) -> Box<dyn State> {
    downlader.write_staples_xlsx(&path);
    let msg = format!(
        "Successfully wrote staples in {}",
        path.clone().to_string_lossy()
    );
    TransitionMessage::new(msg, rfd::MessageLevel::Error, Box::new(NormalState))
}

pub trait StaplesDownloader {
    fn download_staples(&self) -> Result<DownloadStappleOk, DownloadStappleError>;
    fn write_staples_xlsx(&self, xlsx_path: &PathBuf);
    fn default_shift(&self) -> Option<usize>;
}

pub enum DownloadStappleError {
    /// There are several designs and none is selected.
    SeveralDesignNoneSelected,
    /// No strand is set as the scaffold
    NoScaffoldSet,
    /// There is no sequence set for the scaffold
    ScaffoldSequenceNotSet,
}

pub struct DownloadStappleOk {
    pub warnings: Vec<String>,
}
