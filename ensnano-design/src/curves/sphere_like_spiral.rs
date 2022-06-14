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

use crate::Parameters;

use super::Curved;
use std::f64::consts::{PI, TAU};
use ultraviolet::DVec3;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SphereLikeSpiralDescriptor {
    pub theta_0: f64,
    pub radius: f64,
}

impl SphereLikeSpiralDescriptor {
    pub(super) fn with_parameters(self, parameters: Parameters) -> SphereLikeSpiral {
        SphereLikeSpiral {
            theta_0: self.theta_0,
            radius: self.radius,
            parameters,
        }
    }
}

pub(super) struct SphereLikeSpiral {
    pub theta_0: f64,
    pub radius: f64,
    pub parameters: Parameters,
}

impl SphereLikeSpiral {
    fn dist_turn(&self) -> f64 {
        4. * self.parameters.helix_radius as f64 + 2. * self.parameters.inter_helix_gap as f64
    }

    fn nb_turn(&self) -> f64 {
        self.radius / self.dist_turn()
    }
}

impl Curved for SphereLikeSpiral {
    fn position(&self, t: f64) -> DVec3 {
        let phi = t * PI;

        let nb_turn = self.nb_turn();
        let theta = nb_turn * TAU * phi + self.theta_0;
        DVec3 {
            x: self.radius * phi.sin() * theta.cos(),
            y: self.radius * phi.sin() * theta.sin(),
            z: self.radius * phi.cos(),
        }
    }

    fn speed(&self, t: f64) -> DVec3 {
        let phi = t * PI;
        let nb_turn = self.nb_turn();
        let theta = nb_turn * TAU * phi + self.theta_0;

        let x =
            self.radius * PI * (phi.cos() * theta.cos() - nb_turn * TAU * phi.sin() * theta.sin());

        let y =
            self.radius * PI * (phi.cos() * theta.sin() + nb_turn * TAU * phi.sin() * theta.cos());

        let z = -self.radius * PI * phi.sin();

        DVec3 { x, y, z }
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let phi = t * PI;
        let nb_turn = self.nb_turn();
        let theta = nb_turn * TAU * phi + self.theta_0;

        let x = self.radius
            * PI
            * PI
            * (-1. * phi.sin() * theta.cos()
                - phi.cos() * nb_turn * nb_turn * TAU * theta.sin()
                - nb_turn
                    * TAU
                    * (phi.cos() * theta.sin() + nb_turn * TAU * phi.sin() * theta.cos()));

        let y = self.radius
            * PI
            * PI
            * (-1. * phi.sin() * theta.sin()
                + phi.cos() * nb_turn * TAU * theta.cos()
                + nb_turn
                    * TAU
                    * (phi.cos() * theta.cos() - nb_turn * TAU * phi.sin() * theta.sin()));

        let z = -self.radius * PI * PI * phi.cos();

        DVec3 { x, y, z }
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::Finite
    }

    fn subdivision_for_t(&self, t: f64) -> Option<usize> {
        Some((self.nb_turn() * t) as usize)
    }
}
