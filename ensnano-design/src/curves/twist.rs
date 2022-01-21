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
use crate::{
    utils::{rotor_to_drotor, vec_to_dvec},
    Parameters,
};
use ultraviolet::{DVec3, Rotor3, Vec3};

pub fn nb_turn_per_100_nt_to_omega(
    nb_turn_per_100_nt: f64,
    radius: usize,
    parameters: &Parameters,
) -> Option<f64> {
    if nb_turn_per_100_nt.abs() < 1e-3 {
        return Some(0.0);
    }
    #[allow(non_snake_case)]
    let Z: f64 = 100.0 * parameters.z_step as f64;
    use std::f64::consts::PI;
    let angle = PI / radius as f64;
    let r = ((parameters.helix_radius + parameters.inter_helix_gap / 2.) as f64) / angle.sin();
    use std::f64::consts::TAU;
    if (Z / (TAU * nb_turn_per_100_nt)).powi(2) > r.powi(2) {
        let omega = ((Z / (TAU * nb_turn_per_100_nt)).powi(2) - r.powi(2)).powf(-0.5);
        println!(
            "nb_turn_per_100_nt = {}r = {}, omega = {}",
            nb_turn_per_100_nt, r, omega
        );
        Some(omega * nb_turn_per_100_nt.signum())
    } else {
        None
    }
}

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
    /// The radius of the circle arround which the helix turns
    pub radius: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub t_min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub t_max: Option<f64>,
}

impl Curved for Twist {
    fn t_max(&self) -> f64 {
        if let Some(tmax) = self.t_max {
            tmax.max(1.0)
        } else {
            1.0
        }
    }

    fn t_min(&self) -> f64 {
        if let Some(tmin) = self.t_min {
            tmin.min(0.0)
        } else {
            0.0
        }
    }

    fn position(&self, t: f64) -> DVec3 {
        let pos_0 = if self.omega.abs() < 1e-5 {
            DVec3 {
                x: t,
                y: self.radius * self.theta0.sin(),
                z: self.radius * self.theta0.cos(),
            }
        } else {
            let theta = self.theta0 + t * self.omega.signum();
            DVec3 {
                x: t / self.omega.abs(),
                y: self.radius * theta.sin(),
                z: self.radius * theta.cos(),
            }
        };
        let position = vec_to_dvec(self.position);
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0 + position
    }

    fn speed(&self, t: f64) -> DVec3 {
        let pos_0 = if self.omega.abs() < 1e-5 {
            DVec3::unit_x()
        } else {
            let theta = self.theta0 + t * self.omega.signum();
            DVec3 {
                x: 1.0 / self.omega.abs(),
                y: self.radius * theta.cos(),
                z: self.radius * -theta.sin(),
            }
        };
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let pos_0 = if self.omega.abs() < 1e-5 {
            DVec3::zero()
        } else {
            let theta = self.theta0 + t * self.omega.signum();
            DVec3 {
                x: 0.,
                y: self.radius * -theta.sin(),
                z: self.radius * -theta.cos(),
            }
        };
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::BiInfinite
    }

    fn curvilinear_abscissa(&self, t: f64) -> Option<f64> {
        if self.omega.abs() < 1e-5 {
            Some(t)
        } else {
            // https://mathcurve.com/courbes3d.gb/helicecirculaire/helicecirculaire.shtml
            let a = self.radius;
            let b = 1. / self.omega;
            Some((a * a + b * b).sqrt() * t)
        }
    }

    fn inverse_curvilinear_abscissa(&self, x: f64) -> Option<f64> {
        if self.omega.abs() < 1e-5 {
            Some(x)
        } else {
            // https://mathcurve.com/courbes3d.gb/helicecirculaire/helicecirculaire.shtml
            let a = self.radius;
            let b = 1. / self.omega;
            let s = (a * a + b * b).sqrt();
            Some(x / s)
        }
    }
}
