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

use crate::app_state::design_interactor::presenter::NuclCollection;

pub use revolutions::*;

use super::*;

use ensnano_design::{grid::Grid, Parameters};
use ensnano_interactor::{RevolutionSurfaceSystemDescriptor, RigidBodyConstants};
use mathru::algebra::linear::vector::vector::Vector;
use mathru::analysis::differential_equation::ordinary::{
    solver::runge_kutta::{explicit::fixed::FixedStepper, ExplicitEuler, Kutta3},
    ExplicitODE,
};
use ordered_float::OrderedFloat;
use rand::Rng;
use rand_distr::{Exp, StandardNormal};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use ultraviolet::{Bivec3, Mat3};

mod roller;
pub use roller::{PhysicalSystem, RollInterface, RollPresenter};
mod twister;
pub use twister::{TwistInterface, TwistPresenter, Twister};
mod revolutions;

const MAX_DERIVATIVE_NORM: f32 = 1e4;

macro_rules! bound_derivative {
    ($obj:ident) => {
        if $obj.mag() > MAX_DERIVATIVE_NORM {
            $obj.normalize();
            $obj *= MAX_DERIVATIVE_NORM;
        }
    };
}

#[derive(Debug)]
struct HelixSystem {
    springs: Vec<(RigidNucl, RigidNucl)>,
    free_springs: Vec<(usize, usize)>,
    mixed_springs: Vec<(RigidNucl, usize)>,
    free_nucls: Vec<FreeNucl>,
    free_nucl_position: Vec<Vec3>,
    helices: Vec<RigidHelix>,
    time_span: (f32, f32),
    last_state: Option<Vector<f32>>,
    parameters: Parameters,
    anchors: Vec<(RigidNucl, Vec3)>,
    free_anchors: Vec<(usize, Vec3)>,
    current_time: f32,
    next_time: f32,
    brownian_heap: BinaryHeap<(Reverse<OrderedFloat<f32>>, usize)>,
    rigid_parameters: RigidBodyConstants,
    max_time_step: f32,
}

impl HelixSystem {
    fn get_constants(&self, interval_result: IntervalResult) -> RigidHelixConstants {
        let roll = self.helices.iter().map(|h| h.roll).collect();
        RigidHelixConstants {
            roll,
            free_nucls_ids: interval_result.free_nucl_ids,
            nb_helices: interval_result.intervals.len(),
            parameters: self.parameters.clone(),
            nucl_maps: interval_result.nucl_map,
        }
    }
}

#[derive(Debug)]
struct RigidNucl {
    helix: usize,
    position: isize,
    forward: bool,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
struct FreeNucl {
    helix: Option<usize>,
    position: isize,
    forward: bool,
    old_helix: Option<usize>,
}

impl FreeNucl {
    fn with_helix(nucl: &Nucl, helix: Option<usize>) -> Self {
        Self {
            helix,
            position: nucl.position,
            forward: nucl.forward,
            old_helix: helix.xor(Some(nucl.helix)),
        }
    }
}

#[derive(Debug)]
struct RigidHelix {
    pub roll: f32,
    pub orientation: Rotor3,
    pub inertia_inverse: Mat3,
    pub center_of_mass: Vec3,
    pub center_to_origin: Vec3,
    pub mass: f32,
    pub locked: bool,
    interval: (isize, isize),
}

impl ExplicitODE<f32> for HelixSystem {
    // We read the sytem in the following format. For each grid, we read
    // * 3 f32 for position
    // * 4 f32 for rotation
    // * 3 f32 for linear momentum
    // * 3 f32 for angular momentum

    fn func(&self, _t: &f32, x: &Vector<f32>) -> Vector<f32> {
        let (positions, rotations, linear_momentums, angular_momentums) = self.read_state(x);
        let (forces, torques) = self.forces_and_torques(&positions, &rotations);

        let nb_element = self.helices.len() + self.free_nucls.len();
        let mut ret = Vec::with_capacity(13 * nb_element);
        for i in 0..nb_element {
            if i < self.helices.len() {
                let d_position =
                    linear_momentums[i] / (self.helices[i].height() * self.rigid_parameters.mass);
                ret.push(d_position.x);
                ret.push(d_position.y);
                ret.push(d_position.z);
                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("angular momentum{} {:?}", i, angular_momentums[i]);
                }
                let omega = self.helices[i].inertia_inverse * angular_momentums[i]
                    / self.rigid_parameters.mass;
                let mut d_rotation = 0.5
                    * Rotor3::from_quaternion_array([omega.x, omega.y, omega.z, 0f32])
                    * rotations[i];

                bound_derivative!(d_rotation);

                ret.push(d_rotation.s);
                ret.push(d_rotation.bv.xy);
                ret.push(d_rotation.bv.xz);
                ret.push(d_rotation.bv.yz);

                let mut d_linear_momentum = forces[i]
                    - linear_momentums[i] * self.rigid_parameters.k_friction
                        / (self.helices[i].height() * self.rigid_parameters.mass);

                bound_derivative!(d_linear_momentum);

                ret.push(d_linear_momentum.x);
                ret.push(d_linear_momentum.y);
                ret.push(d_linear_momentum.z);

                let mut d_angular_momentum = torques[i]
                    - angular_momentums[i] * self.rigid_parameters.k_friction
                        / (self.helices[i].height() * self.rigid_parameters.mass);

                bound_derivative!(d_angular_momentum);

                ret.push(d_angular_momentum.x);
                ret.push(d_angular_momentum.y);
                ret.push(d_angular_momentum.z);
            } else {
                let mut d_position = linear_momentums[i] / (self.rigid_parameters.mass / 2.);
                bound_derivative!(d_position);
                ret.push(d_position.x);
                ret.push(d_position.y);
                ret.push(d_position.z);

                let mut d_rotation = Rotor3::from_quaternion_array([0., 0., 0., 0.]);
                bound_derivative!(d_rotation);
                ret.push(d_rotation.s);
                ret.push(d_rotation.bv.xy);
                ret.push(d_rotation.bv.xz);
                ret.push(d_rotation.bv.yz);

                let mut d_linear_momentum = forces[i]
                    - linear_momentums[i] * self.rigid_parameters.k_friction
                        / (self.rigid_parameters.mass / 2.);
                bound_derivative!(d_linear_momentum);

                ret.push(d_linear_momentum.x);
                ret.push(d_linear_momentum.y);
                ret.push(d_linear_momentum.z);

                let mut d_angular_momentum = torques[i]
                    - angular_momentums[i] * self.rigid_parameters.k_friction
                        / (self.rigid_parameters.mass / 2.);
                bound_derivative!(d_angular_momentum);
                ret.push(d_angular_momentum.x);
                ret.push(d_angular_momentum.y);
                ret.push(d_angular_momentum.z);
            }
        }

        Vector::new_row(ret)
    }

    fn time_span(&self) -> (f32, f32) {
        self.time_span
    }

