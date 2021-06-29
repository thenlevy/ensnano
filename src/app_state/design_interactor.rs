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
use ensnano_design::{Design, Parameters};
use ensnano_interactor::{operation::Operation, DesignOperation, SimulationState};

mod presenter;
use presenter::{update_presenter, Presenter};
pub(super) mod controller;
use controller::Controller;
pub use controller::InteractorNotification;

pub(super) use controller::ErrOperation;
use controller::OkOperation;

mod file_parsing;
pub use file_parsing::ParseDesignError;

mod grid_data;

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
}

impl DesignInteractor {
    pub(super) fn get_design_reader(&self) -> DesignReader {
        DesignReader {
            presenter: self.presenter.clone(),
        }
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

    pub(super) fn update_pending_operation(
        &self,
        operation: Arc<dyn Operation>,
    ) -> Result<InteractorResult, ErrOperation> {
        let result = self
            .controller
            .update_pending_operation(self.design.as_ref(), operation);
        self.handle_operation_result(result)
    }

    fn handle_operation_result(
        &self,
        result: Result<(OkOperation, Controller), ErrOperation>,
    ) -> Result<InteractorResult, ErrOperation> {
        match result {
            Ok((OkOperation::Replace(design), controller)) => {
                let mut ret = self.clone();
                ret.controller = AddressPointer::new(controller);
                ret.design = AddressPointer::new(design);
                Ok(InteractorResult::Replace(ret))
            }
            Ok((OkOperation::Push(design), controller)) => {
                let mut ret = self.clone();
                ret.controller = AddressPointer::new(controller);
                ret.design = AddressPointer::new(design);
                Ok(InteractorResult::Push(ret))
            }
            Err(e) => Err(e),
        }
    }

    pub(super) fn notify(&self, notification: InteractorNotification) -> Self {
        let mut ret = self.clone();
        ret.controller = AddressPointer::new(ret.controller.notify(notification));
        ret
    }

    pub(super) fn with_updated_design_reader(mut self) -> Self {
        if cfg!(test) {
            print!("Old design: ");
            self.design.show_address();
        }
        let (new_presenter, new_design) = update_presenter(&self.presenter, self.design.clone());
        self.presenter = new_presenter;
        if cfg!(test) {
            print!("New design: ");
            new_design.show_address();
        }
        self.design = new_design;
        self
    }

    pub(super) fn with_updated_design(&self, design: Design) -> Self {
        let mut new_interactor = self.clone();
        new_interactor.design = AddressPointer::new(design);
        new_interactor
    }

    pub(super) fn has_different_design_than(&self, other: &Self) -> bool {
        self.design != other.design
    }

    pub(super) fn has_different_model_matrix_than(&self, other: &Self) -> bool {
        self.presenter
            .has_different_model_matrix_than(other.presenter.as_ref())
    }

    pub(super) fn get_simulation_state(&self) -> SimulationState {
        //TODO
        SimulationState::None
    }

    pub(super) fn get_dna_parameters(&self) -> Parameters {
        self.presenter.current_design.parameters.unwrap_or_default()
    }

    pub(super) fn is_changing_color(&self) -> bool {
        self.controller.is_changing_color()
    }
}

/// An opperation has been successfully applied to the design, resulting in a new modifed
/// interactor. The variants of these enum indicate different ways in which the result should be
/// handled
pub(super) enum InteractorResult {
    Push(DesignInteractor),
    Replace(DesignInteractor),
}

/// A reference to a Presenter that is guaranted to always have up to date internal data
/// structures.
pub struct DesignReader {
    presenter: AddressPointer<Presenter>,
}

use crate::controller::SaveDesignError;
use std::{path::PathBuf, sync::Arc};
impl DesignReader {
    pub fn save_design(&self, path: &PathBuf) -> Result<(), SaveDesignError> {
        use std::io::Write;
        let json_content = serde_json::to_string_pretty(&self.presenter.current_design.as_ref())?;
        let mut f = std::fs::File::create(path)?;
        f.write_all(json_content.as_bytes())?;
        Ok(())
    }

    pub fn oxdna_export(&self, target_dir: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        self.presenter.oxdna_export(target_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::scene::DesignReader as Reader3d;
    use std::path::PathBuf;

    fn one_helix_path() -> PathBuf {
        let mut ret = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
        ret.push("tests");
        ret.push("one_helix.json");
        ret
    }

    fn fake_design_update(state: &mut AppState) {
        let design = state.0.design.design.clone_inner();
        let mut new_state = std::mem::take(state);
        *state = new_state.with_updated_design(design);
    }

    #[test]
    fn read_one_helix() {
        let path = one_helix_path();
        let interactor = DesignInteractor::new_with_path(&path).ok().unwrap();
        let interactor = interactor.with_updated_design_reader();
        let reader = interactor.get_design_reader();
        assert_eq!(reader.get_all_visible_nucl_ids().len(), 24)
    }

    #[test]
    fn first_update_has_effect() {
        use crate::scene::AppState as App3d;
        let path = one_helix_path();
        let mut app_state = AppState::import_design(&path).ok().unwrap();
        let old_app_state = app_state.clone();
        fake_design_update(&mut app_state);
        let app_state = app_state.updated();
        assert!(old_app_state.design_was_modified(&app_state));
    }

    #[test]
    fn second_update_has_no_effect() {
        use crate::scene::AppState as App3d;
        let path = one_helix_path();
        let mut app_state = AppState::import_design(&path).ok().unwrap();
        fake_design_update(&mut app_state);
        app_state = app_state.updated();
        let old_app_state = app_state.clone();
        let app_state = app_state.updated();
        assert!(!old_app_state.design_was_modified(&app_state));
    }

    #[test]
    fn correct_simulation_state() {
        assert!(false)
    }
}
