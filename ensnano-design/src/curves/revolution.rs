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
        Revolution {
            curve,
            revolution_radius: self.revolution_radius,
            curve_scale_factor: self.curve_scale_factor,
            half_turns_count: self.half_turns_count,
        }
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
        let helix_idx = (s.div_euclid(1.) as usize).rem_euclid(self.interpolators.len());
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

    fn subdivision_for_t(&self, t: f64) -> Option<usize> {
        Some(t.floor() as usize)
    }

    fn is_time_maps_singleton(&self) -> bool {
        true
    }
}
