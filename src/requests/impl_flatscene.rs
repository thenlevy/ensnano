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

//! Implements the [Requests](`crate::flatscene::Requests`) trait for [Requests](`super::Requests`).

use super::*;
use crate::flatscene::Requests as FlatSceneRequests;

use ultraviolet::Isometry2;

impl FlatSceneRequests for Requests {
    fn xover_request(&mut self, source: Nucl, target: Nucl, _design_id: usize) {
        self.keep_proceed
            .push_back(Action::DesignOperation(DesignOperation::GeneralXover {
                source,
                target,
            }))
    }

    fn request_center_selection(&mut self, selection: Selection, app_id: AppId) {
        self.center_selection = Some((selection, app_id));
    }

    fn new_selection(&mut self, selection: Vec<Selection>) {
        self.new_selection = Some(selection);
    }

    fn new_candidates(&mut self, candidates: Vec<Selection>) {
        self.new_candidates = Some(candidates);
    }

    fn attempt_paste(&mut self, nucl: Option<Nucl>) {
        self.keep_proceed
            .push_back(Action::PasteCandidate(nucl.map(PastePosition::Nucl)));
        self.keep_proceed.push_back(Action::ApplyPaste);
    }

    fn request_centering_on_nucl(&mut self, nucl: Nucl, design_id: usize) {
        self.centering_on_nucl = Some((nucl, design_id));
    }

    fn update_opperation(&mut self, operation: Arc<dyn Operation>) {
        self.operation_update = Some(operation);
    }

    fn set_isometry(&mut self, helix: usize, segment: usize, isometry: Isometry2) {
        self.keep_proceed.push_back(Action::SilentDesignOperation(
            DesignOperation::SetIsometry {
                helix,
                isometry,
                segment,
            },
        ))
    }

    fn set_visibility_helix(&mut self, helix: usize, visibility: bool) {
        self.keep_proceed.push_back(Action::DesignOperation(
            DesignOperation::SetVisibilityHelix {
                helix,
                visible: visibility,
            },
        ))
    }

    fn flip_group(&mut self, helix: usize) {
        self.keep_proceed
            .push_back(Action::DesignOperation(DesignOperation::FlipHelixGroup {
                helix,
            }))
    }

    fn suspend_op(&mut self) {
        self.keep_proceed.push_back(Action::SuspendOp);
    }

    fn apply_design_operation(&mut self, op: DesignOperation) {
        self.keep_proceed.push_back(Action::DesignOperation(op))
    }

    fn set_paste_candidate(&mut self, candidate: Option<Nucl>) {
        self.new_paste_candiate = Some(candidate);
    }
}
