use super::{Helix, Nucl, Parameters};
use std::collections::{BTreeMap, HashMap};

const MASS_HELIX: f32 = 2.;
const K_SPRING: f32 = 1000.;
const FRICTION: f32 = 100.;

use std::f32::consts::{PI, SQRT_2};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use ultraviolet::Vec3;

pub struct PhysicalSystem {
    data: DesignData,
    roller: RollSystem,
    springer: SpringSystem,
    stop: Arc<Mutex<bool>>,
    sender: Arc<Mutex<Option<Sender<Vec<Helix>>>>>,
}

impl PhysicalSystem {
    pub fn from_design(
        keys: Vec<usize>,
        helices: Vec<Helix>,
        xovers: Vec<(Nucl, Nucl)>,
        parameters: Parameters,
        intervals_map: BTreeMap<usize, (isize, isize)>,
    ) -> Self {
        let mut helix_map = HashMap::new();
        let mut intervals = Vec::with_capacity(helices.len());
        for (n, k) in keys.iter().enumerate() {
            helix_map.insert(*k, n);
            intervals.push(intervals_map.get(k).cloned());
        }
        let roller = RollSystem::new(helices.len());
        let springer = SpringSystem::new(helices.len());
        let data = DesignData {
            helices,
            helix_map,
            xovers,
            parameters,
            intervals,
        };

        Self {
            data,
            stop: Arc::new(Mutex::new(false)),
            sender: Default::default(),
            springer,
            roller,
        }
    }

    pub fn run(mut self) -> (Arc<Mutex<bool>>, Arc<Mutex<Option<Sender<Vec<Helix>>>>>) {
        let stop = self.stop.clone();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            while !*self.stop.lock().unwrap() {
                if let Some(snd) = self.sender.lock().unwrap().take() {
                    snd.send(self.data.helices.clone()).unwrap();
                }
                self.roller.solve_one_step(&mut self.data, 1e-3);
                self.springer.solve_one_step(&mut self.data, 1e-3);
            }
        });
        (stop, sender)
    }
}

fn angle_aoc2(p: &Parameters) -> f32 {
    2. * PI / p.bases_per_turn
}

fn dist_ac(p: &Parameters) -> f32 {
    (dist_ac2(p) * dist_ac2(p) + p.z_step * p.z_step).sqrt()
}

fn dist_ac2(p: &Parameters) -> f32 {
    SQRT_2 * (1. - angle_aoc2(p).cos()).sqrt() * p.helix_radius
}

