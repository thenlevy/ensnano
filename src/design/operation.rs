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
//! This module defines methods for the `Design` type that are meant to break high level operation
//! on a design into atomic undoable operations

use super::*;

impl Design {
    /// Make a cross-over between source and target.
    ///
    /// If source and target are both the 5' end of a strand, or if they are both the 3' end of a
    /// strand, a cross-over between them is considered impossible and this methods returns an
    /// empty vector of operations.
    ///
    /// Otherwise, this methods return a vector of operations that will create a cross-over between
    /// source and target.
    /// When possible, source will be the 5' end of the cross-over and target will be the 3' end.
    /// This is not possible when source is the 5' end of a strand or when target is the 3' end of
    /// a strand. In these cases, source will be the 3' end of the cross-over and target will be
    /// the 5'end of the cross-over.
    pub fn general_cross_over(
        &self,
        source: Nucl,
        target: Nucl,
    ) -> Option<(StrandState, StrandState)> {
        self.data.lock().unwrap().general_cross_over(source, target)
    }
}
