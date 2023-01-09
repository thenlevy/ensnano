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
use crate::gui::AppState as GuiState;
use ensnano_design::{elements::DnaElementKey, Parameters};
use ensnano_gui::ClipboardContent;
use ensnano_interactor::{ScaffoldInfo, SelectionConversion, SimulationState};

mod curve_builders;
use curve_builders::*;

impl GuiState for AppState {
    const POSSIBLE_CURVES: &'static [ensnano_gui::CurveDescriptorBuilder<AppState>] =
        &[ELLIPSE_BUILDER, TWO_SPHERES_BUILDER, BEZIER_CURVE_BUILDER];

    fn get_selection_mode(&self) -> SelectionMode {
        self.0.selection_mode
    }

    fn get_action_mode(&self) -> ActionMode {
        self.0.action_mode
    }

    fn get_widget_basis(&self) -> WidgetBasis {
        self.0.widget_basis
    }

    fn get_simulation_state(&self) -> SimulationState {
        self.0.design.get_simulation_state()
    }

    fn get_dna_parameters(&self) -> Parameters {
        self.0.design.get_dna_parameters()
    }

    fn get_selection(&self) -> &[Selection] {
        self.selection_content().as_ref()
    }

    fn get_selection_as_dnaelement(&self) -> Vec<DnaElementKey> {
        self.selection_content()
            .iter()
            .filter_map(|s| DnaElementKey::from_selection(s, 0))
            .collect()
    }

    fn is_building_hyperboloid(&self) -> bool {
        self.0.design.is_building_hyperboloid()
    }

    fn get_scaffold_info(&self) -> Option<ScaffoldInfo> {
        self.get_design_reader().get_scaffold_info()
    }

    fn can_make_grid(&self) -> bool {
        self.selection_content().len() > 4
            && ensnano_interactor::all_helices_no_grid(
                self.selection_content(),
                &self.get_design_reader(),
            )
    }

    fn get_reader(&self) -> Box<dyn crate::gui::DesignReader> {
        Box::new(self.get_design_reader())
    }

    fn design_was_modified(&self, other: &Self) -> bool {
        self.0.design.has_different_design_than(&other.0.design)
    }

    fn selection_was_updated(&self, other: &Self) -> bool {
        self.selection_content() != other.selection_content()
    }

    fn get_build_helix_mode(&self) -> ActionMode {
        if let Some(NewHelixStrand { length, start }) = self.0.strand_on_new_helix.as_ref() {
            ActionMode::BuildHelix {
                position: *start,
                length: *length,
            }
        } else {
            ActionMode::BuildHelix {
                position: 0,
                length: 0,
            }
        }
    }

    fn has_double_strand_on_new_helix(&self) -> bool {
        self.0.strand_on_new_helix.is_some()
    }

    fn get_curent_operation_state(&self) -> Option<crate::gui::CurentOpState> {
        self.0.design.get_curent_operation_state()
    }

    fn get_strand_building_state(&self) -> Option<crate::gui::StrandBuildingStatus> {
        self.get_strand_building_state()
    }

    fn get_selected_group(&self) -> Option<GroupId> {
        self.0.selection.selected_group.clone()
    }

    fn get_suggestion_parameters(&self) -> &SuggestionParameters {
        &self.0.parameters.suggestion_parameters
    }

    fn get_checked_xovers_parameters(&self) -> CheckXoversParameter {
        self.0.parameters.check_xover_paramters
    }

    fn follow_stereographic_camera(&self) -> bool {
        self.0.parameters.follow_stereography
    }

    fn show_stereographic_camera(&self) -> bool {
        self.0.parameters.show_stereography
    }

    fn get_h_bounds_display(&self) -> HBoundDisplay {
        self.0.parameters.show_h_bonds
    }

    fn get_scroll_sensitivity(&self) -> f32 {
        self.0.parameters.scroll_sensitivity
    }

    fn get_invert_y_scroll(&self) -> bool {
        self.0.parameters.inverted_y_scroll
    }

    fn want_thick_helices(&self) -> bool {
        self.0.parameters.thick_helices
    }

    fn expand_insertions(&self) -> bool {
        !self.0.show_insertion_representents
    }

    fn get_show_bezier_paths(&self) -> bool {
        self.0.parameters.show_bezier_paths
    }

    fn get_selected_bezier_path(&self) -> Option<ensnano_design::BezierPathId> {
        if let Some(Selection::BezierVertex(vertex)) = self.0.selection.selection.get(0) {
            Some(vertex.path_id)
        } else {
            None
        }
    }

    fn is_exporting(&self) -> bool {
        self.0.exporting
    }

    fn is_transitory(&self) -> bool {
        !self.is_in_stable_state()
    }

    fn get_current_revoultion_radius(&self) -> Option<f64> {
        self.0
            .unrooted_surface
            .descriptor
            .as_ref()?
            .revolution_radius
            .to_signed_f64()
    }

    fn get_recommended_scaling_revolution_surface(
        &self,
        scaffold_len: usize,
    ) -> Option<ensnano_gui::RevolutionScaling> {
        let area_surface = self.0.unrooted_surface.area?;
        let perimeter_surface = self
            .0
            .unrooted_surface
            .descriptor
            .as_ref()?
            .curve
            .perimeter();
        let parameters = self.get_dna_parameters();
        let area_one_nucl = parameters.z_step * Parameters::INTER_CENTER_GAP;
        let scaling_factor = (scaffold_len as f64 * area_one_nucl as f64 / area_surface).sqrt();
        let scaled_perimeter = scaling_factor * perimeter_surface;

        // We use floor instead of round, because it works better to increase the revolution radius
        // to gain more nucleotide rather than diminishing it.
        let half_number_helix =
            (scaled_perimeter / 2. / Parameters::INTER_CENTER_GAP as f64).floor() as usize;

        Some(ensnano_gui::RevolutionScaling {
            nb_helix: half_number_helix * 2,
            scale: scaling_factor,
        })
    }

    fn get_clipboard_content(&self) -> ClipboardContent {
        self.0.design.get_clipboard_content()
    }

    fn get_pasting_status(&self) -> PastingStatus {
        self.get_pasting_status()
    }
}

#[cfg(test)]
mod tests {
    #[ignore]
    #[test]
    fn is_building_hyperboloid_implemented() {
        todo!()
    }
}
