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

#[derive(Clone)]
pub(super) struct CloseSurfaceTopology {
    nb_segment: usize,
    nb_section_per_segment: usize,
    prev_section: Vec<usize>,
    next_section: Vec<usize>,
    other_spring_end: Vec<usize>,
    target: RevolutionSurface,
    idx_range: Vec<usize>,
}

impl CloseSurfaceTopology {
    pub fn new(desc: RevolutionSurfaceSystemDescriptor) -> Self {
        let nb_segment = 2 * desc.target.nb_helix_per_half_section;
        let nb_section_per_segment = NB_SECTION_PER_SEGMENT;
        let total_nb_section = nb_segment * nb_section_per_segment;

        let target = RevolutionSurface::new(desc.target);
        let next_section: Vec<usize> = (0..total_nb_section)
            .map(|n| {
                if n % nb_section_per_segment == nb_section_per_segment - 1 {
                    let segment = n / nb_section_per_segment;
                    let next_segment = (segment as isize + target.total_shift())
                        .rem_euclid(nb_segment as isize)
                        as usize;
                    next_segment * nb_section_per_segment
                } else {
                    n + 1
                }
            })
            .collect();

        let prev_section: Vec<usize> = (0..total_nb_section)
            .map(|n| {
                if n % nb_section_per_segment == 0 {
                    let segment = n / nb_section_per_segment;
                    let prev_segment = (segment as isize - target.total_shift())
                        .rem_euclid(nb_segment as isize)
                        as usize;
                    prev_segment * nb_section_per_segment + nb_section_per_segment - 1
                } else {
                    n - 1
                }
            })
            .collect();

        let other_spring_end: Vec<usize> = (0..total_nb_section)
            .map(|n| (n + nb_section_per_segment) % total_nb_section)
            .collect();

        let idx_range: Vec<usize> = (0..total_nb_section).collect();

        Self {
            nb_segment,
            nb_section_per_segment,
            prev_section,
            next_section,
            target,
            other_spring_end,
            idx_range,
        }
    }
}

impl SpringTopology for CloseSurfaceTopology {
    fn nb_balls(&self) -> usize {
        self.nb_section_per_segment * self.nb_segment
    }

    fn balls_with_predecessor(&self) -> &[usize] {
        &self.idx_range
    }
    fn predecessor(&self, ball_id: usize) -> usize {
        self.prev_section[ball_id]
    }

    fn balls_with_successor(&self) -> &[usize] {
        &self.idx_range
    }
    fn successor(&self, ball_id: usize) -> usize {
        self.next_section[ball_id]
    }

    fn balls_with_predecessor_and_successor(&self) -> &[usize] {
        &self.idx_range
    }

    fn balls_involved_in_spring(&self) -> &[usize] {
        &self.idx_range
    }

    fn other_spring_end(&self, ball_id: usize) -> usize {
        self.other_spring_end[ball_id]
    }

    fn surface_position(&self, revolution_angle: f64, theta: f64) -> DVec3 {
        self.target.position(revolution_angle, theta)
    }

    fn revolution_angle_ball(&self, ball_id: usize) -> f64 {
        (ball_id % self.nb_section_per_segment) as f64 * TAU / (self.nb_section_per_segment as f64)
    }

    fn theta_ball_init(&self) -> Vec<f64> {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        let mut ret = Vec::with_capacity(total_nb_segment);

        for segment_idx in 0..self.nb_segment {
            let theta_init = segment_idx as f64 / self.nb_segment as f64;
            let delta_theta = self.target.shift_per_turn as f64
                / (self.target.nb_helix_per_half_section as f64 * 2.);

            for section_idx in 0..self.nb_section_per_segment {
                let a = section_idx as f64 / self.nb_section_per_segment as f64;

                let theta_section = theta_init + a * delta_theta;
                ret.push(theta_section);
            }
        }
        ret
    }

    fn dpos_dtheta(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        self.target.dpos_dtheta(revolution_angle, section_t)
    }

    fn rescale_radius(&mut self, scaling_factor: f64) {
        self.target.revolution_radius *= scaling_factor;
        println!("revolution radius {}", self.target.revolution_radius);
    }

    fn rescale_section(&mut self, scaling_factor: f64) {
        self.target.curve_scale_factor *= scaling_factor;
    }

    fn cloned(&self) -> Box<dyn SpringTopology> {
        Box::new(self.clone())
    }

    fn axis(&self, revolution_angle: f64) -> DVec3 {
        self.target.axis(revolution_angle)
    }

    fn to_curve_descriptor(&self, thetas: Vec<f64>) -> Vec<CurveDescriptor> {
        let mut ret = Vec::new();

        let nb_segment_per_helix = self.nb_segment / self.target.nb_helices;
        for i in 0..self.target.nb_helices {
            let mut interpolations = Vec::new();
            let segment_indicies = (0..nb_segment_per_helix).map(|n| {
                (i as isize + (n as isize * self.target.total_shift()))
                    .rem_euclid(self.nb_segment as isize)
            });
            let theta_0 = thetas[i * self.nb_section_per_segment];
            for s_idx in segment_indicies {
                let start = s_idx as usize * self.nb_section_per_segment;
                let end = start + self.nb_section_per_segment - 1;
                let mut segment_thetas = thetas[start..=end].to_vec();
                let mut next_value = thetas[self.next_section[end]];
                if self.target.half_turns_count % 2 == 1 {
                    next_value += 0.5;
                }
                let last_value = segment_thetas.last().unwrap();
                while next_value >= 0.5 + last_value {
                    next_value -= 1.
                }
                while next_value <= last_value - 0.5 {
                    next_value += 1.
                }
                segment_thetas.push(next_value);
                //println!("thetas {:.2?}", segment_thetas);
                let s = (0..=self.nb_section_per_segment)
                    .map(|x| x as f64 / self.nb_section_per_segment as f64)
                    .collect();
                interpolations.push(InterpolationDescriptor::PointsValues {
                    points: s,
                    values: segment_thetas,
                });
            }
            ret.push((
                InterpolatedCurveDescriptor {
                    curve: self.target.curve.clone(),
                    curve_scale_factor: self.target.curve_scale_factor,
                    chevyshev_smoothening: self.target.junction_smoothening,
                    interpolation: interpolations,
                    half_turns_count: self.target.half_turns_count,
                    revolution_radius: self.target.revolution_radius,
                    nb_turn: None,
                    revolution_angle_init: None,
                    known_number_of_helices_in_shape: Some(self.target.nb_helices),
                    known_helix_id_in_shape: None,
                },
                theta_0,
            ))
        }
        ret.sort_by_key(|(_, k)| ordered_float::OrderedFloat::from(*k));

        ret.into_iter()
            .enumerate()
            .map(|(d_id, (mut desc, _))| {
                desc.known_helix_id_in_shape = Some(d_id);
                CurveDescriptor::InterpolatedCurve(desc)
            })
            .collect()
    }

    fn fixed_points(&self) -> &[usize] {
        &[]
    }
}