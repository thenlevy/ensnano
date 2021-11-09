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

use crate::scene::AppState as App3D;
use ensnano_interactor::StrandBuilder;

use super::*;

impl App3D for AppState {
    type DesignReader = DesignReader;
    fn get_selection(&self) -> &[Selection] {
        self.selection_content().as_slice()
    }

    fn get_candidates(&self) -> &[Selection] {
        self.0.candidates.as_slice()
    }

    fn selection_was_updated(&self, other: &AppState) -> bool {
        self.selection_content() != other.selection_content()
            || self.0.center_of_selection != other.0.center_of_selection
            || self.is_changing_color() != other.is_changing_color()
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

    fn get_action_mode(&self) -> (ActionMode, WidgetBasis) {
        (self.0.action_mode, self.0.widget_basis)
    }

    fn get_design_reader(&self) -> Self::DesignReader {
        self.0.design.get_design_reader()
    }

    fn get_strand_builders(&self) -> &[StrandBuilder] {
        self.0.design.get_strand_builders()
    }

    fn get_widget_basis(&self) -> WidgetBasis {
        self.0.widget_basis
    }

    fn is_changing_color(&self) -> bool {
        self.is_changing_color()
    }

    fn is_pasting(&self) -> bool {
        self.is_pasting().is_pasting()
    }

    fn get_selected_element(&self) -> Option<CenterOfSelection> {
        self.0.center_of_selection.clone()
    }

    fn get_current_group_pivot(&self) -> Option<ensnano_design::group_attributes::GroupPivot> {
        let reader = self.get_design_reader();
        self.0
            .selection
            .selected_group
            .and_then(|g_id| reader.get_group_attributes(g_id))
            .and_then(|attributes| attributes.pivot.clone())
            .or(self.0.selection.pivot.read().as_deref().unwrap().clone())
    }

    fn get_current_group_id(&self) -> Option<ensnano_design::GroupId> {
        self.0.selection.selected_group
    }

    fn suggestion_parameters_were_updated(&self, other: &Self) -> bool {
        self.0.suggestion_parameters != other.0.suggestion_parameters
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
        state = state.with_selection(vec![Selection::Strand(0, 0)], None);
        assert!(state.selection_was_updated(&old_state));
        assert!(!state.candidates_set_was_updated(&old_state));
    }

    #[test]
    fn candidates_update() {
        let mut state = AppState::default();
        let old_state = state.clone();

        // When a new state is created with this methods it should be considered to have a new
        // set of candidates but the same selection
        state = state.with_candidates(vec![Selection::Strand(0, 0)]);
        assert!(state.candidates_set_was_updated(&old_state));
        assert!(!state.selection_was_updated(&old_state));
    }

    #[test]
    fn new_design_is_a_modification() {
        let mut state = AppState::default();
        let old_state = state.clone();

        assert!(!state.design_was_modified(&old_state));
        state.update_design(Default::default());
        assert!(state.design_was_modified(&old_state));
    }

    #[test]
    fn new_selection_is_not_a_modification() {
        let mut state = AppState::default();
        let old_state = state.clone();

        state = state.with_selection(vec![], None);
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
