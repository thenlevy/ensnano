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

use std::f64::consts::{PI, TAU};

const SPRING_STIFFNESS: f64 = 1.;
const TORSION_STIFFNESS: f64 = 30.;
const FLUID_FRICTION: f64 = 0.8;
const BALL_MASS: f64 = 1.;
const NB_SECTION_PER_SEGMENT: usize = 100;

use mathru::algebra::linear::vector::vector::Vector;
use mathru::analysis::differential_equation::ordinary::{
    solver::runge_kutta::{explicit::fixed::FixedStepper, ExplicitEuler, Heun2},
    ExplicitODE,
};

use ensnano_design::{CurveDescriptor2D, DVec3, Parameters as DNAParameters, CurveDescriptor, InterpolationDescriptor, InterpolatedCurveDescriptor};

pub struct RevolutionSurfaceSystem {
    nb_segment: usize,
    nb_section_per_segment: usize,
    target: RevolutionSurface,
    prev_section: Vec<usize>,
    next_section: Vec<usize>,
    dna_parameters: DNAParameters,
    last_thetas: Option<Vec<f64>>,
}

pub struct RevolutionSurfaceSystemDescriptor {
    pub nb_segment: usize,
    pub nb_section_per_segment: usize,
    pub target: RevolutionSurfaceDescriptor,
    pub dna_parameters: DNAParameters,
}

pub struct RevolutionSurface {
    curve: CurveDescriptor2D,
    revolution_radius: f64,
    nb_helix_per_half_section: usize,
    half_turns_count: isize,
    shift_per_turn: isize,
    junction_smoothening: f64,
    dna_paramters: DNAParameters,
    nb_helices: usize,
    curve_scale_factor: f64,
}

pub struct RevolutionSurfaceDescriptor {
    pub curve: CurveDescriptor2D,
    pub revolution_radius: f64,
    pub nb_helix_per_half_section: usize,
    pub half_turns_count: isize,
    pub shift_per_turn: isize,
    pub junction_smoothening: f64,
    pub dna_paramters: DNAParameters,
}

impl RevolutionSurfaceSystem {
    pub fn new(desc: RevolutionSurfaceSystemDescriptor) -> Self {
        let nb_segment = desc.nb_segment;
        let nb_section_per_segment = NB_SECTION_PER_SEGMENT;
        let total_nb_section = nb_segment * nb_section_per_segment;
        let nb_helices = desc.target.nb_helices();

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

        Self {
            nb_segment: desc.nb_segment,
            nb_section_per_segment: NB_SECTION_PER_SEGMENT,
            prev_section,
            next_section,
            dna_parameters: desc.dna_parameters,
            target,
            last_thetas: None,
        }
    }

    fn one_simulation_step(&mut self, first: &mut bool) -> f64 {
        let total_nb_section = self.nb_segment * self.nb_section_per_segment;
        if *first {
            let mut spring_relaxation_state = SpringRelaxationState::new();
            let mut system = RelaxationSystem {
                thetas: self.thetas_init(),
                forces: vec![DVec3::zero(); total_nb_section],
                d_thetas: vec![0.; total_nb_section],
                second_derivative_thetas: vec![0.; total_nb_section],
            };
            self.apply_springs(&mut system, Some(&mut spring_relaxation_state));
            println!("curve scale {}", self.target.curve_scale_factor);
            self.target.curve_scale_factor /= spring_relaxation_state.avg_ext;
            *first = false;
        }

        let solver = FixedStepper::new(1e-2);
        let method = Heun2::default();

        let mut spring_relaxation_state = SpringRelaxationState::new();
        let avg_ext = if let Some(last_state) = solver
            .solve(self, &method)
            .ok()
            .and_then(|(_, y)| y.last().cloned())
        {
            let mut system = RelaxationSystem::from_mathru(last_state, total_nb_section);
            self.apply_springs(&mut system, Some(&mut spring_relaxation_state));
            self.last_thetas = Some(system.thetas.clone());
            spring_relaxation_state.avg_ext
        } else {
            log::error!("error while solving ODE");
            1.
        };
        //self.target.curve_scale_factor /= avg_ext;
        self.target.curve_scale_factor /=
            (spring_relaxation_state.min_ext + spring_relaxation_state.max_ext) / 2.;
        println!("spring_relax state {:?}", spring_relaxation_state);
        println!("curve scale {}", self.target.curve_scale_factor);
        spring_relaxation_state
            .max_ext
            .max(1. / spring_relaxation_state.min_ext)
    }

