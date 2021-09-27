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

use ultraviolet::{Rotor3, Vec3};

/// The attributes of a group.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupAttribute {
    pub pivot: Option<GroupPivot>,
}

/// The position and orientation of the pivot used to rotate/translate the group
#[derive(Copy, Debug, Clone, Serialize, Deserialize)]
pub struct GroupPivot {
    pub position: Vec3,
    pub orientation: Rotor3,
}