pub(super) fn cross_over_force(
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

struct RollSystem {
    speed: Vec<f32>,
    acceleration: Vec<f32>,
    time_scale: f32,
}

impl RollSystem {
    /// Create a system from a design, the system will adjust the helices of the design.
    pub fn new(nb_helices: usize) -> Self {
        let speed = vec![0.; nb_helices];
        let acceleration = vec![0.; nb_helices];
        Self {
            speed,
            acceleration,
            time_scale: 1.,
        }
    }

    fn update_acceleration(&mut self, data: &DesignData) {
        let cross_overs = &data.xovers;
        for i in 0..self.acceleration.len() {
            self.acceleration[i] = -self.speed[i] * FRICTION / MASS_HELIX;
        }
        for (n1, n2) in cross_overs.iter() {
            /*if h1 >= h2 {
                continue;
            }*/
            let h1 = data.helix_map.get(&n1.helix).unwrap();
            let h2 = data.helix_map.get(&n2.helix).unwrap();
            let me = &data.helices[*h1];
            let other = &data.helices[*h2];
            let (delta_1, delta_2) = cross_over_force(
                me,
                other,
                &data.parameters,
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

    fn update_rolls(&mut self, data: &mut DesignData, dt: f32) {
        for i in 0..self.speed.len() {
            data.helices[i].roll(self.speed[i] * dt);
        }
    }

    /// Adjuste the helices of the design, do not show intermediate steps
    pub fn solve(&mut self, data: &mut DesignData, dt: f32) {
        let mut nb_step = 0;
        let mut done = false;
        while !done && nb_step < 10000 {
            self.update_rolls(data, dt);
            self.update_speed(dt);
            self.update_acceleration(data);
            println!("acceleration {:?}", self.acceleration);
            done = self.acceleration.iter().map(|x| x.abs()).sum::<f32>() < 1e-8;
            nb_step += 1;
        }
    }

    /// Do one step of simulation with time step dt
    pub fn solve_one_step(&mut self, data: &mut DesignData, lr: f32) -> f32 {
        self.time_scale = 1.;
        self.update_acceleration(data);
        let grad = self.acceleration.iter().map(|x| x.abs()).sum();
        let dt = lr * self.time_scale;
        self.update_speed(dt);
        self.update_rolls(data, dt);
        grad
    }
}

fn spring_force(
    me: &Helix,
    other: &Helix,
    parameters: &Parameters,
    n_self: isize,
    b_self: bool,
    n_other: isize,
    b_other: bool,
    time_scale: &mut bool,
) -> (Vec3, Vec3) {
    let nucl_self = me.space_pos(parameters, n_self, b_self);
    let nucl_other = other.space_pos(parameters, n_other, b_other);

    let real_dist = (nucl_self - nucl_other).mag();
    if real_dist > dist_ac(parameters) * 10. {
        *time_scale = true;
    }
    let norm = K_SPRING * (real_dist - dist_ac(parameters)) / real_dist;
    (
        norm * (nucl_other - nucl_self),
        norm * (nucl_self - nucl_other),
    )
}

pub struct SpringSystem {
    speed: Vec<Vec3>,
    acceleration: Vec<Vec3>,
    time_scale: f32,
}

impl SpringSystem {
    /// Create a system from a design, the system will adjust the helices of the design.
    pub fn new(nb_helices: usize) -> Self {
        let speed = vec![Vec3::zero(); nb_helices];
        let acceleration = vec![Vec3::zero(); nb_helices];
        SpringSystem {
            speed,
            acceleration,
            time_scale: 1.,
        }
    }

    fn update_acceleration(&mut self, data: &DesignData) {
        for i in 0..self.acceleration.len() {
            self.acceleration[i] = -self.speed[i] * FRICTION / MASS_HELIX;
        }
        let mut update_scale = false;
        for (n1, n2) in data.xovers.iter() {
            /*if h1 >= h2 {
                continue;
            }*/
            let h1 = data.helix_map.get(&n1.helix).unwrap();
            let h2 = data.helix_map.get(&n2.helix).unwrap();
            let me = &data.helices[*h1];
            let other = &data.helices[*h2];
            let (delta_1, delta_2) = spring_force(
                me,
                other,
                &data.parameters,
                n1.position,
                n1.forward,
                n2.position,
                n2.forward,
                &mut update_scale,
            );
            self.acceleration[*h1 as usize] += delta_1 / MASS_HELIX;
            self.acceleration[*h2 as usize] += delta_2 / MASS_HELIX;
        }
        let nb_helices = data.helices.len();
        let param = &data.parameters;

        let r = 2. * param.helix_radius + param.inter_helix_gap;

        for i in 0..(nb_helices - 1) {
            if data.intervals[i].is_none() {
                continue;
            }
            let a = data.helices[i].axis_position(param, data.intervals[i].expect("interval").0);
            let b = data.helices[i].axis_position(param, data.intervals[i].expect("interval").1);
            for j in (i + 1)..nb_helices {
                if data.intervals[j].is_none() {
                    continue;
                }
                let c =
                    data.helices[j].axis_position(param, data.intervals[j].expect("interval").0);
                let d =
                    data.helices[j].axis_position(param, data.intervals[j].expect("interval").1);
                let (dist, vec) = distance_segment(a, b, c, d);
                if dist < r {
                    let norm = ((dist - r) / dist).powi(4) / MASS_HELIX * 100000.;
                    self.acceleration[i] += norm * vec;
                    self.acceleration[j] += -norm * vec;
                }
            }
        }
        if update_scale {
            self.time_scale = 10.;
        }
    }

    fn update_speed(&mut self, dt: f32) {
        for i in 0..self.speed.len() {
            self.speed[i] += dt * self.acceleration[i];
        }
    }

    fn update_position(&self, data: &mut DesignData, dt: f32) {
        for i in 0..self.speed.len() {
            let delta = self.speed[i] * dt;
            data.helices[i].position.x += delta.x;
            data.helices[i].position.y += delta.y;
            data.helices[i].position.z += delta.z;
        }
    }

    /// Adjuste the helices of the design, do not show intermediate steps
    pub fn solve(&mut self, data: &mut DesignData, dt: f32) {
        let mut nb_step = 0;
        let mut done = false;
        while !done && nb_step < 10000 {
            self.update_position(data, dt);
            self.update_speed(dt);
            self.update_acceleration(data);
            println!("acceleration {:?}", self.acceleration);
            done = self.acceleration.iter().map(|x| x.mag()).sum::<f32>() < 1e-8;
            nb_step += 1;
        }
    }

    /// Do one step of simulation with time step dt
    pub fn solve_one_step(&mut self, data: &mut DesignData, lr: f32) -> f32 {
        self.update_acceleration(data);
        let grad = self.acceleration.iter().map(|x| x.mag()).sum::<f32>();
        let dt = lr * self.time_scale;
        self.update_speed(dt);
        self.update_position(data, dt);
        grad
    }
}

pub struct DesignData {
    pub helices: Vec<Helix>,
    pub helix_map: HashMap<usize, usize>,
    pub xovers: Vec<(Nucl, Nucl)>,
    pub parameters: Parameters,
    pub intervals: Vec<Option<(isize, isize)>>,
}

fn distance_segment(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> (f32, Vec3) {
    let u = b - a;
    let v = d - c;
    let n = u.cross(v);

    if n.mag() < 1e-5 {
        // the segment are almost parallel
        return ((a - c).mag(), (a - c));
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
        (vec.mag(), vec)
    } else {
        let mut min_dist = std::f32::INFINITY;
        let mut min_vec = Vec3::zero();
        lambda = 0f32;
        mu = -((c - a).dot(v)) / v.mag_sq();
        if 0f32 <= mu && mu <= 1f32 {
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
        } else {
            mu = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
            mu = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
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
            }
        } else {
            mu = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
            mu = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
        }
        mu = 0f32;
        lambda = (-((a - c).dot(u)) + mu * u.dot(v)) / (u.mag_sq());
        if 0f32 <= lambda && 1f32 >= lambda {
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
        } else {
            lambda = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
            lambda = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
        }
        mu = 1f32;
        lambda = (-((a - c).dot(u)) + mu * u.dot(v)) / (u.mag_sq());
        if 0f32 <= lambda && 1f32 >= lambda {
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
        } else {
            lambda = 0f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
            lambda = 1f32;
            let vec = (a + u * lambda) - (c + v * mu);
            if min_dist > vec.mag() {
                min_dist = vec.mag();
                min_vec = vec.clone();
            }
        }
        (min_dist, min_vec)
    }
}
