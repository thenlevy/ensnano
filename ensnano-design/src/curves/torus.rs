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
use std::sync::Arc;
use ultraviolet::{DVec2, DVec3};

use ordered_float::OrderedFloat;
use std::f64::consts::TAU;

const H: f64 = crate::Parameters::DEFAULT.helix_radius as f64
    + crate::Parameters::DEFAULT.inter_helix_gap as f64 / 2.;

const NB_STEPS: usize = 10_000_000;

/// A torus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torus {
    /// The angle shift a t = 0 along the slice
    pub theta0: f64,
    /// The number of helices on each slice
    pub half_nb_helix: usize,
    /// The radius of the torus
    pub big_radius: f64,
}

impl Torus {
    fn theta(&self, t: f64) -> f64 {
        TAU * (2. * self.half_nb_helix as f64) * t / 2. + self.theta0
    }

    fn t_for_curvilinear_abscissa(&self, s: f64) -> f64 {
        let p = 9.688448061179066_f64;
        let perimeter = 4. * H * self.half_nb_helix as f64;
        let scale = perimeter / p;
        let mut sp = s / scale;
        let a = 2.;
        let b = 1.;
        while sp < 0. {
            sp += p;
        }
        while sp > p {
            sp -= p;
        }
        let nb_steps = NB_STEPS;
        let mut u = DVec2 { x: a, y: 0. };
        let mut t = 0f64;
        for i in 0..nb_steps + 1 {
            // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
            if sp <= 0. {
                break;
            }
            t = TAU * i as f64 / nb_steps as f64;
            let v = DVec2 {
                x: a * t.cos(),
                y: b * t.sin(),
            };
            sp -= (v - u).mag();
            u = v;
        }
        t
    }

    fn position_moebius(&self, t: f64) -> DVec3 {
        let p = 9.688448061179066_f64;
        let perimeter = 4. * H * self.half_nb_helix as f64;
        // println!("p: {}\t P: {}\tφ:{}\tφφ:{}", perimeter, self.perimeter_ellipse(2.,1., NB_STEPS), self.t_for_curvilinear_abscissa_poly(perimeter/2.), self.t_for_curvilinear_abscissa(perimeter/2.));
        let scale = perimeter / p;
        let a = 2. * scale;
        let b = 1. * scale;
        let theta = self.theta(t) - self.theta0;
        let s_dtheta = (perimeter / 2. - 4. * H) / TAU;
        let s = 4. * H * self.theta0 / TAU + s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (x, y) = (a * phi.cos(), b * phi.sin());
        DVec3 {
            x: (x * t2c - y * t2s + self.big_radius) * theta.cos(),
            y: x * t2s + y * t2c,
            z: (x * t2c - y * t2s + self.big_radius) * theta.sin(),
        }
    }

    fn speed_moebius(&self, t: f64) -> DVec3 {
        let dt = 1. / NB_STEPS as f64;
        let x = self.position_moebius(t);
        let x_dx = self.position_moebius(t + dt);
        return (x_dx - x) / dt;
    }

    fn acceleration_moebius(&self, _: f64) -> DVec3 {
        DVec3 {
            x: 0.,
            y: 0.,
            z: 1.,
        }
    }
}

impl Curved for Torus {
    fn position(&self, t: f64) -> DVec3 {
        return self.position_moebius(t);
    }

