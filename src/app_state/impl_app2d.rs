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
use crate::flatscene::AppState as App2D;
use ensnano_interactor::StrandBuilder;

impl App2D for AppState {
    type Reader = DesignReader;
    fn get_selection(&self) -> &[Selection] {
        self.selection_content().as_slice()
    }

    fn get_candidates(&self) -> &[Selection] {
        self.0.candidates.as_slice()
    }

    fn selection_was_updated(&self, other: &Self) -> bool {
        self.selection_content() != other.selection_content()
    }

    fn candidate_was_updated(&self, other: &Self) -> bool {
        self.0.candidates != other.0.candidates
    }

    fn get_selection_mode(&self) -> SelectionMode {
        self.0.selection_mode
    }

    fn get_design_reader(&self) -> Self::Reader {
        self.0.design.get_design_reader()
    }

    fn get_strand_builders(&self) -> &[StrandBuilder] {
        self.0.design.get_strand_builders()
    }

    fn design_was_updated(&self, other: &Self) -> bool {
        self.0.design.has_different_design_than(&other.0.design)
    }

    fn is_changing_color(&self) -> bool {
        self.is_changing_color()
    }

    fn is_pasting(&self) -> bool {
        self.get_pasting_status().is_pasting()
    }

    fn get_building_state(&self) -> Option<ensnano_interactor::StrandBuildingStatus> {
        self.get_strand_building_state()
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
    }

    #[test]
    fn selection_mode_update() {
        let mut state = AppState::default();
        let old_selection_mode = state.get_selection_mode();
        let old_state = state.clone();
        state = state.with_selection_mode(SelectionMode::Helix);
        assert_eq!(old_state.get_selection_mode(), old_selection_mode);
        assert_eq!(state.get_selection_mode(), SelectionMode::Helix);
        assert!(!state.selection_was_updated(&old_state));
    }
}
