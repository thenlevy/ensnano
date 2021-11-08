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

//! This modules defines the `AppState` struct which implements various traits used by the
//! different components of ENSnano.
//!
//! The role of AppState is to provide information about the global state of the program, for
//! example the current selection, or the current state of the design.
//!
//! Each component of ENSnano has specific needs and express them via its own `AppState` trait.

use ensnano_design::group_attributes::GroupPivot;
use ensnano_interactor::{
    operation::Operation, ActionMode, CenterOfSelection, Selection, SelectionMode, WidgetBasis,
};

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
mod address_pointer;
mod design_interactor;
use crate::apply_update;
use crate::controller::SimulationRequest;
use address_pointer::AddressPointer;
use ensnano_design::Design;
use ensnano_interactor::{DesignOperation, RigidBodyConstants, SuggestionParameters};
use ensnano_organizer::GroupId;

pub use design_interactor::controller::ErrOperation;
pub use design_interactor::{
    CopyOperation, DesignReader, InteractorNotification, PastingStatus, ShiftOptimizationResult,
    ShiftOptimizerReader, SimulationInterface, SimulationReader, SimulationTarget,
    SimulationUpdate,
};
use design_interactor::{DesignInteractor, InteractorResult};

mod impl_app2d;
mod impl_app3d;
mod impl_gui;

/// A structure containing the global state of the program.
///
/// At each event loop iteration, a new `AppState` may be created. Successive AppState are stored
/// on an undo/redo stack.
#[derive(Clone, PartialEq, Eq)]
pub struct AppState(AddressPointer<AppState_>);

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish()
    }
}

impl std::fmt::Pointer for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = self.0.get_ptr();
        std::fmt::Pointer::fmt(&ptr, f)
    }
}

impl Default for AppState {
    fn default() -> Self {
        let ret = AppState(Default::default());
        ret.updated()
    }
}

impl AppState {
    pub fn with_selection(
        &self,
        selection: Vec<Selection>,
        selected_group: Option<GroupId>,
    ) -> Self {
        if self.0.selection.selection.content_equal(&selection)
            && selected_group == self.0.selection.selected_group
        {
            self.clone()
        } else {
            let mut new_state = (*self.0).clone();
            let selection_len = selection.len();
            new_state.selection = AppStateSelection {
                selection: AddressPointer::new(selection),
                selected_group,
                pivot: Arc::new(RwLock::new(None)),
                old_pivot: Arc::new(RwLock::new(None)),
            };
            // Set when the selection is modified, the center of selection is set to None. It is up
            // to the caller to set it to a certain value when applicable
            new_state.center_of_selection = None;
            let mut ret = Self(AddressPointer::new(new_state));
            if selection_len > 0 {
                ret = ret.notified(InteractorNotification::NewSelection)
            }
            ret
        }
    }

    pub fn with_center_of_selection(&self, center: Option<CenterOfSelection>) -> Self {
        if center == self.0.center_of_selection {
            self.clone()
        } else {
            let mut new_state = (*self.0).clone();
            new_state.center_of_selection = center;
            Self(AddressPointer::new(new_state))
        }
    }

    pub fn with_candidates(&self, candidates: Vec<Selection>) -> Self {
        if self.0.candidates.content_equal(&candidates) {
            self.clone()
        } else {
            let mut new_state = (*self.0).clone();
            new_state.candidates = AddressPointer::new(candidates);
            Self(AddressPointer::new(new_state))
        }
    }