    fn thetas_init(&self) -> Vec<f64> {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        let mut ret = Vec::with_capacity(total_nb_segment);

        for segment_idx in 0..self.nb_segment {
            let theta_init = TAU * segment_idx as f64 / self.nb_segment as f64;
            let delta_theta = self.target.shift_per_turn as f64 * TAU
                / (self.target.nb_helix_per_half_section as f64 * 2.);

            for section_idx in 0..self.nb_section_per_segment {
                let a = section_idx as f64 / self.nb_section_per_segment as f64;

                let theta_section = theta_init + a * delta_theta;
                ret.push(theta_section);
            }
        }
        ret
    }

    fn next_spring_end(&self, section_idx: usize) -> usize {
        let total_nb_section = self.nb_segment * self.nb_section_per_segment;
        (section_idx + self.nb_section_per_segment) % total_nb_section
    }

    fn revolution_angle_section(&self, section_idx: usize) -> f64 {
        (section_idx % self.nb_section_per_segment) as f64 * TAU
            / (self.nb_section_per_segment as f64)
    }

    fn position_section(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        let angle = self.revolution_angle_section(section_idx);
        let theta = thetas[section_idx];
        self.target.position(angle, theta)
    }

    fn dpos_dtheta(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        let angle = self.revolution_angle_section(section_idx);
        let theta = thetas[section_idx];
        self.target.dpos_dtheta(angle, theta)
    }

    fn helix_axis(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        (self.position_section(self.next_section[section_idx], thetas)
            - self.position_section(self.prev_section[section_idx], thetas))
        .normalized()
    }

    fn apply_springs(
        &self,
        system: &mut RelaxationSystem,
        mut spring_state: Option<&mut SpringRelaxationState>,
    ) {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        if let Some(state) = spring_state.as_mut() {
            state.avg_ext = 0.;
        }
        for i in 0..total_nb_segment {
            let j = self.next_spring_end(i);
            let pos_i = self.position_section(i, &system.thetas);
            let pos_j = self.position_section(j, &system.thetas);

            let ui = self.helix_axis(i, &system.thetas);
            let uj = self.helix_axis(j, &system.thetas);

            let revolution_angle = self.revolution_angle_section(i);
            let z = self.target.axis(revolution_angle);

            let ri = ((self.dna_parameters.helix_radius as f64
                + (self.dna_parameters.inter_helix_gap as f64) / 2.)
                / ui.dot(z))
            .abs();
            let rj = ((self.dna_parameters.helix_radius as f64
                + (self.dna_parameters.inter_helix_gap as f64) / 2.)
                / uj.dot(z))
            .abs();

            let len0_ij = ri + rj;
            let v_ji = pos_i - pos_j;
            let len_ij = v_ji.mag();

            let f_ij = SPRING_STIFFNESS * (1. - len0_ij / len_ij) * v_ji;

            if let Some(state) = spring_state.as_mut() {
                let ext = len_ij / len0_ij;
                state.min_ext = state.min_ext.min(ext);
                state.max_ext = state.max_ext.max(ext);
                state.avg_ext += ext;
            }

            system.forces[i] -= f_ij;
            system.forces[j] += f_ij;
        }

        if let Some(state) = spring_state.as_mut() {
            state.avg_ext /= total_nb_segment as f64;
        }
    }

    fn apply_torsions(&self, system: &mut RelaxationSystem) {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        for section_idx in 0..total_nb_segment {
            let i = self.prev_section[section_idx];
            let j = section_idx;
            let k = self.next_section[section_idx];

            let pos_i = self.position_section(i, &system.thetas);
            let pos_j = self.position_section(j, &system.thetas);
            let pos_k = self.position_section(k, &system.thetas);

            let u_ij = pos_j - pos_i;
            let u_jk = pos_k - pos_j;
            let v = u_jk - u_ij;
            let f_ijk = TORSION_STIFFNESS * v / v.mag().max(1.);
            system.forces[i] -= f_ijk / 2.;
            system.forces[j] += f_ijk;
            system.forces[k] -= f_ijk / 2.;
        }
    }

