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

mod download_staples;
mod quit;
mod set_scaffold_sequence;

use super::Mediator;
use std::path::PathBuf;

use super::dialog;
use dialog::MustAckMessage;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

pub struct Controller {
    mediator: Arc<Mutex<Mediator>>,
    state: Box<dyn State + 'static>,
}

trait State {
    fn make_progress(
        self,
        main_state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State>;
}

struct NormalState;

impl State for NormalState {
    fn make_progress(
        self,
        main_state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        unimplemented!()
    }
}

/// Display a message that must be acknowledged by the user, and transition to a predetermined
/// state.
struct TransitionMessage {
    level: rfd::MessageLevel,
    content: String,
    ack: Option<MustAckMessage>,
    transistion_to: Box<dyn State>,
}

impl TransitionMessage {
    fn new(
        content: String,
        level: rfd::MessageLevel,
        transistion_to: Box<dyn State + 'static>,
    ) -> Box<Self> {
        Box::new(Self {
            level,
            content,
            ack: None,
            transistion_to,
        })
    }
}

impl State for TransitionMessage {
    fn make_progress(
        mut self,
        main_state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State + 'static> {
        if let Some(ack) = self.ack.as_ref() {
            if ack.was_ack() {
                self.transistion_to
            } else {
                Box::new(self)
            }
        } else {
            let ack = dialog::blocking_message(self.content.into(), self.level);
            self.ack = Some(ack);
            Box::new(self)
        }
    }
}

use dialog::YesNoQuestion;
/// Ask the user a yes/no question and transition to a state that depends on their answer.
struct YesNo {
    question: Cow<'static, str>,
    answer: Option<YesNoQuestion>,
    yes: Box<dyn State>,
    no: Box<dyn State>,
}

impl YesNo {
    fn new(question: Cow<'static, str>, yes: Box<dyn State>, no: Box<dyn State>) -> Self {
        Self {
            question,
            yes,
            no,
            answer: None,
        }
    }
}

impl State for YesNo {
    fn make_progress(
        self,
        main_state: &mut dyn MainState,
        mediator: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        if let Some(ans) = self.answer.as_ref() {
            if let Some(b) = ans.answer() {
                if b {
                    self.yes
                } else {
                    self.no
                }
            } else {
                Box::new(self)
            }
        } else {
            let yesno = dialog::yes_no_dialog(self.question.into());
            self.answer = Some(yesno);
            Box::new(self)
        }
    }
}

/// An action to be performed at the end of an event loop iteration
#[derive(Debug, Clone)]
pub enum Action {
    DefaultScaffold,
    CustomScaffold,
    OptimizeShift(usize),
    /// Ask the path of the file in whcih to save the staples of design `d_id`
    AskStaplesPath {
        d_id: usize,
    },
    Quit,
    LoadDesign,
    LoadDesignAfterSave,
    SaveBeforeQuit,
    SaveBeforeOpen,
    SaveBeforeNew,
    /// Replace the current design by an empty one
    NewDesign,
    /// Replace the current design by an empty one, after displaying a "successful save" message
    NewDesignAfterSave,
    Other,
    /// Ask the user if they want to use the m13 sequence or use an other one.
    AskUseDefaultScafSequence,
    /// A request to create a new design has been registered
    NewDesignRequested,
    SaveAs,
    Warning(String),
    ErrorMsg(String),
    DownloadStaplesRequest,
    SetScaffoldSequence(String),
    BlockingInfo(String, Box<Action>),
    GetTargetXlsxStaple(usize),
    DownloadStaples {
        target_file: PathBuf,
        design_id: usize,
    },
    Exit,
    FitDesign,
}

use super::ChanelReader;
pub trait MainState {
    fn pop_action(&mut self) -> Option<Action>;
    fn exit_control_flow(&mut self);
    fn new_design(&mut self);
    fn load_design(&mut self, path: PathBuf) -> Result<(), LoadDesignError>;
    fn get_chanel_reader(&mut self) -> &mut ChanelReader;
}

pub struct LoadDesignError(String);

impl From<String> for LoadDesignError {
    fn from(s: String) -> Self {
        Self(s)
    }
}
