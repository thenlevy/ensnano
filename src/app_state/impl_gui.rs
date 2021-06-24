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
use ensnano_design::Parameters;
use ensnano_interactor::SimulationState;

impl GuiState for AppState {
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
}
