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

mod impl_gui;
mod impl_scene;
mod poll;

use super::gui::UiSize;
use super::*;
use crate::mediator::{AppId, Selection};
pub use poll::*;
use ultraviolet::Vec3;

use super::design::{DnaAttribute, DnaElementKey, Nucl};
use super::gui::OrganizerTree;
use super::mediator::{
    Background3D, HyperboloidRequest, RenderingMode, RigidBodyConstants, SimulationRequest,
};
use super::scene::FogParameters;
use ensnano_design::grid::GridTypeDescr;

use std::collections::VecDeque;

/// A structure that contains all the requests that can be made through the GUI.
#[derive(Default)]
pub struct Requests {
    /// A change of the rotation mode
    pub action_mode: Option<ActionMode>,
    /// A change of the selection mode
    pub selection_mode: Option<SelectionMode>,
    /// A request to move the camera so that the frustrum fits the desgin
    pub fitting: Option<()>,
    /// A request to save the selected design
    pub file_save: Option<()>,
    /// A request to change the color of the selcted strand
    pub strand_color_change: Option<u32>,
    /// A request to change the sequence of the selected strand
    pub sequence_change: Option<String>,
    /// A request to show/hide the sequences
    pub toggle_text: Option<bool>,
    /// A request to change the sensitivity of scrolling
    pub scroll_sensitivity: Option<f32>,
    pub make_grids: Option<()>,
    pub operation_update: Option<Arc<dyn Operation>>,
    pub toggle_persistent_helices: Option<bool>,
    pub new_grid: Option<GridTypeDescr>,
    pub camera_rotation: Option<(f32, f32, f32)>,
    pub camera_target: Option<(Vec3, Vec3)>,
    pub small_spheres: Option<bool>,
    pub set_scaffold_id: Option<Option<usize>>,
    pub recolor_stapples: Option<()>,
    pub roll_request: Option<SimulationRequest>,
    pub show_torsion_request: Option<bool>,
    pub fog: Option<FogParameters>,
    pub hyperboloid_update: Option<HyperboloidRequest>,
    pub new_hyperboloid: Option<HyperboloidRequest>,
    pub finalize_hyperboloid: Option<()>,
    pub cancel_hyperboloid: Option<()>,
    pub helix_roll: Option<f32>,
    pub copy: Option<()>,
    pub paste: Option<()>,
    pub duplication: Option<()>,
    pub rigid_grid_simulation: Option<RigidBodyConstants>,
    pub rigid_helices_simulation: Option<RigidBodyConstants>,
    pub anchor: Option<()>,
    pub rigid_body_parameters: Option<RigidBodyConstants>,
    pub stapples_file: Option<(usize, PathBuf)>,
    pub keep_proceed: VecDeque<Action>,
    pub sequence_input: Option<String>,
    pub new_shift_hyperboloid: Option<f32>,
    pub organizer_selection: Option<Vec<DnaElementKey>>,
    pub organizer_candidates: Option<Vec<DnaElementKey>>,
    pub new_attribute: Option<(DnaAttribute, Vec<DnaElementKey>)>,
    pub new_tree: Option<OrganizerTree<DnaElementKey>>,
    pub split2d: Option<()>,
    pub toggle_visibility: Option<bool>,
    pub all_visible: Option<()>,
    pub redim_2d_helices: Option<bool>,
    pub stop_roll: Option<()>,
    pub toggle_widget: Option<()>,
    pub delete_selection: Option<()>,
    pub select_scaffold: Option<()>,
    pub scaffold_shift: Option<usize>,
    pub rendering_mode: Option<RenderingMode>,
    pub background3d: Option<Background3D>,
    pub undo: Option<()>,
    pub redo: Option<()>,
    pub save_shortcut: Option<()>,
    pub open_shortcut: Option<()>,
    pub force_help: Option<()>,
    pub show_tutorial: Option<()>,
    pub clean_requests: Option<()>,
    pub new_candidates: Option<Vec<Selection>>,
    pub new_selection: Option<Vec<Selection>>,
    pub new_paste_candidate: Option<Option<Nucl>>,
    pub paste_attempt: Option<Option<Nucl>>,
    pub xover_request: Option<(Nucl, Nucl, usize)>,
    pub suspend_op: Option<()>,
    pub center_selection: Option<(Selection, AppId)>,
}
