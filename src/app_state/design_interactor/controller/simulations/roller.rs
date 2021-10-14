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
//! This modules defines the `PhysicalSystem` struct that performs a simulation of a physical
//! system on the design.
//! The system consists of linear springs that moves the helices and torsion springs that rotates
//! them. These springs aim at minimizing the difference between the cross-over length and the
//! normal distance between two consectives nucleotides.
use super::{Design, Helix, Nucl, Parameters, SimulationReader};
use std::collections::{BTreeMap, HashMap};

const MASS_HELIX: f32 = 2.;
const K_SPRING: f32 = 1000.;
const FRICTION: f32 = 100.;

use std::f32::consts::{PI, SQRT_2};
use std::sync::{Arc, Mutex, Weak};
use ultraviolet::Vec3;

/// A structure performing physical simulation on a design.
pub struct PhysicalSystem {
    /// The data representing the design on which the simulation is performed
    data: DesignData,
    /// The structure that handles the simulation of the rotation springs.
    roller: RollSystem,
    interface: Weak<Mutex<RollInterface>>,
}

pub trait RollPresenter {
    fn get_helices(&self) -> BTreeMap<usize, Helix>;
    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)>;
    fn get_design(&self) -> &Design;
}

impl PhysicalSystem {
    pub fn start_new(
        presenter: &dyn RollPresenter,
        target_helices: Option<Vec<usize>>,
        reader: &mut dyn SimulationReader,
    ) -> Arc<Mutex<RollInterface>> {
        let intervals_map = presenter.get_design().get_intervals();
        let helices: Vec<Helix> = presenter.get_helices().values().cloned().collect();
        let keys: Vec<usize> = presenter.get_helices().keys().cloned().collect();
        let parameters = presenter
            .get_design()
            .parameters
            .clone()
            .unwrap_or_default();
        let xovers = presenter.get_xovers_list();
        let mut helix_map = HashMap::new();
        let mut intervals = Vec::with_capacity(helices.len());
        for (n, k) in keys.iter().enumerate() {
            helix_map.insert(*k, n);
            intervals.push(intervals_map.get(k).cloned());
        }
        let roller = RollSystem::new(helices.len(), target_helices, &helix_map);
        let data = DesignData {
            helices,
            helix_map,
            xovers,
            parameters,
            intervals,
        };
        let interface = Arc::new(Mutex::new(RollInterface::default()));
        let interface_dyn: Arc<Mutex<dyn super::SimulationInterface>> = interface.clone();
        reader.attach_state(&interface_dyn);

        let system = Self {
            data,
            roller,
            interface: Arc::downgrade(&interface),
        };
        system.run();
        interface
    }

    /// Spawn a thread to run the physical simulation. Return a pair of pointers. One to request the
    /// termination of the simulation and one to fetch the current state of the helices.
    pub fn run(mut self) {
        std::thread::spawn(move || {
            while let Some(interface_ptr) = self.interface.upgrade() {
                let grad = self.roller.solve_one_step(&mut self.data, 1e-3);
                log::trace!("grad {}", grad);
                interface_ptr.lock().unwrap().stabilized = grad < 0.1;
                interface_ptr.lock().unwrap().new_state = Some(self.data.get_simulation_state())
            }
        });
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

    // vec_self is the derivative of the position of self w.r.t. theta
    // postion of self is [0, sin(theta), cos(theta)]
    // so the derivative is [0, cos(theta), -sin(theta)]
    let vec_self = me.rotate_point([0., theta_self.cos(), -theta_self.sin()].into());
    let vec_other = other.rotate_point([0., theta_other.cos(), -theta_other.sin()].into());

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
    must_roll: Vec<f32>,
}

impl RollSystem {
    /// Create a system from a design, the system will adjust the helices of the design.
    pub fn new(
        nb_helices: usize,
        target_helices: Option<Vec<usize>>,
        helix_map: &HashMap<usize, usize>,
    ) -> Self {
        let speed = vec![0.; nb_helices];
        let acceleration = vec![0.; nb_helices];
        let must_roll = if let Some(target) = target_helices {
            let mut ret = vec![0.; nb_helices];
            for t in target.iter() {
                ret[helix_map[t]] = 1.;
            }
            ret
        } else {
            vec![1.; nb_helices]
        };
        Self {
            speed,
            acceleration,
            time_scale: 1.,
            must_roll,
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
            self.acceleration[*h1] += delta_1 / MASS_HELIX * self.must_roll[*h1];
            self.acceleration[*h2] += delta_2 / MASS_HELIX * self.must_roll[*h2];
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
    #[allow(dead_code)]
    pub fn solve(&mut self, data: &mut DesignData, dt: f32) {
        let mut nb_step = 0;
        let mut done = false;
        while !done && nb_step < 10000 {
            self.update_rolls(data, dt);
            self.update_speed(dt);
            self.update_acceleration(data);
            log::trace!("acceleration {:?}", self.acceleration);
            done = self.acceleration.iter().map(|x| x.abs()).sum::<f32>() < 1e-8;
            nb_step += 1;
        }
    }

    /// Do one step of simulation with time step dt
    pub fn solve_one_step(&mut self, data: &mut DesignData, lr: f32) -> f32 {
        self.time_scale = 1.;
        self.update_acceleration(data);
        let grad = self
            .acceleration
            .iter()
            .map(|x| x.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.)
            .max(
                self.speed
                    .iter()
                    .map(|x| x.abs())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.),
            );
        let dt = lr * self.time_scale;
        self.update_speed(dt);
        self.update_rolls(data, dt);
        grad
    }
}

#[allow(dead_code)]
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

pub struct DesignData {
    pub helices: Vec<Helix>,
    pub helix_map: HashMap<usize, usize>,
    pub xovers: Vec<(Nucl, Nucl)>,
    pub parameters: Parameters,
    pub intervals: Vec<Option<(isize, isize)>>,
}

impl DesignData {
    fn get_simulation_state(&self) -> RollState {
        let mut ret = HashMap::new();
        for (k, n) in self.helix_map.iter() {
            ret.insert(*k, self.helices[*n].clone());
        }
        RollState(ret)
    }
}

#[derive(Default)]
pub struct RollInterface {
    pub new_state: Option<RollState>,
    stabilized: bool,
}

impl super::SimulationInterface for RollInterface {
    fn get_simulation_state(&mut self) -> Option<Box<dyn crate::app_state::SimulationUpdate>> {
        let s = self.new_state.take()?;
        Some(Box::new(s))
    }

    fn still_valid(&self) -> bool {
        !self.stabilized
    }
}

pub struct RollState(HashMap<usize, Helix>);

impl super::SimulationUpdate for RollState {
    fn update_design(&self, design: &mut ensnano_design::Design) {
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for (i, h) in self.0.iter() {
            new_helices.insert(*i, Arc::new(h.clone()));
        }
        design.helices = Arc::new(new_helices)
    }
}
