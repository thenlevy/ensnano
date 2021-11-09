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

use ensnano_interactor::{application::Notification, HyperboloidOperation, SelectionConversion};

use std::ops::DerefMut;
pub(crate) fn poll_all<R: DerefMut<Target = Requests>>(
    mut requests: R,
    main_state: &mut MainState,
) {
    if requests.fitting.take().is_some() {
        main_state.push_action(Action::NotifyApps(Notification::FitRequest))
    }

    /*
    if let Some(ref path) = requests.file_add.take() {
        let design = Design::new_with_path(0, path);
        let path_end = formated_path_end(path);
        if let Ok(design) = design {
            window.set_title(&format!("ENSnano: {}", path_end));
            messages.lock().unwrap().notify_new_design();
            if let Some(tree) = design.get_organizer_tree() {
                messages.lock().unwrap().push_new_tree(tree)
            }
            mediator.lock().unwrap().clear_designs();
            let design = Arc::new(RwLock::new(design));
            mediator.lock().unwrap().add_design(design);
        } else {
            //TODO
        }
    }*/

    /*
    if let Some((path, keep_proceed)) = requests.file_save.take() {
        let path_end = formated_path_end(&path);
        window.set_title(&format!("ENSnano: {}", path_end));
        mediator.lock().unwrap().save_design(&path);
        if let Some(keep_proceed) = keep_proceed {
            requests.keep_proceed.push_back(keep_proceed);
        }
    }*/

    if let Some(value) = requests.toggle_text.take() {
        main_state.push_action(Action::NotifyApps(Notification::ToggleText(value)))
    }

    /*
    if let Some(value) = requests.toggle_scene {
        multiplexer.change_split(value);
        scheduler
            .lock()
            .unwrap()
            .forward_new_size(window.inner_size(), &multiplexer);
        gui.resize(&multiplexer, &window);
        requests.toggle_scene = None;
    }*/

    if requests.make_grids.take().is_some() {
        main_state.push_action(Action::TurnSelectionIntoGrid);
    }

    if let Some(grid_type) = requests.new_grid.take() {
        main_state.push_action(Action::AddGrid(grid_type));
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
        main_state.push_action(Action::NotifyApps(Notification::NewSensitivity(
            sensitivity,
        )))
    }

    /*
    if let Some(overlay_type) = requests.overlay_closed.take() {
        overlay_manager.rm_overlay(overlay_type, &mut multiplexer);
    }

    if let Some(overlay_type) = requests.overlay_opened.take() {
        overlay_manager.add_overlay(overlay_type, &mut multiplexer);
    }*/

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

    if let Some(parameters) = requests.rigid_helices_simulation.take() {
        main_state.push_action(Action::RigidHelicesSimulation { parameters })
    }

    if let Some(parameters) = requests.rigid_body_parameters.take() {
        main_state.push_action(Action::RigidParametersUpdate(parameters))
    }

    if requests.anchor.take().is_some() {
        main_state.push_action(Action::TurnIntoAnchor)
    }

    /*
    if let Some((d_id, path)) = requests.stapples_file.take() {
        mediator.lock().unwrap().proceed_stapples(d_id, &path);
    }*/

    /*
    if let Some(content) = requests.sequence_input.take() {
        main_state.messages.lock().unwrap().push_sequence(content);
    }*/

    if let Some(f) = requests.new_shift_hyperboloid.take() {
        main_state.push_action(Action::UpdateHyperboloidShift(f))
    }

    if let Some((s, g_id, new_group)) = requests.organizer_selection.take() {
        let selection = s.into_iter().map(|e| e.to_selection(0)).collect();
        if new_group && g_id.is_some() {
            main_state.transfer_selection_pivot_to_group(g_id.unwrap());
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

    /*
    if let Some(ui_size) = requests.new_ui_size.take() {
        gui.new_ui_size(ui_size.clone(), &window, &multiplexer);
        multiplexer.change_ui_size(ui_size.clone(), &window);
        messages.lock().unwrap().new_ui_size(ui_size);
        resized = true;
    }*/

    /*
    if requests.oxdna.take().is_some() {
        mediator.lock().unwrap().oxdna_export();
    }*/

    if requests.split2d.take().is_some() {
        main_state.push_action(Action::Split2D)
    }

    if requests.all_visible.take().is_some() {
        main_state.push_action(Action::ClearVisibilitySieve)
    }

    if let Some(b) = requests.toggle_visibility.take() {
        main_state.push_action(Action::SetVisiblitySieve { compl: b })
    }

    /*
    if let Some(b) = requests.invert_scroll.take() {
        multiplexer.invert_y_scroll = b;
    }*/

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
        main_state.push_action(Action::NotifyApps(Notification::RenderingMode(mode)))
    }

    if let Some(bg) = requests.background3d.take() {
        main_state.push_action(Action::NotifyApps(Notification::Background3D(bg)))
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
            .push_back(Action::PasteCandidate(candidate))
    }

    for action in requests.keep_proceed.drain(..) {
        main_state.pending_actions.push_back(action)
    }

    if let Some(param) = requests.new_suggestion_parameters.take() {
        main_state.set_suggestion_parameters(param);
    }
}
