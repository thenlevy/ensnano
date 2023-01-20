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

/// The number of points used in the iterative version of the discretization algorithm.
const NB_DISCRETISATION_STEP: usize = 100;

impl Curve {
    ///Older version of the discretization algorithm
    pub(super) fn discretize_legacy(&mut self, nucl_rise: f64, inclination: f64) {
        let nb_step = NB_DISCRETISATION_STEP;

        let len = self.legacy_length_by_descretisation(self.geometry.t_min(), 1., nb_step);
        let nb_points = (len / nucl_rise) as usize;
        let small_step = 1. / (nb_step as f64 * nb_points as f64);

        let mut points_forward = Vec::with_capacity(nb_points + 1);
        let mut points_backward = Vec::with_capacity(nb_points + 1);
        let mut axis_forward = Vec::with_capacity(nb_points + 1);
        let mut axis_backward = Vec::with_capacity(nb_points + 1);
        let mut curvature = Vec::with_capacity(nb_points + 1);
        let mut t = self.geometry.t_min();
        let mut current_axis = self.legacy_itterative_axis(t, None);

        let mut current_segment = 0;

        let point = self.legacy_point_at_t(t, &current_axis);

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

            let mut p = self.legacy_point_at_t(t, &current_axis);

            if let Some(t_x) = self
                .geometry
                .inverse_curvilinear_abscissa(next_point_abscissa)
            {
                t = t_x;
                current_abcissa = next_point_abscissa;
                current_axis = self.legacy_itterative_axis(t, Some(&current_axis));
                p = self.legacy_point_at_t(t, &current_axis);
            } else {
                while current_abcissa < next_point_abscissa {
                    t += small_step;

                    current_axis = self.legacy_itterative_axis(t, Some(&current_axis));

                    let q = self.legacy_point_at_t(t, &current_axis);

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
                    if self.nucl_pos_full_turn.is_none()
                        && self
                            .geometry
                            .full_turn_at_t()
                            .map(|t_obj| t > t_obj)
                            .unwrap_or(false)
                    {
                        self.nucl_pos_full_turn =
                            Some((points_forward.len() as isize - self.nucl_t0 as isize) as f64);
                    }
                }
            } else {
                points_backward.push(p);
                axis_backward.push(current_axis);
                next_abscissa_backward = current_abcissa + nucl_rise;
            }
        }
        log::info!("Synchronization length by old method {synchronization_length}");

        if self.nucl_pos_full_turn.is_none() && self.geometry.full_turn_at_t().is_some() {
            // We want to make a full turn just after the last nucl
            self.nucl_pos_full_turn =
                Some((points_forward.len() as isize - self.nucl_t0 as isize + 1) as f64);
        }

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

    fn legacy_length_by_descretisation(&self, t0: f64, t1: f64, nb_step: usize) -> f64 {
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
        let mut current_axis = self.legacy_itterative_axis(t0, None);
        let mut p = self.geometry.position(t0);
        let mut len = 0f64;
        for i in 1..=nb_step {
            let t = t0 + (i as f64) / (nb_step as f64) * (t1 - t0);
            current_axis = self.legacy_itterative_axis(t, Some(&current_axis));
            let q = self.geometry.position(t);
            len += (q - p).mag();
            p = q;
        }
        let quad = quadrature::integrate(|x| self.geometry.speed(x).mag(), t0, t1, 1e-7).integral;
        log::info!("by quadrature {}", quad);
        len
    }

    fn legacy_translation_axis(&self, current_axis: &DMat3) -> DMat3 {
        let mut ret = *current_axis;
        if let Some(frame) = self.geometry.initial_frame() {
            let up = frame[1];
            ret[0] = up.cross(ret[2]).normalized();
            ret[1] = ret[2].cross(ret[0]).normalized();
        }
        ret
    }

    fn legacy_point_at_t(&self, t: f64, current_axis: &DMat3) -> DVec3 {
        let mut ret = self.geometry.position(t);
        if let Some(translation) = self.geometry.translation() {
            let translation_axis = self.legacy_translation_axis(current_axis);
            ret += translation_axis * translation;
        }
        ret
    }

    fn legacy_itterative_axis(&self, t: f64, previous: Option<&DMat3>) -> DMat3 {
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
            perpendicular_basis(speed)
            //self.itterative_axis(t, Some(&previous))
        }
    }

    pub(super) fn legacy_nucl_pos(
        &self,
        n: isize,
        forward: bool,
        theta: f64,
        parameters: &Parameters,
    ) -> Option<DVec3> {
        use std::f64::consts::{PI, TAU};
        let idx = self.idx_convertsion(n)?;
        let theta = if let Some(real_theta) = self.geometry.theta_shift(parameters) {
            let base_theta = TAU / parameters.bases_per_turn as f64;
            (base_theta - real_theta) * n as f64 + theta
        } else if let Some(pos_full_turn) = self.nucl_pos_full_turn {
            let final_angle = -pos_full_turn as f64 * TAU / parameters.bases_per_turn as f64;
            let rem = final_angle.rem_euclid(TAU);

            let mut full_delta = -rem - std::f64::consts::FRAC_PI_2;
            full_delta = full_delta.rem_euclid(TAU);
            if full_delta > PI {
                full_delta -= TAU;
            }

            theta + full_delta / pos_full_turn as f64 * n as f64
        } else {
            theta
        };
        let axis = if forward {
            &self.axis_forward
        } else {
            &self.axis_backward
        };
        let positions = if forward {
            &self.positions_forward
        } else {
            &self.positions_backward
        };
        if let Some(matrix) = axis.get(idx).cloned() {
            let mut ret = matrix
                * DVec3::new(
                    -theta.cos() * parameters.helix_radius as f64,
                    theta.sin() * parameters.helix_radius as f64,
                    0.0,
                );
            ret += positions[idx];
            Some(ret)
        } else {
            None
        }
    }
}
