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

use ultraviolet::{Bivec3, DBivec3, DRotor3, DVec3, Rotor3, Vec3};

pub fn vec_to_dvec(v: Vec3) -> DVec3 {
    DVec3 {
        x: v.x as f64,
        y: v.y as f64,
        z: v.z as f64,
    }
}

pub fn bivec_to_dbivec(bv: Bivec3) -> DBivec3 {
    DBivec3 {
        xy: bv.xy as f64,
        xz: bv.xz as f64,
        yz: bv.yz as f64,
    }
}

pub fn rotor_to_drotor(rot: Rotor3) -> DRotor3 {
    DRotor3 {
        s: rot.s as f64,
        bv: bivec_to_dbivec(rot.bv),
    }
}

pub fn dvec_to_vec(dv: DVec3) -> Vec3 {
    Vec3 {
        x: dv.x as f32,
        y: dv.y as f32,
        z: dv.z as f32,
    }
}

// Serialization utils
//===========================================================================
pub(super) fn isize_is_zero(x: &isize) -> bool {
    *x == 0
}

pub(super) fn f32_is_zero(x: &f32) -> bool {
    *x == 0.0
}

pub(super) fn default_visibility() -> bool {
    true
}

pub(super) fn is_false(x: &bool) -> bool {
    !*x
}
//===========================================================================
