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

use super::AddressPointer;
use ensnano_design::{
    grid::GridId, group_attributes::GroupAttribute, Design, HelixCollection, Parameters,
};
use ensnano_interactor::{
    operation::Operation, ActionMode, DesignOperation, RigidBodyConstants, Selection,
    SimulationState, StrandBuilder, SuggestionParameters,
};

mod presenter;
use ensnano_organizer::GroupId;
pub use presenter::SimulationUpdate;
use presenter::{apply_simulation_update, update_presenter, NuclCollection, Presenter};
pub(super) mod controller;
use controller::Controller;
pub use controller::{
    CopyOperation, InteractorNotification, PastePosition, PastingStatus, RigidHelixState,
    ShiftOptimizationResult, ShiftOptimizerReader, SimulationInterface, SimulationReader,
};

use crate::{controller::SimulationRequest, gui::CurentOpState};
pub(super) use controller::ErrOperation;
use controller::{GridPresenter, HelixPresenter, OkOperation, RollPresenter, TwistPresenter};

use std::sync::Arc;
mod file_parsing;

/// The `DesignInteractor` handles all read/write operations on the design. It is a stateful struct
/// so it is meant to be unexpansive to clone.
#[derive(Clone, Default)]
pub struct DesignInteractor {
    /// The current design
    design: AddressPointer<Design>,
    /// The structure that handles "read" operations. The graphic components of EnsNano access the
    /// presenter via a trait that defines each components needs.
    presenter: AddressPointer<Presenter>,
    /// The structure that handles "write" operations.
    controller: AddressPointer<Controller>,
    simulation_update: Option<Arc<dyn SimulationUpdate>>,
    current_operation: Option<Arc<dyn Operation>>,
    current_operation_id: usize,
    new_action_mode: Option<ActionMode>,
}

impl DesignInteractor {
    pub(super) fn get_design_reader(&self) -> DesignReader {
        DesignReader {
            presenter: self.presenter.clone(),
            controller: self.controller.clone(),
        }
    }
    pub(super) fn optimize_shift(
        &self,
        reader: &mut dyn ShiftOptimizerReader,
    ) -> Result<InteractorResult, ErrOperation> {
        let nucl_map = self.presenter.get_owned_nucl_collection();
        let result = self
            .controller
            .optimize_shift(reader, nucl_map, &self.design);
        self.handle_operation_result(result)
    }

    pub(super) fn apply_operation(
        &self,
        operation: DesignOperation,
    ) -> Result<InteractorResult, ErrOperation> {
        let result = self
            .controller
            .apply_operation(self.design.as_ref(), operation);
        self.handle_operation_result(result)
    }

    pub(super) fn apply_copy_operation(
        &mut self,
        operation: CopyOperation,
    ) -> Result<InteractorResult, ErrOperation> {
        println!("nb helices {}", self.design.helices.len());
        let tried_up_to_date = self.design.try_get_up_to_date();
        if let Some(up_to_date) = tried_up_to_date {
            println!("up to date helices {}", up_to_date.design.helices.len());
            let result = self.controller.apply_copy_operation(up_to_date, operation);
            self.handle_operation_result(result)
        } else {
            let desing_mut = self.design.make_mut();
            let up_to_date = desing_mut.get_up_to_date();
            let result = self.controller.apply_copy_operation(up_to_date, operation);
            self.handle_operation_result(result)
        }
    }

    pub(super) fn update_pending_operation(
        &self,
        operation: Arc<dyn Operation>,
    ) -> Result<InteractorResult, ErrOperation> {
        let op_is_new = self.is_in_stable_state();
        let result = self
            .controller
            .update_pending_operation(self.design.as_ref(), operation.clone());
        let mut ret = self.handle_operation_result(result);
        if let Ok(ret) = ret.as_mut() {
            ret.set_operation_state(operation, op_is_new)
        }
        ret
    }

    pub(super) fn start_simulation(
        &self,
        parameters: RigidBodyConstants,
        reader: &mut dyn SimulationReader,
        target: SimulationTarget,
    ) -> Result<InteractorResult, ErrOperation> {
        let operation = match target {
            SimulationTarget::Helices => controller::SimulationOperation::StartHelices {
                presenter: self.presenter.as_ref(),
                parameters,
                reader,
            },
            SimulationTarget::Grids => controller::SimulationOperation::StartGrids {
                presenter: self.presenter.as_ref(),
                parameters,
                reader,
            },
            SimulationTarget::Roll { target_helices } => {
                controller::SimulationOperation::StartRoll {
                    presenter: self.presenter.as_ref(),
                    reader,
                    target_helices,
                }
            }
            SimulationTarget::Twist { grid_id } => controller::SimulationOperation::StartTwist {
                presenter: self.presenter.as_ref(),
                reader,
                grid_id,
            },
        };
        let result = self
            .controller
            .apply_simulation_operation(self.design.clone_inner(), operation);
        self.handle_operation_result(result)
    }

