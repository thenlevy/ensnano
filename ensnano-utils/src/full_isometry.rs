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
use ultraviolet::{Isometry2, Mat3, Rotor2, Vec2};

#[derive(Clone, Debug, Copy)]
/// A transformation made of a symmetry, a rotation and a translation, applied in that order.
pub struct FullIsometry {
    pub translation: Vec2,
    pub rotation: Rotor2,
    pub symmetry: Vec2,
}

impl FullIsometry {
    pub fn from_isommetry_symmetry(iso: Isometry2, symmetry: Vec2) -> Self {
        Self {
            translation: iso.translation,
            rotation: iso.rotation,
            symmetry,
        }
    }

    pub fn into_homogeneous_matrix(self) -> Mat3 {
        let mut sym_rot = self.rotation.into_matrix().into_homogeneous();
        sym_rot[0] *= self.symmetry.x;
        sym_rot[1] *= self.symmetry.y;
        Mat3::from_translation(self.translation) * sym_rot
    }

    pub fn matrix_with_transposed_symetry(self) -> Mat3 {
        let mut sym_rot = self.rotation.into_matrix().into_homogeneous();
        sym_rot[0] *= self.symmetry.y;
        sym_rot[1] *= self.symmetry.x;
        Mat3::from_translation(self.translation) * sym_rot
    }
}