    fn init_cond(&self) -> Vector<f32> {
        if let Some(state) = self.last_state.clone() {
            state
        } else {
            let nb_iter = self.helices.len() + self.free_nucls.len();
            let mut ret = Vec::with_capacity(13 * nb_iter);
            for i in 0..self.helices.len() {
                let position = self.helices[i].center_of_mass();
                ret.push(position.x);
                ret.push(position.y);
                ret.push(position.z);
                let rotation = self.helices[i].orientation;

                ret.push(rotation.s);
                ret.push(rotation.bv.xy);
                ret.push(rotation.bv.xz);
                ret.push(rotation.bv.yz);

                let linear_momentum = Vec3::zero();

                ret.push(linear_momentum.x);
                ret.push(linear_momentum.y);
                ret.push(linear_momentum.z);

                let angular_momentum = Vec3::zero();
                ret.push(angular_momentum.x);
                ret.push(angular_momentum.y);
                ret.push(angular_momentum.z);
            }
            for pos in self.free_nucl_position.iter() {
                ret.push(pos.x);
                ret.push(pos.y);
                ret.push(pos.z);

                let rotation = Rotor3::identity();
                ret.push(rotation.s);
                ret.push(rotation.bv.xy);
                ret.push(rotation.bv.xz);
                ret.push(rotation.bv.yz);

                let linear_momentum = Vec3::zero();

                ret.push(linear_momentum.x);
                ret.push(linear_momentum.y);
                ret.push(linear_momentum.z);

                let angular_momentum = Vec3::zero();
                ret.push(angular_momentum.x);
                ret.push(angular_momentum.y);
                ret.push(angular_momentum.z);
            }
            Vector::new_row(ret)
        }
    }
}

impl HelixSystem {
    fn forces_and_torques(
        &self,
        positions: &[Vec3],
        orientations: &[Rotor3],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let nb_element = self.helices.len() + self.free_nucls.len();
        let mut forces = vec![Vec3::zero(); nb_element];
        let mut torques = vec![Vec3::zero(); nb_element];

        const L0: f32 = 0.7;
        const C_VOLUME: f32 = 2f32;
        let k_anchor = 1000. * self.rigid_parameters.k_spring;

        let point_conversion = |nucl: &RigidNucl| {
            let position = positions[nucl.helix]
                + self.helices[nucl.helix]
                    .center_to_origin
                    .rotated_by(orientations[nucl.helix]);
            let mut helix = Helix::new(position, orientations[nucl.helix]);
            helix.roll(self.helices[nucl.helix].roll);
            helix.space_pos(&self.parameters, nucl.position, nucl.forward)
        };
        let free_nucl_pos = |n: &usize| positions[*n + self.helices.len()];

        for spring in self.springs.iter() {
            let point_0 = point_conversion(&spring.0);
            let point_1 = point_conversion(&spring.1);
            let len = (point_1 - point_0).mag();
            let norm = len - L0;

            // The force applied on point 0
            let force = if len > 1e-5 {
                self.rigid_parameters.k_spring * norm * (point_1 - point_0) / len
            } else {
                Vec3::zero()
            };

            forces[spring.0.helix] += 10. * force;
            forces[spring.1.helix] -= 10. * force;

            let torque0 = (point_0 - positions[spring.0.helix]).cross(force);
            let torque1 = (point_1 - positions[spring.1.helix]).cross(-force);

            torques[spring.0.helix] += torque0;
            torques[spring.1.helix] += torque1;
        }
        for (nucl, free_nucl_id) in self.mixed_springs.iter() {
            let point_0 = point_conversion(nucl);
            let point_1 = free_nucl_pos(free_nucl_id);
            let len = (point_1 - point_0).mag();
            let norm = len - L0;

            // The force applied on point 0
            let force = if len > 1e-5 {
                self.rigid_parameters.k_spring * norm * (point_1 - point_0) / len
            } else {
                Vec3::zero()
            };
            forces[nucl.helix] += 10. * force;
            forces[self.helices.len() + *free_nucl_id] -= 10. * force;

            let torque0 = (point_0 - positions[nucl.helix]).cross(force);

            torques[nucl.helix] += torque0;
        }
        for (id_0, id_1) in self.free_springs.iter() {
            let point_0 = free_nucl_pos(id_0);
            let point_1 = free_nucl_pos(id_1);
            let len = (point_1 - point_0).mag();
            let norm = len - L0;

            // The force applied on point 0
            let force = if len > 1e-5 {
                self.rigid_parameters.k_spring * norm * (point_1 - point_0) / len
            } else {
                Vec3::zero()
            };
            forces[self.helices.len() + *id_0] += 10. * force;
            forces[self.helices.len() + *id_1] -= 10. * force;
        }

        for (nucl, position) in self.anchors.iter() {
            let point_0 = point_conversion(&nucl);
            let len = (point_0 - *position).mag();
            let force = if len > 1e-5 {
                self.rigid_parameters.k_spring * k_anchor * -(point_0 - *position)
            } else {
                Vec3::zero()
            };

            forces[nucl.helix] += 10. * force;

            let torque0 = (point_0 - positions[nucl.helix]).cross(force);

            torques[nucl.helix] += torque0;
        }
        for (id, position) in self.free_anchors.iter() {
            let point_0 = free_nucl_pos(id);
            let len = (point_0 - *position).mag();
            let force = if len > 1e-5 {
                self.rigid_parameters.k_spring * k_anchor * -(point_0 - *position)
            } else {
                Vec3::zero()
            };

            forces[self.helices.len() + *id] += 10. * force;
        }
        let segments: Vec<(Vec3, Vec3)> = (0..self.helices.len())
            .map(|n| {
                let position =
                    positions[n] + self.helices[n].center_to_origin.rotated_by(orientations[n]);
                let helix = Helix::new(position, orientations[n]);
                (
                    helix.axis_position(&self.parameters, self.helices[n].interval.0),
                    helix.axis_position(&self.parameters, self.helices[n].interval.1),
                )
            })
            .collect();
        if self.rigid_parameters.volume_exclusion {
            for i in 0..self.helices.len() {
                let (a, b) = segments[i];
                for j in (i + 1)..self.helices.len() {
                    let (c, d) = segments[j];
                    let r = 1.;
                    let (dist, vec, point_a, point_c) = distance_segment(a, b, c, d);
                    if dist < 2. * r {
                        // VOLUME EXCLUSION
                        let norm =
                            C_VOLUME * self.rigid_parameters.k_spring * (2. * r - dist).powi(2);
                        forces[i] += norm * vec;
                        forces[j] += -norm * vec;
                        let torque0 = (point_a - positions[i]).cross(norm * vec);
                        let torque1 = (point_c - positions[j]).cross(-norm * vec);
                        torques[i] += torque0;
                        torques[j] += torque1;
                    }
                }
                for nucl_id in 0..self.free_nucls.len() {
                    let point = free_nucl_pos(&nucl_id);
                    let (dist, vec, _, _) = distance_segment(a, b, point, point);
                    let r = 1.35 / 2.;
                    if dist < 2. * r {
                        let norm =
                            C_VOLUME * self.rigid_parameters.k_spring * (2. * r - dist).powi(2);
                        let norm = norm.min(1e4);
                        forces[self.helices.len() + nucl_id] -= norm * vec;
                    }
                }
            }
        }

        for (h_id, h) in self.helices.iter().enumerate() {
            if h.locked {
                forces[h_id] = Vec3::zero();
                torques[h_id] = Vec3::zero();
            }
        }

        (forces, torques)
    }
}

impl HelixSystem {
    fn read_state(&self, x: &Vector<f32>) -> (Vec<Vec3>, Vec<Rotor3>, Vec<Vec3>, Vec<Vec3>) {
        let mut positions = Vec::with_capacity(self.helices.len() + self.free_nucls.len());
        let mut rotations = Vec::with_capacity(self.helices.len() + self.free_nucls.len());
        let mut linear_momentums = Vec::with_capacity(self.helices.len() + self.free_nucls.len());
        let mut angular_momentums = Vec::with_capacity(self.helices.len() + self.free_nucls.len());
        let mut iterator = x.iter();
        let nb_iter = self.helices.len() + self.free_nucls.len();
        for _ in 0..nb_iter {
            let position = Vec3::new(
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
            );
            let rotation = Rotor3::new(
                *iterator.next().unwrap(),
                Bivec3::new(
                    *iterator.next().unwrap(),
                    *iterator.next().unwrap(),
                    *iterator.next().unwrap(),
                ),
            )
            .normalized();
            let linear_momentum = Vec3::new(
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
            );
            let angular_momentum = Vec3::new(
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
            );
            positions.push(position);
            rotations.push(rotation);
            linear_momentums.push(linear_momentum);
            angular_momentums.push(angular_momentum);
        }
        (positions, rotations, linear_momentums, angular_momentums)
    }