    pub(super) fn update_simulation(
        &self,
        request: SimulationRequest,
    ) -> Result<InteractorResult, ErrOperation> {
        let operation = match request {
            SimulationRequest::Stop => controller::SimulationOperation::Stop,
            SimulationRequest::Reset => controller::SimulationOperation::Reset,
            SimulationRequest::UpdateParameters(new_parameters) => {
                controller::SimulationOperation::UpdateParameters { new_parameters }
            }
        };
        let result = self
            .controller
            .apply_simulation_operation(self.design.clone_inner(), operation);
        self.handle_operation_result(result)
    }

    fn handle_operation_result(
        &self,
        result: Result<(OkOperation, Controller), ErrOperation>,
    ) -> Result<InteractorResult, ErrOperation> {
        match result {
            Ok((OkOperation::Replace(design), mut controller)) => {
                let mut ret = self.clone();
                ret.new_action_mode = controller.next_action_mode.take();
                ret.controller = AddressPointer::new(controller);
                ret.design = AddressPointer::new(design);
                Ok(InteractorResult::Replace(ret))
            }
            Ok((OkOperation::Push { design, label }, mut controller)) => {
                let mut ret = self.clone();
                ret.current_operation = None;
                ret.new_action_mode = controller.next_action_mode.take();
                ret.controller = AddressPointer::new(controller);
                ret.design = AddressPointer::new(design);
                Ok(InteractorResult::Push {
                    interactor: ret,
                    label,
                })
            }
            Ok((OkOperation::NoOp, mut controller)) => {
                let mut ret = self.clone();
                ret.new_action_mode = controller.next_action_mode.take();
                ret.controller = AddressPointer::new(controller);
                Ok(InteractorResult::Replace(ret))
            }
            Err(e) => Err(e),
        }
    }

    pub(super) fn get_curent_operation_state(&self) -> Option<CurentOpState> {
        self.current_operation.as_ref().map(|op| CurentOpState {
            operation_id: self.current_operation_id,
            current_operation: op.clone(),
        })
    }

    pub(super) fn notify(&self, notification: InteractorNotification) -> Self {
        let mut ret = self.clone();
        ret.controller = AddressPointer::new(ret.controller.notify(notification));
        ret
    }

    pub(super) fn design_need_update(&self, suggestion_parameters: &SuggestionParameters) -> bool {
        presenter::design_need_update(&self.presenter, &self.design, suggestion_parameters)
            || self.simulation_update.is_some()
    }

    pub(super) fn with_updated_design_reader(
        mut self,
        suggestion_parameters: &SuggestionParameters,
    ) -> Self {
        if cfg!(test) || log::log_enabled!(log::Level::Trace) {
            print!("Old design: ");
            self.design.show_address();
        }
        let (new_presenter, new_design) =
            update_presenter(&self.presenter, self.design.clone(), suggestion_parameters);
        self.presenter = new_presenter;
        if cfg!(test) || log::log_enabled!(log::Level::Trace) {
            print!("New design: ");
            new_design.show_address();
        }
        log::trace!("Interactor design <- {:p}", new_design);
        self.design = new_design;
        if let Some(update) = self.simulation_update.clone() {
            if !self.controller.get_simulation_state().is_runing() {
                self.simulation_update = None;
            }
            self.after_applying_simulation_update(update, suggestion_parameters)
        } else {
            self
        }
    }

    pub(super) fn with_simualtion_update_applied(
        mut self,
        update: Box<dyn SimulationUpdate>,
    ) -> Self {
        self.simulation_update = Some(update.into());
        self
    }

    fn after_applying_simulation_update(
        mut self,
        update: Arc<dyn SimulationUpdate>,
        suggestion_parameters: &SuggestionParameters,
    ) -> Self {
        let (new_presenter, new_design) = apply_simulation_update(
            &self.presenter,
            self.design.clone(),
            update,
            suggestion_parameters,
        );
        self.presenter = new_presenter;
        log::trace!("Interactor design <- {:p}", new_design);
        self.design = new_design;
        self
    }

    #[allow(dead_code)] //used in tests
    pub(super) fn with_updated_design(&self, design: Design) -> Self {
        let mut new_interactor = self.clone();
        new_interactor.design = AddressPointer::new(design);
        new_interactor
    }

    pub(super) fn is_in_stable_state(&self) -> bool {
        self.controller.is_in_persistant_state().is_persistant()
    }

    pub(super) fn has_different_design_than(&self, other: &Self) -> bool {
        self.design != other.design
    }

    pub(super) fn has_different_model_matrix_than(&self, other: &Self) -> bool {
        self.presenter
            .has_different_model_matrix_than(other.presenter.as_ref())
    }

    pub(super) fn get_simulation_state(&self) -> SimulationState {
        self.controller.get_simulation_state()
    }

    pub(super) fn get_dna_parameters(&self) -> Parameters {
        self.presenter.current_design.parameters.unwrap_or_default()
    }

    pub(super) fn is_changing_color(&self) -> bool {
        self.controller.is_changing_color()
    }

