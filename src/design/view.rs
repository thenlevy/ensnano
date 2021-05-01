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
use ultraviolet::Mat4;

/// An object that stores the instances to be drawn to represent the desgin.
pub struct View {
    /// The model matrix of the design
    pub model_matrix: Mat4,
    /// True if there are new instances to be fetched
    was_updated: bool,
}

impl View {
    pub fn new() -> Self {
        Self {
            model_matrix: Mat4::identity(),
            was_updated: false,
        }
    }

    /// Return true if the view was updated since the last time this function was called
    pub fn was_updated(&mut self) -> bool {
        let ret = self.was_updated;
        self.was_updated = false;
        ret
    }

    /// Update the model matrix
    pub fn set_matrix(&mut self, matrix: Mat4) {
        self.model_matrix = matrix;
        self.was_updated = true;
    }
}

impl View {
    /// Return the model matrix
    pub fn get_model_matrix(&self) -> Mat4 {
        self.model_matrix
    }
}