    fn next_time(&mut self) {
        self.current_time = self.next_time;
        if let Some((t, _)) = self.brownian_heap.peek() {
            // t.0 because t is a &Reverse<_>
            if self.rigid_parameters.brownian_motion {
                self.next_time = t.0.into_inner().min(self.current_time + self.max_time_step);
            } else {
                self.next_time = self.current_time + self.max_time_step;
            }
        } else {
            self.next_time = self.current_time + self.max_time_step;
        }
        self.time_span = (0., self.next_time - self.current_time);
    }

    fn brownian_jump(&mut self) {
        let mut rnd = rand::thread_rng();
        if let Some((t, _)) = self.brownian_heap.peek() {
            // t.0 because t is a &Reverse<_>
            if self.next_time < t.0.into_inner() {
                return;
            }
        }
        if let Some((_, nucl_id)) = self.brownian_heap.pop() {
            let gx: f32 = rnd.sample(StandardNormal);
            let gy: f32 = rnd.sample(StandardNormal);
            let gz: f32 = rnd.sample(StandardNormal);
            if let Some(state) = self.last_state.as_mut() {
                let entry = 13 * (self.helices.len() + nucl_id);
                state[entry] += self.rigid_parameters.brownian_amplitude * gx;
                state[entry + 1] += self.rigid_parameters.brownian_amplitude * gy;
                state[entry + 2] += self.rigid_parameters.brownian_amplitude * gz;
            }

            let exp_law = Exp::new(self.rigid_parameters.brownian_rate).unwrap();
            let new_date = rnd.sample(exp_law) + self.next_time;
            self.brownian_heap.push((Reverse(new_date.into()), nucl_id));
        }
    }

    fn update_parameters(&mut self, parameters: RigidBodyConstants) {
        self.rigid_parameters = parameters;
        self.brownian_heap.clear();
        let mut rnd = rand::thread_rng();
        let exp_law = Exp::new(self.rigid_parameters.brownian_rate).unwrap();
        for i in 0..self.free_nucls.len() {
            if !self.free_anchors.iter().any(|(x, _)| *x == i) {
                let t = rnd.sample(exp_law) + self.next_time;
                self.brownian_heap.push((Reverse(t.into()), i));
            }
        }
    }

    fn shake_nucl(&mut self, nucl: ShakeTarget) {
        let mut rnd = rand::thread_rng();
        let gx: f32 = rnd.sample(StandardNormal);
        let gy: f32 = rnd.sample(StandardNormal);
        let gz: f32 = rnd.sample(StandardNormal);
        let entry = match nucl {
            ShakeTarget::Helix(h_id) => 13 * h_id,
            ShakeTarget::FreeNucl(n) => 13 * (self.helices.len() + n),
        };
        if let Some(state) = self.last_state.as_mut() {
            state[entry] += 10. * self.rigid_parameters.brownian_amplitude * gx;
            state[entry + 1] += 10. * self.rigid_parameters.brownian_amplitude * gy;
            state[entry + 2] += 10. * self.rigid_parameters.brownian_amplitude * gz;
            if let ShakeTarget::Helix(_) = nucl {
                let delta_roll =
                    rnd.gen::<f32>() * 2. * std::f32::consts::PI - std::f32::consts::PI;
                let mut iterator = state.iter().skip(entry + 3);
                let rotation = Rotor3::new(
                    *iterator.next().unwrap(),
                    Bivec3::new(
                        *iterator.next().unwrap(),
                        *iterator.next().unwrap(),
                        *iterator.next().unwrap(),
                    ),
                )
                .normalized();
                let rotation = rotation * Rotor3::from_rotation_yz(delta_roll);
                let mut iterator = state.iter_mut().skip(entry + 3);
                *iterator.next().unwrap() = rotation.s;
                *iterator.next().unwrap() = rotation.bv.xy;
                *iterator.next().unwrap() = rotation.bv.xz;
                *iterator.next().unwrap() = rotation.bv.yz;
            }
        }
    }
}

impl RigidHelix {
    fn new_from_grid(
        y_pos: f32,
        z_pos: f32,
        x_min: f32,
        x_max: f32,
        roll: f32,
        orientation: Rotor3,
        interval: (isize, isize),
    ) -> RigidHelix {
        Self {
            roll,
            orientation,
            center_of_mass: Vec3::new((x_min + x_max) / 2., y_pos, z_pos),
            center_to_origin: -(x_min + x_max) / 2. * Vec3::unit_x(),
            mass: x_max - x_min,
            inertia_inverse: inertia_helix(x_max - x_min, 1.).inversed(),
            interval,
            locked: false,
        }
    }

    fn new_from_world(
        y_pos: f32,
        z_pos: f32,
        x_pos: f32,
        delta: Vec3,
        mass: f32,
        roll: f32,
        orientation: Rotor3,
        interval: (isize, isize),
    ) -> RigidHelix {
        Self {
            roll,
            orientation,
            center_of_mass: Vec3::new(x_pos, y_pos, z_pos),
            center_to_origin: delta,
            mass,
            inertia_inverse: inertia_helix(mass, 1.).inversed(),
            interval,
            locked: false,
        }
    }

    fn center_of_mass(&self) -> Vec3 {
        self.center_of_mass
    }

    fn height(&self) -> f32 {
        self.mass
    }
}

#[allow(dead_code)]
pub enum ShakeTarget {
    FreeNucl(usize),
    Helix(usize),
}

/// Return the length of the shortes line between a point of [a, b] and a poin of [c, d]
fn distance_segment(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> (f32, Vec3, Vec3, Vec3) {
    let u = b - a;
    let v = d - c;
    let n = u.cross(v);

    if n.mag() < 1e-5 {
        // the segment are almost parallel
        return ((a - c).mag(), (a - c), (a + b) / 2., (c + d) / 2.);
    }

    // lambda u.norm2() - mu u.dot(v) + ((a - c).dot(u)) = 0
    // mu v.norm2() - lambda u.dot(v) + ((c - a).dot(v)) = 0
    let normalise = u.dot(v) / u.mag_sq();

    // mu (v.norm2() - normalise * u.dot(v)) = (-(c - a).dot(v)) - normalise * ((a - c).dot(u))
    let mut mu =
        (-((c - a).dot(v)) - normalise * ((a - c).dot(u))) / (v.mag_sq() - normalise * u.dot(v));

    let mut lambda = (-((a - c).dot(u)) + mu * u.dot(v)) / (u.mag_sq());

    if 0f32 <= mu && mu <= 1f32 && 0f32 <= lambda && lambda <= 1f32 {
        let vec = (a + u * lambda) - (c + v * mu);
        (vec.mag(), vec, a + u * lambda, c + v * mu)
    } else {
        let mut min_dist = std::f32::INFINITY;
        let mut min_vec = Vec3::zero();
        let mut min_point_a = a;
        let mut min_point_c = c;
        lambda = 0f32;
        mu = -((c - a).dot(v)) / v.mag_sq();
        if 0f32 <= mu && mu <= 1f32 {
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        } else {
            mu = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
            mu = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        }
        lambda = 1f32;
        mu = (-(c - a).dot(v) + u.dot(v)) / v.mag_sq();
        if 0f32 <= mu && mu <= 1f32 {
            min_dist = min_dist.min(((a + u * lambda) - (c + v * mu)).mag());
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        } else {
            mu = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
            mu = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        }
        mu = 0f32;
        lambda = (-((a - c).dot(u)) + mu * u.dot(v)) / (u.mag_sq());
        if 0f32 <= lambda && 1f32 >= lambda {
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        } else {
            lambda = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
            lambda = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        }
        mu = 1f32;
        lambda = (-((a - c).dot(u)) + mu * u.dot(v)) / (u.mag_sq());
        if 0f32 <= lambda && 1f32 >= lambda {
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        } else {
            lambda = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
            lambda = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
                min_point_a = a + u * lambda;
                min_point_c = c + v * mu;
            }
        }
        (min_dist, min_vec, min_point_a, min_point_c)
    }
}

