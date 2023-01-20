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

//! Implementation of the curve discretization alogrithm.

use super::*;
use chebyshev_polynomials::ChebyshevPolynomial;

/// The number of points used in the iterative version of the discretization algorithm.
const NB_DISCRETISATION_STEP: usize = 100_000;

/// The number of points used in the quick iterative version of the discretization algorithm.
const NB_FAST_DISCRETIZATION_STEP: usize = 1_000;

impl Curve {
    /// Pre-compute the frames arround which the nucleotides will be positioned.
    ///
    /// This is done by an iterative discretization algorithm that computes forward positions
    /// (f_0,... f_n) (for the forward strand) and backwards positions (b_0,..., b_n) (for the backward strand), so that
    /// * The curvinilear distance between f_{i} and f_{i + 1} is `nucl_rise`.
    /// * The curvinilear distance between b_{i} and b_{i + 1} is `nucl_rise`.
    /// * The curvilinear distance between f_{i} and b_{i} is `inclination`.
    ///
    /// Note that the actual value used for `nucl_rise` may be slightly different from the value
    /// given in argument. This happen when the implementation of `Curved` for `self.geometry`
    /// specifies that a certain number of nucleotides must fit on a specific portion of the curve
    /// (i.e. at least one of the method `full_turn_at_t`, `nucl_pos_full_turn` or
    /// `objective_nb_nt` has been overriden).
    pub(super) fn discretize(&mut self, mut nucl_rise: f64, inclination: f64) {
        if self.geometry.legacy() {
            return self.discretize_legacy(nucl_rise, inclination);
        }
        let polynomials = self.compute_polynomials();

        let nb_step = if self.geometry.discretize_quickly() {
            NB_FAST_DISCRETIZATION_STEP
        } else {
            NB_DISCRETISATION_STEP
        };
        let len = polynomials
            .as_ref()
            .map(|p| p.curvilinear_abcsissa.evaluate(self.geometry.t_max()))
            .unwrap_or_else(|| {
                self.length_by_descretisation(self.geometry.t_min(), self.geometry.t_max(), nb_step)
            });
        let nb_points = (len / nucl_rise) as usize;
        let small_step = 0.1 / (nb_step as f64);
        log::info!("small step = {small_step}");
        log::info!(
            "len = {}",
            self.length_by_descretisation(self.geometry.t_min(), self.geometry.t_max(), nb_step)
        );

        self.adjust_rise(&mut nucl_rise, polynomials.as_ref());

        //overide nucl_pos_full_turn with the value given by the geometry if it exists
        self.nucl_pos_full_turn = self
            .geometry
            .nucl_pos_full_turn()
            .map(|x| x as f64)
            .or(self.nucl_pos_full_turn);

        if let Some(n) = self.nucl_pos_full_turn {
            log::info!("nucl_pos_full_turn = {n}");
        }

        let mut points_forward = Vec::with_capacity(nb_points + 1);
        let mut points_backward = Vec::with_capacity(nb_points + 1);
        let mut axis_forward = Vec::with_capacity(nb_points + 1);
        let mut axis_backward = Vec::with_capacity(nb_points + 1);
        let mut curvature = Vec::with_capacity(nb_points + 1);
        let mut t = self.geometry.t_min();
        let mut current_axis = self.itterative_axis(t, None);

        let mut current_segment = 0;

        let point = self.point_at_t(t, &current_axis);

        let mut t_nucl = Vec::new();
        let mut next_abscissa_forward;
        let mut next_abscissa_backward;

        // Decide if the the point at t = self.geometry.t_min() belongs to the backward or the
        // forward strand.
        let first_forward;
        if inclination >= 0. {
            // The forward strand is behind
            points_forward.push(point);
            axis_forward.push(current_axis);
            curvature.push(self.geometry.curvature(t));
            t_nucl.push(t);
            next_abscissa_forward = nucl_rise;
            next_abscissa_backward = inclination;
            first_forward = true;
        } else {
            // The backward strand is behind
            points_backward.push(point);
            axis_backward.push(current_axis);
            next_abscissa_backward = nucl_rise;
            next_abscissa_forward = -inclination;
            first_forward = false;
        }

        let mut current_abcissa = 0.0;
        let mut first_non_negative = t < 0.0;

        let mut synchronization_length = 0.;

        // The descritisation stops when t > t_max and when we have as many forward as backwards
        // point
        while t <= self.geometry.t_max()
            || next_abscissa_backward < next_abscissa_forward + inclination
        {
            log::debug!("backward {next_abscissa_backward}, forward {next_abscissa_forward}");
            if first_non_negative && t >= 0.0 {
                first_non_negative = false;
                self.nucl_t0 = points_forward.len();
            }

            // Decide on which strand belongs the next point that we are looking for and it's
            // curvilinear abcissa
            let (next_point_abscissa, next_point_forward) = if t <= self.geometry.t_max() {
                (
                    next_abscissa_forward.min(next_abscissa_backward),
                    next_abscissa_forward <= next_abscissa_backward,
                )
            } else if first_forward {
                (next_abscissa_backward, false)
            } else {
                (next_abscissa_forward, true)
            };

            let mut p = self.point_at_t(t, &current_axis);

            if let Some(t_x) = self
                .geometry
                .inverse_curvilinear_abscissa(next_point_abscissa)
                .or_else(|| {
                    polynomials
                        .as_ref()
                        .and_then(|p| p.inverse_abscissa(next_point_abscissa))
                })
            {
                t = t_x;
                current_abcissa = next_point_abscissa;
                current_axis = self.itterative_axis(t, Some(&current_axis));
                p = self.point_at_t(t, &current_axis);
            } else {
                while current_abcissa < next_point_abscissa {
                    t += small_step;

                    current_axis = self.itterative_axis(t, Some(&current_axis));

                    let q = self.point_at_t(t, &current_axis);

                    current_abcissa += (q - p).mag();

                    if let Some(t_obj) = self.geometry.full_turn_at_t() {
                        if t >= 0. && t < t_obj {
                            synchronization_length += (q - p).mag();
                        }
                    }
                    p = q;
                }
            }

            if next_point_forward {
                if t <= self.geometry.t_max() {
                    t_nucl.push(t);
                    let segment_idx = self.geometry.subdivision_for_t(t).unwrap_or(0);
                    if segment_idx != current_segment {
                        current_segment = segment_idx;
                        self.additional_segment_left.push(points_forward.len())
                    }
                    points_forward.push(p);
                    axis_forward.push(current_axis);
                    curvature.push(self.geometry.curvature(t));
                    next_abscissa_forward = current_abcissa + nucl_rise;
                }
            } else {
                points_backward.push(p);
                axis_backward.push(current_axis);
                next_abscissa_backward = current_abcissa + nucl_rise;
            }
        }
        log::info!("Synchronization length by old method {synchronization_length}");
        log::debug!("t_nucl {:.4?}", t_nucl);

        self.axis_backward = axis_backward;
        self.positions_backward = points_backward;
        self.axis_forward = axis_forward;
        self.positions_forward = points_forward;
        self.curvature = curvature;
        self.t_nucl = Arc::new(t_nucl);
        if self.geometry.is_time_maps_singleton() {
            self.abscissa_converter = AbscissaConverter::from_single_map(self.t_nucl.clone());
        }
    }

