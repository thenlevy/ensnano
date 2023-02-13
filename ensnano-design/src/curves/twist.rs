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

#[allow(non_snake_case)]
pub fn nb_turn_per_100_nt_to_omega(
    nb_turn_per_100_nt: f64,
    parameters: &Parameters,
) -> Option<f64> {
    if nb_turn_per_100_nt.abs() < 1e-3 {
        return Some(0.0);
    }
    let Z: f64 = 100.0 * parameters.z_step as f64;
    use std::f64::consts::TAU;
    Some(TAU * nb_turn_per_100_nt / Z)
}

pub fn twist_to_omega(twist: f64, parameters: &Parameters) -> Option<f64> {
    nb_turn_per_100_nt_to_omega(twist, parameters)
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
        let pos_0 = {
            let theta = self.theta0 + t * self.omega;
            DVec3 {
                x: t,
                y: self.radius * theta.sin(),
                z: self.radius * theta.cos(),
            }
        };
        let position = vec_to_dvec(self.position);
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0 + position
    }

    fn speed(&self, t: f64) -> DVec3 {
        let pos_0 = {
            let theta = self.theta0 + t * self.omega;
            DVec3 {
                x: 1.0,
                y: self.radius * self.omega * theta.cos(),
                z: -self.radius * self.omega * theta.sin(),
            }
        };
        let orientation = rotor_to_drotor(self.orientation);
        orientation * pos_0
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let pos_0 = {
            let theta = self.theta0 + t * self.omega;
            DVec3 {
                x: 0.0,
                y: -self.radius * self.omega * self.omega * theta.sin(),
                z: -self.radius * self.omega * self.omega * theta.cos(),
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
            Some((a * a + b * b).sqrt() * t * self.omega.abs())
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
            Some(x / self.omega.abs() / s)
        }
    }

    fn z_step_ratio(&self) -> Option<f64> {
        /*
        if self.omega.abs() < 1e-5 {
            None
        } else {
            self.curvilinear_abscissa(1.0)
        }
        */
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Helix;

    impl Twist {
        fn with_omega(omega: f64) -> Self {
            Self {
                theta0: 0.0,
                omega,
                position: Vec3::zero(),
                orientation: Rotor3::identity(),
                radius: 20.0,
                t_min: None,
                t_max: None,
            }
        }
    }

    #[test]
    fn correct_curvilinear_abscissa() {
        let p = Parameters::DEFAULT;
        let nb_turn = 0.1234;
        let omega = nb_turn_per_100_nt_to_omega(nb_turn, &p).unwrap();
        let twist = Twist::with_omega(omega);
        let mut s = 0.0;
        let mut t = 0.;
        let mut p = twist.position(0.0);
        while t < 1. {
            t += 0.0001;
            let q = twist.position(t);
            s += (p - q).mag();
            p = q;
        }
        let expected = twist.curvilinear_abscissa(1.0).unwrap();
        println!("s = {}", s);
        println!("expected = {}", expected);
        assert!((s - expected).abs() < 1e-3);
    }

    #[allow(non_snake_case)]
    #[test]
    fn nb_turn_per_100_nt_is_correct() {
        let p = Parameters::DEFAULT;
        let nb_turn = 0.1234;
        let omega = nb_turn_per_100_nt_to_omega(nb_turn, &p).unwrap();
        let Z = 100. * p.z_step as f64;
        assert!(((omega * Z) - (std::f64::consts::TAU * nb_turn)).abs() < 1e-5)
    }

    #[ignore = "need fix"]
    #[allow(non_snake_case)]
    #[test]
    fn z_step_ratio_is_correct() {
        let p = Parameters::DEFAULT;
        let Z = 100.0 * p.z_step as f64;
        let nb_turn = 0.1234;
        let omega = nb_turn_per_100_nt_to_omega(nb_turn, &p).unwrap();
        let mut twist = Twist::with_omega(omega);
        twist.t_max = Some(Z);
        let descriptor = super::super::InstanciatedCurveDescriptor_::Twist(twist);
        let curve = descriptor.try_into_curve(&p).unwrap();
        let flat_helix = Helix::new(Vec3::zero(), Rotor3::identity());
        let theta = flat_helix.theta(99, true, &p);
        let nucl_curved = curve.nucl_pos(99, true, theta as f64, &p).unwrap();
        let nucl_flat = crate::utils::vec_to_dvec(flat_helix.space_pos(&p, 99, true));

        println!("curved {:?} \n flat {:?}", nucl_curved, nucl_flat);
        // The two nucleotides are not in the same position
        assert!((nucl_curved - nucl_flat).mag() > 0.5);
        // But have almost the same x coordinate
        assert!((nucl_curved.x - nucl_flat.x).abs() < 1e-2);
    }

    #[allow(non_snake_case)]
    fn roll_adjustment_is_correct(nb_turn: f64) {
        let p = Parameters::DEFAULT;
        let Z = 100.0 * p.z_step as f64;
        let omega = nb_turn_per_100_nt_to_omega(nb_turn, &p).unwrap();
        let mut twist = Twist::with_omega(omega);
        twist.t_max = Some(Z);
        let descriptor = super::super::InstanciatedCurveDescriptor_::Twist(twist.clone());
        let curve = descriptor.try_into_curve(&p).unwrap();
        println!("abscissa {:?}", twist.curvilinear_abscissa(Z));
        println!("z ratio {:?}", twist.z_step_ratio());
        assert!(twist.theta_shift(&p).is_some());
        let flat_helix = Helix::new(Vec3::zero(), Rotor3::identity());
        let theta_99 = flat_helix.theta(99, true, &p);
        let theta_98 = flat_helix.theta(98, true, &p);
        let nucl_98 = curve.nucl_pos(98, true, theta_98 as f64, &p).unwrap();
        let nucl_99 = curve.nucl_pos(99, true, theta_99 as f64, &p).unwrap();

        let dist = (nucl_99 - nucl_98).mag() as f32;
        println!("dist {} \n  vs \n dist_ac {}", dist, p.dist_ac());
        assert!((dist - p.dist_ac()).abs() < 1e-2);
    }

    #[ignore = "need fix"]
    #[test]
    fn roll_adjustment_is_correct_right() {
        roll_adjustment_is_correct(0.4);
    }

    #[ignore = "need fix"]
    #[test]
    fn roll_adjustment_is_correct_left() {
        roll_adjustment_is_correct(-0.4);
    }
}
