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

impl Curve {
    pub(super) fn discretize(&mut self, mut len_segment: f64, nb_step: usize, inclination: f64) {
        let len =
            self.length_by_descretisation(self.geometry.t_min(), self.geometry.t_max(), nb_step);
        let nb_points = (len / len_segment) as usize;
        let small_step = 1. / (nb_step as f64 * nb_points as f64);

        if let Some(last_t) = self.geometry.full_turn_at_t() {
            let synchronization_length =
                self.length_by_descretisation(self.geometry.t_min(), last_t, nb_points);
            let epsilon = synchronization_length.rem_euclid(len_segment);

            log::info!("Synchronization length by descretisation {synchronization_length}");

            if epsilon > len_segment / 2. {
                // n and espilon are chosen so that
                // synchronization_length = n * len_segment - epsilon
                //                        = n * (len_segment - epsilon / n)

                let n: f64 = synchronization_length.div_euclid(len_segment) + 1.;
                let epsilon = len_segment - epsilon; // epsilon > 0
                len_segment = len_segment - epsilon / n;
            } else {
                // n and espilon are chosen so that
                // synchronization_length = n * len_segment + epsilon
                //                        = n * (len_segment + epsilon / n)

                let n: f64 = synchronization_length.div_euclid(len_segment) + 1.;
                len_segment = len_segment + epsilon / n;
            }
            self.nucl_pos_full_turn = Some(synchronization_length / len_segment + 1.);
        }
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
        let mut abscissa_forward;
        let mut abscissa_backward;

        if inclination >= 0. {
            // The forward strand is behind
            points_forward.push(point);
            axis_forward.push(current_axis);
            curvature.push(self.geometry.curvature(t));
            t_nucl.push(t);
            abscissa_forward = len_segment;
            abscissa_backward = inclination;
        } else {
            // The backward strand is behind
            points_backward.push(point);
            axis_backward.push(current_axis);
            abscissa_backward = len_segment;
            abscissa_forward = -inclination;
        }

        let mut current_abcissa = 0.0;
        let mut first_non_negative = t < 0.0;

        let mut synchronization_length = 0.;

        while t <= self.geometry.t_max() {
            if first_non_negative && t >= 0.0 {
                first_non_negative = false;
                self.nucl_t0 = points_forward.len();
            }
            let (objective, forward) = (
                abscissa_forward.min(abscissa_backward),
                abscissa_forward <= abscissa_backward,
            );
            let mut translation_axis = current_axis;
            if let Some(frame) = self.geometry.initial_frame() {
                let up = frame[1];
                translation_axis[1] = up;
                translation_axis[0] = up.cross(self.geometry.speed(t).normalized());
                translation_axis[2] = translation_axis[0].cross(translation_axis[1]);
            }
            let mut p = self.geometry.position(t)
                + translation_axis * self.geometry.translation().unwrap_or_else(DVec3::zero);

            if let Some(t_x) = self.geometry.inverse_curvilinear_abscissa(objective) {
                t = t_x;
                current_abcissa = objective;
                current_axis = self.itterative_axis(t, Some(&current_axis));
                p = self.geometry.position(t);
                if let Some(t) = self.geometry.translation() {
                    p += current_axis * t;
                }
            } else {
                while current_abcissa < objective {
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
            if t <= self.geometry.t_max() || self.geometry.bounds() != CurveBounds::Finite {
                if forward {
                    t_nucl.push(t);
                    let segment_idx = self.geometry.subdivision_for_t(t).unwrap_or(0);
                    if segment_idx != current_segment {
                        current_segment = segment_idx;
                        self.additional_segment_left.push(points_forward.len())
                    }
                    points_forward.push(p);
                    axis_forward.push(current_axis);
                    curvature.push(self.geometry.curvature(t));
                    abscissa_forward = current_abcissa + len_segment;
                } else {
                    points_backward.push(p);
                    axis_backward.push(current_axis);
                    abscissa_backward = current_abcissa + len_segment;
                }
            }
        }
        log::info!("Synchronization length by old method {synchronization_length}");

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
        len
    }

    fn translation_axis(&self, t: f64, current_axis: &DMat3) -> DMat3 {
        let mut ret = current_axis.clone();
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
}
