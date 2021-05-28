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

use crate::gui::{Requests as GuiRequests, RigidBodyParametersRequest};
use crate::mediator::RigidBodyConstants;

use super::*;

impl GuiRequests for Requests {
    fn ask_use_default_scaffold(&mut self) {
        self.keep_proceed = Some(KeepProceed::AskUseDefaultScafSequence)
    }

    fn close_overlay(&mut self, overlay_type: OverlayType) {
        self.overlay_closed = Some(overlay_type);
    }

    fn open_overlay(&mut self, overlay_type: OverlayType) {
        self.overlay_opened = Some(overlay_type);
    }

    fn change_strand_color(&mut self, color: u32) {
        self.strand_color_change = Some(color);
    }

    fn change_3d_background(&mut self, bg: Background3D) {
        self.background3d = Some(bg);
    }

    fn change_3d_rendering_mode(&mut self, mode: RenderingMode) {
        self.rendering_mode = Some(mode);
    }

    fn set_scaffold_from_selection(&mut self) {
        self.select_scaffold = Some(())
    }

    fn cancel_hyperboloid(&mut self) {
        self.cancel_hyperboloid = Some(())
    }

    fn invert_scroll(&mut self, inverted: bool) {
        self.invert_scroll = Some(inverted);
    }

    fn resize_2d_helices(&mut self, all: bool) {
        self.redim_2d_helices = Some(all);
    }

    fn make_all_elements_visible(&mut self) {
        self.all_visible = Some(());
    }

    fn toggle_visibility(&mut self, visible: bool) {
        self.toggle_visibility = Some(visible);
    }

    fn remove_empty_domains(&mut self) {
        self.clean_requests = Some(());
    }

    fn change_action_mode(&mut self, action_mode: ActionMode) {
        self.action_mode = Some(action_mode);
    }

    fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.selection_mode = Some(selection_mode);
    }

    fn toggle_widget_basis(&mut self) {
        self.toggle_widget = Some(());
    }

    fn set_dna_sequences_visibility(&mut self, visible: bool) {
        self.toggle_text = Some(visible);
    }

    fn download_stapples(&mut self) {
        self.stapples_request = Some(())
    }

    fn set_selected_strand_sequence(&mut self, sequence: String) {
        self.sequence_change = Some(sequence);
    }

    fn set_scaffold_sequence(&mut self, sequence: String) {
        self.scaffold_sequence = Some(sequence);
    }

    fn set_scaffold_shift(&mut self, shift: usize) {
        self.scaffold_shift = Some(shift);
    }

    fn set_ui_size(&mut self, size: UiSize) {
        self.new_ui_size = Some(size);
    }

    fn finalize_hyperboloid(&mut self) {
        self.finalize_hyperboloid = Some(())
    }

    fn stop_roll_simulation(&mut self) {
        self.stop_roll = Some(())
    }

    fn start_roll_simulation(&mut self, request: SimulationRequest) {
        self.roll_request = Some(request);
    }

    fn make_grid_from_selection(&mut self) {
        self.make_grids = Some(());
    }

    fn update_rigid_helices_simulation(&mut self, parameters: RigidBodyParametersRequest) {
        let rigid_body_paramters = rigid_parameters(parameters);
        self.rigid_body_parameters = Some(rigid_body_paramters);
    }
}

fn rigid_parameters(parameters: RigidBodyParametersRequest) -> RigidBodyConstants {
    let ret = RigidBodyConstants {
        k_spring: 10f32.powf(parameters.k_springs),
        k_friction: 10f32.powf(parameters.k_friction),
        mass: 10f32.powf(parameters.mass_factor),
        volume_exclusion: parameters.volume_exclusion,
        brownian_motion: parameters.brownian_motion,
        brownian_rate: 10f32.powf(parameters.brownian_rate),
        brownian_amplitude: parameters.brownian_amplitude,
    };
    println!("{:?}", ret);
    ret
}