    /// If `self.geometry` sepcifies that a certain number of nucleotide must fit on a given
    /// portion of the curve, adjust the value of nucl_rise accordingly.
    fn adjust_rise(&mut self, nucl_rise: &mut f64, polynomials: Option<&PreComputedPolynomials>) {
        let nb_step = if self.geometry.discretize_quickly() {
            NB_FAST_DISCRETIZATION_STEP
        } else {
            NB_DISCRETISATION_STEP
        };
        if let Some(last_t) = self.geometry.full_turn_at_t() {
            let synchronization_length = polynomials
                .map(|p| p.curvilinear_abcsissa.evaluate(last_t))
                .unwrap_or_else(|| {
                    self.length_by_descretisation(self.geometry.t_min(), last_t, nb_step)
                });

            if let Some(n) = self.geometry.objective_nb_nt() {
                // If a given number of nucleotide is specified we adjust nucl_rise accordingly
                *nucl_rise = synchronization_length / n as f64;
            } else {
                // Otherwise, we just adjust nucl_rise in order to obtain an integer number of
                // nucleotide

                // The remaining curvilinear length after positioning the last nucleotide.
                let epsilon = synchronization_length.rem_euclid(*nucl_rise);

                log::info!("Synchronization length by descretisation {synchronization_length}");

                if epsilon > *nucl_rise / 2. {
                    // n and espilon are chosen so that
                    // synchronization_length = n * len_segment - epsilon
                    //                        = n * (len_segment - epsilon / n)
                    let n: f64 = synchronization_length.div_euclid(*nucl_rise);

                    // synchronization_length = len_segment * n + epsilon
                    //                        = len_segment * (n + 1) - len_segment + epsilon
                    //                        = len_segment * (n + 1) - epsilon_
                    let epsilon_ = *nucl_rise - epsilon; // epsilon_ > 0

                    *nucl_rise -= epsilon_ / (n + 1.);
                } else {
                    // n and espilon are chosen so that
                    // synchronization_length = n * len_segment + epsilon
                    //                        = n * (len_segment + epsilon / n)
                    let n: f64 = synchronization_length.div_euclid(*nucl_rise);

                    *nucl_rise += epsilon / n;
                }
            }

            self.nucl_pos_full_turn = Some(synchronization_length / *nucl_rise);
        }
    }