    fn apply_forces(&self, system: &mut RelaxationSystem) {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        for section_idx in 0..total_nb_segment {
            let tengent = self.dpos_dtheta(section_idx, &system.thetas);
            let acceleration_without_friction =
                system.forces[section_idx].dot(tengent) / tengent.mag_sq();
            system.second_derivative_thetas[section_idx] += (acceleration_without_friction
                - FLUID_FRICTION * system.d_thetas[section_idx])
                / BALL_MASS;
        }
    }

    fn to_curve_desc(&self) -> Option<Vec<CurveDescriptor>> {
        let thetas = self.last_thetas.clone()?;

        let mut ret = Vec::new();

        let nb_segment_per_helix = self.nb_segment / self.target.nb_helices;
        for i in 0..self.target.nb_helices {
            let mut interpolations = Vec::new();
            let segment_indicies = (0..nb_segment_per_helix).map(|n| (i as isize + (n as isize * self.target.total_shift())).rem_euclid(self.nb_segment as isize));
            for s_idx in segment_indicies {
                let start = s_idx as usize * self.nb_section_per_segment;
                let end = start + self.nb_section_per_segment;
                let thetas = thetas[start..end].to_vec();
                println!("segment {s_idx}, thetas {:?}", thetas);
                let s = (0..self.nb_section_per_segment).map(|x| x as f64 / self.nb_section_per_segment as f64).collect();
                interpolations.push(InterpolationDescriptor::PointsValues { points: s, values: thetas });
            }
            ret.push(CurveDescriptor::InterpolatedCurve(InterpolatedCurveDescriptor {
                curve: self.target.curve.clone(),
                curve_scale_factor: self.target.curve_scale_factor,
                chevyshev_smoothening: self.target.junction_smoothening,
                interpolation: interpolations,
                half_turns_count: self.target.half_turns_count,
                revolution_radius: self.target.revolution_radius,
            }))
        }

        Some(ret)

    }
}

struct RelaxationSystem {
    thetas: Vec<f64>,
    d_thetas: Vec<f64>,
    second_derivative_thetas: Vec<f64>,
    forces: Vec<DVec3>,
}

#[derive(Default, Debug)]
struct SpringRelaxationState {
    min_ext: f64,
    max_ext: f64,
    avg_ext: f64,
}

impl SpringRelaxationState {
    fn new() -> Self {
        Self {
            min_ext: std::f64::INFINITY,
            max_ext: 0.,
            avg_ext: 0.,
        }
    }
}

impl RelaxationSystem {
    fn to_mathru(self) -> Vector<f64> {
        let mut data = self.d_thetas;
        data.extend(self.second_derivative_thetas);
        Vector::new_row(data)
    }

    fn from_mathru(vec: Vector<f64>, nb_section: usize) -> Self {
        let vec = vec.convert_to_vec();
        let thetas = vec[0..nb_section].to_vec();
        let d_thetas = vec[nb_section..].to_vec();

        Self {
            thetas,
            d_thetas,
            second_derivative_thetas: vec![0.; nb_section],
            forces: vec![DVec3::zero(); nb_section],
        }
    }
}

impl ExplicitODE<f64> for RevolutionSurfaceSystem {
    fn init_cond(&self) -> Vector<f64> {
        let total_nb_section = self.nb_segment * self.nb_section_per_segment;

        let mut data = self
            .last_thetas
            .clone()
            .unwrap_or_else(|| self.thetas_init());
        data.extend(vec![0.; total_nb_section]);
        Vector::new_row(data)
    }

    fn func(&self, _t: &f64, x: &Vector<f64>) -> Vector<f64> {
        let total_nb_section = self.nb_segment * self.nb_section_per_segment;
        let mut system = RelaxationSystem::from_mathru(x.clone(), total_nb_section);
        self.apply_springs(&mut system, None);
        self.apply_torsions(&mut system);
        self.apply_forces(&mut system);
        system.to_mathru()
    }

    fn time_span(self: &Self) -> (f64, f64) {
        (0., 5.)
    }
}

/*
 * let q be the total shift and n be the number of sections
 * Helices seen as set of section are class of equivalence for the relation ~
 * where a ~ b iff there exists k1, k2 st a = b  + k1 q + k2 n
 *
 * let d = gcd(q, n). If a ~ b then a = b (mod d)
 *
 * Recp. if a = b (mod d) there exists x y st xq + yn = d
 *
 * a = k (xq + yn) + b
 * so a ~ b
 *
 * So ~ is the relation of equivalence modulo d and has d classes.
 */

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

