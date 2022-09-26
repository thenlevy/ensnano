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
    solver::runge_kutta::{ExplicitEuler, Kutta3},
    ExplicitODE,
};

use ensnano_design::{CurveDescriptor2D, DVec3, Parameters as DNAParameters};

pub struct RevolutionSurfaceSystem {
    nb_segment: usize,
    nb_section_per_segment: usize,
    target: RevolutionSurface,
    prev_section: Vec<usize>,
    next_section: Vec<usize>,
    dna_parameters: DNAParameters,
}

pub struct RevolutionSurfaceSystemDescriptor {
    pub nb_segment: usize,
    pub nb_section_per_segment: usize,
    pub target: RevolutionSurface,
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

        let next_section: Vec<usize> = (0..nb_section_per_segment)
            .cycle()
            .skip(1)
            .take(total_nb_section)
            .zip(0..total_nb_section)
            .map(|(shift, n)| n / nb_section_per_segment + shift)
            .collect();

        let prev_section: Vec<usize> = (0..nb_section_per_segment)
            .cycle()
            .skip(nb_section_per_segment - 1)
            .take(total_nb_section)
            .zip(0..total_nb_section)
            .map(|(shift, n)| n / nb_section_per_segment + shift)
            .collect();

        Self {
            nb_segment: desc.nb_segment,
            nb_section_per_segment: NB_SECTION_PER_SEGMENT,
            prev_section,
            next_section,
            dna_parameters: desc.dna_parameters,
            target: desc.target,
        }
    }

    fn theta_init(&self) -> Vec<f64> {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        let mut ret = Vec::with_capacity(total_nb_segment);

        for segment_idx in 0..self.nb_segment {
            let theta_init = TAU * segment_idx as f64 / self.nb_segment as f64;

            for section_idx in 0..self.nb_section_per_segment {
                let a = section_idx as f64 / self.nb_section_per_segment as f64;
                let b = 1. - a;


            }
        }
    }

    fn next_spring_end(&self, section_idx: usize) -> usize {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        (section_idx + self.nb_section_per_segment) % total_nb_segment
    }

    fn revolution_angle_section(&self, section_idx: usize) -> f64 {
        section_idx as f64 * TAU / (self.nb_section_per_segment as f64)
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

    fn apply_springs(&self, system: &mut RelaxationSystem) {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        for i in 0..total_nb_segment {
            let j = self.next_spring_end(i);
            let pos_i = self.position_section(i, &system.thetas);
            let pos_j = self.position_section(j, &system.thetas);

            let ui = self.helix_axis(i, &system.thetas);
            let uj = self.helix_axis(j, &system.thetas);

            let revolution_angle = self.revolution_angle_section(i);
            let z = self.target.axis(revolution_angle);

            let ri = ((self.dna_parameters.inter_helix_gap as f64) / 2. / ui.dot(z)).abs();
            let rj = ((self.dna_parameters.inter_helix_gap as f64) / 2. / uj.dot(z)).abs();

            let len0_ij = ri + rj;
            let v_ji = pos_i - pos_j;
            let len_ij = v_ji.mag();

            let f_ij = SPRING_STIFFNESS * (1. - len0_ij / len_ij) * v_ji;

            system.forces[i] -= f_ij;
            system.forces[j] += f_ij;
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
}

struct RelaxationSystem {
    thetas: Vec<f64>,
    d_thetas: Vec<f64>,
    second_derivative_thetas: Vec<f64>,
    forces: Vec<DVec3>,
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
        /*
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;

        let mut data: Vec<f64> = (0..total_nb_segment).map(|x| x as f64 / total_nb_segment as f64).collect();
        data.extend(vec![0.; total_nb_segment]);
        Vector::new_row(data)
        */ //TODO
    }

    fn func(&self, _t: &f64, x: &Vector<f64>) -> Vector<f64> {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        let mut system = RelaxationSystem::from_mathru(x.clone(), total_nb_segment);
        self.apply_springs(&mut system);
        self.apply_torsions(&mut system);
        self.apply_forces(&mut system);
        system.to_mathru()
    }

    fn time_span(self: &Self) -> (f64, f64) {
        (0., 1.)
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
        let additional_shift = if self.half_turns_count % 2 == 1 { self.nb_helix_per_half_section / 2 } else { 0 };
        let total_shift = self.shift_per_turn + additional_shift as isize;
        gcd(total_shift, self.nb_helix_per_half_section as isize * 2)
    }
}

impl RevolutionSurface {

    pub fn new(desc: RevolutionSurfaceDescriptor) -> Self {

        let nb_helices = desc.nb_helices();
        let curve_scale_factor = DNAParameters::INTER_CENTER_GAP as f64 / desc.curve.perimeter();

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

        let x_2d = self.revolution_radius
            + self.curve_scale_factor
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
}
