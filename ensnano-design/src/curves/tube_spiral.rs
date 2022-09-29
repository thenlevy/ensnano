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
use ultraviolet::{DRotor3, DVec3};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TubeSpiralDescritor {
    pub theta_0: f64,
    pub big_axis: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default = "default_number_of_helices")]
    pub number_of_helices: usize,
    pub small_axis: f64,
    pub t_min: f64,
    pub t_max: f64,
}

fn default_number_of_helices() -> usize {
    2
}

impl TubeSpiralDescritor {
    pub(super) fn with_parameters(self, parameters: Parameters) -> TubeSpiral {
        TubeSpiral {
            theta_0: self.theta_0,
            big_axis: self.big_axis,
            _parameters: parameters,
            height: self.height,
            number_of_helices: self.number_of_helices,
            small_axis: self.small_axis,
            perimeter: self.perimeter(),
            t_min: self.t_min,
            t_max: self.t_max,
        }
    }

    pub fn perimeter(&self) -> f64 {
        let lambda = (self.big_axis - self.small_axis) / (self.big_axis + self.small_axis);

        PI * (self.big_axis + self.small_axis)
            * (1. + (3. * lambda.powi(2)) / (10. + (4. - 3. * lambda.powi(2)).sqrt()))
    }
}

pub(super) struct TubeSpiral {
    pub theta_0: f64,
    pub big_axis: f64,
    pub _parameters: Parameters,
    pub height: f64,
    pub number_of_helices: usize,
    pub small_axis: f64,
    pub perimeter: f64,
    pub t_min: f64,
    pub t_max: f64,
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
        if self.number_of_helices == 1 {
            0.
        } else {
            let nb_helices = self.number_of_helices as f64;
            // FIXME: this is wrong when nb_helices > 2 and small_axis < big axis
            // the correct result is the perimeter of the polygon inscribed in the helix
            let slice_width = self.perimeter / 2. / PI * (PI / nb_helices).sin();

            (Parameters::INTER_CENTER_GAP as f64 / 2. / slice_width).asin()
        }
    }

    fn theta(&self, t: f64) -> f64 {
        self.nb_turn() * t * TAU + self.theta_0
    }

    pub(super) fn last_theta(&self) -> f64 {
        self.theta(1.)
    }
}

impl Curved for TubeSpiral {
    fn position(&self, t: f64) -> DVec3 {
        let nb_turn = self.nb_turn();
        let theta = nb_turn * t * TAU;
        DVec3 {
            x: self.big_axis * theta.cos(),
            y: self.small_axis * theta.sin(),
            z: self.height * t,
        }
        .rotated_by(DRotor3::from_rotation_xy(self.theta_0))
    }

    fn speed(&self, t: f64) -> DVec3 {
        let nb_turn = self.nb_turn();
        let theta = nb_turn * t * TAU;

        let x = -self.big_axis * nb_turn * TAU * theta.sin();

        let y = self.small_axis * nb_turn * TAU * theta.cos();

        let z = self.height;

        DVec3 { x, y, z }.rotated_by(DRotor3::from_rotation_xy(self.theta_0))
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let nb_turn = self.nb_turn();
        let theta = nb_turn * t * TAU;

        let x = -self.big_axis * TAU * TAU * theta.cos();

        let y = -self.small_axis * TAU * TAU * theta.sin();

        let z = 0.;

        DVec3 { x, y, z }.rotated_by(DRotor3::from_rotation_xy(self.theta_0))
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::Finite
    }

    fn subdivision_for_t(&self, t: f64) -> Option<usize> {
        Some((((self.nb_turn() * t * TAU + self.theta_0 + 1e-3) / TAU) + self.nb_turn()) as usize)
    }

    fn is_time_maps_singleton(&self) -> bool {
        true
    }

    fn first_theta(&self) -> Option<f64> {
        Some(self.theta_0)
    }

    fn last_theta(&self) -> Option<f64> {
        Some(self.last_theta())
    }

    fn full_turn_at_t(&self) -> Option<f64> {
        Some(self.t_max())
    }

    fn t_max(&self) -> f64 {
        self.t_max
    }

    fn t_min(&self) -> f64 {
        self.t_min
    }
}
