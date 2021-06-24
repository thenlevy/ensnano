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

use scene::AppState;

#[test]
fn undoable_selection() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone());
    state.update_selection(vec![]);
    state.undo();
    assert_eq!(state.app_state.get_selection().clone(), selection_1);
}

#[test]
fn redoable_selection() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone());
    state.undo();
    assert_eq!(state.app_state.get_selection().clone(), vec![]);
    state.redo();
    assert_eq!(state.app_state.get_selection().clone(), selection_1);
}

#[test]
fn empty_selections_dont_pollute_undo_stack() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone());
    state.update_selection(vec![]);
    state.update_selection(vec![]);
    state.undo();
    assert_eq!(state.app_state.get_selection().clone(), selection_1);
}

#[test]
fn recolor_stapple_undoable() {
    let mut state = new_state();
    state.apply_operation(DesignOperation::RecolorStaples);
    assert!(!state.undo_stack.is_empty())
}
