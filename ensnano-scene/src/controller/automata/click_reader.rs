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

/// A structure that translate cursor position to SceneElement
pub(super) struct ClickReader<'a, S: AppState> {
    pub controller: &'a Controller<S>,
    pub pixel_reader: &'a mut ElementSelector,
    pub app_state: &'a S,
    pub cursor_position: PhysicalPosition<f64>,
}

impl<'a, S: AppState> ClickReader<'a, S> {
    /// Return the SceneElement on which to place the camera rotation pivot
    pub fn get_pivot_element(&mut self) -> Option<SceneElement> {
        match self.pixel_reader.set_selected_id(self.cursor_position) {
            Some(SceneElement::Grid(d_id, g_id)) => {
                // for grids we take the precise grid position on which the user clicked.
                let mouse_x = self.cursor_position.x / self.controller.area_size.width as f64;
                let mouse_y = self.cursor_position.y / self.controller.area_size.height as f64;
                if let Some(intersection) = self
                    .controller
                    .view
                    .borrow()
                    .specific_grid_intersection(mouse_x as f32, mouse_y as f32, g_id)
                {
                    Some(SceneElement::GridCircle(d_id, intersection.grid_position()))
                } else {
                    Some(SceneElement::Grid(d_id, g_id))
                }
            }
            element => element,
        }
    }
}