/// Inertia matrix of an helix of axis e_x, radius r, height h with respect to its center of mass.
fn inertia_helix(h: f32, r: f32) -> Mat3 {
    // The mass is proportinal to the height of the cylinder times its radius squared, we assume that all
    // the cylinder that we work with have the same density
    let m = h * r * r;
    let c = m * r * r / 2.;
    let a = m * (r * r / 4. + h * h / 12.);
    Mat3::new(c * Vec3::unit_x(), a * Vec3::unit_y(), a * Vec3::unit_z())
}

fn center_of_mass_helices(helices: &[RigidHelix]) -> Vec3 {
    let mut total_mass = 0f32;
    let mut ret = Vec3::zero();
    for h in helices.iter() {
        ret += h.center_of_mass() * h.height();
        total_mass += h.height();
    }
    ret / total_mass
}

/// The Inertia matrix of a point with respect to the origin
fn inertia_point(point: Vec3) -> Mat3 {
    Mat3::new(
        Vec3::new(
            point.y * point.y + point.z + point.z,
            -point.x * point.y,
            -point.x * point.z,
        ),
        Vec3::new(
            -point.y * point.x,
            point.x * point.x + point.z * point.z,
            -point.y * point.z,
        ),
        Vec3::new(
            -point.z * point.x,
            -point.z * point.y,
            point.x * point.x + point.y * point.y,
        ),
    )
}

fn inertia_helices(helices: &[RigidHelix], center_of_mass: Vec3) -> Mat3 {
    const HELIX_RADIUS: f32 = 1.;
    let mut ret = Mat3::from_scale(0f32);
    for h in helices.iter() {
        let helix_center = h.center_of_mass();
        let inertia = inertia_helix(h.height(), HELIX_RADIUS);
        ret += inertia_point(helix_center - center_of_mass) * h.height() + inertia;
    }
    ret
}

pub(super) struct HelixSystemThread {
    helix_system: HelixSystem,
    /// The interface of the thread. A weak pointer is used so that the thread execution will
    /// immeadiatly stop when the listener is dropped.
    interface: Weak<Mutex<HelixSystemInterface>>,
    constants: Arc<RigidHelixConstants>,
}

#[derive(Default)]
pub struct HelixSystemInterface {
    pub new_state: Option<RigidHelixState>,
    pub(super) nucl_shake: Option<ShakeTarget>,
    pub(super) parameters_update: Option<RigidBodyConstants>,
}

#[derive(Debug, Clone)]
pub struct RigidHelixState {
    positions: Vec<Vec3>,
    orientations: Vec<Rotor3>,
    center_of_mass_from_helix: Vec<Vec3>,
    constants: Arc<RigidHelixConstants>,
}

#[derive(Debug)]
struct RigidHelixConstants {
    nb_helices: usize,
    parameters: Parameters,
    free_nucls_ids: HashMap<FreeNucl, usize>,
    roll: Vec<f32>,
    nucl_maps: HashMap<Nucl, FreeNucl>,
}

impl HelixSystemThread {
    pub(super) fn start_new(
        presenter: &dyn HelixPresenter,
        rigid_parameters: RigidBodyConstants,
        reader: &mut dyn SimulationReader,
    ) -> Result<Arc<Mutex<HelixSystemInterface>>, ErrOperation> {
        let interval_results = read_intervals(presenter)?;
        let helix_system =
            make_flexible_helices_system((0., 1.), rigid_parameters, presenter, &interval_results)?;
        let ret = Arc::new(Mutex::new(HelixSystemInterface::default()));
        let ret_dyn: Arc<Mutex<dyn SimulationInterface>> = ret.clone();
        reader.attach_state(&ret_dyn);
        let helix_system_thread = Self::new(helix_system, &ret, interval_results);
        helix_system_thread.run();
        Ok(ret)
    }

    fn new(
        helix_system: HelixSystem,
        interface: &Arc<Mutex<HelixSystemInterface>>,
        interval_result: IntervalResult,
    ) -> Self {
        let constants = helix_system.get_constants(interval_result);
        Self {
            helix_system,
            interface: Arc::downgrade(interface),
            constants: Arc::new(constants),
        }
    }

    /// Spawn a thread to run the physical simulation.
    fn run(mut self) -> () {
        std::thread::spawn(move || {
            while let Some(interface_ptr) = self.interface.upgrade() {
                let mut interface = interface_ptr.lock().unwrap();
                if let Some(parameters) = interface.parameters_update.take() {
                    self.helix_system.update_parameters(parameters)
                }
                interface.new_state = Some(self.get_state());
                drop(interface);
                self.helix_system.next_time();
                let solver = FixedStepper::new(1e-4f32);
                let method = ExplicitEuler::default();
                if self.helix_system.rigid_parameters.brownian_motion {
                    self.helix_system.brownian_jump();
                }
                let mut interface = interface_ptr.lock().unwrap();
                if let Some(nucl) = interface.nucl_shake.take() {
                    self.helix_system.shake_nucl(nucl)
                }
                drop(interface);
                if let Ok((_, y)) = solver.solve(&self.helix_system, &method) {
                    self.helix_system.last_state = y.last().cloned();
                }
            }
        });
    }

    fn get_state(&self) -> RigidHelixState {
        let state = self.helix_system.init_cond();
        let (positions, orientations, _, _) = self.helix_system.read_state(&state);
        let center_of_mass_from_helix = self
            .helix_system
            .helices
            .iter()
            .map(|h| h.center_to_origin)
            .collect();
        RigidHelixState {
            positions,
            orientations,
            center_of_mass_from_helix,
            constants: self.constants.clone(),
        }
    }
}

#[derive(Clone)]
pub struct GridSystemState {
    positions: Vec<Vec3>,
    orientations: Vec<Rotor3>,
    center_of_mass_from_grid: Vec<Vec3>,
    ids: Vec<GridId>,
}

pub(super) struct GridsSystemThread {
    grid_system: GridsSystem,
    interface: Weak<Mutex<GridSystemInterface>>,
}

#[derive(Default)]
pub(super) struct GridSystemInterface {
    new_state: Option<GridSystemState>,
    pub(super) parameters_update: Option<RigidBodyConstants>,
}

impl GridsSystemThread {
    pub(super) fn start_new(
        presenter: &dyn GridPresenter,
        rigid_parameters: RigidBodyConstants,
        reader: &mut dyn SimulationReader,
    ) -> Result<Arc<Mutex<GridSystemInterface>>, ErrOperation> {
        let grid_system = make_grid_system(presenter, (0., 1.), rigid_parameters)?;
        let ret = Arc::new(Mutex::new(GridSystemInterface::default()));
        let ret_dyn: Arc<Mutex<dyn SimulationInterface>> = ret.clone();
        reader.attach_state(&ret_dyn);
        let grid_system_thread = Self {
            grid_system,
            interface: Arc::downgrade(&ret),
        };
        grid_system_thread.run();
        Ok(ret)
    }

