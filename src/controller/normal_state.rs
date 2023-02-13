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

use crate::app_state::PastePosition;

use super::download_intervals::DownloadIntervals;
use super::messages::CHANGING_DNA_PARAMETERS_WARNING;
use super::*;
use ensnano_design::group_attributes::GroupPivot;
use ensnano_design::{grid::GridId, Parameters};
use ensnano_interactor::{
    graphics::FogParameters, HyperboloidOperation, RevolutionSurfaceSystemDescriptor,
};

/// User is interacting with graphical components.
pub(super) struct NormalState;

impl State for NormalState {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        if let Some(action) = main_state.pop_action() {
            match action {
                Action::NewDesign => Box::new(NewDesign::init(main_state.need_save())),
                Action::SaveAs => save_as(),
                Action::QuickSave => {
                    if let Some(path) = main_state
                        .get_current_file_name()
                        .filter(|p| p.extension() == Some(crate::consts::ENS_EXTENSION.as_ref()))
                    {
                        quicksave(path)
                    } else {
                        save_as()
                    }
                }
                Action::DownloadStaplesRequest => Box::new(DownloadStaples::default()),
                Action::DownloadOrigamiRequest => Box::new(DownloadIntervals::default()),
                Action::SetScaffoldSequence { shift } => Box::new(SetScaffoldSequence::init(shift)),
                Action::Exit => Quit::quit(main_state.need_save()),
                Action::ToggleSplit(mode) => {
                    main_state.toggle_split_mode(mode);
                    self
                }
                Action::Export(export_type) => export(export_type),
                Action::CloseOverlay(_) | Action::OpenOverlay(_) => {
                    println!("unexpected action");
                    self
                }
                Action::ChangeUiSize(size) => {
                    main_state.change_ui_size(size);
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
                Action::LoadDesign(None) => Load::load(main_state.need_save(), LoadType::Design),
                Action::Import3DObject => {
                    if main_state.get_current_design_directory().is_some() {
                        Load::load(None, LoadType::Object3D)
                    } else {
                        TransitionMessage::new(
                            messages::SET_DESIGN_DIRECTORY_FIRST,
                            rfd::MessageLevel::Error,
                            Box::new(NormalState),
                        )
                    }
                }
                Action::ImportSvg => Load::load(None, LoadType::SvgPath),
                Action::SuspendOp => {
                    log::info!("Suspending operation");
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
                Action::DeleteSelection => {
                    main_state.delete_selection();
                    self
                }
                Action::ScaffoldToSelection => {
                    main_state.scaffold_to_selection();
                    self
                }
                Action::NewHyperboloid(request) => {
                    if let Some((position, orientation)) = main_state.get_grid_creation_position() {
                        main_state.apply_operation(DesignOperation::HyperboloidOperation(
                            HyperboloidOperation::New {
                                request,
                                position,
                                orientation,
                            },
                        ));
                    }
                    self
                }
                Action::AddBezierPlane => {
                    if let Some((position, orientation)) =
                        main_state.get_bezier_sheet_creation_position()
                    {
                        main_state.apply_operation(DesignOperation::AddBezierPlane {
                            desc: ensnano_design::BezierPlaneDescriptor {
                                position,
                                orientation,
                            },
                        })
                    }
                    self
                }
                Action::RigidHelicesSimulation { parameters } => {
                    main_state.start_helix_simulation(parameters);
                    self
                }
                Action::RigidGridSimulation { parameters } => {
                    main_state.start_grid_simulation(parameters);
                    self
                }
                Action::RevolutionSimulation { desc } => {
                    main_state.start_revolution_simulation(desc);
                    self
                }
                Action::StopSimulation => {
                    main_state.update_simulation(SimulationRequest::Stop);
                    self
                }
                Action::FinishRelaxationSimulation => {
                    main_state.update_simulation(SimulationRequest::FinishRelaxation);
                    self
                }
                Action::RollHelices(roll) => {
                    main_state.set_roll_of_selected_helices(roll);
                    self
                }
                Action::ResetSimulation => {
                    main_state.update_simulation(SimulationRequest::Reset);
                    self
                }
                Action::RigidParametersUpdate(parameters) => {
                    main_state.update_simulation(SimulationRequest::UpdateParameters(parameters));
                    self
                }
                Action::RollRequest(request) => {
                    main_state.start_roll_simulation(request.target_helices);
                    self
                }
                Action::Fog(fog) => {
                    main_state.notify_apps(Notification::Fog(fog));
                    self
                }
                Action::Split2D => {
                    main_state.notify_apps(Notification::Split2d);
                    self
                }
                Action::TurnIntoAnchor => {
                    main_state.turn_selection_into_anchor();
                    self
                }
                Action::SetVisiblitySieve { compl } => {
                    main_state.set_visibility_sieve(compl);
                    self
                }
                Action::ClearVisibilitySieve => {
                    main_state.clear_visibility_sieve();
                    self
                }
                Action::ReloadFile => {
                    if let Some(path) = main_state.get_current_file_name() {
                        Load::init_reolad(main_state.need_save(), path.to_path_buf())
                    } else {
                        self
                    }
                }
                Action::SetGroupPivot(pivot) => {
                    main_state.set_current_group_pivot(pivot);
                    self
                }
                Action::TranslateGroupPivot(translation) => {
                    log::info!("Translating group pivot {:?}", translation);
                    main_state.translate_group_pivot(translation);
                    self
                }
                Action::RotateGroupPivot(rotation) => {
                    main_state.rotate_group_pivot(rotation);
                    self
                }
                Action::NewCamera => {
                    main_state.create_new_camera();
                    self
                }
                Action::SelectCamera(camera_id) => {
                    main_state.select_camera(camera_id);
                    self
                }
                Action::SelectFavoriteCamera(n) => {
                    main_state.select_favorite_camera(n);
                    self
                }
                Action::UpdateCamera(camera_id) => {
                    main_state.update_camera(camera_id);
                    self
                }
                Action::Toggle2D => {
                    main_state.toggle_2d();
                    self
                }

                Action::MakeAllSuggestedXover { doubled } => {
                    main_state.make_all_suggested_xover(doubled);
                    self
                }

                Action::FlipSplitViews => {
                    main_state.flip_split_views();
                    self
                }
                Action::Twist(g_id) => {
                    main_state.start_twist(g_id);
                    self
                }
                Action::SetDnaParameters(param) => Box::new(YesNo::new(
                    CHANGING_DNA_PARAMETERS_WARNING,
                    Box::new(ChangindDnaParameters(param)),
                    self,
                )),
                Action::SetExpandInsertions(b) => {
                    main_state.set_expand_insertions(b);
                    self
                }
                Action::SetExporting(exporting) => {
                    main_state.set_exporting(exporting);
                    self
                }
                Action::OptimizeShift => Box::new(SetScaffoldSequence::optimize_shift()),
                action => {
                    println!("Not implemented {:?}", action);
                    self
                }
            }
        } else {
            self
        }
    }
}

struct ChangindDnaParameters(Parameters);

impl State for ChangindDnaParameters {
    fn make_progress(self: Box<Self>, main_state: &mut dyn MainState) -> Box<dyn State> {
        main_state.apply_operation(DesignOperation::SetDnaParameters { parameters: self.0 });
        Box::new(NormalState)
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
                invisible: false,
                bezier_vertex: None,
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
        if !grid_ids.is_empty() {
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
        if !grid_ids.is_empty() {
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
    let on_error = could_not_save_design();
    Box::new(SaveAs::new(on_success, on_error))
}

fn could_not_save_design() -> Box<dyn State> {
    TransitionMessage::new(
        messages::SAVE_DESIGN_FAILED,
        rfd::MessageLevel::Error,
        Box::new(NormalState),
    )
}

fn quicksave<P: AsRef<Path>>(starting_path: P) -> Box<dyn State> {
    Box::new(SaveWithPath {
        path: starting_path.as_ref().to_path_buf(),
        on_success: Box::new(NormalState),
        on_error: could_not_save_design(),
    })
}

fn export(export_type: ExportType) -> Box<dyn State> {
    let on_success = Box::new(NormalState);
    let on_error = TransitionMessage::new(
        messages::OXDNA_EXPORT_FAILED,
        rfd::MessageLevel::Error,
        Box::new(NormalState),
    );
    Box::new(Exporting::new(on_success, on_error, export_type))
}

use ensnano_design::grid::{GridDescriptor, GridTypeDescr};

use ensnano_interactor::HyperboloidRequest;
use ensnano_interactor::{
    application::Notification, DesignOperation, RigidBodyConstants, RollRequest,
};
/// An action to be performed at the end of an event loop iteration, and that will have an effect
/// on the main application state, e.g. Closing the window, or toggling between 3D/2D views.
#[derive(Debug, Clone)]
pub enum Action {
    LoadDesign(Option<PathBuf>),
    NewDesign,
    SaveAs,
    QuickSave,
    DownloadStaplesRequest,
    DownloadOrigamiRequest,
    /// Trigger the sequence of action that will set the scaffold of the sequence.
    SetScaffoldSequence {
        shift: usize,
    },
    Exit,
    ToggleSplit(SplitMode),
    Export(ExportType),
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
    RollRequest(RollRequest),
    StopSimulation,
    RollHelices(f32),
    Copy,
    PasteCandidate(Option<PastePosition>),
    InitPaste,
    ApplyPaste,
    Duplicate,
    RigidGridSimulation {
        parameters: RigidBodyConstants,
    },
    RevolutionSimulation {
        desc: RevolutionSurfaceSystemDescriptor,
    },
    FinishRelaxationSimulation,
    RigidHelicesSimulation {
        parameters: RigidBodyConstants,
    },
    ResetSimulation,
    RigidParametersUpdate(RigidBodyConstants),
    TurnIntoAnchor,
    NewHyperboloid(HyperboloidRequest),
    UpdateHyperboloidShift(f32),
    SetVisiblitySieve {
        compl: bool,
    },
    DeleteSelection,
    ScaffoldToSelection,
    /// Remove empty domains and merge consecutive domains
    CleanDesign,
    SuspendOp,
    Fog(FogParameters),
    Split2D,
    ReloadFile,
    ClearVisibilitySieve,
    SetGroupPivot(GroupPivot),
    TranslateGroupPivot(Vec3),
    RotateGroupPivot(Rotor3),
    NewCamera,
    SelectCamera(ensnano_design::CameraId),
    SelectFavoriteCamera(u32),
    UpdateCamera(ensnano_design::CameraId),
    Toggle2D,
    MakeAllSuggestedXover {
        doubled: bool,
    },
    FlipSplitViews,
    Twist(GridId),
    SetDnaParameters(Parameters),
    SetExpandInsertions(bool),
    AddBezierPlane,
    SetExporting(bool),
    Import3DObject,
    ImportSvg,
    OptimizeShift,
}
