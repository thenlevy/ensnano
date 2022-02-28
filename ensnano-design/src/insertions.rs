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
use std::f32::consts::{PI, TAU};

const EPSILON_DESC: f32 = 0.05;

struct InsertionDescriptor {
    edge: InsertionEdge,
    nb_nucl: usize,
}

struct CircleArc {
    center: Vec3,
    up: Vec3,
    right: Vec3,
    radius: f32,
    start_angle: f32,
    bigger_than_half_circle: bool,
}

impl CircleArc {
    fn position(&self, t: f32) -> Vec3 {
        let angle = if self.bigger_than_half_circle {
            (PI - self.start_angle) * (1. - t) + t * (-PI + self.start_angle)
        } else {
            (self.start_angle) * (1. - t) - t * self.start_angle
        };
        self.center + self.radius * (self.up * angle.cos() - self.right * angle.sin())
    }
}

impl InsertionDescriptor {
    fn source_pos(&self) -> Vec3 {
        self.edge.prime_5.position
    }

    fn dest_pos(&self) -> Vec3 {
        self.edge.prime_3.position
    }

    fn get_circle(&self, parameters: &Parameters) -> Option<CircleArc> {
        let bisector_origin = (self.edge.prime_5.position + self.edge.prime_3.position) / 2.;
        let mean_of_up_vecs = (self.edge.prime_5.up_vec + self.edge.prime_3.up_vec) / 2.;
        if mean_of_up_vecs.mag() < 1e-3 {
            None
        } else {
            let edge_direction = self.dest_pos() - self.source_pos();
            let bisector_direction = (mean_of_up_vecs.normalized()
                - edge_direction
                    * (mean_of_up_vecs
                        .normalized()
                        .dot(edge_direction.normalized())))
            .normalized();
            let objective_len = parameters.dist_ac() * self.nb_nucl as f32;
            if objective_len < edge_direction.mag() {
                None
            } else {
                let d = edge_direction.mag() / 2.;
                let (mut a, mut b, increasing) = if objective_len > PI * edge_direction.mag() {
                    let a = 0.0;
                    let b = ((2. * objective_len).powi(2) - d.powi(2)).sqrt();
                    (a, 2. * b, true)
                } else {
                    let a = 0.0;
                    let b = 10. * d;
                    if cord_length(a, b, false, self.nb_nucl) > parameters.dist_ac() {
                        // the objective_len is very close to the length of the straight line
                        // between the to exremities
                        return None;
                    }
                    (a, b, false)
                };
                let mut c = (b + a) / 2.;
                while b - a > 1e-3 {
                    if (cord_length(d, c, increasing, self.nb_nucl) > parameters.dist_ac())
                        == increasing
                    {
                        // decrease the length
                        b = c;
                    } else {
                        // increase the length
                        a = c;
                    }
                    c = (b + a) / 2.;
                }
                let center = if increasing {
                    bisector_origin + bisector_direction * c
                } else {
                    bisector_origin - bisector_direction * c
                };
                let start_angle = (d / c).atan();
                Some(CircleArc {
                    center,
                    up: bisector_direction,
                    right: edge_direction.normalized(),
                    radius: (center - self.source_pos()).mag(),
                    start_angle,
                    bigger_than_half_circle: increasing,
                })
            }
        }
    }
}

fn cord_length(d: f32, h: f32, increasing: bool, nb_nucl: usize) -> f32 {
    let r = (d * d + h * h).sqrt();
    let total_angle = if increasing {
        TAU - 2. * (d / h).atan()
    } else {
        2. * (d / h).atan()
    };
    let small_angle = total_angle / (nb_nucl as f32 + 1.);
    2. * r * (small_angle / 2.).sin()
}
struct InsertionEdge {
    prime_5: InsertionEnd,
    prime_3: InsertionEnd,
}

struct InsertionEnd {
    position: Vec3,
    up_vec: Vec3,
}

impl InsertionDescriptor {
    fn is_up_to_date(&self, other: &Self) -> bool {
        self.nb_nucl == other.nb_nucl
            && (self.edge.prime_5.position - other.edge.prime_5.position).mag() < EPSILON_DESC
            && (self.edge.prime_3.position - other.edge.prime_3.position).mag() < EPSILON_DESC
    }
}

pub struct InstanciatedInsertion {
    descriptor: InsertionDescriptor,
    instanciation: Vec<Vec3>,
}

impl InstanciatedInsertion {
    pub fn pos(&self) -> &[Vec3] {
        self.instanciation.as_slice()
    }
}

