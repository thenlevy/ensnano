use super::{Helix, Nucl, Parameters};
use std::collections::HashMap;

const MASS_HELIX: f32 = 2.;
const K_SPRING: f32 = 1000.;
const FRICTION: f32 = 100.;

use std::f32::consts::{PI, SQRT_2};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

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
    helix_map: HashMap<usize, usize>,
    xovers: Vec<(Nucl, Nucl)>,
    parameters: Parameters,
    stop: Arc<Mutex<bool>>,
    sender: Arc<Mutex<Option<Sender<Vec<Helix>>>>>,
}

impl RollSystem {
    /// Create a system from a design, the system will adjust the helices of the design.
    pub fn from_design(
        keys: Vec<usize>,
        helices: Vec<Helix>,
        xovers: Vec<(Nucl, Nucl)>,
        parameters: Parameters,
    ) -> Self {
        let speed = vec![0.; helices.len()];
        let acceleration = vec![0.; helices.len()];
        let mut helix_map = HashMap::new();
        for (n, k) in keys.iter().enumerate() {
            helix_map.insert(*k, n);
        }
        RollSystem {
            speed,
            acceleration,
            time_scale: 1.,
            xovers,
            parameters,
            helices,
            helix_map,
            stop: Arc::new(Mutex::new(false)),
            sender: Default::default(),
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
            let h1 = self.helix_map.get(&n1.helix).unwrap();
            let h2 = self.helix_map.get(&n2.helix).unwrap();
            let me = &self.helices[*h1];
            let other = &self.helices[*h2];
            let (delta_1, delta_2) = cross_over_force(
                me,
                other,
                &self.parameters,
                n1.position,
                n1.forward,
                n2.position,
                n2.forward,
            );
            self.acceleration[*h1] += delta_1 / MASS_HELIX;
            self.acceleration[*h2] += delta_2 / MASS_HELIX;
        }
    }

    fn update_speed(&mut self, dt: f32) {
        for i in 0..self.speed.len() {
            self.speed[i] += dt * self.acceleration[i];
        }
    }

    fn update_rolls(&mut self, dt: f32) {
        for i in 0..self.speed.len() {
            self.helices[i].roll(self.speed[i] * dt);
        }
    }

    /// Adjuste the helices of the design, do not show intermediate steps
    pub fn solve(&mut self, dt: f32) {
        let mut nb_step = 0;
        let mut done = false;
        while !done && nb_step < 10000 {
            self.update_rolls(dt);
            self.update_speed(dt);
            self.update_acceleration();
            println!("acceleration {:?}", self.acceleration);
            done = self.acceleration.iter().map(|x| x.abs()).sum::<f32>() < 1e-8;
            nb_step += 1;
        }
    }

    /// Do one step of simulation with time step dt
    pub fn solve_one_step(&mut self, lr: f32) -> f32 {
        self.time_scale = 1.;
        self.update_acceleration();
        let grad = self.acceleration.iter().map(|x| x.abs()).sum();
        let dt = lr * self.time_scale;
        self.update_speed(dt);
        self.update_rolls(dt);
        grad
    }

    pub fn run(mut self) -> (Arc<Mutex<bool>>, Arc<Mutex<Option<Sender<Vec<Helix>>>>>) {
        let stop = self.stop.clone();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            while !*self.stop.lock().unwrap() {
                if let Some(snd) = self.sender.lock().unwrap().take() {
                    snd.send(self.helices.clone()).unwrap();
                }
                self.solve_one_step(1e-5);
            }
        });
        (stop, sender)
    }
}
