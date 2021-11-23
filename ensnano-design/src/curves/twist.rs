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
use crate::utils::{rotor_to_drotor, vec_to_dvec};
use ultraviolet::{DVec3, Rotor3, Vec3};

/// An helicoidal curve
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Twist {
    /// The angle at t=0
    pub theta0: f64,
    /// d theta / dt
    pub omega: f64,
    /// The center of the circle at t = 0,
    pub position: Vec3,
    /// The orientation of the curve. The normal vector is orientation * unit_x
    pub orientation: Rotor3,
    /// The length of the curve projected on the x axis
    pub length_x: f64,
    /// The radius of the circle arround which the helix turns
    pub radius: f64,
}

impl Curved for Twist {
    fn position(&self, t: f64) -> DVec3 {
        let theta = self.theta0 + self.omega * t;
        let pos_0 = DVec3 {
            x: self.length_x * t,
            y: self.radius * theta.sin(),
            z: self.radius * theta.cos(),
        };
        let position = vec_to_dvec(self.position);
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0 + position
    }

    fn speed(&self, t: f64) -> DVec3 {
        let theta = self.theta0 + self.omega * t;
        let pos_0 = DVec3 {
            x: self.length_x,
            y: self.radius * self.omega * theta.cos(),
            z: self.radius * self.omega * -theta.sin(),
        };
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let theta = self.theta0 + self.omega * t;
        let pos_0 = DVec3 {
            x: 0.,
            y: self.radius * self.omega * self.omega * -theta.sin(),
            z: self.radius * self.omega * self.omega * -theta.cos(),
        };
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0
    }
}