    pub(super) fn get_strand_builders(&self) -> &[StrandBuilder] {
        self.controller.get_strand_builders()
    }

    pub(super) fn is_pasting(&self) -> PastingStatus {
        self.controller.is_pasting()
    }

    pub(super) fn can_iterate_duplication(&self) -> bool {
        self.controller.can_iterate_duplication()
    }

    pub(super) fn is_building_hyperboloid(&self) -> bool {
        self.controller.is_building_hyperboloid()
    }

    pub(super) fn with_visibility_sieve(
        mut self,
        selection: Vec<Selection>,
        compl: bool,
    ) -> InteractorResult {
        let mut presenter = self.presenter.clone_inner();
        presenter.set_visibility_sieve(selection, compl);
        self.presenter = AddressPointer::new(presenter);
        self.design = AddressPointer::new(self.design.clone_inner());
        InteractorResult::Push {
            interactor: self,
            label: crate::consts::UPDATE_VISIBILITY_SIEVE_LABEL.into(),
        }
    }

    pub(super) fn get_new_selection(&self) -> Option<Vec<Selection>> {
        self.controller.get_new_selection()
    }

    pub fn get_new_action_mode(&mut self) -> Option<ActionMode> {
        self.new_action_mode.take()
    }
}

/// An opperation has been successfully applied to the design, resulting in a new modifed
/// interactor. The variants of these enum indicate different ways in which the result should be
/// handled
pub(super) enum InteractorResult {
    Push {
        interactor: DesignInteractor,
        label: std::borrow::Cow<'static, str>,
    },
    Replace(DesignInteractor),
}

impl InteractorResult {
    pub fn set_operation_state(&mut self, operation: Arc<dyn Operation>, new_op: bool) {
        let interactor = match self {
            Self::Push { interactor, .. } => interactor,
            Self::Replace(interactor) => interactor,
        };
        if new_op {
            interactor.current_operation_id += 1;
            log::info!("New operation id {}", interactor.current_operation_id);
        }
        interactor.current_operation = Some(operation);
    }
}

/// A reference to a Presenter that is guaranted to always have up to date internal data
/// structures.
pub struct DesignReader {
    presenter: AddressPointer<Presenter>,
    controller: AddressPointer<Controller>,
}

use crate::controller::SaveDesignError;
use std::path::PathBuf;
impl DesignReader {
    pub fn save_design(
        &self,
        path: &PathBuf,
        saving_info: ensnano_design::SavingInformation,
    ) -> Result<(), SaveDesignError> {
        use std::io::Write;
        let mut design = self.presenter.current_design.clone_inner();
        design.prepare_for_save(saving_info);
        let json_content = serde_json::to_string_pretty(&design)?;
        let mut f = std::fs::File::create(path)?;
        f.write_all(json_content.as_bytes())?;
        Ok(())
    }

    pub fn oxdna_export(&self, target_dir: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        self.presenter.oxdna_export(target_dir)
    }

    pub fn get_strand_domain(&self, s_id: usize, d_id: usize) -> Option<&ensnano_design::Domain> {
        self.presenter.get_strand_domain(s_id, d_id)
    }