    /// Spawn a thread to run the physical simulation
    fn run(mut self) -> () {
        std::thread::spawn(move || {
            while let Some(interface_ptr) = self.interface.upgrade() {
                if let Some(parameters) = interface_ptr.lock().unwrap().parameters_update.take() {
                    self.grid_system.update_parameters(parameters);
                }
                interface_ptr.lock().unwrap().new_state = Some(self.get_state());
                let solver = FixedStepper::new(1e-4f32);
                let method = Kutta3::default();
                if let Ok((_, y)) = solver.solve(&self.grid_system, &method) {
                    self.grid_system.last_state = y.last().cloned();
                }
            }
        });
    }

    fn get_state(&self) -> GridSystemState {
        let state = self.grid_system.init_cond();
        let (positions, orientations, _, _) = self.grid_system.read_state(&state);
        let ids = self.grid_system.grids.iter().map(|g| g.id).collect();
        let center_of_mass_from_grid = self
            .grid_system
            .grids
            .iter()
            .map(|g| g.center_of_mass_from_grid)
            .collect();
        GridSystemState {
            positions,
            orientations,
            center_of_mass_from_grid,
            ids,
        }
    }
}

fn make_flexible_helices_system(
    time_span: (f32, f32),
    rigid_parameters: RigidBodyConstants,
    presenter: &dyn HelixPresenter,
    interval_results: &IntervalResult,
) -> Result<HelixSystem, ErrOperation> {
    let parameters = presenter
        .get_design()
        .parameters
        .clone()
        .unwrap_or_default();
    let mut rigid_helices = Vec::with_capacity(interval_results.helix_map.len());
    for i in 0..interval_results.helix_map.len() {
        let h_id = interval_results.helix_map[i];
        let interval = interval_results.intervals[i];
        let mut rigid_helix = make_rigid_helix_world_pov_interval(
            presenter.get_design(),
            h_id,
            interval,
            &parameters,
        );
        rigid_helix.locked = presenter
            .get_design()
            .helices
            .get(&h_id)
            .map(|h| h.locked_for_simulations)
            .unwrap_or_default();
        rigid_helices.push(rigid_helix);
    }
    let xovers = presenter.get_xovers_list();
    let mut springs = Vec::with_capacity(xovers.len());
    let mut mixed_springs = Vec::with_capacity(xovers.len());
    let mut free_springs = Vec::with_capacity(xovers.len());
    for (n1, n2) in xovers {
        log::debug!("xover {:?}", (n1, n2));
        let free_nucl1 = interval_results.nucl_map[&n1];
        let free_nucl2 = interval_results.nucl_map[&n2];
        if let Some((h1, h2)) = free_nucl1.helix.zip(free_nucl2.helix) {
            let rigid_1 = RigidNucl {
                helix: h1,
                position: n1.position,
                forward: n1.forward,
            };
            let rigid_2 = RigidNucl {
                helix: h2,
                position: n2.position,
                forward: n2.forward,
            };
            springs.push((rigid_1, rigid_2));
        }
    }
    for (n1, n2) in presenter.get_all_bounds() {
        let free_nucl1 = interval_results.nucl_map[&n1];
        let free_nucl2 = interval_results.nucl_map[&n2];
        if let Some((_, _)) = free_nucl1.helix.zip(free_nucl2.helix) {
            // Do nothing, this case has either been handled in the xover loop
            // or this bound is rigid
        } else if let Some(h1) = free_nucl1.helix {
            let rigid_1 = RigidNucl {
                helix: h1,
                position: n1.position,
                forward: n1.forward,
            };
            let free_id = interval_results.free_nucl_ids[&free_nucl2];
            mixed_springs.push((rigid_1, free_id));
        } else if let Some(h2) = free_nucl2.helix {
            let rigid_2 = RigidNucl {
                helix: h2,
                position: n2.position,
                forward: n2.forward,
            };
            let free_id = interval_results.free_nucl_ids[&free_nucl1];
            mixed_springs.push((rigid_2, free_id));
        } else {
            let free_id_1 = interval_results.free_nucl_ids[&free_nucl1];
            let free_id_2 = interval_results.free_nucl_ids[&free_nucl2];
            free_springs.push((free_id_1, free_id_2));
        }
    }
    let mut anchors = vec![];
    let mut free_anchors = vec![];
    for anchor in presenter.get_design().anchors.iter() {
        if let Some(position) = presenter.get_space_position(anchor) {
            if let Some(free_nucl) = interval_results.nucl_map.get(anchor) {
                if let Some(rigid_helix) = free_nucl.helix {
                    let rigid_nucl = RigidNucl {
                        helix: rigid_helix,
                        position: anchor.position,
                        forward: anchor.forward,
                    };
                    anchors.push((rigid_nucl, position));
                } else if let Some(id) = interval_results.free_nucl_ids.get(free_nucl) {
                    free_anchors.push((*id, position));
                }
            }
        }
    }
    let mut rnd = rand::thread_rng();
    let mut brownian_heap = BinaryHeap::new();
    let exp_law = Exp::new(rigid_parameters.brownian_rate).unwrap();
    for i in 0..interval_results.free_nucls.len() {
        if !free_anchors.iter().any(|(x, _)| *x == i) {
            let t = rnd.sample(exp_law);
            brownian_heap.push((Reverse(t.into()), i));
        }
    }
    Ok(HelixSystem {
        helices: rigid_helices,
        springs,
        mixed_springs,
        free_springs,
        free_nucls: interval_results.free_nucls.clone(),
        free_nucl_position: interval_results.free_nucl_position.clone(),
        last_state: None,
        time_span,
        parameters,
        anchors,
        free_anchors,
        brownian_heap,
        current_time: 0.,
        next_time: 0.,
        rigid_parameters,
        max_time_step: time_span.1,
    })
}

fn make_rigid_helix_world_pov_interval(
    design: &Design,
    h_id: usize,
    interval: (isize, isize),
    parameters: &Parameters,
) -> RigidHelix {
    let (x_min, x_max) = &interval;
    let helix = design.helices.get(&h_id).expect("helix");
    let left = helix.axis_position(parameters, *x_min);
    let right = helix.axis_position(parameters, *x_max);
    let position = (left + right) / 2.;
    let position_delta = -(*x_max as f32 * parameters.z_step + *x_min as f32 * parameters.z_step)
        / 2.
        * Vec3::unit_x();
    RigidHelix::new_from_world(
        position.y,
        position.z,
        position.x,
        position_delta,
        (right - left).mag(),
        helix.roll,
        helix.orientation,
        interval,
    )
}

