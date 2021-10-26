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

use super::Curved;
use std::f32::consts::{PI, TAU};
use ultraviolet::Vec3;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SphereLikeSpiral {
    pub theta_0: f32,
    pub radius: f32,
}

const DIST_TURN: f32 = 2. * 2.65;



impl Curved for SphereLikeSpiral {
    fn position(&self, t: f32) -> Vec3 {
        let phi = t * PI;

        let nb_turn = self.radius / DIST_TURN;
        let theta = nb_turn * TAU * phi + self.theta_0;
        Vec3 {
            x: self.radius * phi.sin() * theta.cos(),
            y: self.radius * phi.sin() * theta.sin(),
            z: self.radius * phi.cos(),
        }
    }

    fn speed(&self, t: f32) -> Vec3 {
        let phi = t * PI;
        let nb_turn = self.radius / DIST_TURN;
        let theta = nb_turn * TAU * phi + self.theta_0;

        let x = self.radius * PI * (phi.cos() * theta.cos() - nb_turn * TAU * phi.sin() * theta.sin());

        let y = self.radius * PI * (phi.cos() * theta.sin() + nb_turn * TAU * phi.sin() * theta.cos());

        let z = -self.radius * PI * phi.sin();

        Vec3 { x, y, z }
    }

    fn acceleration(&self, t: f32) -> Vec3 {
        let phi = t * PI;
        let nb_turn = self.radius / DIST_TURN;
        let theta = nb_turn * TAU * phi + self.theta_0;

        let x = self.radius
            * PI
            * PI
            * (-1. * phi.sin() * theta.cos()
                - phi.cos() * nb_turn * nb_turn * TAU * theta.sin()
                - nb_turn * TAU * (phi.cos() * theta.sin() + nb_turn * TAU * phi.sin() * theta.cos()));

        let y = self.radius
            * PI
            * PI
            * (-1. * phi.sin() * theta.sin()
                + phi.cos() * nb_turn * TAU * theta.cos()
                + nb_turn * TAU * (phi.cos() * theta.cos() - nb_turn * TAU * phi.sin() * theta.sin()));

        let z = -self.radius * PI * PI * phi.cos();

        Vec3 { x, y, z }
    }
}
