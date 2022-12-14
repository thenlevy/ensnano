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

//! This modules defines the `poll_all` method that polls the requests stored in a `Requests`
//! object.

use super::*;
use crate::PastePosition;

use ensnano_interactor::{application::Notification, HyperboloidOperation, SelectionConversion};

use std::ops::DerefMut;
pub(crate) fn poll_all<R: DerefMut<Target = Requests>>(
    mut requests: R,
    main_state: &mut MainState,
) {
    if requests.fitting.take().is_some() {
        main_state.push_action(Action::NotifyApps(Notification::FitRequest))
    }

    if let Some(value) = requests.toggle_text.take() {
        main_state.push_action(Action::NotifyApps(Notification::ToggleText(value)))
    }

    if requests.make_grids.take().is_some() {
        main_state.push_action(Action::TurnSelectionIntoGrid);
    }

    if let Some(grid_type) = requests.new_grid.take() {
        main_state.push_action(Action::AddGrid(grid_type));
    }

    if requests.new_bezier_plane.take().is_some() {
        main_state.push_action(Action::AddBezierPlane);
    }

    if let Some(selection_mode) = requests.selection_mode.take() {
        main_state.change_selection_mode(selection_mode)
    }

    if let Some(action_mode) = requests.action_mode.take() {
        main_state.change_action_mode(action_mode)
    }

    if let Some(double_strand_parameters) = requests.new_double_strand_parameters.take() {
        main_state.change_double_strand_parameters(double_strand_parameters)
    }

    if let Some(sequence) = requests.sequence_change.take() {
        main_state.push_action(Action::ChangeSequence(sequence))
    }

    if let Some(color) = requests.strand_color_change.take() {
        main_state.push_action(Action::ChangeColorStrand(color))
    }

    if let Some(sensitivity) = requests.scroll_sensitivity.take() {
        main_state.set_scroll_sensitivity(sensitivity)
    }

    if let Some(op) = requests.operation_update.take() {
        main_state.update_pending_operation(op);
    }

    if let Some(b) = requests.toggle_persistent_helices.take() {
        main_state.push_action(Action::ToggleHelicesPersistance(b))
    }

    if let Some(b) = requests.small_spheres.take() {
        main_state.push_action(Action::ToggleSmallSphere(b))
    }

    if let Some(target) = requests.camera_target.take() {
        main_state.push_action(Action::NotifyApps(Notification::CameraTarget(target)))
    }

    if let Some(rotation) = requests.camera_rotation.take() {
        main_state.push_action(Action::NotifyApps(Notification::CameraRotation(
            rotation.0, rotation.1, rotation.2,
        )))
    }

    if let Some(scaffold_id) = requests.set_scaffold_id.take() {
        main_state.push_action(Action::DesignOperation(DesignOperation::SetScaffoldId(
            scaffold_id,
        )))
    }

    if requests.recolor_stapples.take().is_some() {
        main_state.push_action(Action::DesignOperation(DesignOperation::RecolorStaples))
    }

    if let Some(roll_request) = requests.roll_request.take() {
        main_state.push_action(Action::RollRequest(roll_request))
    }

    if let Some(b) = requests.show_torsion_request.take() {
        main_state.push_action(Action::NotifyApps(Notification::ShowTorsion(b)))
    }

    if let Some(fog) = requests.fog.take() {
        main_state.push_action(Action::Fog(fog))
    }

    if let Some(hyperboloid) = requests.new_hyperboloid.take() {
        main_state.push_action(Action::NewHyperboloid(hyperboloid))
    }

    if let Some(hyperboloid) = requests.hyperboloid_update.take() {
        main_state.push_action(Action::DesignOperation(
            DesignOperation::HyperboloidOperation(HyperboloidOperation::Update(hyperboloid)),
        ))
    }

    if requests.finalize_hyperboloid.take().is_some() {
        main_state.push_action(Action::DesignOperation(
            DesignOperation::HyperboloidOperation(HyperboloidOperation::Finalize),
        ))
    }

    if requests.cancel_hyperboloid.take().is_some() {
        main_state.push_action(Action::DesignOperation(
            DesignOperation::HyperboloidOperation(HyperboloidOperation::Cancel),
        ))
    }

    if let Some(roll) = requests.helix_roll.take() {
        main_state.push_action(Action::RollHelices(roll))
    }

    if requests.copy.take().is_some() {
        main_state.push_action(Action::Copy)
    }

    if requests.paste.take().is_some() {
        main_state.push_action(Action::InitPaste);
        requests.duplication = None;
    } else if requests.duplication.take().is_some() {
        main_state.push_action(Action::Duplicate)
    }

    if let Some(parameters) = requests.rigid_grid_simulation.take() {
        main_state.push_action(Action::RigidGridSimulation { parameters })
    }

    if let Some(g_id) = requests.twist_simulation.take() {
        main_state.push_action(Action::Twist(g_id))
    }

    if let Some(parameters) = requests.rigid_helices_simulation.take() {
        main_state.push_action(Action::RigidHelicesSimulation { parameters })
    }

    if let Some(parameters) = requests.rigid_body_parameters.take() {
        main_state.push_action(Action::RigidParametersUpdate(parameters))
    }

    if requests.anchor.take().is_some() {
        main_state.push_action(Action::TurnIntoAnchor)
    }

    if let Some(f) = requests.new_shift_hyperboloid.take() {
        main_state.push_action(Action::UpdateHyperboloidShift(f))
    }

    if let Some((s, g_id, new_group)) = requests.organizer_selection.take() {
        let selection = s.into_iter().map(|e| e.to_selection(0)).collect();

        if new_group {
            if let Some(g_id) = g_id {
                main_state.transfer_selection_pivot_to_group(g_id);
            }
        }

        main_state.update_selection(selection, g_id);
    }

    if let Some(c) = requests.organizer_candidates.take() {
        let candidates = c.into_iter().map(|e| e.to_selection(0)).collect();
        main_state.update_candidates(candidates);
    }

    if let Some((attribute, elements)) = requests.new_attribute.take() {
        main_state.push_action(Action::DesignOperation(DesignOperation::UpdateAttribute {
            attribute,
            elements,
        }))
    }

    if let Some(tree) = requests.new_tree.take() {
        main_state.push_action(Action::DesignOperation(DesignOperation::SetOrganizerTree(
            tree,
        )));
    }

    if requests.clean_requests.take().is_some() {
        main_state.push_action(Action::DesignOperation(DesignOperation::CleanDesign))
    }

    if requests.split2d.take().is_some() {
        main_state.push_action(Action::Split2D)
    }

    if requests.all_visible.take().is_some() {
        main_state.push_action(Action::ClearVisibilitySieve)
    }

    if let Some(b) = requests.toggle_visibility.take() {
        main_state.push_action(Action::SetVisiblitySieve { compl: b })
    }

    if let Some(b) = requests.set_invert_y_scroll.take() {
        main_state.set_invert_y_scroll(b)
    }

    if requests.delete_selection.take().is_some() {
        main_state.push_action(Action::DeleteSelection)
    }

    if requests.select_scaffold.take().is_some() {
        println!("select scaffold");
        main_state.push_action(Action::ScaffoldToSelection)
    }

    if let Some(n) = requests.scaffold_shift.take() {
        main_state.push_action(Action::DesignOperation(DesignOperation::SetScaffoldShift(
            n,
        )))
    }

    if let Some(mode) = requests.rendering_mode.take() {
        main_state.set_rendering_mode(mode);
    }

    if let Some(bg) = requests.background3d.take() {
        main_state.set_background_3d(bg);
    }

    if requests.undo.take().is_some() {
        main_state.push_action(Action::Undo);
    }

    if requests.redo.take().is_some() {
        main_state.push_action(Action::Redo);
    }

    if requests.save_shortcut.take().is_some() {
        main_state.pending_actions.push_back(Action::QuickSave);
    }

    if requests.show_tutorial.take().is_some() {
        main_state.messages.lock().unwrap().push_show_tutorial()
    }

    if requests.force_help.take().is_some() {
        main_state.messages.lock().unwrap().show_help()
    }

    if let Some(candidates) = requests.new_candidates.take() {
        main_state.update_candidates(candidates);
    }

    if let Some(selection) = requests.new_selection.take() {
        main_state.update_selection(selection, None);
        if let Some(center) = requests.new_center_of_selection.take() {
            main_state.update_center_of_selection(center);
        }
    }

    if requests.toggle_widget_basis.take().is_some() {
        main_state.toggle_widget_basis()
    }

    if requests.stop_roll.take().is_some() {
        main_state.pending_actions.push_back(Action::StopSimulation)
    }

    if requests.suspend_op.take().is_some() {
        requests.keep_proceed.push_back(Action::SuspendOp);
    }

    if requests.horizon_targeted.take().is_some() {
        main_state
            .pending_actions
            .push_back(Action::NotifyApps(Notification::HorizonAligned))
    }

    if let Some(all_helices) = requests.redim_2d_helices.take() {
        main_state
            .pending_actions
            .push_back(Action::NotifyApps(Notification::Redim2dHelices(
                all_helices,
            )))
    }

    if let Some((selection, app_id)) = requests.center_selection.take() {
        main_state
            .pending_actions
            .push_back(Action::NotifyApps(Notification::CenterSelection(
                selection, app_id,
            )))
    }

    if let Some(candidate) = requests.new_paste_candiate.take() {
        main_state
            .pending_actions
            .push_back(Action::PasteCandidate(candidate.map(PastePosition::Nucl)))
    }

    if let Some(candidate) = requests.new_grid_paste_candidate.take() {
        main_state
            .pending_actions
            .push_back(Action::PasteCandidate(Some(PastePosition::GridPosition(
                candidate,
            ))))
    }

    for action in requests.keep_proceed.drain(..) {
        main_state.pending_actions.push_back(action)
    }

    if let Some(param) = requests.new_suggestion_parameters.take() {
        main_state.set_suggestion_parameters(param);
    }

    if let Some(param) = requests.check_xover_parameters.take() {
        main_state.set_check_xovers_parameters(param);
    }

    if let Some(b) = requests.follow_stereographic_camera.take() {
        main_state.set_follow_stereographic_camera(b);
    }

    if let Some(b) = requests.set_show_stereographic_camera.take() {
        main_state.set_show_stereographic_camera(b);
    }

    if let Some(b) = requests.set_show_h_bonds.take() {
        main_state.set_show_h_bonds(b);
    }

    if let Some(b) = requests.set_show_bezier_paths.take() {
        main_state.set_show_bezier_paths(b);
    }

    if let Some(b) = requests.set_thick_helices.take() {
        main_state.set_thick_helices(b);
    }

    if let Some(()) = requests.toggle_thick_helices.take() {
        main_state.toggle_thick_helices();
    }

    if let Some(id) = requests.new_bezier_revolution_id.take() {
        main_state.set_bezier_revolution_id(id)
    }

    if let Some(radius) = requests.new_bezier_revolution_radius.take() {
        main_state.set_bezier_revolution_radius(radius)
    }

    if let Some(position) = requests.new_bezier_revolution_axis_position.take() {
        main_state.set_revolution_axis_position(position)
    }

    if let Some(surface) = requests.new_unrooted_surface.take() {
        main_state.set_unrooted_surface(surface);
    }

    if requests.switched_to_revolution_tab.take().is_some() {
        main_state.create_default_bezier_plane();
    }
}
