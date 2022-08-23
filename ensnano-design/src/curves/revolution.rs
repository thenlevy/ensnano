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

use super::*;
use std::f64::consts::{PI, TAU};
use ultraviolet::DVec2;

use chebyshev_polynomials::ChebyshevPolynomial;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterpolatedCurveDescriptor {
    pub curve: CurveDescriptor2D,
    pub half_turns_count: isize,
    /// Radius of the revolution trajectory
    pub revolution_radius: f64,
    /// Scale factor of the section
    pub curve_scale_factor: f64,
    pub interpolation: Vec<InterpolationDescriptor>,
    pub chevyshev_smoothening: f64,
}

impl InterpolatedCurveDescriptor {
    pub(super) fn instanciate(self) -> Revolution {
        let curve = self.curve.clone();
        let mut discontinuities = vec![0.];
        for i in 0..self.interpolation.len() {
            discontinuities.push(i as f64 + 1.);
        }
        let curve = SmoothInterpolatedCurve::from_curve_interpolation(
            curve,
            self.interpolation,
            self.chevyshev_smoothening,
            self.half_turns_count,
        );
        let mut ret = Revolution {
            curve,
            revolution_radius: self.revolution_radius,
            curve_scale_factor: self.curve_scale_factor,
            half_turns_count: self.half_turns_count,
            inverse_curvilinear_abscissa: vec![],
            curvilinear_abscissa: vec![],
        };
        ret.init_interpolators();
        ret
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InterpolationDescriptor {
    PointsValues {
        points: Vec<f64>,
        values: Vec<f64>,
    },
    Chebyshev {
        coeffs: Vec<f64>,
        interval: [f64; 2],
    },
}

struct SmoothInterpolatedCurve {
    interpolators: Vec<ChebyshevPolynomial>,
    curve: CurveDescriptor2D,
    smoothening_coeff: f64,
    half_turn: bool,
}

impl SmoothInterpolatedCurve {
    fn from_curve_interpolation(
        curve: CurveDescriptor2D,
        interpolations: Vec<InterpolationDescriptor>,
        smoothening_coeff: f64,
        nb_half_turn: isize,
    ) -> Self {
        let mut interpolators = Vec::with_capacity(interpolations.len());
        for interpolation in interpolations.into_iter() {
            let interpolator = match interpolation {
                InterpolationDescriptor::PointsValues { points, values } => {
                    let points_values = points.into_iter().zip(values.into_iter()).collect();
                    chebyshev_polynomials::interpolate_points(points_values, 1e-4)
                }
                InterpolationDescriptor::Chebyshev { coeffs, interval } => {
                    chebyshev_polynomials::ChebyshevPolynomial::from_coeffs_interval(
                        coeffs, interval,
                    )
                }
            };
            interpolators.push(interpolator);
        }
        Self {
            curve,
            interpolators,
            smoothening_coeff,
            half_turn: nb_half_turn.rem_euclid(2) != 0,
        }
    }

    fn smooth_chebyshev(&self, s: f64) -> f64 {
        let u = s.rem_euclid(1.);
        let helix_idx =
            (s.div_euclid(1.) as isize).rem_euclid(self.interpolators.len() as isize) as usize;
        let prev_idx =
            (helix_idx as isize - 1).rem_euclid(self.interpolators.len() as isize) as usize;
        let next_idx = (helix_idx + 1).rem_euclid(self.interpolators.len());

        let a = self.smoothening_coeff;

        let shift = if self.half_turn { 0.5 } else { 0. };

        if u < a {
            // second half of the interpolation region, v = 0.5 + 1/2 ( u / a)
            let v = (1. + u / a) / 2.;
            let mut v1 =
                (self.interpolators[prev_idx].evaluate(1. - a + v * a) + shift).rem_euclid(1.);
            let v2 = (self.interpolators[helix_idx].evaluate(v * a)).rem_euclid(1.);

            while v1 > v2 + 0.5 {
                v1 -= 1.
            }
            while v1 < v2 - 0.5 {
                v1 += 1.
            }
            (1. - v) * v1 + v * v2
        } else if u > 1. - a {
            // first half of the interpolation region
            let v = (u - (1. - a)) / a / 2.;
            let v1 = (self.interpolators[helix_idx].evaluate(1. - a + v * a)).rem_euclid(1.);
            let mut v2 = (self.interpolators[next_idx].evaluate(v * a) - shift).rem_euclid(1.);

            while v2 > v1 + 0.5 {
                v2 -= 1.
            }
            while v2 < v1 - 0.5 {
                v2 += 1.
            }

            (1. - v) * v1 + v * v2
        } else {
            self.interpolators[helix_idx].evaluate(u)
        }
    }

    fn point(&self, t: f64) -> DVec2 {
        let s = self.smooth_chebyshev(t);
        self.curve.point(s.rem_euclid(1.))
    }

    fn t_max(&self) -> f64 {
        self.interpolators.len() as f64
    }
}

pub(super) struct Revolution {
    curve: SmoothInterpolatedCurve,
    revolution_radius: f64,
    curve_scale_factor: f64,
    half_turns_count: isize,
    inverse_curvilinear_abscissa: Vec<ChebyshevPolynomial>,
    curvilinear_abscissa: Vec<ChebyshevPolynomial>,
}

const NB_POINT_INTERPOLATION: usize = 100_000;
const INTERPOLATION_ERROR: f64 = 1e-4;
impl Revolution {
    fn init_interpolators(&mut self) {
        let mut abscissa = 0.;

        let mut point = self.position(0.);
        let mut t0 = 0.;
        while t0 < self.t_max() {
            let mut ts = Vec::with_capacity(NB_POINT_INTERPOLATION);
            let mut abscissas = Vec::with_capacity(NB_POINT_INTERPOLATION);
            ts.push(t0);
            abscissas.push(abscissa);
            for n in 1..=NB_POINT_INTERPOLATION {
                let t = t0 + n as f64 / NB_POINT_INTERPOLATION as f64;
                let next_point = self.position(t);
                abscissa += (point - next_point).mag();
                abscissas.push(abscissa);
                point = next_point;
                ts.push(t);
            }
            log::info!("Interpolating inverse...");
            let abscissa_t = abscissas.iter().cloned().zip(ts.iter().cloned()).collect();
            self.inverse_curvilinear_abscissa
                .push(chebyshev_polynomials::interpolate_points(
                    abscissa_t,
                    INTERPOLATION_ERROR,
                ));
            log::info!(
                "OK, deg = {}",
                self.inverse_curvilinear_abscissa
                    .last()
                    .unwrap()
                    .coeffs
                    .len()
            );

            let t_abscissa = ts.into_iter().zip(abscissas.into_iter()).collect();
            log::info!("Interpolating abscissa...");
            self.curvilinear_abscissa
                .push(chebyshev_polynomials::interpolate_points(
                    t_abscissa,
                    10. * INTERPOLATION_ERROR,
                ));
            log::info!(
                "OK, deg = {}",
                self.curvilinear_abscissa.last().unwrap().coeffs.len()
            );
            t0 += 1.;
        }
    }
}

impl Curved for Revolution {
    fn position(&self, t: f64) -> DVec3 {
        let revolution_angle = TAU * t;

        let section_rotation = PI * self.half_turns_count as f64 * t.rem_euclid(1.);

        let section_point = self.curve.point(t);
        let x = self.revolution_radius
            + self.curve_scale_factor
                * (section_point.x * section_rotation.cos()
                    - section_rotation.sin() * section_point.y);
        let y = self.curve_scale_factor
            * (section_point.x * section_rotation.sin() + section_rotation.cos() * section_point.y);

        DVec3 {
            x: revolution_angle.cos() * x,
            y: revolution_angle.sin() * x,
            z: y,
        }
    }

    fn bounds(&self) -> CurveBounds {
        CurveBounds::Finite
    }

    fn t_max(&self) -> f64 {
        self.curve.t_max()
    }

    fn t_min(&self) -> f64 {
        0.
    }

    fn subdivision_for_t(&self, t: f64) -> Option<usize> {
        Some(self.t_max().min(t).floor() as usize)
    }

    fn is_time_maps_singleton(&self) -> bool {
        true
    }

    fn full_turn_at_t(&self) -> Option<f64> {
        Some(self.curve.t_max())
    }

    fn curvilinear_abscissa(&self, t: f64) -> Option<f64> {
        if t == self.t_max() {
            self.curvilinear_abscissa.last().map(|p| p.evaluate(t))
        } else {
            self.curvilinear_abscissa
                .get(t.floor() as usize)
                .map(|p| p.evaluate(t))
        }
    }

    fn inverse_curvilinear_abscissa(&self, x: f64) -> Option<f64> {
        for t in 0..self.curvilinear_abscissa.len() {
            if self
                .curvilinear_abscissa(t as f64 + 1.)
                .filter(|y| y > &x)
                .is_some()
            {
                return self
                    .inverse_curvilinear_abscissa
                    .get(t)
                    .map(|p| p.evaluate(x));
            }
        }
        None
    }
}