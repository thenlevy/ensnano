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

//! Test suite for the `MainState` structure

use super::*;

fn new_state() -> MainState {
    let messages = Arc::new(Mutex::new(IcedMessages::new()));
    let constructor = MainStateConstructor { messages };
    MainState::new(constructor)
}

#[test]
fn undoable_selection() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone());
    state.update_selection(vec![]);
    state.undo();
    assert_eq!(
        state.app_state.get_selection().as_ref().clone(),
        selection_1
    );
}

#[test]
fn redoable_selection() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone());
    state.undo();
    assert_eq!(state.app_state.get_selection().as_ref().clone(), vec![]);
    state.redo();
    assert_eq!(
        state.app_state.get_selection().as_ref().clone(),
        selection_1
    );
}

#[test]
fn empty_selections_dont_pollute_undo_stack() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone());
    state.update_selection(vec![]);
    state.update_selection(vec![]);
    state.undo();
    assert_eq!(
        state.app_state.get_selection().as_ref().clone(),
        selection_1
    );
}

#[test]
fn recolor_stapple_undoable() {
    let mut state = new_state();
    state.apply_operation(DesignOperation::RecolorStaples);
    assert!(!state.undo_stack.is_empty())
}

/// A design with one strand h1: -1 -> 7 ; h2: -1 <- 7 ; h3: 0 -> 9 that can be pasted on
/// helices 4, 5 and 6
fn pastable_design() -> AppState {
    let path = test_path("pastable.json");
    AppState::import_design(&path).ok().unwrap()
}

fn test_path(design_name: &'static str) -> PathBuf {
    let mut ret = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
    ret.push("tests");
    ret.push(design_name);
    ret
}

#[test]
fn duplication_via_requests_correct_status() {
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Strand(0, 0)]);
    main_state.request_duplication();
    assert_eq!(
        main_state.app_state.is_pasting(),
        PastingStatus::Duplication
    );
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(Nucl {
        helix: 1,
        position: 10,
        forward: true,
    })));
    main_state.apply_paste();
    assert_eq!(main_state.app_state.is_pasting(), PastingStatus::None);
    main_state.request_duplication();
    assert_eq!(main_state.app_state.is_pasting(), PastingStatus::None);
}

#[test]
fn duplication_via_requests_strands_are_duplicated() {
    use crate::scene::DesignReader;
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Strand(0, 0)]);
    let initial_amount = main_state
        .get_app_state()
        .get_design_reader()
        .get_all_nucl_ids()
        .len();
    assert!(initial_amount > 0);
    main_state.request_duplication();
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(Nucl {
        helix: 1,
        position: 10,
        forward: true,
    })));
    main_state.apply_paste();
    main_state.update();
    let amount = main_state
        .get_app_state()
        .get_design_reader()
        .get_all_nucl_ids()
        .len();
    assert_eq!(amount, 2 * initial_amount);
    main_state.request_duplication();
    main_state.update();
    let amount = main_state
        .get_app_state()
        .get_design_reader()
        .get_all_nucl_ids()
        .len();
    assert_eq!(amount, 3 * initial_amount);
    main_state.request_duplication();
    main_state.update();
    let amount = main_state
        .get_app_state()
        .get_design_reader()
        .get_all_nucl_ids()
        .len();
    assert_eq!(amount, 4 * initial_amount);
}

#[test]
fn new_selection_empties_duplication_clipboard() {
    use crate::scene::DesignReader;
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Strand(0, 0)]);
    let initial_amount = main_state
        .get_app_state()
        .get_design_reader()
        .get_all_nucl_ids()
        .len();
    main_state.request_duplication();
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(Nucl {
        helix: 1,
        position: 10,
        forward: true,
    })));
    main_state.apply_paste();
    main_state.request_duplication();
    assert_eq!(main_state.app_state.is_pasting(), PastingStatus::None);
    main_state.update_selection(vec![Selection::Strand(0, 0), Selection::Strand(0, 1)]);
    main_state.request_duplication();
    assert_eq!(
        main_state.app_state.is_pasting(),
        PastingStatus::Duplication
    );
    main_state.update();
}
