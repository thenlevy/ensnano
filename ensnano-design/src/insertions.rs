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

use rand::Rng;
use rand_distr::StandardNormal;
use std::f32::consts::{SQRT_2, TAU};

struct InsertionDescriptor {
    source: Vec3,
    dest: Vec3,
    nb_nucl: usize,
}

pub struct InstanciatedInsertion {
    descriptor: InsertionDescriptor,
    instanciation: Vec<Vec3>,
}

const NB_STEP: usize = 100;
const DT_STEP: f32 = 1e-2;
const K_SPRING: f32 = 1.0;
const FRICTION: f32 = 0.1;
const MASS_NUCL: f32 = 1.0;

impl InsertionDescriptor {
    fn instanciate(&self, parameters: &Parameters) -> Vec<Vec3> {
        let mut rnd = rand::thread_rng();
        let mut ret = Vec::with_capacity(self.nb_nucl);
        let len_0 = parameters.dist_ac();

        for i in 0..self.nb_nucl {
            let gx: f32 = rnd.sample(StandardNormal);
            let gy: f32 = rnd.sample(StandardNormal);
            let gz: f32 = rnd.sample(StandardNormal);
            let rand_vec = Vec3::new(gx, gy, gz) * parameters.dist_ac() / 3f32.sqrt();
            let t = ((i + 1) as f32) / ((self.nb_nucl + 2) as f32);
            let initial_pos = t * self.source + (1. - t) * self.dest + rand_vec;
            ret.push(initial_pos);
        }

        let mut speed = vec![Vec3::zero(); self.nb_nucl];
        for _ in 0..NB_STEP {
            let mut forces: Vec<Vec3> = speed.iter().map(|s| -*s * FRICTION / MASS_NUCL).collect();

            for ((a_id, a), (b_id, b)) in ret.iter().enumerate().zip(ret.iter().enumerate().skip(1))
            {
                let force = K_SPRING * (*b - *a) * ((*b - *a).mag() - len_0);
                forces[a_id] += force;
                forces[b_id] -= force;
            }

            for (a_id, speed_a) in speed.iter_mut().enumerate() {
                *speed_a += DT_STEP * forces[a_id] / MASS_NUCL
            }

            for (a_id, pos_a) in ret.iter_mut().enumerate() {
                *pos_a *= speed[a_id] * DT_STEP
            }
        }

        ret
    }
}

impl Parameters {
    /// The angle AOC_2 where
    ///
    /// * A is a base on the helix
    /// * B is the base paired to A
    /// * O is the projection of A on the axis of the helix
    /// * C is the 3' neighbour of A
    /// * C_2 is the projection of C in the AOB plane
    fn angle_aoc2(&self) -> f32 {
        TAU / self.bases_per_turn
    }

    /// The distance |AC| where
    ///
    /// * A is a base on the helix
    /// * C is the 3' neighbour of A
    fn dist_ac(&self) -> f32 {
        (self.dist_ac2() * self.dist_ac2() + self.z_step * self.z_step).sqrt()
    }

    /// The distance |AC_2| where
    ///
    /// * A is a base on the helix
    /// * B is the base paired to A
    /// * O is the projection of A on the axis of the helix
    /// * C is the 3' neighbour of A
    /// * C_2 is the projection of C in the AOB plane
    fn dist_ac2(&self) -> f32 {
        SQRT_2 * (1. - self.angle_aoc2().cos()).sqrt() * self.helix_radius
    }
}
