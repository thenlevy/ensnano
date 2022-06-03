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

use ensnano_design::grid::{GridDescriptor, GridTypeDescr};
use ensnano_design::{grid::*, Collection, CurveDescriptor, HelixCollection, Parameters, Twist};

use super::roller::{DesignData, RollPresenter, RollSystem};
use super::{Design, Helix, SimulationReader};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

pub trait TwistPresenter: RollPresenter {}

struct TwistSystem {
    current_omega: f64,
    best_omega: f64,
    best_square_error: f64,
}

pub struct Twister {
    data: DesignData,
    system: TwistSystem,
    interface: Weak<Mutex<TwistInterface>>,
    state: TwistState,
}

const NB_ROLL_STEP_PER_TWIST: usize = 500;
const MIN_OMEGA: f64 = -0.2;
const MAX_OMEGA: f64 = 0.2;
const NB_STEP_OMEGA: usize = 300;

#[derive(Clone)]
pub struct TwistState {
    grid_id: GridId,
    helices: HashMap<usize, Helix>,
    grid: GridDescriptor,
}

impl super::SimulationUpdate for TwistState {
    fn update_design(&self, design: &mut Design) {
        let mut new_helices = design.helices.make_mut();
        for (i, h) in self.helices.iter() {
            new_helices.insert(*i, h.clone())
        }

        let mut grids_mut = design.free_grids.make_mut();
        if let Some(grid) =
            FreeGridId::try_from_grid_id(self.grid_id).and_then(|g_id| grids_mut.get_mut(&g_id))
        {
            *grid = self.grid.clone()
        } else {
            log::error!("COULD NOT UPDATE GRID {:?}", self.grid_id)
        }
    }
}

#[derive(Default)]
pub struct TwistInterface {
    pub new_state: Option<TwistState>,
    stabilized: bool,
}

impl Twister {
    pub fn start_new(
        presenter: &dyn TwistPresenter,
        target_grid: GridId,
        reader: &mut dyn SimulationReader,
    ) -> Option<Arc<Mutex<TwistInterface>>> {
        let intervals_map = presenter.get_design().strands.get_intervals();
        let mut helices: Vec<Helix> = Vec::new();
        let mut keys: Vec<usize> = Vec::new();
        for (key, helix) in presenter.get_helices().iter().filter(|(_, h)| {
            h.grid_position
                .filter(|pos| pos.grid == target_grid)
                .is_some()
        }) {
            keys.push(key.clone());
            helices.push(helix.clone());
        }
        let parameters = presenter
            .get_design()
            .parameters
            .clone()
            .unwrap_or_default();
        let mut xovers = presenter.get_xovers_list();
        xovers.retain(|(n1, n2)| keys.contains(&n1.helix) && keys.contains(&n2.helix));
        let mut helix_map = HashMap::new();
        let mut intervals = Vec::with_capacity(helices.len());
        for (n, k) in keys.iter().enumerate() {
            helix_map.insert(*k, n);
            intervals.push(intervals_map.get(k).cloned());
        }
        let system = TwistSystem {
            current_omega: MIN_OMEGA,
            best_omega: MIN_OMEGA,
            best_square_error: f64::INFINITY,
        };

        let data = DesignData {
            helices,
            helix_map,
            xovers,
            parameters,
            intervals,
        };

        let interface = Arc::new(Mutex::new(TwistInterface::default()));
        let interface_dyn: Arc<Mutex<dyn super::SimulationInterface>> = interface.clone();
        reader.attach_state(&interface_dyn);

        let initial_state = if let Some(grid) = FreeGridId::try_from_grid_id(target_grid)
            .and_then(|target_grid| presenter.get_design().free_grids.get(&target_grid))
        {
            TwistState {
                grid_id: target_grid,
                grid: grid.clone(),
                helices: presenter
                    .get_design()
                    .helices
                    .iter()
                    .map(|(k, h)| (k.clone(), h.clone()))
                    .collect(),
            }
        } else {
            log::error!("Could not get grid {:?}", target_grid);
            return None;
        };

        let twister = Self {
            data,
            system,
            state: initial_state,
            interface: Arc::downgrade(&interface),
        };

        twister.run();
        Some(interface)
    }
}

