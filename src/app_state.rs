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

use super::mediator::{Selection, SelectionMode};

mod address_pointer;
use address_pointer::AddressPointer;

mod impl_app2d;
mod impl_app3d;

/// A structure containing the global state of the program.
///
/// At each event loop iteration, a new `AppState` may be created. Successive AppState are stored
/// on an undo/redo stack.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct AppState(AddressPointer<AppState_>);

impl AppState {
    pub fn with_selection(&self, selection: Vec<Selection>) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.selection = AddressPointer::new(selection);
        Self(AddressPointer::new(new_state))
    }

    pub fn with_candidates(&self, candidates: Vec<Selection>) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.candidates = AddressPointer::new(candidates);
        Self(AddressPointer::new(new_state))
    }

    pub fn with_selection_mode(&self, selection_mode: SelectionMode) -> Self {
        let mut new_state = (*self.0).clone();
        new_state.selection_mode = selection_mode;
        Self(AddressPointer::new(new_state))
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
struct AppState_ {
    /// The set of currently selected objects
    selection: AddressPointer<Vec<Selection>>,
    /// The set of objects that are "one click away from beeing selected"
    candidates: AddressPointer<Vec<Selection>>,
    selection_mode: SelectionMode,
}
