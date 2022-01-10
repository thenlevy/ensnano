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
//! This modules defines operations that can be performed on a design to modify it.
//! The functions that apply thes operations take a mutable reference to the design that they are
//! modifying and may return an `ErrOperation` if the opperation could not be applied.

use super::Design;

/// An error that occured when trying to apply an operation.
pub enum ErrOperation {
    NotEnoughHelices { actual: usize, needed: usize },
}

/// The minimum number of helices requiered to infer a grid
pub const MIN_HELICES_TO_MAKE_GRID: usize = 4;

/// Try to create a grid from a set of helices.
pub fn make_grid_from_helices(design: &mut Design, helices: &[usize]) -> Result<(), ErrOperation> {
    super::grid::make_grid_from_helices(design, helices)?;
    Ok(())
}