    pub fn length_by_descretisation(&self, t0: f64, t1: f64, nb_step: usize) -> f64 {
        if t0 > t1 {
            log::error!(
                "Bad parameters ofr length by descritisation: \n t0 {} \n t1 {} \n nb_step {}",
                t0,
                t1,
                nb_step
            );
        }
        if let Some((x0, x1)) = self
            .geometry
            .curvilinear_abscissa(t0)
            .zip(self.geometry.curvilinear_abscissa(t1))
        {
            let ret = x1 - x0;
            log::info!("length by curvilinear_abscissa = {ret}");
            return x1 - x0;
        }
        let mut current_axis = self.itterative_axis(t0, None);
        let mut p = self.point_at_t(t0, &current_axis);
        let mut len = 0f64;
        for i in 1..=nb_step {
            let t = t0 + (i as f64) / (nb_step as f64) * (t1 - t0);
            current_axis = self.itterative_axis(t, Some(&current_axis));
            let q = self.point_at_t(t, &current_axis);
            len += (q - p).mag();
            p = q;
        }
        let quad = quadrature::integrate(|x| self.geometry.speed(x).mag(), t0, t1, 1e-7).integral;
        log::info!("by quadrature {}", quad);
        len
    }

    fn translation_axis(&self, t: f64, current_axis: &DMat3) -> DMat3 {
        let mut ret = *current_axis;
        if let Some(frame) = self.geometry.initial_frame() {
            let up = frame[1];
            ret[1] = up;
            ret[0] = up.cross(self.geometry.speed(t).normalized());
            ret[2] = ret[0].cross(ret[1]);
        }
        ret
    }

    fn point_at_t(&self, t: f64, current_axis: &DMat3) -> DVec3 {
        let mut ret = self.geometry.position(t);
        if let Some(translation) = self.geometry.translation() {
            let translation_axis = self.translation_axis(t, current_axis);
            ret += translation_axis * translation;
        }
        ret
    }

    fn itterative_axis(&self, t: f64, previous: Option<&DMat3>) -> DMat3 {
        let speed = self.geometry.speed(t);
        if speed.mag_sq() < EPSILON {
            let acceleration = self.geometry.acceleration(t);
            let mat = perpendicular_basis(acceleration);
            return DMat3::new(mat.cols[2], mat.cols[1], mat.cols[0]);
        }

        if let Some(previous) = previous {
            let forward = speed.normalized();
            let up = forward.cross(previous.cols[0]).normalized();
            let right = up.cross(forward);

            DMat3::new(right, up, forward)
        } else {
            let previous = perpendicular_basis(speed);
            self.itterative_axis(t, Some(&previous))
        }
    }