impl TwistState {
    fn set_twist(&mut self, twist: f64, parameters: &Parameters) {
        let omega = match &mut self.grid.grid_type {
            GridTypeDescr::Hyperboloid {
                nb_turn_per_100_nt, ..
            } => {
                *nb_turn_per_100_nt = twist;
                ensnano_design::nb_turn_per_100_nt_to_omega(*nb_turn_per_100_nt, parameters)
            }
            GridTypeDescr::Square { twist: grid_twist } => {
                *grid_twist = Some(twist);
                ensnano_design::twist_to_omega(twist, parameters)
            }
            GridTypeDescr::Honeycomb { twist: grid_twist } => {
                *grid_twist = Some(twist);
                ensnano_design::twist_to_omega(twist, parameters)
            }
        };

        if let Some(new_omega) = omega {
            for h in self.helices.values_mut() {
                if let Some(CurveDescriptor::Twist(Twist { omega, .. })) =
                    h.curve.as_mut().map(Arc::make_mut)
                {
                    *omega = new_omega;
                    // no need to update the curve because the helices here are not used to make
                    // computations
                } else {
                    log::error!("Wrong kind of curve descriptor");
                }
            }
        }
    }
}

impl Twister {
    fn evaluate_twist(&mut self, twist: f64) -> f64 {
        self.data.update_twist(twist);
        let mut roll_system = RollSystem::new(self.data.helices.len(), None, &self.data.helix_map);
        for _ in 0..NB_ROLL_STEP_PER_TWIST {
            roll_system.solve_one_step(&mut self.data, 1e-3);
        }
        self.data.square_xover_constraints()
    }

    fn solve_one_step(&mut self) {
        let err = self.evaluate_twist(self.system.current_omega);
        println!("err = {}", err);
        if err < self.system.best_square_error {
            println!("best omega = {}", self.system.current_omega);
            self.system.best_square_error = err;
            self.system.best_omega = self.system.current_omega;
            self.state
                .set_twist(self.system.best_omega, &self.data.parameters);
        }
        self.system.current_omega += (MAX_OMEGA - MIN_OMEGA) / (NB_STEP_OMEGA as f64);
        println!("current_omega = {}", self.system.current_omega);
    }

    pub fn run(mut self) {
        std::thread::spawn(move || {
            while let Some(interface_ptr) = self.interface.upgrade() {
                self.solve_one_step();
                interface_ptr.lock().unwrap().stabilized = self.system.current_omega >= MAX_OMEGA;
                interface_ptr.lock().unwrap().new_state = Some(self.state.clone());
            }
        });
    }
}

impl super::SimulationInterface for TwistInterface {
    fn get_simulation_state(&mut self) -> Option<Box<dyn crate::app_state::SimulationUpdate>> {
        let s = self.new_state.take()?;
        Some(Box::new(s))
    }

    fn still_valid(&self) -> bool {
        !self.stabilized
    }
}

impl DesignData {
    fn square_xover_constraints(&self) -> f64 {
        use ensnano_design::utils::vec_to_dvec;
        let mut ret = 0.0;
        let len_0 = super::roller::dist_ac(&self.parameters) as f64;
        for (n1, n2) in self.xovers.iter() {
            let hid_1 = self.helix_map.get(&n1.helix).unwrap();
            let hid_2 = self.helix_map.get(&n2.helix).unwrap();
            let helix_1 = &self.helices[*hid_1];
            let helix_2 = &self.helices[*hid_2];

            if self.support_helix_idx(&helix_1).unwrap_or(*hid_1)
                != self.support_helix_idx(&helix_2).unwrap_or(*hid_2)
            {
                let pos_1 =
                    vec_to_dvec(helix_1.space_pos(&self.parameters, n1.position, n1.forward));
                let pos_2 =
                    vec_to_dvec(helix_2.space_pos(&self.parameters, n2.position, n2.forward));

                let len = (pos_1 - pos_2).mag();

                ret += (len - len_0) * (len - len_0);
            }
        }
        ret
    }

    fn support_helix_idx(&self, helix: &Helix) -> Option<usize> {
        helix
            .support_helix
            .as_ref()
            .and_then(|h_id| self.helix_map.get(h_id))
            .cloned()
    }

    fn update_twist(&mut self, twist: f64) {
        for h in self.helices.iter_mut() {
            if let Some(CurveDescriptor::Twist(Twist { omega, .. })) =
                h.curve.as_mut().map(Arc::make_mut)
            {
                *omega = ensnano_design::nb_turn_per_100_nt_to_omega(twist, &self.parameters)
                    .unwrap_or(*omega);
                h.try_update_curve(&self.parameters);
            } else {
                log::error!("Update twist: Wrong kind of curve descriptor");
            }
        }
    }
}
