use std::collections::HashMap;
use super::{Nucl, Helix, Parameters};

const MASS_HELIX: f32 = 2.;
const K_SPRING: f32 = 100.;
const FRICTION: f32 = 100.;

use std::f32::consts::{PI, SQRT_2};

fn angle_aoc2(p: &Parameters) -> f32 {
    2. * PI / p.bases_per_turn
}

fn dist_ac(p: &Parameters) -> f32 {
    (dist_ac2(p) * dist_ac2(p) + p.z_step * p.z_step).sqrt()
}

fn dist_ac2(p: &Parameters) -> f32 {
    SQRT_2 * (1. - angle_aoc2(p).cos()).sqrt() * p.helix_radius
}

fn cross_over_force(
    me: &Helix,
    other: &Helix,
    parameters: &Parameters,
    n_self: isize,
    b_self: bool,
    n_other: isize,
    b_other: bool,
) -> (f32, f32) {
    let nucl_self = me.space_pos(parameters, n_self, b_self);
    let nucl_other = other.space_pos(parameters, n_other, b_other);

    let real_dist = (nucl_self - nucl_other).mag();

    let norm = K_SPRING * (real_dist - dist_ac(parameters));

    let theta_self = me.theta(n_self, b_self, parameters);
    let theta_other = other.theta(n_other, b_other, parameters);

    let vec_self = me.rotate_point([0., -theta_self.sin(), theta_self.cos()].into());
    let vec_other = other.rotate_point([0., -theta_other.sin(), theta_other.cos()].into());

    (
        (0..3)
            .map(|i| norm * vec_self[i] * (nucl_other[i] - nucl_self[i]) / real_dist)
            .sum(),
        (0..3)
            .map(|i| norm * vec_other[i] * (nucl_self[i] - nucl_other[i]) / real_dist)
            .sum(),
    )
}

pub struct RollSystem {
    speed: Vec<f32>,
    acceleration: Vec<f32>,
    time_scale: f32,
    helices: Vec<Helix>,
    xovers: Vec<(Nucl, Nucl)>,
    parameters: Parameters,
}


impl RollSystem {
    /// Create a system from a design, the system will adjust the helices of the design.
    pub fn from_design(helices: Vec<Helix>, xovers: Vec<(Nucl, Nucl)>, parameters: Parameters) -> Self {
        let speed = vec![0.; helices.len()];
        let acceleration = vec![0.; helices.len()];
        RollSystem {
            speed,
            acceleration,
            time_scale: 1.,
            xovers,
            parameters,
            helices,
        }
    }

    fn update_acceleration(&mut self) {
        let cross_overs = &self.xovers;
        for i in 0..self.acceleration.len() {
            self.acceleration[i] = -self.speed[i] * FRICTION / MASS_HELIX;
        }
        for (n1, n2) in cross_overs.iter() {
            /*if h1 >= h2 {
                continue;
            }*/
            let me = &self.helices[n1.helix];
            let other = &self.helices[n2.helix];
            let (delta_1, delta_2) =
                cross_over_force(me, other, &self.parameters, n1.position, n1.forward, n2.position, n2.forward);
            self.acceleration[n1.helix] += delta_1 / MASS_HELIX;
            self.acceleration[n2.helix] += delta_2 / MASS_HELIX;
        }
    }

    fn update_speed(&mut self, dt: f32) {
        for i in 0..self.speed.len() {
            self.speed[i] += dt * self.acceleration[i];
        }
    }

    fn update_rolls(&self, dt: f32) {
        for i in 0..self.speed.len() {
            self.helices[i].roll += self.speed[i] * dt;
        }
    }

    /// Adjuste the helices of the design, do not show intermediate steps
    pub fn solve(&mut self, dt: f32) {
        let mut nb_step = 0;
        let mut done = false;
        while !done && nb_step < 10000 {
            self.update_rolls(design, dt);
            self.update_speed(dt);
            self.update_acceleration(design);
            println!("acceleration {:?}", self.acceleration);
            done = self.acceleration.iter().map(|x| x.abs()).sum::<f64>() < 1e-8;
            nb_step += 1;
        }
    }

    /// Do one step of simulation with time step dt
    pub fn solve_one_step(&mut self, design: &mut Design, lr: f64) -> f64 {
        self.time_scale = 1.;
        self.update_acceleration(design);
        let grad = self.acceleration.iter().map(|x| x.abs()).sum::<f64>();
        let dt = lr * self.time_scale;
        self.update_speed(dt);
        self.update_rolls(design, dt);
        grad
    }
}

