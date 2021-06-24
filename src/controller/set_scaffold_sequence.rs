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

use super::{dialog, Arc, MainState, Mutex, State, TransitionMessage, YesNo};

use dialog::PathInput;

#[derive(Default)]
/// User is in the process of setting the sequence of the scaffold
pub(super) struct SetScaffoldSequence {
    step: Step,
}

impl Default for Step {
    fn default() -> Self {
        Self::Init
    }
}

impl SetScaffoldSequence {
    fn use_default() -> Self {
        let sequence = include_str!("p7249-Tilibit.txt").to_string();
        Self {
            step: Step::SetSequence(sequence),
        }
    }

    fn ask_path() -> Self {
        Self {
            step: Step::AskPath { path_input: None },
        }
    }
}

use std::path::PathBuf;
enum Step {
    /// The request to set the sequence of the scaffold has been acknowledged. User is asked to
    /// chose between the default m13 scaffold or a custom one.
    Init,
    /// The user has chosen to use a custom scaffold, and is asked a path the sequence file.
    AskPath { path_input: Option<PathInput> },
    /// The user has chosen a sequence file. The content of the file is checked.
    GotPath(PathBuf),
    /// The new sequence of the scaffold has been decided, user is asked if they want to optimize
    /// the starting position
    SetSequence(String),
    /// The user has chosen to optimize the scaffold position.
    OptimizeScaffoldPosition { design_id: usize },
}

impl State for SetScaffoldSequence {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        match self.step {
            Step::Init => init_set_scaffold_sequence(),
            Step::AskPath { path_input } => ask_path(path_input),
            Step::GotPath(path) => got_path(path),
            Step::SetSequence(seq) => set_sequence(seq, main_state),
            Step::OptimizeScaffoldPosition { design_id } => {
                optimize_scaffold_position(design_id, main_state)
            }
        }
    }
}

fn init_set_scaffold_sequence() -> Box<dyn State> {
    let yes = Box::new(SetScaffoldSequence::use_default());
    let no = Box::new(SetScaffoldSequence::ask_path());
    Box::new(YesNo::new("Use default m13 sequence?".into(), yes, no))
}

fn ask_path(path_input: Option<PathInput>) -> Box<dyn State> {
    if let Some(path_input) = path_input {
        if let Some(result) = path_input.get() {
            if let Some(path) = result {
                Box::new(SetScaffoldSequence {
                    step: Step::GotPath(path),
                })
            } else {
                TransitionMessage::new(
                    "Did not recieve any file to load".into(),
                    rfd::MessageLevel::Error,
                    Box::new(super::NormalState),
                )
            }
        } else {
            Box::new(SetScaffoldSequence {
                step: Step::AskPath {
                    path_input: Some(path_input),
                },
            })
        }
    } else {
        let path_input = dialog::load();
        Box::new(SetScaffoldSequence {
            step: Step::AskPath {
                path_input: Some(path_input),
            },
        })
    }
}

fn got_path(path: PathBuf) -> Box<dyn State> {
    let mut content = std::fs::read_to_string(path).unwrap();
    content.make_ascii_uppercase();
    if let Some(n) =
        content.find(|c: char| c != 'A' && c != 'T' && c != 'G' && c != 'C' && !c.is_whitespace())
    {
        let msg = format!(
            "This text file does not contain a valid DNA sequence.\n
             First invalid char at position {}",
            n
        );
        TransitionMessage::new(msg, rfd::MessageLevel::Error, Box::new(super::NormalState))
    } else {
        Box::new(SetScaffoldSequence {
            step: Step::SetSequence(content),
        })
    }
}

fn set_sequence(sequence: String, scaffold_setter: &mut dyn MainState) -> Box<dyn State> {
    let result = scaffold_setter.set_scaffold_sequence(sequence);
    match result {
        Ok(SetScaffoldSequenceOk { default_shift }) => {
            let message = format!("Optimize the scaffold position ?\n
              If you chose \"Yes\", ENSnano will position the scaffold in a way that minimizes the \
              number of anti-patern (G^4, C^4 (A|T)^7) in the stapples sequence. If you chose \"No\", \
              the scaffold sequence will begin at position {}", default_shift.unwrap_or(0));

            let yes = Box::new(SetScaffoldSequence {
                step: Step::OptimizeScaffoldPosition { design_id: 0 },
            });
            let no = Box::new(super::NormalState);
            Box::new(YesNo::new(message.into(), yes, no))
        }
        Err(err) => TransitionMessage::new(
            format!("{:?}", err),
            rfd::MessageLevel::Error,
            Box::new(super::NormalState),
        ),
    }
}

fn optimize_scaffold_position(design_id: usize, main_state: &mut dyn MainState) -> Box<dyn State> {
    main_state.optimize_shift();
    Box::new(super::NormalState)
}

pub trait ScaffoldSetter {
    fn set_scaffold_sequence(
        &mut self,
        sequence: String,
    ) -> Result<SetScaffoldSequenceOk, SetScaffoldSequenceError>;
    fn optimize_shift(&mut self);
}

use std::sync::mpsc;
pub trait ShiftOptimizerReader: Send {
    fn attach_progress_chanel(&mut self, chanel: mpsc::Receiver<f32>);
    fn attach_result_chanel(&mut self, chanel: mpsc::Receiver<ShiftOptimizationResult>);
}

pub struct ShiftOptimizationResult {
    pub position: usize,
    pub score: String,
}

pub struct SetScaffoldSequenceOk {
    pub default_shift: Option<usize>,
}

#[derive(Debug)]
pub struct SetScaffoldSequenceError(pub String);
