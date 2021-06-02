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

use super::*;

pub(super) struct NormalState;

impl State for NormalState {
    fn make_progress(
        self: Box<Self>,
        main_state: &mut dyn MainState,
        _: Arc<Mutex<Mediator>>,
    ) -> Box<dyn State> {
        if let Some(action) = main_state.pop_action() {
            match action {
                Action::NewDesign => todo!(),
                Action::LoadDesign => Box::new(Load::default()),
                Action::SaveAs => save_as(),
                Action::SetScaffoldSequence => Box::new(SetScaffoldSequence::default()),
                Action::DownloadStaplesRequest => Box::new(DownloadStaples::default()),
                Action::Exit => Box::new(Quit::default()),
                Action::ErrorMsg(msg) => {
                    TransitionMessage::new(msg, rfd::MessageLevel::Error, Box::new(NormalState))
                }
                _ => todo!(),
            }
        } else {
            self
        }
    }
}

fn save_as() -> Box<dyn State> {
    let on_success = Box::new(NormalState);
    let on_error = TransitionMessage::new(
        "Could not save design".into(),
        rfd::MessageLevel::Error,
        Box::new(NormalState),
    );
    Box::new(Save::new(on_success, on_error))
}

/// An action to be performed at the end of an event loop iteration, and that will have an effect
/// on the main application state, e.g. Closing the window, or toggling between 3D/2D views.
#[derive(Debug, Clone)]
pub enum Action {
    LoadDesign,
    NewDesign,
    SaveAs,
    DownloadStaplesRequest,
    SetScaffoldSequence,
    Exit,
    ToggleSplit(SplitMode),
    OxDnaExport,
    CloseOverlay(OverlayType),
    OpenOverlay(OverlayType),
    ChangeUiSize(UiSize),
    InvertScrollY(bool),
    ErrorMsg(String),
}