impl RevolutionSurfaceDescriptor {
    fn nb_helices(&self) -> usize {
        let additional_shift = if self.half_turns_count % 2 == 1 {
            self.nb_helix_per_half_section / 2
        } else {
            0
        };
        let total_shift = self.shift_per_turn + additional_shift as isize;
        gcd(total_shift, self.nb_helix_per_half_section as isize * 2)
    }
}

impl RevolutionSurface {
    pub fn new(desc: RevolutionSurfaceDescriptor) -> Self {
        let nb_helices = desc.nb_helices();
        let curve_scale_factor =
            desc.nb_helix_per_half_section as f64 * 2. * DNAParameters::INTER_CENTER_GAP as f64
                / desc.curve.perimeter();

        Self {
            curve: desc.curve,
            revolution_radius: desc.revolution_radius,
            nb_helices,
            nb_helix_per_half_section: desc.nb_helix_per_half_section,
            shift_per_turn: desc.shift_per_turn,
            dna_paramters: desc.dna_paramters,
            half_turns_count: desc.half_turns_count,
            curve_scale_factor,
            junction_smoothening: desc.junction_smoothening,
        }
    }

    fn position(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        // must be equal to PI * half_turns when revolution_angle = TAU.
        let section_rotation = revolution_angle * (self.half_turns_count as f64) / 2.;

        let section_point = self.curve.point(section_t);

        let x_2d = self.revolution_radius
            + self.curve_scale_factor
                * (section_point.x * section_rotation.cos()
                    - section_rotation.sin() * section_point.y);

        let y_2d = self.curve_scale_factor
            * (section_point.x * section_rotation.sin() + section_rotation.cos() * section_point.y);

        DVec3 {
            x: revolution_angle.cos() * x_2d,
            y: revolution_angle.sin() * x_2d,
            z: y_2d,
        }
    }

    fn dpos_dtheta(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        let section_rotation = revolution_angle * (self.half_turns_count as f64) / 2.;

        let dpos_curve = self.curve.derivative(section_t);

        let x_2d = self.curve_scale_factor
            * (dpos_curve.x * section_rotation.cos() - section_rotation.sin() * dpos_curve.y);

        let y_2d = self.curve_scale_factor
            * (dpos_curve.x * section_rotation.sin() + section_rotation.cos() * dpos_curve.y);

        DVec3 {
            x: revolution_angle.cos() * x_2d,
            y: revolution_angle.sin() * x_2d,
            z: y_2d,
        }
    }

    fn axis(&self, revolution_angle: f64) -> DVec3 {
        DVec3 {
            x: -revolution_angle.sin(),
            y: revolution_angle.cos(),
            z: 0.,
        }
    }

    fn total_shift(&self) -> isize {
        let additional_shift = if self.half_turns_count % 2 == 1 { self.nb_helix_per_half_section } else { 0 };
        self.shift_per_turn + additional_shift as isize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relax_hexagon() {
        let surface_desc = RevolutionSurfaceDescriptor {
            curve: CurveDescriptor2D::Ellipse {
                semi_minor_axis: 1f64.into(),
                semi_major_axis: 2f64.into(),
            },
            half_turns_count: 6,
            revolution_radius: 23.99710394464801,
            junction_smoothening: 0.,
            nb_helix_per_half_section: 7,
            dna_paramters: DNAParameters::GEARY_2014_DNA,
            shift_per_turn: -12,
        };
        let system_desc = RevolutionSurfaceSystemDescriptor {
            nb_section_per_segment: NB_SECTION_PER_SEGMENT,
            dna_parameters: DNAParameters::GEARY_2014_DNA,
            nb_segment: 14,
            target: surface_desc,
        };
        let mut system = RevolutionSurfaceSystem::new(system_desc);
        let mut current_default = std::f64::INFINITY;

        let mut first = true;
        while current_default > 1.05 {
            current_default = system.one_simulation_step(&mut first);
            println!("current default {current_default}")
        }
        let curve_desc = system.to_curve_desc().unwrap();

        for desc in curve_desc {
            let len = desc.compute_length();
            println!("length ~= {:?}", len);
            println!("length ~= {:?} nt", len.map(|l| l/ DNAParameters::GEARY_2014_DNA.z_step as f64));
        }
    }
}
