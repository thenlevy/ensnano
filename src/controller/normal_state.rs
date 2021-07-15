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
use ensnano_design::Nucl;

/// User is interacting with graphical components.
pub(super) struct NormalState;

impl State for NormalState {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        if let Some(action) = main_state.pop_action() {
            match action {
                Action::NewDesign => self,
                Action::SaveAs => save_as(),
                Action::DownloadStaplesRequest => Box::new(DownloadStaples::default()),
                Action::SetScaffoldSequence => Box::new(SetScaffoldSequence::default()),
                Action::Exit => Box::new(Quit::default()),
                Action::ToggleSplit(mode) => {
                    main_state.toggle_split_mode(mode);
                    self
                }
                Action::OxDnaExport => oxdna_export(),
                Action::CloseOverlay(_) | Action::OpenOverlay(_) => {
                    println!("unexpected action");
                    self
                }
                Action::ChangeUiSize(size) => {
                    main_state.change_ui_size(size);
                    self
                }
                Action::InvertScrollY(inverted) => {
                    main_state.invert_scroll_y(inverted);
                    self
                }
                Action::ErrorMsg(msg) => {
                    TransitionMessage::new(msg, rfd::MessageLevel::Error, Box::new(NormalState))
                }
                Action::DesignOperation(op) => {
                    main_state.apply_operation(op);
                    self.make_progress(main_state)
                }
                Action::SilentDesignOperation(op) => {
                    main_state.apply_silent_operation(op);
                    self.make_progress(main_state)
                }
                Action::Undo => {
                    main_state.undo();
                    self
                }
                Action::Redo => {
                    main_state.redo();
                    self
                }
                Action::NotifyApps(notificiation) => {
                    main_state.notify_apps(notificiation);
                    self
                }
                Action::TurnSelectionIntoGrid => self.turn_selection_into_grid(main_state),
                Action::AddGrid(descr) => self.add_grid(main_state, descr),
                Action::ChangeSequence(_) => {
                    println!("Sequence input is not yet implemented");
                    self
                }
                Action::ChangeColorStrand(color) => self.change_color(main_state, color),
                Action::FinishChangingColor => {
                    main_state.finish_operation();
                    self
                }
                Action::ToggleHelicesPersistance(persistant) => {
                    self.toggle_helices_persistance(main_state, persistant)
                }
                Action::ToggleSmallSphere(small) => self.toggle_small_spheres(main_state, small),
                Action::LoadDesign(Some(path)) => Box::new(Load::known_path(path)),
                Action::LoadDesign(None) => Box::new(Load::default()),
                Action::SuspendOp => {
                    main_state.finish_operation();
                    self
                }
                Action::Copy => {
                    main_state.request_copy();
                    self
                }
                Action::InitPaste => {
                    main_state.init_paste();
                    self
                }
                Action::ApplyPaste => {
                    println!("Applying paste");
                    main_state.apply_paste();
                    self
                }
                Action::PasteCandidate(candidate) => {
                    main_state.request_pasting_candidate(candidate);
                    self
                }
                Action::Duplicate => {
                    main_state.duplicate();
                    self
                }
                _ => todo!(),
            }
        } else {
            self
        }
    }
}

impl NormalState {
    fn turn_selection_into_grid(self: Box<Self>, main_state: &mut dyn MainState) -> Box<Self> {
        let selection = main_state.get_selection();
        if ensnano_interactor::all_helices_no_grid(
            selection.as_ref().as_ref(),
            main_state.get_design_reader().as_ref(),
        ) {
            let selection = selection.as_ref().as_ref().iter().cloned().collect();
            main_state.apply_operation(DesignOperation::HelicesToGrid(selection));
        }
        self
    }

    fn add_grid(
        self: Box<Self>,
        main_state: &mut dyn MainState,
        descr: GridTypeDescr,
    ) -> Box<Self> {
        if let Some((position, orientation)) = main_state.get_grid_creation_position() {
            main_state.apply_operation(DesignOperation::AddGrid(GridDescriptor {
                grid_type: descr,
                position,
                orientation,
            }))
        } else {
            println!("Could not get position and orientation for new grid");
        }
        self
    }

    fn change_color(self: Box<Self>, main_state: &mut dyn MainState, color: u32) -> Box<Self> {
        let strands = ensnano_interactor::extract_strands_from_selection(
            main_state.get_selection().as_ref().as_ref(),
        );
        main_state.apply_operation(DesignOperation::ChangeColor { color, strands });
        self
    }

    fn toggle_small_spheres(
        self: Box<Self>,
        main_state: &mut dyn MainState,
        small: bool,
    ) -> Box<Self> {
        let grid_ids =
            ensnano_interactor::extract_grids(main_state.get_selection().as_ref().as_ref());
        if grid_ids.len() > 0 {
            main_state.apply_operation(DesignOperation::SetSmallSpheres { grid_ids, small });
        }
        self
    }

    fn toggle_helices_persistance(
        self: Box<Self>,
        main_state: &mut dyn MainState,
        persistant: bool,
    ) -> Box<Self> {
        let grid_ids =
            ensnano_interactor::extract_grids(main_state.get_selection().as_ref().as_ref());
        if grid_ids.len() > 0 {
            main_state.apply_operation(DesignOperation::SetHelicesPersistance {
                grid_ids,
                persistant,
            });
        }
        self
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

fn oxdna_export() -> Box<dyn State> {
    let on_success = Box::new(NormalState);
    let on_error = TransitionMessage::new(
        "Export failed".into(),
        rfd::MessageLevel::Error,
        Box::new(NormalState),
    );
    Box::new(OxDnaExport::new(on_success, on_error))
}

use ensnano_design::{
    elements::{DnaAttribute, DnaElementKey},
    grid::{GridDescriptor, GridTypeDescr},
};
use ensnano_interactor::{
    application::Notification, DesignOperation, RigidBodyConstants, Selection, SimulationRequest,
};
/// An action to be performed at the end of an event loop iteration, and that will have an effect
/// on the main application state, e.g. Closing the window, or toggling between 3D/2D views.
#[derive(Debug, Clone)]
pub enum Action {
    LoadDesign(Option<PathBuf>),
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
    DesignOperation(DesignOperation),
    SilentDesignOperation(DesignOperation),
    Undo,
    Redo,
    NotifyApps(Notification),
    TurnSelectionIntoGrid,
    AddGrid(GridTypeDescr),
    /// Set the sequence of all the selected strands
    ChangeSequence(String),
    /// Change the color of all the selected strands
    ChangeColorStrand(u32),
    FinishChangingColor,
    ToggleHelicesPersistance(bool),
    ToggleSmallSphere(bool),
    SimulationRequest(SimulationRequest),
    StopRoll,
    RollHelices(f32),
    Copy,
    PasteCandidate(Option<Nucl>),
    InitPaste,
    ApplyPaste,
    Duplicate,
    RigidGridSimulation {
        parameters: RigidBodyConstants,
    },
    RigidHelicesSimulation {
        parameters: RigidBodyConstants,
    },
    RigidParametersUpdate(RigidBodyConstants),
    TurnIntoAnchor,
    UpdateHyperboloidShift(f32),
    SetVisiblitySieve {
        visible: bool,
    },
    DeleteSelection,
    ScaffoldFromSelection,
    /// Remove empty domains and merge consecutive domains
    CleanDesign,
    UpdateAttribute {
        attribute: DnaAttribute,
        elements: Vec<Selection>,
    },
    UpdateOrganizerTree(ensnano_organizer::OrganizerTree<DnaElementKey>),
    SuspendOp,
}
