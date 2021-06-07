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
use crate::flatscene::Requests as FlatSceneRequests;

impl FlatSceneRequests for Requests {
    fn xover_request(&mut self, source: Nucl, target: Nucl, design_id: usize) {
        self.xover_request = Some((source, target, design_id));
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
        self.paste_attempt = Some(nucl);
    }

    fn request_centering_on_nucl(&mut self, nucl: Nucl, design_id: usize) {
        self.centering_on_nucl = Some((nucl, design_id));
    }

    fn update_opperation(&mut self, operation: Arc<dyn Operation>) {
        self.operation_update = Some(operation);
    }
}