fn read_intervals(presenter: &dyn HelixPresenter) -> Result<IntervalResult, ErrOperation> {
    // TODO remove pub after testing
    let mut nucl_map = HashMap::new();
    let mut current_helix = None;
    let mut helix_map = Vec::new();
    let mut free_nucls = Vec::new();
    let mut free_nucl_ids = HashMap::new();
    let mut free_nucl_position = Vec::new();
    let mut intervals = Vec::new();
    for s in presenter.get_design().strands.values() {
        for d in s.domains.iter() {
            log::debug!("New dom");
            if let Some(nucl) = d.prime5_end() {
                if !nucl_map.contains_key(&nucl) || !nucl.forward {
                    let starting_doubled = presenter.has_nucl(&nucl.compl());
                    let starting_nucl = nucl.clone();
                    let mut prev_doubled = false;
                    let mut moving_nucl = starting_nucl;
                    let mut starting_helix = if starting_doubled {
                        Some(current_helix.clone())
                    } else {
                        None
                    };
                    while presenter.has_nucl(&moving_nucl) {
                        log::debug!("nucl {:?}", moving_nucl);
                        let doubled = presenter.has_nucl(&moving_nucl.compl());
                        if doubled && nucl.forward {
                            log::debug!("has compl");
                            let helix = if prev_doubled {
                                current_helix.unwrap()
                            } else {
                                helix_map.push(nucl.helix);
                                intervals.push((moving_nucl.position, moving_nucl.position));
                                if let Some(n) = current_helix.as_mut() {
                                    *n += 1;
                                    *n
                                } else {
                                    current_helix = Some(0);
                                    0
                                }
                            };
                            log::debug!("helix {}", helix);
                            nucl_map.insert(
                                moving_nucl,
                                FreeNucl::with_helix(&moving_nucl, Some(helix)),
                            );
                            nucl_map.insert(
                                moving_nucl.compl(),
                                FreeNucl::with_helix(&moving_nucl.compl(), Some(helix)),
                            );
                            intervals[helix].0 = intervals[helix].0.min(moving_nucl.position);
                            intervals[helix].1 = intervals[helix].1.max(moving_nucl.position);
                        } else if !doubled {
                            log::debug!("has not compl");
                            nucl_map.insert(moving_nucl, FreeNucl::with_helix(&moving_nucl, None));
                            free_nucl_ids
                                .insert(FreeNucl::with_helix(&moving_nucl, None), free_nucls.len());
                            free_nucls.push(FreeNucl::with_helix(&moving_nucl, None));
                            let position = presenter
                                .get_space_position(&moving_nucl)
                                .ok_or(ErrOperation::NuclDoesNotExist(moving_nucl))?;
                            free_nucl_position.push(position);
                        }
                        prev_doubled = doubled;
                        moving_nucl = moving_nucl.left();
                    }
                    prev_doubled = starting_doubled;
                    moving_nucl = starting_nucl.right();
                    while presenter.has_nucl(&moving_nucl) {
                        log::debug!("nucl {:?}", moving_nucl);
                        let doubled = presenter.has_nucl(&moving_nucl.compl());
                        if doubled && nucl.forward {
                            log::debug!("has compl");
                            let helix = if prev_doubled {
                                current_helix.unwrap()
                            } else {
                                if let Some(helix) = starting_helix.take() {
                                    if let Some(n) = helix {
                                        n + 1
                                    } else {
                                        0
                                    }
                                } else {
                                    helix_map.push(nucl.helix);
                                    intervals.push((moving_nucl.position, moving_nucl.position));
                                    if let Some(n) = current_helix.as_mut() {
                                        *n += 1;
                                        *n
                                    } else {
                                        current_helix = Some(0);
                                        0
                                    }
                                }
                            };
                            log::debug!("helix {}", helix);
                            intervals[helix].0 = intervals[helix].0.min(moving_nucl.position);
                            intervals[helix].1 = intervals[helix].1.max(moving_nucl.position);
                            nucl_map.insert(
                                moving_nucl,
                                FreeNucl::with_helix(&moving_nucl, Some(helix)),
                            );
                            nucl_map.insert(
                                moving_nucl.compl(),
                                FreeNucl::with_helix(&moving_nucl.compl(), Some(helix)),
                            );
                        } else if !doubled {
                            log::debug!("has not compl");
                            nucl_map.insert(moving_nucl, FreeNucl::with_helix(&moving_nucl, None));
                            free_nucl_ids
                                .insert(FreeNucl::with_helix(&moving_nucl, None), free_nucls.len());
                            free_nucls.push(FreeNucl::with_helix(&moving_nucl, None));
                            let position = presenter
                                .get_space_position(&moving_nucl)
                                .ok_or(ErrOperation::NuclDoesNotExist(moving_nucl))?;
                            free_nucl_position.push(position);
                        }
                        prev_doubled = doubled;
                        moving_nucl = moving_nucl.right();
                    }
                }
            }
        }
    }
    Ok(IntervalResult {
        nucl_map,
        helix_map,
        free_nucl_ids,
        free_nucls,
        intervals,
        free_nucl_position,
    })
}

pub trait HelixPresenter {
    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)>;
    fn get_design(&self) -> &Design;
    fn get_all_bounds(&self) -> Vec<(Nucl, Nucl)>;
    fn get_identifier(&self, nucl: &Nucl) -> Option<u32>;
    fn get_space_position(&self, nucl: &Nucl) -> Option<Vec3>;
    fn has_nucl(&self, nucl: &Nucl) -> bool;
}

#[derive(Debug)]
pub struct IntervalResult {
    nucl_map: HashMap<Nucl, FreeNucl>,
    helix_map: Vec<usize>,
    free_nucls: Vec<FreeNucl>,
    free_nucl_ids: HashMap<FreeNucl, usize>,
    free_nucl_position: Vec<Vec3>,
    intervals: Vec<(isize, isize)>,
}

pub enum SimulationOperation<'pres, 'reader> {
    StartHelices {
        presenter: &'pres dyn HelixPresenter,
        parameters: RigidBodyConstants,
        reader: &'reader mut dyn SimulationReader,
    },
    StartGrids {
        presenter: &'pres dyn GridPresenter,
        parameters: RigidBodyConstants,
        reader: &'reader mut dyn SimulationReader,
    },
    UpdateParameters {
        new_parameters: RigidBodyConstants,
    },
    #[allow(dead_code)]
    Shake(ShakeTarget),
    Stop,
    Reset,
    StartRoll {
        presenter: &'pres dyn RollPresenter,
        reader: &'reader mut dyn SimulationReader,
        target_helices: Option<Vec<usize>>,
    },
    StartTwist {
        grid_id: GridId,
        presenter: &'pres dyn TwistPresenter,
        reader: &'reader mut dyn SimulationReader,
    },
    RevolutionRelaxation {
        system: RevolutionSurfaceSystemDescriptor,
        reader: &'reader mut dyn SimulationReader,
    },
    FinishRelaxation,
}

pub trait SimulationReader {
    fn attach_state(&mut self, state_chanel: &Arc<Mutex<dyn SimulationInterface>>);
}

pub trait SimulationInterface: Send {
    /// Return the state of the design as determined by the current advancement of the simulation
    fn get_simulation_state(&mut self) -> Option<Box<dyn SimulationUpdate>>;
    /// return true if the simulation should still be running. By overriding this methods, some
    /// simulations can implement automatic termination conditions.
    fn still_valid(&self) -> bool {
        true
    }
}

impl SimulationInterface for HelixSystemInterface {
    fn get_simulation_state(&mut self) -> Option<Box<dyn SimulationUpdate>> {
        let s = self.new_state.take()?;
        Some(Box::new(s))
    }
}

impl SimulationUpdate for RigidHelixState {
    fn update_design(&self, _design: &mut Design) {
        ()
        // since update positions is implemented, we do not need to move the helices.
    }

    fn update_positions(
        &self,
        identifier_nucl: &dyn NuclCollection,
        space_position: &mut HashMap<u32, [f32; 3], ahash::RandomState>,
    ) {
        let helices: Vec<Helix> = (0..self.constants.nb_helices)
            .map(|n| {
                let orientation = self.orientations[n].normalized();
                let position =
                    self.positions[n] + self.center_of_mass_from_helix[n].rotated_by(orientation);
                let mut h = Helix::new(position, orientation);
                h.roll(self.constants.roll[n]);
                h
            })
            .collect();
        for (nucl, id) in identifier_nucl.iter_nucls_ids() {
            let free_nucl = self.constants.nucl_maps[nucl];
            if let Some(n) = free_nucl.helix {
                space_position.insert(
                    *id,
                    helices[n]
                        .space_pos(
                            &self.constants.parameters,
                            free_nucl.position,
                            free_nucl.forward,
                        )
                        .into(),
                );
            } else {
                let free_id = self.constants.free_nucls_ids[&free_nucl];
                space_position.insert(
                    *id,
                    self.positions[self.constants.nb_helices + free_id].into(),
                );
            }
        }
    }
}

