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
    pub half_turns_count: usize,
    /// Radius of the revolution trajectory
    pub revolution_radius: f64,
    /// Scale factor of the section
    pub curve_scale_factor: f64,
    pub interpolation: Vec<InterpolationDescriptor>,
}

impl InterpolatedCurveDescriptor {
    pub(super) fn instanciate(self) -> Revolution {
        let curve = self.curve.clone();
        let curves = self
            .interpolation
            .into_iter()
            .map(|i| InstanciatedInterpolatedCurve::from_curve_interpolation(curve.clone(), i))
            .collect();
        Revolution {
            curves,
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

struct InstanciatedInterpolatedCurve {
    interpolator: ChebyshevPolynomial,
    curve: CurveDescriptor2D,
}

impl InstanciatedInterpolatedCurve {
    fn from_curve_interpolation(
        curve: CurveDescriptor2D,
        interpolation: InterpolationDescriptor,
    ) -> Self {
        match interpolation {
            InterpolationDescriptor::PointsValues { points, values } => {
                let points_values = points.into_iter().zip(values.into_iter()).collect();
                let interpolator = chebyshev_polynomials::interpolate_points(points_values, 1e-4);
                Self {
                    curve,
                    interpolator,
                }
            }
            InterpolationDescriptor::Chebyshev { coeffs, interval } => {
                let interpolator = chebyshev_polynomials::ChebyshevPolynomial::from_coeffs_interval(
                    coeffs, interval,
                );
                Self {
                    curve,
                    interpolator,
                }
            }
        }
    }

    fn point(&self, t: f64) -> DVec2 {
        let s = self.interpolator.evaluate(t);
        self.curve.point(s)
    }
}

pub(super) struct Revolution {
    curves: Vec<InstanciatedInterpolatedCurve>,
    revolution_radius: f64,
    curve_scale_factor: f64,
    half_turns_count: usize,
}

impl Revolution {
    fn position_(&self, t: f64, curve_idx: usize) -> DVec3 {
        let t = t.fract();
        let revolution_angle = TAU * t;

        let section_rotation = PI * self.half_turns_count as f64 * t;

        let section_point = self.curves[curve_idx].point(t);
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
}

impl Curved for Revolution {
    fn position(&self, t: f64) -> DVec3 {
        let curve_idx = (t.floor() as usize).min(self.curves.len() - 1);
        self.position_(t, curve_idx)
    }

    // By using
    fn speed(&self, t: f64) -> DVec3 {
        let curve_idx = (t.floor() as usize).min(self.curves.len() - 1);
        (self.position_(t + EPSILON_DERIVATIVE / 2., curve_idx)
            - self.position_(t - EPSILON_DERIVATIVE / 2., curve_idx))
            / EPSILON_DERIVATIVE
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let curve_idx = (t.floor() as usize).min(self.curves.len() - 1);
        ((self.position_(t + EPSILON_DERIVATIVE, curve_idx)
            + self.position_(t - EPSILON_DERIVATIVE, curve_idx))
            - 2. * self.position_(t, curve_idx))
            / (EPSILON_DERIVATIVE * EPSILON_DERIVATIVE)
    }

    fn bounds(&self) -> CurveBounds {
        CurveBounds::Finite
    }

    fn t_max(&self) -> f64 {
        self.curves.len() as f64
    }

    fn subdivision_for_t(&self, t: f64) -> Option<usize> {
        Some(t.floor() as usize)
    }

    fn is_time_maps_singleton(&self) -> bool {
        true
    }
}

/*
    func point(s: Double, t: Double) -> Point3D {
        let α = 2 * Double.pi * s
        let cα = cos(α)
        let sα = sin(α)

        let β = Double.pi * s * Double(half_turns_count)
        let cβ = cos(β)
        let sβ = sin(β)

        let p = curve.point(t)
        let x = radius + scale * (cβ * p.x - sβ * p.y)
        let y = scale * (sβ * p.x + cβ * p.y)

        return Point3D(x: x * cα, y: x * sα, z: y)
    }
*/
