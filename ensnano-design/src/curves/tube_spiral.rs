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
pub struct TubeSpiralDescritor {
    pub theta_0: f64,
    pub radius: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default = "default_number_of_helices")]
    pub number_of_helices: usize,
}

fn default_number_of_helices() -> usize {
    2
}

impl TubeSpiralDescritor {
    pub(super) fn with_parameters(self, parameters: Parameters) -> TubeSpiral {
        TubeSpiral {
            theta_0: self.theta_0,
            radius: self.radius,
            parameters,
            height: self.height,
            number_of_helices: self.number_of_helices,
        }
    }
}

pub(super) struct TubeSpiral {
    pub theta_0: f64,
    pub radius: f64,
    pub parameters: Parameters,
    pub height: f64,
    pub number_of_helices: usize,
}

impl TubeSpiral {
    fn dist_turn(&self) -> f64 {
        let nb_helices = self.number_of_helices as f64;
        nb_helices * Parameters::INTER_CENTER_GAP as f64 / self.inclination().cos()
    }

    fn nb_turn(&self) -> f64 {
        self.height / self.dist_turn()
    }

    fn inclination(&self) -> f64 {
        let nb_helices = self.number_of_helices as f64;
        let slice_width = self.radius * (PI / nb_helices).sin();

        (Parameters::INTER_CENTER_GAP as f64 / slice_width).asin()
    }
}

impl Curved for TubeSpiral {
    fn position(&self, t: f64) -> DVec3 {
        let nb_turn = self.nb_turn();
        let theta = nb_turn * t * TAU + self.theta_0;
        DVec3 {
            x: self.radius * theta.cos(),
            y: self.radius * theta.sin(),
            z: self.height * t,
        }
    }

    fn speed(&self, t: f64) -> DVec3 {
        let nb_turn = self.nb_turn();
        let theta = nb_turn * t * TAU + self.theta_0;

        let x = -self.radius * nb_turn * TAU * theta.sin();

        let y = self.radius * nb_turn * TAU * theta.cos();

        let z = self.height;

        DVec3 { x, y, z }
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let nb_turn = self.nb_turn();
        let theta = nb_turn * t * TAU + self.theta_0;

        let x = -self.radius * TAU * TAU * theta.cos();

        let y = -self.radius * TAU * TAU * theta.sin();

        let z = 0.;

        DVec3 { x, y, z }
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::Finite
    }

    fn subdivision_for_t(&self, t: f64) -> Option<usize> {
        Some((self.nb_turn() * t * PI + self.theta_0 / TAU) as usize)
    }

    fn is_time_maps_singleton(&self) -> bool {
        true
    }
}