    fn speed(&self, t: f64) -> DVec3 {
        return self.speed_moebius(t);
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        return self.acceleration_moebius(t);
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::Finite
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CurveDescriptor2D {
    Ellipse {
        semi_minor_axis: OrderedFloat<f64>,
        semi_major_axis: OrderedFloat<f64>,
    },
}

struct InstanciatedEllipse {
    semi_major_axis: f64,
    semi_minor_axis: f64,
    cached_curvlinear_abscissa: Vec<f64>,
}

impl InstanciatedEllipse {
    fn new(semi_minor_axis: f64, semi_major_axis: f64) -> Self {
        let mut ret = Self {
            semi_minor_axis: semi_minor_axis as f64,
            semi_major_axis: semi_major_axis as f64,
            cached_curvlinear_abscissa: Vec::new(),
        };
        ret.initialise_cache();
        ret
    }
}

impl Curve2D for InstanciatedEllipse {
    fn position(&self, t: f64) -> DVec2 {
        let t = TAU * t;
        DVec2 {
            x: self.semi_major_axis * t.cos(),
            y: self.semi_minor_axis * t.sin(),
        }
    }

    fn symetry_order(&self) -> usize {
        2
    }

    fn get_cached_curvlinear_abscissa(&self) -> Option<&[f64]> {
        Some(&self.cached_curvlinear_abscissa)
    }

    fn get_cached_curvlinear_abscissa_mut(&mut self) -> Option<&mut Vec<f64>> {
        Some(&mut self.cached_curvlinear_abscissa)
    }
}

impl CurveDescriptor2D {
    fn instanciate(self) -> Arc<dyn Curve2D + Sync + Send> {
        match self {
            Self::Ellipse {
                semi_minor_axis,
                semi_major_axis,
            } => Arc::new(InstanciatedEllipse::new(*semi_minor_axis, *semi_major_axis)),
        }
    }
}

trait Curve2D {
    fn position(&self, t: f64) -> DVec2;

    fn symetry_order(&self) -> usize;

    fn get_cached_curvlinear_abscissa_mut(&mut self) -> Option<&mut Vec<f64>>;

    fn get_cached_curvlinear_abscissa(&self) -> Option<&[f64]>;

    fn t_for_curvilinear_abscissa(&self, s_objective: f64) -> f64 {
        self.get_cached_curvlinear_abscissa()
            .map(|cache| {
                let idx = search_dicho(s_objective, cache).expect("search dicho");
                let s = cache[idx];
                let mut t = idx as f64 / (cache.len() - 1) as f64;
                if idx < cache.len() - 1 {
                    let s_ = cache[idx + 1];
                    let interpolation = (s_objective - s) / (s_ - s);
                    t += interpolation / (cache.len() - 1) as f64;
                }
                t
            })
            .unwrap_or_else(|| {
                let mut sp = s_objective;
                let mut u = self.position(0.);
                let mut t = 0.;
                for i in 0..=NB_STEPS {
                    // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
                    if sp <= 0. {
                        return t;
                    }
                    t = TAU * i as f64 / NB_STEPS as f64;
                    let v = self.position(t);
                    sp -= (v - u).mag();
                    u = v;
                }
                t
            })
    }

    fn curvilinear_abscissa(&self, t: f64) -> f64 {
        if let Some(cache) = self.get_cached_curvlinear_abscissa() {
            let idx = (t * (cache.len() - 1) as f64) as usize;
            let s = cache[idx];
            let p = self.position(idx as f64 / (cache.len() - 1) as f64);
            let p_ = self.position(t);
            s + (p - p_).mag()
        } else {
            let mut s = 0.;
            let mut p = self.position(0.);
            let t_obj = t;
            for i in 0..NB_STEPS {
                let t = t_obj * (i as f64 / (NB_STEPS - 1) as f64);
                let p_ = self.position(t);
                s += (p - p_).mag();
                p = p_;
            }
            s
        }
    }

    fn initialise_cache(&mut self) {
        let len = if let Some(_) = self.get_cached_curvlinear_abscissa_mut() {
            NB_STEPS
        } else {
            0
        };

        if len > 1 {
            let mut cache = Vec::with_capacity(len);
            let mut s = 0.;
            let mut p = self.position(0.);
            cache.push(s);
            for i in 1..len {
                let t = i as f64 / (len - 1) as f64;
                let p_ = self.position(t);
                s += (p - p_).mag();
                p = p_;
                cache.push(s);
            }
            if let Some(saved_cache) = self.get_cached_curvlinear_abscissa_mut() {
                *saved_cache = cache;
            }
        }
    }

    fn perimeter(&self) -> f64 {
        self.curvilinear_abscissa(1.)
    }
}

fn search_dicho(goal: f64, slice: &[f64]) -> Option<usize> {
    if slice.len() > 0 {
        let mut a = 0usize;
        let mut b = slice.len() - 1;
        while b - a > 2 {
            let c = (b + a) / 2;
            if slice[c] < goal {
                a = c;
            } else {
                b = c;
            }
        }
        if slice[a] < goal {
            Some(a)
        } else {
            Some(b)
        }
    } else {
        None
    }
}

pub(super) struct TwistedTorus {
    instanciated_curve: Arc<dyn Curve2D + Sync + Send>,
    descriptor: TwistedTorusDescriptor,
    scale: f64,
    perimeter: f64,
    nb_turn_per_helix: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TwistedTorusDescriptor {
    pub curve: CurveDescriptor2D,
    /// Number of half rotation of the Ellipse per turn
    pub half_twist_count_per_turn: isize,
    /// Radius of the structure,
    pub big_radius: OrderedFloat<f64>,
    pub number_of_helix_per_section: usize,
    pub helix_index_shift_per_turn: isize,
    // Common to all helices of the shape
    pub initial_curvilinear_abscissa: OrderedFloat<f64>,
    pub initial_index_shift: isize,
}

impl TwistedTorus {
    pub fn new(descriptor: TwistedTorusDescriptor) -> Self {
        let instanciated_curve = descriptor.curve.clone().instanciate();
        let scale =
            2. * H * descriptor.number_of_helix_per_section as f64 / instanciated_curve.perimeter();
        let k = descriptor.helix_index_shift_per_turn;
        let n = descriptor.number_of_helix_per_section;
        let q = descriptor.half_twist_count_per_turn;
        let rho = instanciated_curve.symetry_order();
        let nb_turn_per_helix = n as usize / gcd(n as isize, k + (n as isize * q) / rho as isize);
        Self {
            descriptor,
            scale,
            perimeter: instanciated_curve.perimeter(),
            instanciated_curve,
            nb_turn_per_helix,
        }
    }
}

fn gcd(a: isize, b: isize) -> usize {
    let mut a = a.abs() as usize;
    let mut b = b.abs() as usize;

    if a < b {
        std::mem::swap(&mut a, &mut b);
    }

    while b > 0 {
        let b_ = b;
        b = a % b;
        a = b_;
    }
    return a;
}

impl TwistedTorus {
    fn theta(&self, t: f64) -> f64 {
        self.nb_turn_per_helix as f64 * t * TAU
    }

    fn objective_s(&self, theta: f64) -> f64 {
        *self.descriptor.initial_curvilinear_abscissa
            + 2. * H
                * (self.descriptor.helix_index_shift_per_turn as f64 * theta / TAU
                    + self.descriptor.initial_index_shift as f64)
    }
}

impl Curved for TwistedTorus {
    fn position(&self, t: f64) -> DVec3 {
        let theta = self.theta(t);
        let s_theta = self.objective_s(theta) / self.scale;

        let t_curve = self
            .instanciated_curve
            .t_for_curvilinear_abscissa(s_theta.rem_euclid(self.perimeter));
        let point_curve = self.instanciated_curve.position(t_curve) * self.scale;
        let phi = self.descriptor.half_twist_count_per_turn as f64 * theta
            / (self.instanciated_curve.symetry_order() as f64);

        DVec3 {
            x: (point_curve.x * phi.cos() - point_curve.y * phi.sin()
                + *self.descriptor.big_radius)
                * theta.cos(),
            y: point_curve.x * phi.sin() + point_curve.y * phi.cos(),
            z: (point_curve.x * phi.cos() - point_curve.y * phi.sin()
                + *self.descriptor.big_radius)
                * theta.sin(),
        }
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::Finite
    }
}
