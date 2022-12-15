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
use chebyshev_polynomials::ChebyshevPolynomial;

#[derive(Clone)]
pub(super) struct CloseSurfaceTopology {
    nb_segment: usize,
    nb_section_per_segment: usize,
    prev_section: Vec<usize>,
    next_section: Vec<usize>,
    other_spring_end: Vec<usize>,
    target: RootedRevolutionSurface,
    idx_range: Vec<usize>,
    target_scaffold_length: usize,
    interpolator: ChebyshevPolynomial,
}

impl CloseSurfaceTopology {
    pub fn new(desc: RevolutionSurfaceSystemDescriptor) -> Self {
        let nb_segment = 2 * desc.target.rooting_parameters.nb_helix_per_half_section;
        let nb_section_per_segment = desc.simulation_parameters.nb_section_per_segment;
        let total_nb_section = nb_segment * nb_section_per_segment;

        let target = &desc.target;
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

        let interpolator = interpolator_inverse_curvilinear_abscissa(target.curve_2d());

        Self {
            nb_segment,
            nb_section_per_segment,
            prev_section,
            next_section,
            target: target.clone(),
            other_spring_end,
            idx_range,
            target_scaffold_length: desc.scaffold_len_target,
            interpolator,
        }
    }
}

const NB_POINT_INTERPOLATION: usize = 100_000;
const INTERPOLATION_ERROR: f64 = 1e-4;
const T_MAX: f64 = 1.;
fn interpolator_inverse_curvilinear_abscissa(curve: &CurveDescriptor2D) -> ChebyshevPolynomial {
    let mut abscissa = 0.;

    let mut point = curve.point(0.);

    let mut ts = Vec::with_capacity(NB_POINT_INTERPOLATION);
    let mut abscissas = Vec::with_capacity(NB_POINT_INTERPOLATION);
    ts.push(0.);
    abscissas.push(abscissa);
    for n in 1..=NB_POINT_INTERPOLATION {
        let t = T_MAX * n as f64 / NB_POINT_INTERPOLATION as f64;
        let next_point = curve.point(t);
        abscissa += (point - next_point).mag();
        abscissas.push(abscissa);
        point = next_point;
        ts.push(t);
    }

    let perimetter = *abscissas.last().unwrap();

    for x in abscissas.iter_mut() {
        *x /= perimetter;
    }

    log::info!("Interpolating inverse...");
    let abscissa_t = abscissas.iter().cloned().zip(ts.iter().cloned()).collect();
    chebyshev_polynomials::interpolate_points(abscissa_t, INTERPOLATION_ERROR)
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
            let delta_theta = self.target.rooting_parameters.shift_per_turn as f64
                / (self.target.rooting_parameters.nb_helix_per_half_section as f64 * 2.);

            for section_idx in 0..self.nb_section_per_segment {
                let a = section_idx as f64 / self.nb_section_per_segment as f64;

                let theta_section = theta_init + a * delta_theta;
                ret.push(
                    theta_section.div_euclid(1.)
                        + self.interpolator.evaluate(theta_section.rem_euclid(1.)),
                );
            }
        }
        ret
    }

    fn dpos_dtheta(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        self.target.dpos_dtheta(revolution_angle, section_t)
    }

    fn d2pos_dtheta2(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        self.target.d2pos_dtheta2(revolution_angle, section_t)
    }

    fn rescale_radius(&mut self, objective_number_of_nts: usize, actual_number_of_nt: usize) {
        self.target
            .rescale_radius(objective_number_of_nts, actual_number_of_nt);
    }

    fn rescale_section(&mut self, scaling_factor: f64) {
        self.target.rescale_section(scaling_factor)
    }

    fn cloned(&self) -> Box<dyn SpringTopology> {
        Box::new(self.clone())
    }

    fn axis(&self, revolution_angle: f64) -> DVec3 {
        self.target.axis(revolution_angle)
    }

    fn to_curve_descriptor(&self, thetas: Vec<f64>, finished: bool) -> Vec<CurveDescriptor> {
        let mut ret = Vec::new();

        let nb_segment_per_helix = self.nb_segment / self.target.nb_spirals();
        println!("Nb spirals {}", self.target.nb_spirals());
        for i in 0..self.target.nb_spirals() {
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
                if self.target.half_turn_count() % 2 == 1 {
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
            let rem = self.target_scaffold_length % self.target.nb_spirals();

            let target_len = if i >= self.target.nb_spirals() - rem {
                self.target_scaffold_length / self.target.nb_spirals() + 1
            } else {
                self.target_scaffold_length / self.target.nb_spirals()
            };

            let objective_number_of_nts = finished.then_some(target_len);
            ret.push((
                self.target
                    .curve_descriptor(interpolations, objective_number_of_nts),
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

    fn revolution_radius(&self) -> RevolutionSurfaceRadius {
        self.target.get_revolution_radius()
    }

    fn get_frame(&self) -> Similarity3 {
        self.target.get_frame()
    }
}
