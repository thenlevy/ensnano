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

use std::ops::DerefMut;
pub fn poll_all<R: DerefMut<Target = Requests>>(mut requests: R, main_state: &mut MainState) {
    if requests.fitting.take().is_some() {
        mediator.lock().unwrap().request_fits();
    }

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
    }

    if requests.file_clear.take().is_some() {
        mediator.lock().unwrap().clear_designs();
    }

    if let Some((path, keep_proceed)) = requests.file_save.take() {
        let path_end = formated_path_end(&path);
        window.set_title(&format!("ENSnano: {}", path_end));
        mediator.lock().unwrap().save_design(&path);
        if let Some(keep_proceed) = keep_proceed {
            requests.keep_proceed.push_back(keep_proceed);
        }
    }

    if let Some(value) = requests.toggle_text {
        mediator.lock().unwrap().toggle_text(value);
        requests.toggle_text = None;
    }

    if let Some(value) = requests.toggle_scene {
        multiplexer.change_split(value);
        scheduler
            .lock()
            .unwrap()
            .forward_new_size(window.inner_size(), &multiplexer);
        gui.resize(&multiplexer, &window);
        requests.toggle_scene = None;
    }

    if requests.make_grids.take().is_some() {
        mediator.lock().unwrap().make_grids();
    }

    if let Some(grid_type) = requests.new_grid.take() {
        scene.lock().unwrap().make_new_grid(grid_type);
    }

    if let Some(selection_mode) = requests.selection_mode {
        mediator
            .lock()
            .unwrap()
            .change_selection_mode(selection_mode);
        requests.selection_mode = None;
    }

    if let Some(action_mode) = requests.action_mode.take() {
        println!("action mode {:?}", action_mode);
        mediator.lock().unwrap().change_action_mode(action_mode);
    }

    if let Some(sequence) = requests.sequence_change.take() {
        mediator.lock().unwrap().change_sequence(sequence);
    }
    if let Some(color) = requests.strand_color_change {
        mediator.lock().unwrap().change_strand_color(color);
        requests.strand_color_change = None;
    }
    if let Some(sensitivity) = requests.scroll_sensitivity.take() {
        mediator.lock().unwrap().change_sensitivity(sensitivity);
        //flat_scene.lock().unwrap().change_sensitivity(sensitivity);
    }

    if let Some(overlay_type) = requests.overlay_closed.take() {
        overlay_manager.rm_overlay(overlay_type, &mut multiplexer);
    }

    if let Some(overlay_type) = requests.overlay_opened.take() {
        overlay_manager.add_overlay(overlay_type, &mut multiplexer);
    }

    if let Some(op) = requests.operation_update.take() {
        mediator.lock().unwrap().update_pending(op)
    }

    if let Some(b) = requests.toggle_persistent_helices.take() {
        mediator.lock().unwrap().set_persistent_phantom(b)
    }

    if let Some(b) = requests.small_spheres.take() {
        println!("requested small spheres");
        mediator.lock().unwrap().set_small_spheres(b)
    }

    if let Some(point) = requests.camera_target.take() {
        mediator.lock().unwrap().set_camera_target(point)
    }

    if let Some(rotation) = requests.camera_rotation.take() {
        mediator.lock().unwrap().request_camera_rotation(rotation)
    }

    if let Some(scaffold_id) = requests.set_scaffold_id.take() {
        mediator.lock().unwrap().set_scaffold(scaffold_id)
    }

    if requests.recolor_stapples.take().is_some() {
        mediator.lock().unwrap().recolor_stapples();
    }

    if requests.clean_requests.take().is_some() {
        mediator.lock().unwrap().clean_designs();
    }

    if let Some(roll_request) = requests.roll_request.take() {
        mediator.lock().unwrap().roll_request(roll_request);
    }

    if let Some(b) = requests.show_torsion_request.take() {
        mediator.lock().unwrap().show_torsion_request(b)
    }

    if let Some(fog) = requests.fog.take() {
        scene.lock().unwrap().fog_request(fog)
    }

    if let Some(hyperboloid) = requests.new_hyperboloid.take() {
        use crate::design::Hyperboloid;
        let h = Hyperboloid {
            radius: hyperboloid.radius,
            length: hyperboloid.length,
            shift: hyperboloid.shift,
            radius_shift: hyperboloid.radius_shift,
            forced_radius: None,
        };
        scene.lock().unwrap().make_hyperboloid(h)
    }

    if let Some(hyperboloid) = requests.hyperboloid_update.take() {
        mediator.lock().unwrap().hyperboloid_update(hyperboloid)
    }

    if requests.finalize_hyperboloid.take().is_some() {
        mediator.lock().unwrap().finalize_hyperboloid();
    }

    if requests.cancel_hyperboloid.take().is_some() {
        mediator.lock().unwrap().cancel_hyperboloid();
    }

    if let Some(roll) = requests.helix_roll.take() {
        mediator.lock().unwrap().roll_helix(roll)
    }

    if requests.copy.take().is_some() {
        mediator.lock().unwrap().request_copy();
    }

    if requests.paste.take().is_some() {
        mediator.lock().unwrap().request_pasting_mode();
        requests.duplication = None;
    } else if requests.duplication.take().is_some() {
        mediator.lock().unwrap().request_duplication();
    }

    if let Some(b) = requests.rigid_grid_simulation.take() {
        mediator.lock().unwrap().rigid_grid_request(b);
    }

    if let Some(b) = requests.rigid_helices_simulation.take() {
        mediator.lock().unwrap().rigid_helices_request(b);
    }

    if let Some(p) = requests.rigid_body_parameters.take() {
        mediator.lock().unwrap().rigid_parameters_request(p);
    }

    if requests.anchor.take().is_some() {
        mediator.lock().unwrap().request_anchor();
    }
    if let Some((d_id, path)) = requests.stapples_file.take() {
        mediator.lock().unwrap().proceed_stapples(d_id, path);
    }

    if let Some(content) = requests.sequence_input.take() {
        messages.lock().unwrap().push_sequence(content);
    }

    if let Some(f) = requests.new_shift_hyperboloid.take() {
        mediator.lock().unwrap().new_shift_hyperboloid(f);
    }

    if let Some(s) = requests.organizer_selection.take() {
        mediator.lock().unwrap().organizer_selection(s);
    }

    if let Some(c) = requests.organizer_candidates.take() {
        mediator.lock().unwrap().organizer_candidates(c);
    }

    if let Some((a, elts)) = requests.new_attribute.take() {
        mediator.lock().unwrap().update_attribute(a, elts);
    }

    if let Some(tree) = requests.new_tree.take() {
        mediator.lock().unwrap().update_tree(tree);
    }

    if let Some(ui_size) = requests.new_ui_size.take() {
        gui.new_ui_size(ui_size.clone(), &window, &multiplexer);
        multiplexer.change_ui_size(ui_size.clone(), &window);
        messages.lock().unwrap().new_ui_size(ui_size);
        resized = true;
    }

    if requests.oxdna.take().is_some() {
        mediator.lock().unwrap().oxdna_export();
    }

    if requests.split2d.take().is_some() {
        mediator.lock().unwrap().split_2d();
    }

    if requests.all_visible.take().is_some() {
        mediator.lock().unwrap().make_everything_visible();
    }

    if let Some(b) = requests.toggle_visibility.take() {
        mediator.lock().unwrap().toggle_visibility(b);
    }

    if let Some(b) = requests.redim_2d_helices.take() {
        mediator.lock().unwrap().redim_2d_helices(b);
    }

    if let Some(b) = requests.invert_scroll.take() {
        multiplexer.invert_y_scroll = b;
    }

    if requests.stop_roll.take().is_some() {
        mediator.lock().unwrap().stop_roll();
    }

    if requests.toggle_widget.take().is_some() {
        mediator.lock().unwrap().toggle_widget();
    }

    if requests.delete_selection.take().is_some() {
        mediator.lock().unwrap().delete_selection();
    }

    if requests.select_scaffold.take().is_some() {
        mediator.lock().unwrap().select_scaffold();
    }

    if let Some(n) = requests.scaffold_shift.take() {
        mediator.lock().unwrap().set_scaffold_shift(n);
    }

    if let Some(mode) = requests.rendering_mode.take() {
        mediator.lock().unwrap().rendering_mode(mode);
    }

    if let Some(bg) = requests.background3d.take() {
        mediator.lock().unwrap().background3d(bg);
    }

    if requests.undo.take().is_some() {
        mediator.lock().unwrap().undo()
    }

    if requests.redo.take().is_some() {
        mediator.lock().unwrap().redo()
    }

    if requests.save_shortcut.take().is_some() {
        requests.keep_proceed.push_back(KeepProceed::SaveAs);
    }

    if requests.show_tutorial.take().is_some() {
        messages.lock().unwrap().push_show_tutorial()
    }

    if requests.force_help.take().is_some() {
        messages.lock().unwrap().show_help()
    }
}
