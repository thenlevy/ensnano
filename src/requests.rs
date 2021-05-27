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

use super::gui::UiSize;
use super::*;
use ultraviolet::Vec3;

/// A structure that contains all the requests that can be made through the GUI.
#[derive(Default)]
pub struct Requests {
    /// A change of the rotation mode
    pub action_mode: Option<ActionMode>,
    /// A change of the selection mode
    pub selection_mode: Option<SelectionMode>,
    /// A request to move the camera so that the frustrum fits the desgin
    pub fitting: bool,
    /// A request to load a design into the scene
    pub file_add: Option<PathBuf>,
    /// A request to remove all designs
    pub file_clear: bool,
    /// A request to save the selected design
    pub file_save: Option<(PathBuf, Option<KeepProceed>)>,
    /// A request to change the color of the selcted strand
    pub strand_color_change: Option<u32>,
    /// A request to change the sequence of the selected strand
    pub sequence_change: Option<String>,
    /// A request to show/hide the sequences
    pub toggle_text: Option<bool>,
    /// A request to change the view
    pub toggle_scene: Option<SplitMode>,
    /// A request to change the sensitivity of scrolling
    pub scroll_sensitivity: Option<f32>,
    pub make_grids: bool,
    pub overlay_closed: Option<OverlayType>,
    pub overlay_opened: Option<OverlayType>,
    pub operation_update: Option<Arc<dyn Operation>>,
    pub toggle_persistent_helices: Option<bool>,
    pub new_grid: Option<GridTypeDescr>,
    pub camera_rotation: Option<(f32, f32, f32)>,
    pub camera_target: Option<(Vec3, Vec3)>,
    pub small_spheres: Option<bool>,
    pub set_scaffold_id: Option<Option<usize>>,
    pub scaffold_sequence: Option<(String, usize)>,
    pub stapples_request: bool,
    pub recolor_stapples: bool,
    pub clean_requests: bool,
    pub roll_request: Option<SimulationRequest>,
    pub show_torsion_request: Option<bool>,
    pub fog: Option<FogParameters>,
    pub hyperboloid_update: Option<HyperboloidRequest>,
    pub new_hyperboloid: Option<HyperboloidRequest>,
    pub finalize_hyperboloid: bool,
    pub cancel_hyperboloid: bool,
    pub helix_roll: Option<f32>,
    pub copy: bool,
    pub paste: bool,
    pub duplication: bool,
    pub rigid_grid_simulation: Option<RigidBodyParametersRequest>,
    pub rigid_helices_simulation: Option<RigidBodyParametersRequest>,
    pub anchor: bool,
    pub rigid_body_parameters: Option<RigidBodyParametersRequest>,
    pub stapples_file: Option<(usize, PathBuf)>,
    pub keep_proceed: Option<KeepProceed>,
    pub sequence_input: Option<String>,
    pub new_shift_hyperboloid: Option<f32>,
    pub organizer_selection: Option<Vec<crate::design::DnaElementKey>>,
    pub organizer_candidates: Option<Vec<crate::design::DnaElementKey>>,
    pub new_attribute: Option<(
        crate::design::DnaAttribute,
        Vec<crate::design::DnaElementKey>,
    )>,
    pub new_tree: Option<OrganizerTree<crate::design::DnaElementKey>>,
    pub new_ui_size: Option<UiSize>,
    pub oxdna: bool,
    pub split2d: bool,
    pub toggle_visibility: Option<bool>,
    pub all_visible: bool,
    pub redim_2d_helices: Option<bool>,
    pub invert_scroll: Option<bool>,
    pub stop_roll: bool,
    pub toggle_widget: bool,
    pub delete_selection: bool,
    pub select_scaffold: Option<()>,
    pub scaffold_shift: Option<usize>,
    pub rendering_mode: Option<crate::mediator::RenderingMode>,
    pub background3d: Option<crate::mediator::Background3D>,
    pub undo: Option<()>,
    pub redo: Option<()>,
    pub save_shortcut: Option<()>,
    pub open_shortcut: Option<()>,
    pub force_help: Option<()>,
    pub show_tutorial: Option<()>,
}
