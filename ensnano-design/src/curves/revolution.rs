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

use crate::{curves::torus::PointOnSurface_, utils::dvec_to_vec};

use super::*;
use std::f64::consts::{PI, TAU};
use ultraviolet::{DRotor2, DVec2, Mat3};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revolution_angle_init: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nb_turn: Option<f64>,
    #[serde(skip)] // can be skipped because it is only used the first time the helix is created
    pub known_number_of_helices_in_shape: Option<usize>,
    #[serde(skip)] // can be skipped because it is only used the first time the helix is created
    pub known_helix_id_in_shape: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_number_of_nts: Option<usize>,

    // There is currently no way to set this value through the GUI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_turn_at_nt: Option<isize>,
}

impl InterpolatedCurveDescriptor {
    pub(super) fn instanciate(self, init_interpolators: bool) -> Revolution {
        let curve = self.curve.clone();
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
            init_revolution_angle: self.revolution_angle_init.unwrap_or(0.),
            nb_turn: self.nb_turn.unwrap_or(1.),
            known_number_of_helices_in_shape: self.known_number_of_helices_in_shape,
            knwon_helix_id_in_shape: self.known_helix_id_in_shape,
            objective_nb_nt: self.objective_number_of_nts,
            full_turn_at_nt: self.full_turn_at_nt,
        };
        if init_interpolators {
            ret.init_interpolators();
        }
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

enum SmoothInterpolatedCurve {
    Closed {
        interpolators: Vec<ChebyshevPolynomial>,
        curve: CurveDescriptor2D,
        smoothening_coeff: f64,
        half_turn: bool,
    },
    Open {
        interpolator: ChebyshevPolynomial,
        curve: CurveDescriptor2D,
        t_max: f64,
    },
}

impl SmoothInterpolatedCurve {
    fn from_curve_interpolation(
        curve: CurveDescriptor2D,
        mut interpolations: Vec<InterpolationDescriptor>,
        smoothening_coeff: f64,
        nb_half_turn: isize,
    ) -> Self {
        if curve.is_open() {
            let interpolator = match interpolations.swap_remove(0) {
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
            Self::Open {
                interpolator,
                curve,
                t_max: 1.0,
            }
        } else {
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
            Self::Closed {
                curve,
                interpolators,
                smoothening_coeff,
                half_turn: nb_half_turn.rem_euclid(2) != 0,
            }
        }
    }

    /// Given a time t, return the time u at which the section must be evaluated.
    /// Smoothen the junction between consecutive one-turn segments.
    fn smooth_chebyshev(&self, t: f64) -> f64 {
        match self {
            Self::Closed {
                interpolators,
                smoothening_coeff,
                half_turn,
                ..
            } => {
                // the position on the current segment. If u is close the 0, we interpolate with the
                // previous segment. If u is close to 1, we interpolate with the next segment.
                let u = t.rem_euclid(1.);

                let helix_idx =
                    (t.div_euclid(1.) as isize).rem_euclid(interpolators.len() as isize) as usize;
                let prev_idx =
                    (helix_idx as isize - 1).rem_euclid(interpolators.len() as isize) as usize;
                let next_idx = (helix_idx + 1).rem_euclid(interpolators.len());

                // Quantify what "close to 0" and "close to 1" mean.
                let a = *smoothening_coeff;

                let shift = if *half_turn { 0.5 } else { 0. };

                if u < a {
                    // second half of the interpolation region, v = 0.5 + 1/2 ( u / a)
                    let v = (1. + u / a) / 2.;
                    let mut v1 =
                        (interpolators[prev_idx].evaluate(1. - a + v * a) + shift).rem_euclid(1.);
                    let v2 = (interpolators[helix_idx].evaluate(v * a)).rem_euclid(1.);

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
                    let v1 = (interpolators[helix_idx].evaluate(1. - a + v * a)).rem_euclid(1.);
                    let mut v2 = (interpolators[next_idx].evaluate(v * a) - shift).rem_euclid(1.);

                    while v2 > v1 + 0.5 {
                        v2 -= 1.
                    }
                    while v2 < v1 - 0.5 {
                        v2 += 1.
                    }

                    (1. - v) * v1 + v * v2
                } else {
                    interpolators[helix_idx].evaluate(u)
                }
            }
            Self::Open { interpolator, .. } => interpolator.evaluate(t),
        }
    }

    fn point(&self, t: f64) -> DVec2 {
        let s = self.smooth_chebyshev(t);
        self.point_at_s(s)
    }

    fn curvilinear_abscissa(&self, t: f64) -> f64 {
        self.smooth_chebyshev(t)
    }

    fn t_max(&self) -> f64 {
        match self {
            Self::Closed { interpolators, .. } => interpolators.len() as f64,
            Self::Open { t_max, .. } => *t_max,
        }
    }

    fn normalized_tangent_at_s(&self, s: f64) -> DVec2 {
        match self {
            Self::Closed { curve, .. } => curve.normalized_tangent(s.rem_euclid(1.)),
            Self::Open { curve, .. } => curve.normalized_tangent(s),
        }
    }

    fn point_at_s(&self, s: f64) -> DVec2 {
        match self {
            Self::Closed { curve, .. } => curve.point(s.rem_euclid(1.)),
            Self::Open { curve, .. } => curve.point(s),
        }
    }
}

pub(super) struct Revolution {
    curve: SmoothInterpolatedCurve,
    revolution_radius: f64,
    curve_scale_factor: f64,
    half_turns_count: isize,
    /// The element at index i of this vector is a polynomial interpolating the function that maps
    /// a point x in [curvilinear_abscissa(i), curvilinear_abscissa(i+1)] to a time t so that
    /// curvilinear_abscissa(t) = x
    inverse_curvilinear_abscissa: Vec<ChebyshevPolynomial>,
    /// The element at index i of this vector is a polynomial interpolating the curvilinear
    /// abscissa between 0 and t for t in [i, i+1]
    curvilinear_abscissa: Vec<ChebyshevPolynomial>,
    init_revolution_angle: f64,
    nb_turn: f64,
    known_number_of_helices_in_shape: Option<usize>,
    knwon_helix_id_in_shape: Option<usize>,
    objective_nb_nt: Option<usize>,
    full_turn_at_nt: Option<isize>,
}

const NB_POINT_INTERPOLATION: usize = 100_000;
const INTERPOLATION_ERROR: f64 = 1e-4;
impl Revolution {
    /// Computes the polynomials that interpolate the curvilinear function and its inverse
    fn init_interpolators(&mut self) {
        let mut abscissa = 0.;

        let mut point = self.position(0.);
        let mut t0 = 0.;

        // The interpolating polynomials are computed in parallel. First we compute the
        // interpolation points for each polynomial.
        let mut curvilinear_abscissa_interpolation_points = Vec::new();
        let mut inverse_ca_interpolation_points = Vec::new();
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
            let abscissa_t: Vec<_> = abscissas
                .iter()
                .cloned()
                .zip(ts.iter().cloned())
                .step_by(10) // (1)
                .collect();

            inverse_ca_interpolation_points.push(abscissa_t);

            let t_abscissa: Vec<_> = ts
                .into_iter()
                .zip(abscissas.into_iter())
                .step_by(10) // (1)
                .collect();

            curvilinear_abscissa_interpolation_points.push(t_abscissa);

            // (1) Allows for quicker computation of the interpolating polynomial with little
            // impact on the quality of the interpolation

            t0 += 1.;
        }