const NB_STEP: usize = 1000;
const DT_STEP: f32 = 1e-2;
const K_SPRING: f32 = 1.0;
const FRICTION: f32 = 0.1;
const MASS_NUCL: f32 = 1.0;

impl InsertionDescriptor {
    fn instanciate(&self, parameters: &Parameters) -> Vec<Vec3> {
        let mut rnd = rand::thread_rng();
        let mut ret = Vec::with_capacity(self.nb_nucl);
        let len_0 = parameters.dist_ac();

        let circle_arc = self.get_circle(parameters);
        for i in 0..self.nb_nucl {
            let gx: f32 = rnd.sample(StandardNormal);
            let gy: f32 = rnd.sample(StandardNormal);
            let gz: f32 = rnd.sample(StandardNormal);
            let rand_vec = Vec3::new(gx, gy, gz) * parameters.dist_ac() / 3f32.sqrt() / 10.0;
            let t = ((i + 1) as f32) / ((self.nb_nucl + 1) as f32);
            let initial_pos = if let Some(arc) = circle_arc.as_ref() {
                arc.position(t) + rand_vec
            } else {
                self.dest_pos() * t + self.source_pos() * (1. - t) + rand_vec
            };
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
                if a_id == 0 {
                    let force = K_SPRING
                        * (*a - self.source_pos())
                        * ((*a - self.source_pos()).mag() - len_0);
                    forces[a_id] -= force;
                }
                if b_id == self.nb_nucl - 1 {
                    let force =
                        K_SPRING * (self.dest_pos() - *b) * ((self.dest_pos() - *b).mag() - len_0);
                    forces[b_id] += force;
                }
            }

            for (a_id, speed_a) in speed.iter_mut().enumerate() {
                *speed_a += DT_STEP * forces[a_id] / MASS_NUCL
            }

            for (a_id, pos_a) in ret.iter_mut().enumerate() {
                *pos_a += speed[a_id] * DT_STEP
            }
        }

        ret
    }
}

impl Strand {
    pub fn update_insertions(&mut self, helices: &dyn HelixCollection, parameters: &Parameters) {
        let mut to_be_updated = Vec::new();
        let nb_domain = self.domains.len();
        for (d_prev, ((d_id, d), d_next)) in self.domains.iter().cycle().skip(nb_domain - 1).zip(
            self.domains
                .iter()
                .enumerate()
                .zip(self.domains.iter().cycle().skip(1)),
        ) {
            if let Domain::Insertion { .. } = d {
                if let Some((prime_5, prime_3)) = d_prev.prime3_end().zip(d_next.prime5_end()) {
                    let prime_5 = helices.get(&prime_5.helix).map(|h| {
                        let position = h.space_pos(parameters, prime_5.position, prime_5.forward);
                        let up_vec = position - h.axis_position(parameters, prime_5.position);
                        InsertionEnd { position, up_vec }
                    });
                    let prime_3 = helices.get(&prime_3.helix).map(|h| {
                        let position = h.space_pos(parameters, prime_3.position, prime_3.forward);
                        let up_vec = position - h.axis_position(parameters, prime_3.position);
                        InsertionEnd { position, up_vec }
                    });
                    if let Some((prime_5, prime_3)) = prime_5.zip(prime_3) {
                        to_be_updated.push((d_id, InsertionEdge { prime_5, prime_3 }));
                    } else {
                        log::error!("Could not get space pos for insertion");
                    }
                } else {
                    log::error!("two insertions next to eachother");
                }
            }
        }
        for (d_id, edge) in to_be_updated.into_iter() {
            self.update_insertion(d_id, edge, parameters);
        }
    }

    fn update_insertion(&mut self, d_id: usize, edge: InsertionEdge, parameters: &Parameters) {
        if let Some(Domain::Insertion {
            nb_nucl,
            instanciation,
            ..
        }) = self.domains.get_mut(d_id)
        {
            let descriptor = InsertionDescriptor {
                nb_nucl: *nb_nucl,
                edge,
            };
            let up_to_date = instanciation
                .as_ref()
                .map(|i| i.descriptor.is_up_to_date(&descriptor))
                .unwrap_or(false);
            if !up_to_date {
                *instanciation = Some(Arc::new(InstanciatedInsertion {
                    instanciation: descriptor.instanciate(parameters),
                    descriptor,
                }))
            }
        } else {
            log::error!("Wrong domain id");
        }
    }
}