    /// Return a value of t_min that would allow self to have nucl
    pub fn left_extension_to_have_nucl(&self, nucl: isize, parameters: &Parameters) -> Option<f64> {
        let nucl_min = -(self.nucl_t0 as isize);
        if nucl < nucl_min {
            if let CurveBounds::BiInfinite = self.geometry.bounds() {
                let objective = (-nucl) as f64
                    * parameters.z_step as f64
                    * self.geometry.z_step_ratio().unwrap_or(1.);
                if let Some(t_min) = self.geometry.inverse_curvilinear_abscissa(objective) {
                    return Some(t_min);
                }
                let mut delta = 1.0;
                while delta < DELTA_MAX {
                    let new_tmin = self.geometry.t_min() - delta;
                    if self.length_by_descretisation(new_tmin, 0.0, NB_DISCRETISATION_STEP / 100)
                        > objective
                    {
                        return Some(new_tmin);
                    }
                    delta *= 2.0;
                }
                None
            } else {
                None
            }
        } else {
            Some(self.geometry.t_min())
        }
    }

    /// Return a value of t_max that would allow self to have nucl
    pub fn right_extension_to_have_nucl(
        &self,
        nucl: isize,
        parameters: &Parameters,
    ) -> Option<f64> {
        let nucl_max = (self.nb_points() - self.nucl_t0) as isize;
        if nucl >= nucl_max - 1 {
            match self.geometry.bounds() {
                CurveBounds::BiInfinite | CurveBounds::PositiveInfinite => {
                    let objective = nucl as f64
                        * parameters.z_step as f64
                        * self.geometry.z_step_ratio().unwrap_or(1.)
                        + parameters.inclination as f64;
                    if let Some(t_max) = self.geometry.inverse_curvilinear_abscissa(objective) {
                        return Some(t_max);
                    }
                    let mut delta = 1.0;
                    while delta < DELTA_MAX {
                        let new_tmax = self.geometry.t_max() + delta;
                        if self.length_by_descretisation(
                            0.0,
                            new_tmax,
                            NB_DISCRETISATION_STEP / 100,
                        ) > objective
                        {
                            return Some(new_tmax);
                        }
                        delta *= 2.0;
                    }
                    None
                }
                CurveBounds::Finite => None,
            }
        } else {
            Some(self.geometry.t_max())
        }
    }

    fn compute_polynomials(&self) -> Option<PreComputedPolynomials> {
        self.geometry.pre_compute_polynomials().then(|| {
            let mut t = self.geometry.t_min();
            let mut abscissa = 0.;
            let mut current_axis = self.itterative_axis(t, None);
            current_axis = self.itterative_axis(t, Some(&current_axis));
            let mut p = self.point_at_t(t, &current_axis);

            let mut ts = vec![t];
            let mut abscissas = vec![abscissa];
            let t0 = self.geometry.t_min();
            let t1 = self.geometry.t_max();

            let nb_step = NB_DISCRETISATION_STEP / 10;
            for i in 1..=nb_step {
                t = t0 + (i as f64) / (nb_step as f64) * (t1 - t0);
                current_axis = self.itterative_axis(t, Some(&current_axis));
                let q = self.point_at_t(t, &current_axis);
                abscissa += (p - q).mag();
                ts.push(t);
                abscissas.push(abscissa);
                p = q;
            }

            let abscissa_t = abscissas
                .iter()
                .cloned()
                .zip(ts.iter().cloned())
                .step_by(10) // (1)
                .collect();
            let t_abscissa = ts
                .into_iter()
                .zip(abscissas.into_iter())
                .step_by(10) // (1)
                .collect();

            // (1) This allows the interpolation to run much quicker with very little impact on
            // precision.

            let curvilinear_abcsissa = chebyshev_polynomials::interpolate_points(t_abscissa, 1e-4);
            let inverse_abcsissa = chebyshev_polynomials::interpolate_points(abscissa_t, 1e-4);

            PreComputedPolynomials {
                curvilinear_abcsissa,
                inverse_abscissa: inverse_abcsissa,
            }
        })
    }
}

/// Polynomials computed at the start of the discretization procedure
struct PreComputedPolynomials {
    curvilinear_abcsissa: ChebyshevPolynomial,
    inverse_abscissa: ChebyshevPolynomial,
}

impl PreComputedPolynomials {
    /// If x is in the interval on which `self.inverse_abscissa` is defined, return the evaluation
    /// at x.
    fn inverse_abscissa(&self, x: f64) -> Option<f64> {
        let interval = self.inverse_abscissa.definition_interval();
        (x >= interval[0] && x <= interval[1]).then(|| self.inverse_abscissa.evaluate(x))
    }
}