    pub fn get_group_attributes(&self, group_id: GroupId) -> Option<&GroupAttribute> {
        self.presenter
            .current_design
            .as_ref()
            .group_attributes
            .get(&group_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::OkOperation as TopOkOperation;
    use super::super::*;
    use super::controller::CopyOperation;
    use super::file_parsing::StrandJunction;
    use super::*;
    use crate::app_state;
    use crate::scene::DesignReader as Reader3d;
    use ensnano_design::grid::HelixGridPosition;
    use ensnano_design::HelixCollection;
    use ensnano_design::{grid::GridDescriptor, Collection, DomainJunction, Nucl, Strand};
    use ensnano_interactor::operation::GridHelixCreation;
    use ensnano_interactor::DesignReader;
    use std::path::PathBuf;
    use ultraviolet::{Rotor3, Vec3};

    fn test_path(design_name: &'static str) -> PathBuf {
        let mut ret = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
        ret.push("tests");
        ret.push(design_name);
        ret
    }

    fn one_helix_path() -> PathBuf {
        test_path("one_helix.json")
    }

    fn design_for_sequence_testing() -> AppState {
        let path = test_path("test_sequence.json");
        AppState::import_design(&path).ok().unwrap()
    }

    fn assert_good_strand<S: std::ops::Deref<Target = str>>(strand: &Strand, objective: S) {
        println!("self {:?}", strand.formated_domains());
        println!("objective {}", objective.deref());
        use regex::Regex;
        let re = Regex::new(r#"\[[^\]]*\]"#).unwrap();
        let formated_strand = strand.formated_domains();
        let left = re.find_iter(&formated_strand);
        let right = re.find_iter(&objective);
        for (a, b) in left.zip(right) {
            assert_eq!(a.as_str(), b.as_str());
        }
    }

    fn assert_good_junctions<S: std::ops::Deref<Target = str>>(strand: &Strand, objective: S) {
        println!("self {:?}", strand.formated_anonymous_junctions());
        println!("objective {}", objective.deref());
        use regex::Regex;
        let re = Regex::new(r#"\[[^\]]*\]"#).unwrap();
        let formated_strand = strand.formated_anonymous_junctions();
        let left = re.find_iter(&formated_strand);
        let right = re.find_iter(&objective);
        for (a, b) in left.zip(right) {
            assert_eq!(a.as_str(), b.as_str());
        }
    }

    /// A design with one strand h1: 0 -> 5 ; h2: 0 <- 5
    fn one_xover() -> AppState {
        let path = test_path("one_xover.json");
        AppState::import_design(&path).ok().unwrap()
    }

    /// A design with one strand h1: -1 -> 7 ; h2: -1 <- 7 ; h3: 0 -> 9 that can be pasted on
    /// helices 4, 5 and 6
    fn pastable_design() -> AppState {
        let path = test_path("pastable.json");
        AppState::import_design(&path).ok().unwrap()
    }

    /// A design with one cyclic strand h1: -1 -> 7 ; h2: -1 <- 7 ; h3: 0 -> 9 that can be pasted on
    /// helices 4, 5 and 6
    fn pastable_cyclic() -> AppState {
        let path = test_path("pastable_cyclic.json");
        AppState::import_design(&path).ok().unwrap()
    }

    fn fake_design_update(state: &mut AppState) {
        let design = state.0.design.design.clone_inner();
        let new_state = std::mem::take(state);
        *state = new_state.with_updated_design(design);
    }

    #[test]
    fn read_one_helix() {
        let path = one_helix_path();
        let interactor = DesignInteractor::new_with_path(&path).ok().unwrap();
        let suggestion_parameters = Default::default();
        let interactor = interactor.with_updated_design_reader(&suggestion_parameters);
        let reader = interactor.get_design_reader();
        assert_eq!(reader.get_all_visible_nucl_ids().len(), 24)
    }

    #[test]
    fn first_update_has_effect() {
        let path = one_helix_path();
        let mut app_state = AppState::import_design(&path).ok().unwrap();
        let old_app_state = app_state.clone();
        fake_design_update(&mut app_state);
        let app_state = app_state.updated();
        assert!(old_app_state.design_was_modified(&app_state));
    }

    #[test]
    fn second_update_has_no_effect() {
        let path = one_helix_path();
        let mut app_state = AppState::import_design(&path).ok().unwrap();
        fake_design_update(&mut app_state);
        app_state = app_state.updated();
        let old_app_state = app_state.clone();
        let app_state = app_state.updated();
        assert!(!old_app_state.design_was_modified(&app_state));
    }

    #[test]
    fn strand_builder_on_xover_end() {
        let mut app_state = one_xover();
        app_state
            .apply_design_op(DesignOperation::RequestStrandBuilders {
                nucls: vec![
                    Nucl {
                        helix: 1,
                        position: 5,
                        forward: true,
                    },
                    Nucl {
                        helix: 2,
                        position: 5,
                        forward: false,
                    },
                ],
            })
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.get_strand_builders().len(), 2);
    }

    #[test]
    fn moving_one_strand_builder() {
        let mut app_state = one_xover();
        app_state
            .apply_design_op(DesignOperation::RequestStrandBuilders {
                nucls: vec![Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                }],
            })
            .unwrap();
        app_state.update();
        app_state
            .apply_design_op(DesignOperation::MoveBuilders(7))
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        assert_good_strand(strand, "[H1: 0 -> 7] [H2: 0 <- 5]");
        assert_eq!(
            strand.junctions,
            vec![DomainJunction::IdentifiedXover(0), DomainJunction::Prime3]
        )
    }

    const INSERTION_LEN_0: usize = 3;
    const INSERTION_LEN_1: usize = 7;
    const INSERTION_LEN_2: usize = 12;
    const INSERTION_LEN_3: usize = 45;
    const INSERTION_LEN_4: usize = 97;

    /// Test insertions on prime5 of strand, in middle of domains in prime 5 of xover in prime 3 of
    /// xover and in prime3 of strand
    fn non_cyclic_strand_with_insertions() -> AppState {
        // A design with one strand h1: -1 -> 7 ; h2: -1 <- 7 ; h3: 0 -> 9
        let mut app_state = pastable_design();

        // prime5 of strand
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: -1,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: false,
                },
                length: INSERTION_LEN_0,
            })
            .unwrap();
        app_state.update();

