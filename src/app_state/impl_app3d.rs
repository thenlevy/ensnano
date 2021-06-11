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

use crate::design::StrandBuilder;
use crate::scene::AppState as App3D;

use super::*;

impl App3D for AppState {
    type DesignReader = DesignReader;
    fn get_selection(&self) -> &[Selection] {
        self.0.selection.as_slice()
    }

    fn get_candidates(&self) -> &[Selection] {
        self.0.candidates.as_slice()
    }

    fn selection_was_updated(&self, other: &AppState) -> bool {
        self.0.selection != other.0.selection
    }

    fn candidates_set_was_updated(&self, other: &AppState) -> bool {
        self.0.candidates != other.0.candidates
    }

    fn design_was_modified(&self, other: &Self) -> bool {
        self.0.design.has_different_design_than(&other.0.design)
    }

    fn design_model_matrix_was_updated(&self, other: &Self) -> bool {
        self.0
            .design
            .has_different_model_matrix_than(&other.0.design)
    }

    fn get_selection_mode(&self) -> SelectionMode {
        self.0.selection_mode
    }

    fn get_action_mode(&self) -> ActionMode {
        self.0.action_mode
    }

    fn get_design_reader(&self) -> Self::DesignReader {
        self.0.design.get_design_reader()
    }

    fn get_strand_builders(&self) -> Vec<StrandBuilder> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_update() {
        let mut state = AppState::default();
        let old_state = state.clone();

        // When a new state is created with this methods it should be considered to have a new
        // selection but the same selection
        state = state.with_selection(vec![]);
        assert!(state.selection_was_updated(&old_state));
        assert!(!state.candidates_set_was_updated(&old_state));
    }

    #[test]
    fn candidates_update() {
        let mut state = AppState::default();
        let old_state = state.clone();

        // When a new state is created with this methods it should be considered to have a new
        // set of candidates but the same selection
        state = state.with_candidates(vec![]);
        assert!(state.candidates_set_was_updated(&old_state));
        assert!(!state.selection_was_updated(&old_state));
    }

    #[test]
    fn new_design_is_a_modification() {
        let mut state = AppState::default();
        let old_state = state.clone();

        assert!(!state.design_was_modified(&old_state));
        state = AppState::new_design(Default::default());
        assert!(state.design_was_modified(&old_state));
    }

    #[test]
    fn new_selection_is_not_a_modification() {
        let mut state = AppState::default();
        let old_state = state.clone();

        state = state.with_selection(vec![]);
        assert!(!state.design_was_modified(&old_state));
    }

    #[test]
    fn new_candidates_is_not_a_modification() {
        let mut state = AppState::default();
        let old_state = state.clone();

        state = state.with_candidates(vec![]);
        assert!(!state.design_was_modified(&old_state));
    }
}
