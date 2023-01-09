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
use ensnano_design::Nucl;

struct DummyScene {}
impl Application for DummyScene {
    type AppState = AppState;
    fn on_notify(&mut self, _notification: Notification) {
        ()
    }

    fn needs_redraw(&mut self, _dt: Duration, _app_state: Self::AppState) -> bool {
        false
    }

    fn on_redraw_request(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _target: &wgpu::TextureView,
        _dt: Duration,
    ) {
        ()
    }

    fn is_splited(&self) -> bool {
        false
    }

    fn on_event(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        app_state: &Self::AppState,
    ) -> Option<CursorIcon> {
        None
    }

    fn on_resize(
        &mut self,
        window_size: PhysicalSize<u32>,
        area: ensnano_interactor::graphics::DrawArea,
    ) {
        ()
    }

    fn get_camera(&self) -> Option<Arc<(ensnano_interactor::application::Camera3D, f32)>> {
        use ensnano_interactor::application::Camera3D;
        Some(Arc::new((
            Camera3D {
                position: Vec3::zero(),
                orientation: Rotor3::identity(),
                pivot_position: None,
            },
            1.0,
        )))
    }
}

fn new_state() -> MainState {
    let messages = Arc::new(Mutex::new(IcedMessages::new()));
    let constructor = MainStateConstructor { messages };
    let mut ret = MainState::new(constructor);
    ret.applications
        .insert(ElementType::Scene, Arc::new(Mutex::new(DummyScene {})));
    ret
}

#[test]
fn undoable_selection() {
    let mut state = new_state();
    let selection_1 = vec![Selection::Strand(0, 0), Selection::Strand(0, 1)];
    state.update_selection(selection_1.clone(), None);
    state.update_selection(vec![], None);
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
    state.update_selection(selection_1.clone(), None);
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
    state.update_selection(selection_1.clone(), None);
    state.update_selection(vec![], None);
    state.update_selection(vec![], None);
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
    AppState::import_design(path).ok().unwrap()
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
    main_state.update_selection(vec![Selection::Strand(0, 0)], None);
    main_state.request_duplication();
    assert_eq!(
        main_state.app_state.get_pasting_status(),
        PastingStatus::Duplication
    );
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 10,
            forward: true,
        }),
    )));
    main_state.apply_paste();
    assert_eq!(
        main_state.app_state.get_pasting_status(),
        PastingStatus::None
    );
    main_state.request_duplication();
    assert_eq!(
        main_state.app_state.get_pasting_status(),
        PastingStatus::None
    );
}

#[test]
fn duplication_via_requests_strands_are_duplicated() {
    use crate::scene::DesignReader;
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Strand(0, 0)], None);
    let initial_amount = main_state
        .get_app_state()
        .get_design_reader()
        .get_all_nucl_ids()
        .len();
    assert!(initial_amount > 0);
    main_state.request_duplication();
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 10,
            forward: true,
        }),
    )));
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
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Strand(0, 0)], None);
    main_state.request_duplication();
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 10,
            forward: true,
        }),
    )));
    main_state.apply_paste();
    main_state.request_duplication();
    assert_eq!(
        main_state.app_state.get_pasting_status(),
        PastingStatus::None
    );
    main_state.update_selection(vec![Selection::Strand(0, 0), Selection::Strand(0, 1)], None);
    main_state.request_duplication();
    assert_eq!(
        main_state.app_state.get_pasting_status(),
        PastingStatus::Duplication
    );
    main_state.update();
}

#[test]
fn position_paste_via_requests() {
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Xover(0, 0)], None);
    main_state.request_copy();
    let nucl = Nucl {
        helix: 1,
        position: 3,
        forward: true,
    };
    assert!(!main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&nucl)
        .to_opt()
        .is_some());
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(None));
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 3,
            forward: true,
        }),
    )));
    main_state.update();
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&nucl)
        .to_opt()
        .is_some());
}

#[test]
fn undo_redo_copy_paste_xover() {
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Xover(0, 0)], None);
    main_state.request_copy();
    let nucl = Nucl {
        helix: 1,
        position: 3,
        forward: true,
    };
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(None));
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 3,
            forward: true,
        }),
    )));
    main_state.apply_copy_operation(CopyOperation::Paste);
    main_state.update();
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&nucl)
        .to_opt()
        .is_some());
    main_state.undo();
    main_state.update();
    assert!(!main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&nucl)
        .to_opt()
        .is_some());
    main_state.redo();
    main_state.update();
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&nucl)
        .to_opt()
        .is_some());
}

#[test]
fn undo_redo_copy_paste_xover_pasting_status() {
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Xover(0, 0)], None);
    main_state.request_copy();
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(None));
    assert!(main_state.app_state.get_pasting_status().is_pasting());
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 3,
            forward: true,
        }),
    )));
    assert!(main_state.app_state.get_pasting_status().is_pasting());
    main_state.apply_copy_operation(CopyOperation::Paste);
    main_state.update();
    assert!(!main_state.app_state.get_pasting_status().is_pasting());
    main_state.undo();
    main_state.update();
    assert!(!main_state.app_state.get_pasting_status().is_pasting());
    main_state.redo();
    main_state.update();
    assert!(!main_state.app_state.get_pasting_status().is_pasting());
}

#[test]
fn duplicate_xover_pasting_status() {
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Xover(0, 0)], None);
    main_state.request_duplication();
    assert!(main_state.app_state.get_pasting_status().is_pasting());
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(Nucl {
            helix: 1,
            position: 5,
            forward: true,
        }),
    )));
    main_state.apply_paste();
    main_state.update();
    assert!(!main_state.app_state.get_pasting_status().is_pasting());
    main_state.request_duplication();
    main_state.update();
    assert!(!main_state.app_state.get_pasting_status().is_pasting());
}

#[test]
fn duplicate_xover() {
    let mut main_state = new_state();
    let app_state = pastable_design();
    main_state.clear_app_state(app_state);
    main_state.update_selection(vec![Selection::Xover(0, 0)], None);
    main_state.request_duplication();
    let n1 = Nucl {
        helix: 1,
        position: 5,
        forward: true,
    };
    let n2 = Nucl {
        helix: 1,
        position: 3,
        forward: true,
    };
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&n1)
        .to_opt()
        .is_none());
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&n2)
        .to_opt()
        .is_none());
    main_state.apply_copy_operation(CopyOperation::PositionPastingPoint(Some(
        PastePosition::Nucl(n1),
    )));
    main_state.apply_paste();
    main_state.update();
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&n1)
        .to_opt()
        .is_some());
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&n2)
        .to_opt()
        .is_none());
    main_state.request_duplication();
    main_state.update();
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&n1)
        .to_opt()
        .is_some());
    assert!(main_state
        .app_state
        .get_design_reader()
        .is_xover_end(&n2)
        .to_opt()
        .is_some());
}

#[test]
fn default_app_state_does_not_need_save() {
    let mut main_state = new_state();
    assert!(!main_state.need_save(), "Need save before update");
    main_state.update();
    assert!(!main_state.need_save(), "Need save after update");
}

#[test]
fn no_need_to_save_after_new_design() {
    let mut main_state = new_state();
    main_state.new_design();
    assert!(!main_state.need_save(), "Need save before update");
    main_state.update();
    assert!(!main_state.need_save(), "Need save after update");
}