        // middle of domain
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: 3,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: INSERTION_LEN_1,
            })
            .unwrap();
        app_state.update();

        // prime5 of xover
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: 7,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: INSERTION_LEN_2,
            })
            .unwrap();
        app_state.update();

        // prime3 of xover
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 0,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: false,
                },
                length: INSERTION_LEN_3,
            })
            .unwrap();
        app_state.update();

        //prime3 of strand
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 9,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: INSERTION_LEN_4,
            })
            .unwrap();
        app_state.update();
        app_state
    }

    /// Add an insertion on 3'end of a strand and check that the last two junctions are in correct
    /// oreder
    #[test]
    fn junction_on_xover_ends() {
        // A design with one strand h1: -1 -> 7 ; h2: -1 <- 7 ; h3: 0 -> 9
        let mut app_state = pastable_design();

        // prime5 of xover
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: 7,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: INSERTION_LEN_2,
            })
            .unwrap();
        app_state.update();

        // prime3 of xover
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 0,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: false,
                },
                length: INSERTION_LEN_3,
            })
            .unwrap();
        app_state.update();

        //prime3 of strand
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 9,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: INSERTION_LEN_4,
            })
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let exptected_result = format!("[->] [x] [->] [x] [->] [3']");
        assert_good_junctions(strand, exptected_result);
    }

    #[test]
    fn insertions_on_non_cyclic_strand_have_correct_effect_on_topology() {
        let app_state = non_cyclic_strand_with_insertions();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[@{}] [H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [@{}] [H3: 0 -> 9] [@{}]",
            INSERTION_LEN_0, INSERTION_LEN_1, INSERTION_LEN_2, INSERTION_LEN_3, INSERTION_LEN_4
        );
        assert_good_strand(strand, expected_result);
    }

    #[test]
    fn making_a_strand_cyclic_with_insertions_on_prime5_and_prime3() {
        let mut app_state = non_cyclic_strand_with_insertions();
        app_state
            .apply_design_op(DesignOperation::Xover {
                prime5_id: 0,
                prime3_id: 0,
            })
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result =
            format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [@{}] [H3: 0 -> 9] [@{}] [cycle]",
            INSERTION_LEN_1, INSERTION_LEN_2, INSERTION_LEN_3, INSERTION_LEN_4 + INSERTION_LEN_0
        );
        assert_good_strand(strand, expected_result);
    }

    #[test]
    fn making_a_strand_cyclic_with_insertions_on_prime5() {
        let mut app_state = non_cyclic_strand_with_insertions();
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 9,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: 0,
            })
            .unwrap();
        app_state
            .apply_design_op(DesignOperation::Xover {
                prime5_id: 0,
                prime3_id: 0,
            })
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [@{}] [H3: 0 -> 9] [@{}] [cycle]",
            INSERTION_LEN_1, INSERTION_LEN_2, INSERTION_LEN_3, INSERTION_LEN_0
        );
        assert_good_strand(strand, expected_result);
    }

    #[test]
    fn making_a_strand_cyclic_with_insertions_on_prime3() {
        let mut app_state = non_cyclic_strand_with_insertions();
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: -1,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: false,
                },
                length: 0,
            })
            .unwrap();
        app_state
            .apply_design_op(DesignOperation::Xover {
                prime5_id: 0,
                prime3_id: 0,
            })
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [@{}] [H3: 0 -> 9] [@{}] [cycle]",
            INSERTION_LEN_1, INSERTION_LEN_2, INSERTION_LEN_3, INSERTION_LEN_4
        );
        assert_good_strand(strand, expected_result);
    }

    #[test]
    /// Test insertions on prime5 of strand, in middle of domains in prime 5 of xover in prime 3 of
    /// xover and in prime3 of strand
    fn insertions_on_cyclic_strand() {
        // A design with one strand h1: -1 -> 7 ; h2: -1 <- 7 ; h3: 0 -> 9
        let mut app_state = pastable_cyclic();
        let insertion_len_0 = 3;
        let insertion_len_1 = 7;
        let insertion_len_2 = 12;
        let insertion_len_3 = 45;
        let insertion_len_4 = 97;

        // prime5 of strand
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: -1,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: false,
                },
                length: insertion_len_0,
            })
            .unwrap();
        app_state.update();

        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[H1: -1 -> 7] [H2: -1 <- 7] [H3: 0 -> 9] [@{}] [cycle]",
            insertion_len_0
        );
        assert_good_strand(strand, expected_result);

        // middle of domain
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: 3,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: insertion_len_1,
            })
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [H2: -1 <- 7] [H3: 0 -> 9] [@{}] [cycle]",
            insertion_len_1, insertion_len_0
        );
        assert_good_strand(strand, expected_result);

        // prime5 of xover
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 1,
                        position: 7,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: insertion_len_2,
            })
            .unwrap();
        app_state.update();
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [H3: 0 -> 9] [@{}] [cycle]",
            insertion_len_1, insertion_len_2, insertion_len_0
        );
        assert_good_strand(strand, expected_result);

        // prime3 of xover
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 0,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: false,
                },
                length: insertion_len_3,
            })
            .unwrap();
        app_state.update();

        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result = format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [@{}] [H3: 0 -> 9] [@{}] [cycle]",
            insertion_len_1, insertion_len_2, insertion_len_3, insertion_len_0
        );
        assert_good_strand(strand, expected_result);

        //prime3 of strand
        app_state
            .apply_design_op(DesignOperation::SetInsertionLength {
                insertion_point: ensnano_interactor::InsertionPoint {
                    nucl: Nucl {
                        helix: 3,
                        position: 9,
                        forward: true,
                    },
                    nucl_is_prime5_of_insertion: true,
                },
                length: insertion_len_4,
            })
            .unwrap();
        app_state.update();

        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&0)
            .expect("No strand 0");
        let expected_result =
            format!(
            "[H1: -1 -> 3] [@{}] [H1: 4 -> 7] [@{}] [H2: -1 <- 7] [@{}] [H3: 0 -> 9] [@{}] [cycle]",
            insertion_len_1, insertion_len_2, insertion_len_3, insertion_len_4 + insertion_len_0
        );
        assert_good_strand(strand, expected_result);
    }

    #[test]
    fn moving_xover_preserve_ids() {
        let mut app_state = one_xover();
        app_state
            .apply_design_op(DesignOperation::RequestStrandBuilders {
                nucls: vec![Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                }],
            })
            .unwrap();
        app_state.update();
        app_state
            .apply_design_op(DesignOperation::MoveBuilders(7))
            .unwrap();
        app_state.update();

        let n1 = Nucl {
            helix: 1,
            position: 7,
            forward: true,
        };
        let n2 = Nucl {
            helix: 2,
            position: 5,
            forward: false,
        };
        let xover_id = app_state
            .0
            .design
            .presenter
            .junctions_ids
            .get_all_elements();
        assert_eq!(xover_id, vec![(0, (n1, n2))]);
    }

    #[test]
    fn add_grid() {
        let mut app_state = AppState::default();
        app_state
            .apply_design_op(DesignOperation::AddGrid(GridDescriptor {
                position: Vec3::zero(),
                orientation: Rotor3::identity(),
                grid_type: ensnano_design::grid::GridTypeDescr::Square { twist: None },
                invisible: false,
                bezier_vertex: None,
            }))
            .unwrap();
        app_state.update();
        assert_eq!(
            app_state.0.design.presenter.current_design.free_grids.len(),
            1
        )
    }

    #[test]
    fn add_grid_helix_via_op() {
        let mut app_state = AppState::default();
        app_state
            .apply_design_op(DesignOperation::AddGrid(GridDescriptor {
                position: Vec3::zero(),
                orientation: Rotor3::identity(),
                grid_type: ensnano_design::grid::GridTypeDescr::Square { twist: None },
                invisible: false,
                bezier_vertex: None,
            }))
            .unwrap();
        app_state.update();
        app_state
            .update_pending_operation(Arc::new(GridHelixCreation {
                design_id: 0,
                grid_id: GridId::FreeGrid(0),
                x: 0,
                y: 0,
                position: 0,
                length: 0,
            }))
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.presenter.current_design.helices.len(), 1)
    }

    #[test]
    fn add_grid_helix_directly() {
        let mut app_state = AppState::default();
        app_state
            .apply_design_op(DesignOperation::AddGrid(GridDescriptor {
                position: Vec3::zero(),
                orientation: Rotor3::identity(),
                grid_type: ensnano_design::grid::GridTypeDescr::Square { twist: None },
                invisible: false,
                bezier_vertex: None,
            }))
            .unwrap();
        app_state.update();
        app_state
            .apply_design_op(DesignOperation::AddGridHelix {
                position: HelixGridPosition::from_grid_id_x_y(GridId::FreeGrid(0), 0, 0),
                start: 0,
                length: 0,
            })
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.presenter.current_design.helices.len(), 1)
    }

    #[test]
    fn copy_creates_clipboard() {
        let mut app_state = pastable_design();
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        assert_eq!(app_state.0.design.controller.size_of_clipboard(), 1)
    }

    #[test]
    fn coping_one_strand() {
        let mut app_state = pastable_design();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 4,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::Paste)
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
    }

    #[test]
    fn pasting_is_undoable() {
        let mut app_state = pastable_design();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 4,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        assert!(matches!(
            app_state
                .apply_copy_operation(CopyOperation::Paste)
                .unwrap(),
            TopOkOperation::Undoable { .. }
        ));
    }

    #[test]
    fn can_paste_on_same_helix_if_not_intersecting() {
        let mut app_state = pastable_design();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 10,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::Paste)
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
    }

    #[test]
    fn copy_cannot_intersect_existing_strand() {
        let mut app_state = pastable_design();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        match app_state.apply_copy_operation(CopyOperation::Paste) {
            Err(ErrOperation::CannotPasteHere) => (),
            x => panic!("expected CannotPasteHere, got {:?}", x),
        }
    }

    #[test]
    fn not_pasting_after_copy() {
        let mut app_state = pastable_design();
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::None)
    }

    #[test]
    fn pasting_after_copy_and_request_paste() {
        let mut app_state = pastable_design();
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(None))
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::Copy)
    }

    #[test]
    fn not_pasting_after_paste() {
        let mut app_state = pastable_design();
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 4,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::Paste)
            .unwrap();
        app_state.update();
        assert_eq!(app_state.is_pasting(), PastingStatus::None)
    }

    #[test]
    fn pasting_after_duplicate() {
        let mut app_state = pastable_design();
        app_state
            .apply_copy_operation(CopyOperation::InitStrandsDuplication(vec![0]))
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::Duplication)
    }

    #[test]
    fn duplication_of_one_strand() {
        let mut app_state = pastable_design();
        app_state
            .apply_copy_operation(CopyOperation::InitStrandsDuplication(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 10,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 3);
    }

    #[ignore]
    #[test]
    fn correct_simulation_state() {
        assert!(false)
    }

    #[test]
    fn pasting_candidate_position_are_accessible() {
        let mut app_state = pastable_design();
        assert_eq!(app_state.0.design.controller.get_pasted_position().len(), 0);
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        assert!(app_state.0.design.controller.get_pasted_position().len() > 0);
    }

    #[test]
    fn setting_a_candidate_triggers_update() {
        let mut app_state = pastable_design();
        let old_app_state = app_state.clone();
        assert!(!old_app_state.design_was_modified(&app_state));
        assert_eq!(app_state.0.design.controller.get_pasted_position().len(), 0);
        app_state
            .apply_copy_operation(CopyOperation::CopyStrands(vec![0]))
            .unwrap();
        let ret = app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 10,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        println!("{:?}", ret);
        app_state.update();
        assert!(app_state.design_was_modified(&old_app_state));
    }

    #[test]
    fn positioning_xovers_paste() {
        let mut app_state = pastable_design();
        let (n1, n2) = app_state.get_design_reader().get_xover_with_id(0).unwrap();
        app_state
            .apply_copy_operation(CopyOperation::CopyXovers(vec![(n1, n2)]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(None))
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 3,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(None))
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
    }

    #[test]
    fn pasting_when_positioning_xovers() {
        let mut app_state = pastable_design();
        let (n1, n2) = app_state.get_design_reader().get_xover_with_id(0).unwrap();
        app_state
            .apply_copy_operation(CopyOperation::CopyXovers(vec![(n1, n2)]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(None))
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::Copy);
    }

    #[test]
    fn duplicating_xovers() {
        let mut app_state = pastable_design();
        let (n1, n2) = app_state.get_design_reader().get_xover_with_id(0).unwrap();
        app_state
            .apply_copy_operation(CopyOperation::InitXoverDuplication(vec![(n1, n2)]))
            .unwrap();
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(None))
            .unwrap();
        assert_eq!(app_state.0.design.design.strands.len(), 1);
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.design.strands.len(), 2);
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.design.strands.len(), 3);
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        app_state.update();
        assert_eq!(app_state.0.design.design.strands.len(), 4);
    }

    #[test]
    fn duplicating_xovers_pasting_status() {
        let mut app_state = pastable_design();
        let (n1, n2) = app_state.get_design_reader().get_xover_with_id(0).unwrap();
        app_state
            .apply_copy_operation(CopyOperation::InitXoverDuplication(vec![(n1, n2)]))
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::Duplication);
        app_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(
                Some(Nucl {
                    helix: 1,
                    position: 5,
                    forward: true,
                })
                .map(PastePosition::Nucl),
            ))
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::Duplication);
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::None);
        app_state
            .apply_copy_operation(CopyOperation::Duplicate)
            .unwrap();
        assert_eq!(app_state.is_pasting(), PastingStatus::None);
    }

    #[test]
    fn correct_staples_no_scaffold_shift() {
        let mut app_state = design_for_sequence_testing();
        let sequence = std::fs::read_to_string(test_path("seq_test.txt")).unwrap();
        app_state
            .apply_design_op(DesignOperation::SetScaffoldSequence { sequence, shift: 0 })
            .unwrap();
        app_state.update();
        let s_id = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&Nucl {
                helix: 1,
                position: 0,
                forward: true,
            })
            .unwrap();
        app_state
            .apply_design_op(DesignOperation::SetScaffoldId(Some(s_id)))
            .unwrap();
        app_state.update();
        let stapples = app_state.get_design_reader().presenter.get_staples();
        for s in stapples.iter() {
            if s.name.contains("5':h1:nt7") {
                assert_eq!(s.sequence, "CCAA TTTT")
            } else if s.name.contains("5':h2:nt0") {
                assert_eq!(s.sequence, "AAAA GGTT")
            } else {
                panic!("Incorrect staple name {:?}", s.name);
            }
        }
    }

    #[test]
    fn correct_staples_scaffold_shift() {
        let mut app_state = design_for_sequence_testing();
        let sequence = std::fs::read_to_string(test_path("seq_test.txt")).unwrap();
        app_state
            .apply_design_op(DesignOperation::SetScaffoldSequence { sequence, shift: 3 })
            .unwrap();
        app_state.update();
        let s_id = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&Nucl {
                helix: 1,
                position: 0,
                forward: true,
            })
            .unwrap();
        app_state
            .apply_design_op(DesignOperation::SetScaffoldId(Some(s_id)))
            .unwrap();
        app_state.update();
        let stapples = app_state.get_design_reader().presenter.get_staples();
        for s in stapples.iter() {
            if s.name.contains("5':h1:nt7") {
                assert_eq!(s.sequence, "AGGT TCCA")
            } else if s.name.contains("5':h2:nt0") {
                assert_eq!(s.sequence, "ATTT TAAA")
            } else {
                panic!("Incorrect staple name {:?}", s.name);
            }
        }
    }

    /// A design with two strands h1: 0 -> 5 and h1: 6 -> 10
    fn two_neighbour_one_helix() -> AppState {
        let path = test_path("two_neighbour_strands.ens");
        AppState::import_design(&path).ok().unwrap()
    }

    #[test]
    fn xover_same_helix_neighbour_strands() {
        let mut app_state = two_neighbour_one_helix();
        let first_nucl = Nucl {
            helix: 1,
            position: 0,
            forward: true,
        };
        let last_nucl = Nucl {
            position: 10,
            ..first_nucl
        };
        app_state
            .apply_design_op(DesignOperation::GeneralXover {
                source: last_nucl,
                target: first_nucl,
            })
            .unwrap();
        app_state.update();

        let s_id = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&first_nucl)
            .expect(&format!("no strand containing {:?}", first_nucl));
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&s_id)
            .expect(&format!("No strand {s_id}"));

        assert_good_strand(strand, "[H1: 6 -> 10] [H1: 0 -> 5]");
    }

    #[test]
    fn merge_neighbour_strands_same_helix() {
        let mut app_state = two_neighbour_one_helix();
        let first_nucl = Nucl {
            helix: 1,
            position: 0,
            forward: true,
        };
        let last_nucl = Nucl {
            position: 10,
            ..first_nucl
        };
        let s_id_first = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&first_nucl)
            .expect(&format!("no strand containing {:?}", first_nucl));
        let s_id_last = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&last_nucl)
            .expect(&format!("no strand containing {:?}", last_nucl));
        app_state
            .apply_design_op(DesignOperation::Xover {
                prime5_id: s_id_last,
                prime3_id: s_id_first,
            })
            .unwrap();
        app_state.update();

        let s_id = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&first_nucl)
            .expect(&format!("no strand containing {:?}", first_nucl));
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&s_id)
            .expect(&format!("No strand {s_id}"));

        assert_good_strand(strand, "[H1: 6 -> 10] [H1: 0 -> 5]");
    }

    /// A design with two strands [h1: 0 -> 10] and [@10] [h2: 0 <- 10]
    fn loopout_5prime_end() -> AppState {
        let path = test_path("loopout_5prime.ens");
        AppState::import_design(&path).ok().unwrap()
    }

    #[test]
    fn xover_on_prime5_end_with_loopout() {
        let mut app_state = loopout_5prime_end();
        let source_nucl = Nucl {
            helix: 1,
            position: 10,
            forward: true,
        };
        let dest_nucl = Nucl {
            helix: 2,
            position: 10,
            forward: false,
        };
        app_state
            .apply_design_op(DesignOperation::GeneralXover {
                source: source_nucl,
                target: dest_nucl,
            })
            .unwrap();
        app_state.update();
        let mut xover_ids = ensnano_utils::id_generator::IdGenerator::default();

        let s_id = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&source_nucl)
            .expect(&format!("no strand containing {:?}", source_nucl));
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&s_id)
            .expect(&format!("No strand {s_id}"));
        let mut strand = Strand::clone(&strand);
        strand.read_junctions(&mut xover_ids, true);
        strand.read_junctions(&mut xover_ids, false);
    }

    /// A design with two strands [h1: 0 -> 10] and [@20] [h2: 0 <- 10]
    fn loopout_5prime_and_3prime_ends() -> AppState {
        let path = test_path("loopout_5prime_and_3prime.ens");
        AppState::import_design(&path).ok().unwrap()
    }

    #[test]
    fn merge_insertions() {
        let mut app_state = loopout_5prime_and_3prime_ends();
        let source_nucl = Nucl {
            helix: 1,
            position: 10,
            forward: true,
        };
        let dest_nucl = Nucl {
            helix: 2,
            position: 10,
            forward: false,
        };
        app_state
            .apply_design_op(DesignOperation::GeneralXover {
                source: source_nucl,
                target: dest_nucl,
            })
            .unwrap();
        app_state.update();
        let s_id = app_state
            .get_design_reader()
            .get_id_of_strand_containing_nucl(&source_nucl)
            .expect(&format!("no strand containing {:?}", source_nucl));
        let strand = app_state
            .0
            .design
            .presenter
            .current_design
            .strands
            .get(&s_id)
            .expect(&format!("No strand {s_id}"));

        assert_good_strand(strand, "[H1: 0 -> 10] [@20] [H2: 0 <- 10]");
    }
}

pub enum SimulationTarget {
    Grids,
    Helices,
    Roll { target_helices: Option<Vec<usize>> },
    Twist { grid_id: GridId },
}
