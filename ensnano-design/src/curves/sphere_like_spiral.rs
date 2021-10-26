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

pub(super) struct SphereLikeSpiral {
    theta_0: f32,
    radius: f32,
}

impl Curved for SphereLikeSpiral {
    fn position(&self, t: f32) -> Vec3 {
        let phi = t * PI;
        let theta = TAU * phi + self.theta_0;
        Vec3 {
            x: self.radius * phi.sin() * theta.cos(),
            y: self.radius * phi.sin() * theta.sin(),
            z: self.radius * phi.cos(),
        }
    }

    fn speed(&self, t: f32) -> Vec3 {
        let phi = t * PI;
        let theta = TAU * phi + self.theta_0;

        let x = self.radius * PI * (phi.cos() * theta.cos() - TAU * phi.sin() * theta.sin());

        let y = self.radius * PI * (phi.cos() * theta.sin() + TAU * phi.sin() * theta.cos());

        let z = -self.radius * PI * phi.sin();

        Vec3 { x, y, z }
    }

    fn acceleration(&self, t: f32) -> Vec3 {
        let phi = t * PI;
        let theta = TAU * phi + self.theta_0;

        let x = self.radius
            * PI
            * PI
            * (-1. * phi.sin() * theta.cos()
                - phi.cos() * TAU * theta.sin()
                - TAU * (phi.cos() * theta.sin() + TAU * phi.sin() * theta.cos()));

        let y = self.radius
            * PI
            * PI
            * (-1. * phi.sin() * theta.sin()
                + phi.cos() * TAU * theta.cos()
                + TAU * (phi.cos() * theta.cos() - TAU * phi.sin() * theta.sin()));

        let z = --self.radius * PI * PI * phi.cos();

        Vec3 { x, y, z }
    }
}