        use rayon::prelude::*;
        self.curvilinear_abscissa = curvilinear_abscissa_interpolation_points
            .into_par_iter()
            .map(|v| chebyshev_polynomials::interpolate_points(v, 10. * INTERPOLATION_ERROR))
            .collect();
        self.inverse_curvilinear_abscissa = inverse_ca_interpolation_points
            .into_par_iter()
            .map(|v| chebyshev_polynomials::interpolate_points(v, INTERPOLATION_ERROR))
            .collect()
    }

    fn get_surface_info(&self, point: SurfacePoint) -> Option<SurfaceInfo> {
        log::info!("Info point point {:?}", point);
        let section_rotation = point.section_rotation_angle;

        let section_tangent = self
            .curve
            .normalized_tangent_at_s(point.abscissa_along_section)
            .rotated_by(DRotor2::from_angle(section_rotation));
        log::info!("section tangent {:?}", section_tangent);

        let right = crate::utils::dvec_to_vec(DVec3 {
            x: -point.revolution_angle.sin(),
            y: point.revolution_angle.cos(),
            z: 0.,
        });
        let up = crate::utils::dvec_to_vec(DVec3 {
            x: section_tangent.x * point.revolution_angle.cos(),
            y: section_tangent.x * point.revolution_angle.sin(),
            z: section_tangent.y,
        });
        let direction = right.cross(up);

        let local_frame = if point.reversed_direction {
            Mat3::new(-right, up, -direction).into_rotor3()
        } else {
            Mat3::new(right, up, direction).into_rotor3()
        };

        let position = self.curve_point_to_3d(
            self.curve.point_at_s(point.abscissa_along_section),
            point.revolution_angle,
            Some(point.section_rotation_angle),
        );

        Some(SurfaceInfo {
            point,
            section_tangent: Vec2::new(section_tangent.x as f32, section_tangent.y as f32),
            local_frame,
            position: dvec_to_vec(position),
        })
    }