    pub fn with_selection_mode(&self, selection_mode: SelectionMode) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.selection_mode = selection_mode;
        Self(AddressPointer::new(new_state))
    }

    pub fn with_suggestion_parameters(&self, suggestion_parameters: SuggestionParameters) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.suggestion_parameters = suggestion_parameters;
        Self(AddressPointer::new(new_state))
    }

    pub fn with_action_mode(&self, action_mode: ActionMode) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.action_mode = action_mode;
        Self(AddressPointer::new(new_state))
    }

    pub fn with_strand_on_helix(&self, parameters: Option<(isize, usize)>) -> Self {
        let new_strand_parameters =
            parameters.map(|(start, length)| NewHelixStrand { length, start });
        if let ActionMode::BuildHelix { .. } = self.0.action_mode {
            let mut new_state = (*self.0).clone();
            let length = new_strand_parameters
                .as_ref()
                .map(|strand| strand.length)
                .unwrap_or_default();
            let start = new_strand_parameters
                .as_ref()
                .map(|strand| strand.start)
                .unwrap_or_default();
            new_state.strand_on_new_helix = new_strand_parameters;
            new_state.action_mode = ActionMode::BuildHelix {
                length,
                position: start,
            };
            Self(AddressPointer::new(new_state))
        } else {
            self.clone()
        }
    }

    pub fn with_toggled_widget_basis(&self) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.widget_basis.toggle();
        Self(AddressPointer::new(new_state))
    }

    #[allow(dead_code)] //used in tests
    pub fn update_design(&mut self, design: Design) {
        apply_update(self, |s| s.with_updated_design(design))
    }

    #[allow(dead_code)] //used in tests
    pub fn with_updated_design(&self, design: Design) -> Self {
        let mut new_state = self.0.clone_inner();
        let new_interactor = new_state.design.with_updated_design(design);
        new_state.design = AddressPointer::new(new_interactor);
        Self(AddressPointer::new(new_state))
    }

    pub fn import_design(path: &PathBuf) -> Result<Self, design_interactor::ParseDesignError> {
        let design_interactor = DesignInteractor::new_with_path(path)?;
        Ok(Self(AddressPointer::new(AppState_ {
            design: AddressPointer::new(design_interactor),
            ..Default::default()
        })))
    }

    pub(super) fn update(&mut self) {
        apply_update(self, Self::updated)
    }

    pub(super) fn apply_simulation_update(&mut self, update: Box<dyn SimulationUpdate>) {
        apply_update(self, |s| s.with_simualtion_update_applied(update))
    }

    fn with_simualtion_update_applied(self, update: Box<dyn SimulationUpdate>) -> Self {
        let mut design = self.0.design.clone_inner();
        design = design.with_simualtion_update_applied(update);
        self.with_interactor(design)
    }

    fn updated(self) -> Self {
        let old_self = self.clone();
        let mut interactor = self.0.design.clone_inner();
        interactor = interactor.with_updated_design_reader(&self.0.suggestion_parameters);
        let new = self.with_interactor(interactor);
        if old_self.design_was_modified(&new) {
            new
        } else {
            old_self
        }
    }

    fn with_interactor(self, interactor: DesignInteractor) -> Self {
        let mut new_state = self.0.clone_inner();
        new_state.design = AddressPointer::new(interactor);
        Self(AddressPointer::new(new_state))
    }

    pub(super) fn apply_design_op(
        &mut self,
        op: DesignOperation,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.apply_operation(op);
        self.handle_operation_result(result)
    }

    pub(super) fn apply_copy_operation(
        &mut self,
        op: CopyOperation,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.apply_copy_operation(op);
        self.handle_operation_result(result)
    }

    pub(super) fn update_pending_operation(
        &mut self,
        op: Arc<dyn Operation>,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.update_pending_operation(op);
        self.handle_operation_result(result)
    }

    pub(super) fn start_simulation(
        &mut self,
        parameters: RigidBodyConstants,
        reader: &mut dyn SimulationReader,
        target: SimulationTarget,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.start_simulation(parameters, reader, target);
        self.handle_operation_result(result)
    }

    pub(super) fn update_simulation(
        &mut self,
        request: SimulationRequest,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.update_simulation(request);
        self.handle_operation_result(result)
    }

    fn handle_operation_result(
        &mut self,
        result: Result<InteractorResult, ErrOperation>,
    ) -> Result<Option<Self>, ErrOperation> {
        match result {
            Ok(InteractorResult::Push(design)) => {
                let ret = Some(self.clone());
                let new_state = self.clone().with_interactor(design);
                *self = new_state;
                Ok(ret)
            }
            Ok(InteractorResult::Replace(design)) => {
                let new_state = self.clone().with_interactor(design);
                *self = new_state;
                Ok(None)
            }
            Err(e) => {
                log::error!("error {:?}", e);
                Err(e)
            }
        }
    }

    pub fn notified(&self, notification: InteractorNotification) -> Self {
        let new_interactor = self.0.design.notify(notification);
        self.clone().with_interactor(new_interactor)
    }

    pub fn finish_operation(&mut self) {
        let pivot = self.0.selection.pivot.read().unwrap().clone();
        log::info!("Setting pivot {:?}", pivot);
        log::info!("was {:?}", self.0.selection.old_pivot.read().unwrap());
        *self.0.selection.old_pivot.write().unwrap() = pivot;
        log::info!("is {:?}", self.0.selection.old_pivot.read().unwrap());
        log::debug!(
            "old pivot after reset {:p}",
            Arc::as_ptr(&self.0.selection.old_pivot)
        );
    }

    pub fn get_design_reader(&self) -> DesignReader {
        self.0.design.get_design_reader()
    }

    pub fn oxdna_export(&self, target_dir: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        self.get_design_reader().oxdna_export(target_dir)
    }

    pub fn get_selection(&self) -> impl AsRef<[Selection]> {
        self.0.selection.selection.clone()
    }

    fn is_changing_color(&self) -> bool {
        self.0.design.as_ref().is_changing_color()
    }

    pub(super) fn prepare_for_replacement(&mut self, source: &Self) {
        *self = self.with_candidates(vec![]);
        *self = self.with_action_mode(source.0.action_mode.clone());
        *self = self.with_selection_mode(source.0.selection_mode.clone());
        *self = self.with_suggestion_parameters(source.0.suggestion_parameters.clone());
    }

    pub(super) fn is_pasting(&self) -> PastingStatus {
        self.0.design.is_pasting()
    }

    pub(super) fn can_iterate_duplication(&self) -> bool {
        self.0.design.can_iterate_duplication()
    }

    pub(super) fn optimize_shift(
        &mut self,
        reader: &mut dyn ShiftOptimizerReader,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.optimize_shift(reader);
        self.handle_operation_result(result)
    }

    pub(super) fn is_in_stable_state(&self) -> bool {
        self.0.design.is_in_stable_state()
    }

    #[must_use]
    pub(super) fn set_visibility_sieve(
        &mut self,
        selection: Vec<Selection>,
        compl: bool,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self
            .0
            .design
            .clone_inner()
            .with_visibility_sieve(selection, compl);
        self.handle_operation_result(Ok(result))
    }

    pub fn design_was_modified(&self, other: &Self) -> bool {
        self.0.design.has_different_design_than(&other.0.design)
    }

    fn get_strand_building_state(&self) -> Option<crate::gui::StrandBuildingStatus> {
        use crate::gui::StrandBuildingStatus;
        let builders = self.0.design.get_strand_builders();
        builders.get(0).and_then(|b| {
            let domain_id = b.get_domain_identifier();
            let reader = self.get_design_reader();
            let domain = reader.get_strand_domain(domain_id.strand, domain_id.domain)?;
            let param = self.0.design.get_dna_parameters();
            if let ensnano_design::Domain::HelixDomain(interval) = domain {
                let prime5 = interval.prime5();
                let prime3 = interval.prime3();
                let nt_length = domain.length();
                Some(StrandBuildingStatus {
                    prime5,
                    prime3,
                    nt_length,
                    nm_length: param.z_step * nt_length as f32,
                    dragged_nucl: b.moving_end,
                })
            } else {
                None
            }
        })
    }

    fn selection_content(&self) -> &AddressPointer<Vec<Selection>> {
        &self.0.selection.selection
    }

    pub fn get_current_group_id(&self) -> Option<GroupId> {
        self.0.selection.selected_group.clone()
    }

    pub fn set_current_group_pivot(&mut self, pivot: GroupPivot) {
        if self.0.selection.pivot.read().unwrap().is_none() {
            log::info!("reseting selection pivot {:?}", pivot);
            *self.0.selection.pivot.write().unwrap() = Some(pivot);
            *self.0.selection.old_pivot.write().unwrap() = Some(pivot);
            log::debug!(
                "old pivot after reset {:p}",
                Arc::as_ptr(&self.0.selection.old_pivot)
            );
        }
    }

    pub fn translate_group_pivot(&mut self, translation: ultraviolet::Vec3) {
        log::debug!("old pivot {:p}", Arc::as_ptr(&self.0.selection.old_pivot));
        log::info!("is {:?}", self.0.selection.old_pivot.read().unwrap());
        let new_pivot = {
            if let Some(Some(mut old_pivot)) =
                self.0.selection.old_pivot.read().as_deref().ok().cloned()
            {
                old_pivot.position += translation;
                old_pivot
            } else {
                log::error!("Translating a pivot that does not exist");
                return;
            }
        };
        *self.0.selection.pivot.write().unwrap() = Some(new_pivot);
    }

    pub fn rotate_group_pivot(&mut self, rotation: ultraviolet::Rotor3) {
        log::debug!("old pivot {:p}", Arc::as_ptr(&self.0.selection.old_pivot));
        log::info!("is {:?}", self.0.selection.old_pivot.read().unwrap());
        let new_pivot = {
            if let Some(Some(mut old_pivot)) =
                self.0.selection.old_pivot.read().as_deref().ok().cloned()
            {
                old_pivot.orientation = rotation * old_pivot.orientation;
                old_pivot
            } else {
                log::error!("Rotating a pivot that does not exist");
                return;
            }
        };
        *self.0.selection.pivot.write().unwrap() = Some(new_pivot);
    }
}

#[derive(Clone, Default)]
struct AppState_ {
    /// The set of currently selected objects
    selection: AppStateSelection,
    /// The set of objects that are "one click away from beeing selected"
    candidates: AddressPointer<Vec<Selection>>,
    selection_mode: SelectionMode,
    /// A pointer to the design currently beign eddited. The pointed design is never mutatated.
    /// Instead, when a modification is requested, the design is cloned and the `design` pointer is
    /// replaced by a pointer to a modified `Design`.
    design: AddressPointer<DesignInteractor>,
    action_mode: ActionMode,
    widget_basis: WidgetBasis,
    strand_on_new_helix: Option<NewHelixStrand>,
    center_of_selection: Option<CenterOfSelection>,
    suggestion_parameters: SuggestionParameters,
}

#[derive(Clone, Default)]
struct AppStateSelection {
    selection: AddressPointer<Vec<Selection>>,
    selected_group: Option<ensnano_organizer::GroupId>,
    pivot: Arc<RwLock<Option<GroupPivot>>>,
    old_pivot: Arc<RwLock<Option<GroupPivot>>>,
}

#[derive(Clone, PartialEq, Eq)]
struct NewHelixStrand {
    length: usize,
    start: isize,
}
