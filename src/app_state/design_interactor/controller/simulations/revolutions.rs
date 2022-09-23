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

use ensnano_design::{CurveDescriptor2D, DVec3, Parameters as DNAParameters};

struct RevolutionSurfaceSystem {
    nb_segment: usize,
    nb_section_per_segment: usize,
    target: RevolutionSurface,
    prev_section: Vec<usize>,
    next_section: Vec<usize>,
    dna_parameters: DNAParameters,
}

pub struct RevolutionSurface {
    curve: CurveDescriptor2D,
    revolution_radius: f64,
    curve_scale_factor: f64,
    half_turns_count: isize,
    junction_smoothening: f64,
}

impl RevolutionSurfaceSystem {
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

    fn helix_axis(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        (self.position_section(self.next_section[section_idx], thetas)
            - self.position_section(self.prev_section[section_idx], thetas))
        .normalized()
    }

    fn apply_springs(&self, forces: &mut [DVec3], thetas: &[f64]) {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        for i in 0..total_nb_segment {
            let j = self.next_spring_end(i);
            let pos_i = self.position_section(i, thetas);
            let pos_j = self.position_section(j, thetas);

            let ui = self.helix_axis(i, thetas);
            let uj = self.helix_axis(j, thetas);

            let revolution_angle = self.revolution_angle_section(i);
            let z = self.target.axis(revolution_angle);

            let ri = ((self.dna_parameters.inter_helix_gap as f64) / 2. / ui.dot(z)).abs();
            let rj = ((self.dna_parameters.inter_helix_gap as f64) / 2. / uj.dot(z)).abs();

            let len0_ij = ri + rj;
            let v_ji = pos_i - pos_j;
            let len_ij = v_ji.mag();

            let f_ij = SPRING_STIFFNESS * (1. - len0_ij / len_ij) * v_ji;

            forces[i] -= f_ij;
            forces[j] += f_ij;
        }
    }

    fn apply_torsions(&self, forces: &mut [DVec3], thetas: &[f64]) {
        let total_nb_segment = self.nb_segment * self.nb_section_per_segment;
        for section_idx in 0..total_nb_segment {
            let i = self.prev_section[section_idx];
            let j = section_idx;
            let k = self.next_section[section_idx];

            let pos_i = self.position_section(i, thetas);
            let pos_j = self.position_section(j, thetas);
            let pos_k = self.position_section(k, thetas);

            // TODO...
        }
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

impl RevolutionSurface {
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

    fn axis(&self, revolution_angle: f64) -> DVec3 {
        DVec3 {
            x: -revolution_angle.sin(),
            y: revolution_angle.cos(),
            z: 0.,
        }
    }
}
