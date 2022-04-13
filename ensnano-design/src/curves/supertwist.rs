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

use std::f64::consts::{PI, TAU};

use ultraviolet::{DVec2, DVec3};

const INTER_HELIX_GAP: f64 = 2.65;

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct SuperTwist {
    r: f64,
    delta: f64,
    omega: f64,
    nb_helices: usize,
    helix_idx: usize,
}

impl super::Curved for SuperTwist {
    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::BiInfinite
    }

    fn position(&self, t: f64) -> ultraviolet::DVec3 {
        let ct = (t * self.omega).cos();
        let st = (t * self.omega).sin();

        let m = DVec3 {
            x: self.r * ct,
            y: self.r * st,
            z: self.delta * t,
        };

        let dm_dt = DVec3 {
            x: -self.r * self.delta * st,
            y: self.r * self.delta * ct,
            z: self.delta,
        };

        let ds = DVec2::new(self.r * self.omega, self.delta).mag();

        let tengent = dm_dt / ds;

        let _ddm_ddt = DVec3 {
            x: -self.r * self.omega * self.omega * ct,
            y: -self.r * self.omega * self.omega * st,
            z: 0.0,
        };

        let normal = DVec3 {
            x: -ct,
            y: -st,
            z: 0.0,
        };

        let third_vec = tengent.cross(normal);

        let omega_ = TAU * ds / (self.nb_helices as f64 * INTER_HELIX_GAP);

        let angle_per_helix = PI / self.nb_helices as f64;
        let r = INTER_HELIX_GAP / 2. / (PI / self.nb_helices as f64).sin();
        let angle = omega_ * t + self.helix_idx as f64 * angle_per_helix;
        let ct = r * angle.cos();
        let st = r * angle.sin();

        m + ct * tengent + st * third_vec
    }
}
