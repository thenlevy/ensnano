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

use crate::scene::{AppState as App3D, DrawOptions};
use ensnano_design::grid::GridId;
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
        // When the selected object is a grid associated to a bezier vertex, we always want to
        // return WidgetBasis::Object. We do so to enforce that all rotation applied to that grid
        // happen in a cannonical plane
        if self.has_selected_a_bezier_grid() {
            WidgetBasis::Object
        } else {
            self.0.widget_basis
        }
    }

    fn is_changing_color(&self) -> bool {
        self.is_changing_color()
    }

    fn is_pasting(&self) -> bool {
        self.get_pasting_status().is_pasting()
    }

    fn get_selected_element(&self) -> Option<CenterOfSelection> {
        self.0.center_of_selection
    }

    fn get_current_group_pivot(&self) -> Option<ensnano_design::group_attributes::GroupPivot> {
        let reader = self.get_design_reader();
        self.0
            .selection
            .selected_group
            .and_then(|g_id| reader.get_group_attributes(g_id))
            .and_then(|attributes| attributes.pivot)
            .or(*self.0.selection.pivot.read().as_deref().unwrap())
    }

    fn get_current_group_id(&self) -> Option<ensnano_design::GroupId> {
        self.0.selection.selected_group
    }

    fn suggestion_parameters_were_updated(&self, other: &Self) -> bool {
        self.0.parameters.suggestion_parameters != other.0.parameters.suggestion_parameters
    }

    fn get_check_xover_parameters(&self) -> CheckXoversParameter {
        self.0.parameters.check_xover_paramters
    }

    fn follow_stereographic_camera(&self) -> bool {
        self.0.parameters.follow_stereography
    }

    fn get_draw_options(&self) -> DrawOptions {
        DrawOptions {
            background3d: self.0.parameters.background3d,
            rendering_mode: self.0.parameters.rendering_mode,
            show_stereographic_camera: self.0.parameters.show_stereography,
            thick_helices: self.0.parameters.thick_helices,
            h_bonds: self.0.parameters.show_h_bonds,
            show_bezier_planes: self.0.parameters.show_bezier_paths,
        }
    }

    fn draw_options_were_updated(&self, other: &Self) -> bool {
        self.get_draw_options() != other.get_draw_options()
    }

    fn get_scroll_sensitivity(&self) -> f32 {
        let sign = if self.0.parameters.inverted_y_scroll {
            -1.0
        } else {
            1.0
        };
        sign * crate::consts::scroll_sensitivity_convertion(self.0.parameters.scroll_sensitivity)
    }

    fn show_insertion_representents(&self) -> bool {
        self.0.show_insertion_representents
    }

    fn show_bezier_paths(&self) -> bool {
        self.0.parameters.show_bezier_paths
    }

    fn get_design_path(&self) -> Option<PathBuf> {
        self.0.path_to_current_design.clone()
    }

    fn get_selected_bezier_vertex(&self) -> Option<ensnano_design::BezierVertexId> {
        if let Some(Selection::BezierVertex(vertex)) = self.0.selection.selection.get(0) {
            Some(*vertex)
        } else {
            None
        }
    }

    fn has_selected_a_bezier_grid(&self) -> bool {
        matches!(
            self.get_selection().as_ref().get(0),
            Some(Selection::Grid(_, GridId::BezierPathGrid(_)))
        )
    }

    fn revolution_bezier_updated(&self, other: &Self) -> bool {
        self.0.unrooted_surface.descriptor != other.0.unrooted_surface.descriptor
    }

    fn get_current_unrooted_surface(&self) -> Option<UnrootedRevolutionSurfaceDescriptor> {
        self.0.unrooted_surface.descriptor.clone()
    }

    fn get_revolution_axis_position(&self) -> Option<f64> {
        Some(
            self.0
                .unrooted_surface
                .descriptor
                .as_ref()?
                .get_revolution_axis_position(),
        )
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
        state.update();
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