    fn curve_point_to_3d(
        &self,
        section_point: DVec2,
        revolution_angle: f64,
        section_angle: Option<f64>,
    ) -> DVec3 {
        let t = self.revolution_angle_to_t(revolution_angle);
        let section_rotation =
            section_angle.unwrap_or_else(|| self.default_section_rotation_angle(t));

        let surface = PointOnSurface_ {
            section_rotation,
            revolution_axis_position: -self.revolution_radius,
            revolution_angle,
            curve_scale_factor: self.curve_scale_factor,
        };
        CurveDescriptor2D::_3d(section_point, &surface)
    }

    fn default_section_rotation_angle(&self, t: f64) -> f64 {
        PI * self.half_turns_count as f64 * t.rem_euclid(1.)
    }

    fn t_to_revolution_angle(&self, t: f64) -> f64 {
        self.init_revolution_angle + self.nb_turn * TAU * t
    }

    fn revolution_angle_to_t(&self, angle: f64) -> f64 {
        let angle_t1 = TAU * self.nb_turn;
        (angle - self.init_revolution_angle).rem_euclid(angle_t1) / TAU / self.nb_turn
    }
}

impl Curved for Revolution {
    fn position(&self, t: f64) -> DVec3 {
        let revolution_angle = self.t_to_revolution_angle(t);

        let section_point = self.curve.point(t);
        self.curve_point_to_3d(section_point, revolution_angle, None)
    }

    fn speed(&self, t: f64) -> DVec3 {
        (self.position(t + EPSILON_DERIVATIVE) - self.position(t)) / EPSILON_DERIVATIVE
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

    fn inverse_curvilinear_abscissa(&self, mut x: f64) -> Option<f64> {
        let n = x.div_euclid(self.curvilinear_abscissa(self.t_max())?);
        x = x.rem_euclid(self.curvilinear_abscissa(self.t_max())?);

        for t in 0..self.curvilinear_abscissa.len() {
            if self
                .curvilinear_abscissa(t as f64 + 1.)
                .filter(|y| y > &x)
                .is_some()
            {
                return self
                    .inverse_curvilinear_abscissa
                    .get(t)
                    .map(|p| p.evaluate(x))
                    .map(|r| n * self.t_max() + r);
            }
        }
        self.inverse_curvilinear_abscissa
            .last()
            .map(|p| p.evaluate(x))
            .map(|r| n * self.t_max() + r)
    }

    fn surface_info_time(&self, t: f64, helix_id: usize) -> Option<SurfaceInfo> {
        let point = super::SurfacePoint {
            revolution_angle: self.t_to_revolution_angle(t),
            abscissa_along_section: self.curve.curvilinear_abscissa(t),
            helix_id,
            section_rotation_angle: self.default_section_rotation_angle(t),
            reversed_direction: false,
        };
        self.get_surface_info(point)
    }

    fn surface_info(&self, point: SurfacePoint) -> Option<SurfaceInfo> {
        self.get_surface_info(point)
    }

    fn additional_isometry(&self, segment_idx: usize) -> Option<Isometry2> {
        self.known_number_of_helices_in_shape
            .zip(self.knwon_helix_id_in_shape)
            .map(|(nb_helices, h_id)| Isometry2 {
                translation: (h_id as f32 + (segment_idx + 1) as f32 * nb_helices as f32)
                    * 5.
                    * Vec2::unit_y(),
                rotation: ultraviolet::Rotor2::identity(),
            })
    }

    fn objective_nb_nt(&self) -> Option<usize> {
        self.objective_nb_nt
    }

    fn nucl_pos_full_turn(&self) -> Option<isize> {
        self.full_turn_at_nt
    }
}
