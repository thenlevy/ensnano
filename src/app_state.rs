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

use ensnano_interactor::{operation::Operation, ActionMode, Selection, SelectionMode, WidgetBasis};

use std::path::PathBuf;
use std::sync::Arc;
mod address_pointer;
mod design_interactor;
use crate::apply_update;
use address_pointer::AddressPointer;
use ensnano_design::Design;
use ensnano_interactor::DesignOperation;

pub use design_interactor::controller::ErrOperation;
use design_interactor::{DesignInteractor, InteractorResult};
pub use design_interactor::{DesignReader, InteractorNotification};

mod impl_app2d;
mod impl_app3d;
mod impl_gui;

/// A structure containing the global state of the program.
///
/// At each event loop iteration, a new `AppState` may be created. Successive AppState are stored
/// on an undo/redo stack.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct AppState(AddressPointer<AppState_>);

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish()
    }
}

impl AppState {
    pub fn with_selection(&self, selection: Vec<Selection>) -> Self {
        if self.0.selection.content_equal(&selection) {
            self.clone()
        } else {
            let mut new_state = (*self.0).clone();
            new_state.selection = AddressPointer::new(selection);
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

    pub fn with_action_mode(&self, action_mode: ActionMode) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.action_mode = action_mode;
        Self(AddressPointer::new(new_state))
    }

    pub fn with_toggled_widget_basis(&self) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.widget_basis.toggle();
        Self(AddressPointer::new(new_state))
    }

    pub fn update_design(&mut self, design: Design) {
        apply_update(self, |s| s.with_updated_design(design))
    }

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

    fn updated(self) -> Self {
        let mut interactor = self.0.design.clone_inner();
        interactor = interactor.with_updated_design_reader();
        self.with_interactor(interactor)
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

    pub(super) fn update_pending_operation(
        &mut self,
        op: Arc<dyn Operation>,
    ) -> Result<Option<Self>, ErrOperation> {
        let result = self.0.design.update_pending_operation(op);
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
                println!("error");
                Err(e)
            }
        }
    }

    pub fn notified(&self, notification: InteractorNotification) -> Self {
        let new_interactor = self.0.design.notify(notification);
        self.clone().with_interactor(new_interactor)
    }

    pub fn get_design_reader(&self) -> DesignReader {
        self.0.design.get_design_reader()
    }

    pub fn oxdna_export(&self, target_dir: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        self.get_design_reader().oxdna_export(target_dir)
    }

    pub fn get_selection(&self) -> impl AsRef<[Selection]> {
        self.0.selection.clone()
    }

    fn is_changing_color(&self) -> bool {
        self.0.design.as_ref().is_changing_color()
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
struct AppState_ {
    /// The set of currently selected objects
    selection: AddressPointer<Vec<Selection>>,
    /// The set of objects that are "one click away from beeing selected"
    candidates: AddressPointer<Vec<Selection>>,
    selection_mode: SelectionMode,
    /// A pointer to the design currently beign eddited. The pointed design is never mutatated.
    /// Instead, when a modification is requested, the design is cloned and the `design` pointer is
    /// replaced by a pointer to a modified `Design`.
    design: AddressPointer<DesignInteractor>,
    action_mode: ActionMode,
    widget_basis: WidgetBasis,
}