struct GridsSystem {
    springs: Vec<(ApplicationPoint, ApplicationPoint)>,
    grids: Vec<RigidGrid>,
    time_span: (f32, f32),
    last_state: Option<Vector<f32>>,
    #[allow(dead_code)]
    anchors: Vec<(ApplicationPoint, Vec3)>,
    parameters: RigidBodyConstants,
}

#[derive(Debug)]
struct RigidGrid {
    /// Center of mass of of the grid in world coordinates
    center_of_mass: Vec3,
    /// Center of mass of the grid in the grid coordinates
    center_of_mass_from_grid: Vec3,
    /// Orientation of the grid in the world coordinates
    orientation: Rotor3,
    inertia_inverse: Mat3,
    mass: f32,
    id: GridId,
}

impl RigidGrid {
    pub fn from_helices(
        id: GridId,
        helices: Vec<RigidHelix>,
        position_grid: Vec3,
        orientation: Rotor3,
    ) -> Self {
        // Center of mass in the grid coordinates.
        log::debug!("helices {:?}", helices);
        let center_of_mass = center_of_mass_helices(&helices);

        // Inertia matrix when the orientation is the identity
        let inertia_matrix = inertia_helices(&helices, center_of_mass);
        let inertia_inverse = inertia_matrix.inversed();
        let mass = helices.iter().map(|h| h.height()).sum();
        Self {
            center_of_mass: center_of_mass.rotated_by(orientation) + position_grid,
            center_of_mass_from_grid: center_of_mass,
            inertia_inverse,
            orientation,
            mass,
            id,
        }
    }
}

impl GridsSystem {
    fn forces_and_torques(
        &self,
        positions: &[Vec3],
        orientations: &[Rotor3],
        _volume_exclusion: f32,
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut forces = vec![Vec3::zero(); self.grids.len()];
        let mut torques = vec![Vec3::zero(); self.grids.len()];

        const L0: f32 = 0.7;
        let k_springs = self.parameters.k_spring;

        let point_conversion = |application_point: &ApplicationPoint| {
            let g_id = application_point.grid_id;
            let position = positions[g_id];
            let orientation = orientations[g_id];
            application_point.position_on_grid.rotated_by(orientation) + position
        };

        for spring in self.springs.iter() {
            let point_0 = point_conversion(&spring.0);
            let point_1 = point_conversion(&spring.1);
            let len = (point_1 - point_0).mag();
            //println!("len {}", len);
            let norm = len - L0;

            // The force applied on point 0
            let force = if len > 1e-5 {
                k_springs * norm * (point_1 - point_0) / len
            } else {
                Vec3::zero()
            };

            forces[spring.0.grid_id] += force;
            forces[spring.1.grid_id] -= force;

            let torque0 = (point_0 - positions[spring.0.grid_id]).cross(force);
            let torque1 = (point_1 - positions[spring.1.grid_id]).cross(-force);

            torques[spring.0.grid_id] += torque0;
            torques[spring.1.grid_id] += torque1;
        }
        /*
        for i in 0..self.grids.len() {
            for j in (i + 1)..self.grids.len() {
                let grid_1 = &self.grids[i];
                let grid_2 = &self.grids[j];
                for h1 in grid_1.helices.iter() {
                    let a = Vec3::new(h1.x_min, h1.y_pos, h1.z_pos);
                    let a = a.rotated_by(orientations[i]) + positions[i];
                    let b = Vec3::new(h1.x_max, h1.y_pos, h1.z_pos);
                    let b = b.rotated_by(orientations[i]) + positions[i];
                    for h2 in grid_2.helices.iter() {
                        let c = Vec3::new(h2.x_min, h2.y_pos, h2.z_pos);
                        let c = c.rotated_by(orientations[j]) + positions[j];
                        let d = Vec3::new(h2.x_max, h2.y_pos, h2.z_pos);
                        let d = d.rotated_by(orientations[j]) + positions[j];
                        let r = 2.;
                        let (dist, vec, point_a, point_c) = distance_segment(a, b, c, d);
                        if dist < r {
                            let norm = ((dist - r) / dist).powi(2) / 1. * 1000.;
                            forces[i] += norm * vec;
                            forces[j] += -norm * vec;
                            let torque0 = (point_a - positions[i]).cross(norm * vec);
                            let torque1 = (point_c - positions[j]).cross(-norm * vec);
                            torques[i] += torque0;
                            torques[j] += torque1;
                        }
                    }
                }
            }
        }*/

        (forces, torques)
    }

    fn update_parameters(&mut self, mut parameters: RigidBodyConstants) {
        let friction_multiplier = 1e3;
        let k_spring_multiplier = 1e2;

        parameters.k_friction *= friction_multiplier;
        parameters.k_spring *= k_spring_multiplier;
        self.parameters = parameters;
    }
}

impl ExplicitODE<f32> for GridsSystem {
    // We read the sytem in the following format. For each grid, we read
    // * 3 f32 for position
    // * 4 f32 for rotation
    // * 3 f32 for linear momentum
    // * 3 f32 for angular momentum

    fn func(&self, _t: &f32, x: &Vector<f32>) -> Vector<f32> {
        let (positions, rotations, linear_momentums, angular_momentums) = self.read_state(x);
        let volume_exclusion = 1.;
        let (forces, torques) = self.forces_and_torques(&positions, &rotations, volume_exclusion);

        let mut ret = Vec::with_capacity(13 * self.grids.len());
        for i in 0..self.grids.len() {
            let d_position = linear_momentums[i] / (self.grids[i].mass * self.parameters.mass);
            ret.push(d_position.x);
            ret.push(d_position.y);
            ret.push(d_position.z);
            let omega = self.grids[i].inertia_inverse * angular_momentums[i] / self.parameters.mass;
            let mut d_rotation = 0.5
                * Rotor3::from_quaternion_array([omega.x, omega.y, omega.z, 0f32])
                * rotations[i];
            bound_derivative!(d_rotation);

            ret.push(d_rotation.s);
            ret.push(d_rotation.bv.xy);
            ret.push(d_rotation.bv.xz);
            ret.push(d_rotation.bv.yz);

            let mut d_linear_momentum = forces[i]
                - linear_momentums[i] * self.parameters.k_friction
                    / (self.grids[i].mass * self.parameters.mass);
            bound_derivative!(d_linear_momentum);

            ret.push(d_linear_momentum.x);
            ret.push(d_linear_momentum.y);
            ret.push(d_linear_momentum.z);

            let mut d_angular_momentum = torques[i]
                - angular_momentums[i] * self.parameters.k_friction / (self.parameters.mass);
            bound_derivative!(d_angular_momentum);
            ret.push(d_angular_momentum.x);
            ret.push(d_angular_momentum.y);
            ret.push(d_angular_momentum.z);
        }

        Vector::new_row(ret)
    }

    fn time_span(&self) -> (f32, f32) {
        self.time_span
    }

    fn init_cond(&self) -> Vector<f32> {
        if let Some(state) = self.last_state.clone() {
            state
        } else {
            let mut ret = Vec::with_capacity(13 * self.grids.len());
            for i in 0..self.grids.len() {
                let position = self.grids[i].center_of_mass;
                ret.push(position.x);
                ret.push(position.y);
                ret.push(position.z);
                let rotation = self.grids[i].orientation;

                ret.push(rotation.s);
                ret.push(rotation.bv.xy);
                ret.push(rotation.bv.xz);
                ret.push(rotation.bv.yz);

                let linear_momentum = Vec3::zero();

                ret.push(linear_momentum.x);
                ret.push(linear_momentum.y);
                ret.push(linear_momentum.z);

                let angular_momentum = Vec3::zero();
                ret.push(angular_momentum.x);
                ret.push(angular_momentum.y);
                ret.push(angular_momentum.z);
            }
            Vector::new_row(ret)
        }
    }
}

impl GridsSystem {
    fn read_state(&self, x: &Vector<f32>) -> (Vec<Vec3>, Vec<Rotor3>, Vec<Vec3>, Vec<Vec3>) {
        let mut positions = Vec::with_capacity(self.grids.len());
        let mut rotations = Vec::with_capacity(self.grids.len());
        let mut linear_momentums = Vec::with_capacity(self.grids.len());
        let mut angular_momentums = Vec::with_capacity(self.grids.len());
        let mut iterator = x.iter();
        for _ in 0..self.grids.len() {
            let position = Vec3::new(
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
            );
            let rotation = Rotor3::new(
                *iterator.next().unwrap(),
                Bivec3::new(
                    *iterator.next().unwrap(),
                    *iterator.next().unwrap(),
                    *iterator.next().unwrap(),
                ),
            )
            .normalized();
            let linear_momentum = Vec3::new(
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
            );
            let angular_momentum = Vec3::new(
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
                *iterator.next().unwrap(),
            );
            positions.push(position);
            rotations.push(rotation);
            linear_momentums.push(linear_momentum);
            angular_momentums.push(angular_momentum);
        }
        (positions, rotations, linear_momentums, angular_momentums)
    }
}

#[derive(Debug)]
struct ApplicationPoint {
    grid_id: usize,
    position_on_grid: Vec3,
}

fn make_grid_system(
    presenter: &dyn GridPresenter,
    time_span: (f32, f32),
    rigid_paramaters: RigidBodyConstants,
) -> Result<GridsSystem, ErrOperation> {
    let intervals = presenter.get_design().strands.get_intervals();
    let parameters = presenter
        .get_design()
        .parameters
        .clone()
        .unwrap_or_default();
    let mut selected_grids = HashMap::with_capacity(presenter.get_design().free_grids.len());
    let mut rigid_grids = Vec::with_capacity(presenter.get_design().free_grids.len());
    for g_id in presenter
        .get_design()
        .free_grids
        .keys()
        .cloned()
        .map(FreeGridId::to_grid_id)
    {
        if let Some(rigid_grid) = make_rigid_grid(presenter, g_id, &intervals, &parameters) {
            selected_grids.insert(g_id, rigid_grids.len());
            rigid_grids.push(rigid_grid);
        }
    }
    if rigid_grids.len() == 0 {
        return Err(ErrOperation::NoGrids);
    }
    let xovers = presenter.get_xovers_list();
    let mut springs = Vec::new();
    for (n1, n2) in xovers {
        let h1 = presenter
            .get_design()
            .helices
            .get(&n1.helix)
            .ok_or(ErrOperation::HelixDoesNotExists(n1.helix))?;
        let h2 = presenter
            .get_design()
            .helices
            .get(&n2.helix)
            .ok_or(ErrOperation::HelixDoesNotExists(n2.helix))?;
        let g_id1 = h1.grid_position.map(|gp| gp.grid);
        let g_id2 = h2.grid_position.map(|gp| gp.grid);
        if let Some((g_id1, g_id2)) = g_id1.zip(g_id2) {
            if g_id1 != g_id2 {
                let rigid_id1 = selected_grids.get(&g_id1).cloned();
                let rigid_id2 = selected_grids.get(&g_id2).cloned();
                if let Some((rigid_id1, rigid_id2)) = rigid_id1.zip(rigid_id2) {
                    let grid1 = presenter
                        .get_grid(g_id1)
                        .ok_or(ErrOperation::GridDoesNotExist(g_id1))?;
                    let grid2 = presenter
                        .get_grid(g_id2)
                        .ok_or(ErrOperation::GridDoesNotExist(g_id2))?;
                    let pos1 = (h1.space_pos(&parameters, n1.position, n1.forward)
                        - rigid_grids[rigid_id1].center_of_mass)
                        .rotated_by(grid1.orientation.reversed());
                    let pos2 = (h2.space_pos(&parameters, n2.position, n2.forward)
                        - rigid_grids[rigid_id2].center_of_mass)
                        .rotated_by(grid2.orientation.reversed());
                    let application_point1 = ApplicationPoint {
                        position_on_grid: pos1,
                        grid_id: rigid_id1,
                    };
                    let application_point2 = ApplicationPoint {
                        position_on_grid: pos2,
                        grid_id: rigid_id2,
                    };
                    springs.push((application_point1, application_point2));
                }
            }
        }
    }
    let mut ret = GridsSystem {
        springs,
        grids: rigid_grids,
        time_span,
        last_state: None,
        anchors: vec![],
        parameters: rigid_paramaters.clone(),
    };
    ret.update_parameters(rigid_paramaters);
    Ok(ret)
}

fn make_rigid_grid(
    presenter: &dyn GridPresenter,
    g_id: GridId,
    intervals: &BTreeMap<usize, (isize, isize)>,
    parameters: &Parameters,
) -> Option<RigidGrid> {
    let helices: Vec<usize> = presenter.get_helices_attached_to_grid(g_id)?;
    let grid = presenter.get_grid(g_id)?;
    let mut rigid_helices = Vec::with_capacity(helices.len());
    for h in helices {
        if let Some(rigid_helix) = make_rigid_helix_grid_pov(presenter, h, intervals, parameters) {
            rigid_helices.push(rigid_helix)
        }
    }
    if rigid_helices.len() > 0 {
        Some(RigidGrid::from_helices(
            g_id,
            rigid_helices,
            grid.position,
            grid.orientation,
        ))
    } else {
        None
    }
}

fn make_rigid_helix_grid_pov(
    presenter: &dyn GridPresenter,
    h_id: usize,
    intervals: &BTreeMap<usize, (isize, isize)>,
    parameters: &Parameters,
) -> Option<RigidHelix> {
    let (x_min, x_max) = intervals.get(&h_id)?;
    let helix = presenter.get_design().helices.get(&h_id)?;
    let grid_position = helix.grid_position?;
    let grid = presenter.get_grid(grid_position.grid)?;
    let position = grid.position_helix(grid_position.x, grid_position.y) - grid.position;
    Some(RigidHelix::new_from_grid(
        position.y,
        position.z,
        *x_min as f32 * parameters.z_step,
        *x_max as f32 * parameters.z_step,
        helix.roll,
        helix.orientation,
        (*x_min, *x_max),
    ))
}

pub trait GridPresenter {
    fn get_design(&self) -> &Design;
    fn get_grid(&self, g_id: GridId) -> Option<&Grid>;
    fn get_helices_attached_to_grid(&self, g_id: GridId) -> Option<Vec<usize>>;
    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)>;
}

impl SimulationInterface for GridSystemInterface {
    fn get_simulation_state(&mut self) -> Option<Box<dyn SimulationUpdate>> {
        let s = self.new_state.take()?;
        Some(Box::new(s))
    }
}

impl SimulationUpdate for GridSystemState {
    fn update_design(&self, design: &mut Design) {
        let mut new_grids = design.free_grids.make_mut();
        for i in 0..self.ids.len() {
            let position = self.positions[i];
            let orientation = self.orientations[i].normalized();
            if let Some(grid) = new_grids.get_mut_g_id(&self.ids[i]) {
                grid.position = position - self.center_of_mass_from_grid[i].rotated_by(orientation);
                grid.orientation = orientation;
            }
        }
    }
}
