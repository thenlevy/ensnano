use super::*;
use mathru::algebra::linear::vector::vector::Vector;
use mathru::analysis::differential_equation::ordinary::{ExplicitODE, Kutta3};
use ultraviolet::{Bivec3, Mat3, Rotor3, Vec3};

struct GridsSystem {
    springs: Vec<(ApplicationPoint, ApplicationPoint)>,
    grids: Vec<RigidGrid>,
    time_span: (f32, f32),
    last_state: Option<Vector<f32>>,
}

impl GridsSystem {
    fn forces_and_torques(
        &self,
        positions: &[Vec3],
        orientations: &[Rotor3],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut forces = vec![Vec3::zero(); self.grids.len()];
        let mut torques = vec![Vec3::zero(); self.grids.len()];

        const L0: f32 = 0.7;
        const K_SPRING: f32 = 1.;

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
            let norm = len - L0;

            // The force applied on point 0
            let force = K_SPRING * norm * (point_1 - point_0) / len;

            forces[spring.0.grid_id] += force;
            forces[spring.1.grid_id] -= force;

            let torque0 = spring.0.position_on_grid.cross(force);
            let torque1 = spring.1.position_on_grid.cross(-force);

            torques[spring.0.grid_id] += torque0;
            torques[spring.1.grid_id] += torque1;
        }

        (forces, torques)
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
        let (forces, torques) = self.forces_and_torques(&positions, &rotations);

        let mut ret = Vec::with_capacity(13 * self.grids.len());
        for i in 0..self.grids.len() {
            let d_position = linear_momentums[i] / self.grids[i].mass;
            ret.push(d_position.x);
            ret.push(d_position.y);
            ret.push(d_position.z);
            let omega = self.grids[i].inertia_inverse * angular_momentums[i];
            let d_rotation = 0.5 * Rotor3::from_quaternion_array([omega.x, omega.y, omega.z, 0f32]);

            ret.push(d_rotation.s);
            ret.push(d_rotation.bv.xy);
            ret.push(d_rotation.bv.xz);
            ret.push(d_rotation.bv.yz);

            let d_linear_momentum = forces[i] - linear_momentums[i] * 10. / self.grids[i].mass;

            ret.push(d_linear_momentum.x);
            ret.push(d_linear_momentum.y);
            ret.push(d_linear_momentum.z);

            let d_angular_momentum = torques[i] - angular_momentums[i] * 10. / self.grids[i].mass;
            ret.push(d_angular_momentum.x);
            ret.push(d_angular_momentum.y);
            ret.push(d_angular_momentum.z);
        }

        Vector::new_row(ret.len(), ret)
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
            Vector::new_row(ret.len(), ret)
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

#[derive(Debug)]
struct RigidHelix {
    pub y_pos: f32,
    pub z_pos: f32,
    pub x_min: f32,
    pub x_max: f32,
}

impl RigidHelix {
    fn center_of_mass(&self) -> Vec3 {
        Vec3::new((self.x_min + self.x_max) / 2., self.y_pos, self.z_pos)
    }

    fn height(&self) -> f32 {
        self.x_max - self.x_min
    }
}

#[derive(Debug)]
struct RigidGrid {
    center_of_mass: Vec3,
    center_of_mass_from_grid: Vec3,
    orientation: Rotor3,
    inertia_inverse: Mat3,
    mass: f32,
    id: usize,
}

impl RigidGrid {
    pub fn from_helices(
        id: usize,
        helices: Vec<RigidHelix>,
        position_grid: Vec3,
        orientation: Rotor3,
    ) -> Self {
        // Center of mass in the grid coordinates.
        println!("helices {:?}", helices);
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

struct GridsSystemThread {
    grid_system: GridsSystem,
    /// When the wrapped boolean is set to true, stop the simulation perfomed by self.
    stop: Arc<Mutex<bool>>,
    /// When the wrapped option takes the value of some channel, the thread that performs the
    /// simulation sends the last computed state of the system
    sender: Arc<Mutex<Option<Sender<GridSystemState>>>>,
}

impl GridsSystemThread {
    fn new(grid_system: GridsSystem) -> Self {
        Self {
            grid_system,
            stop: Default::default(),
            sender: Default::default(),
        }
    }

    /// Spawn a thread to run the physical simulation. Return a pair of pointers. One to request the
    /// termination of the simulation and one to fetch the current state of the helices.
    fn run(
        mut self,
        computing: Arc<Mutex<bool>>,
    ) -> (
        Arc<Mutex<bool>>,
        Arc<Mutex<Option<Sender<GridSystemState>>>>,
    ) {
        let stop = self.stop.clone();
        let sender = self.sender.clone();
        *computing.lock().unwrap() = true;
        std::thread::spawn(move || {
            while !*self.stop.lock().unwrap() {
                if let Some(snd) = self.sender.lock().unwrap().take() {
                    snd.send(self.get_state()).unwrap();
                }
                let solver = Kutta3::new(1e-4f32);
                if let Ok((_, y)) = solver.solve(&self.grid_system) {
                    self.grid_system.last_state = y.last().cloned();
                }
            }
            *computing.lock().unwrap() = false;
        });
        (stop, sender)
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

#[derive(Clone)]
struct GridSystemState {
    positions: Vec<Vec3>,
    orientations: Vec<Rotor3>,
    center_of_mass_from_grid: Vec<Vec3>,
    ids: Vec<usize>,
}

pub(super) struct RigidBodyPtr {
    stop: Arc<Mutex<bool>>,
    state: Arc<Mutex<Option<Sender<GridSystemState>>>>,
    instant: Instant,
}

impl Data {
    pub fn grid_simulation(&mut self, time_span: (f32, f32)) {
        if let Some(grid_system) = self.make_grid_system(time_span) {
            let solver = Kutta3::new(1e-4f32);
            println!("launching simulation");
            if let Ok((_, y)) = solver.solve(&grid_system) {
                let last_state = y.last().unwrap();
                let (positions, rotations, _, _) = grid_system.read_state(last_state);
                for (i, rigid_grid) in grid_system.grids.iter().enumerate() {
                    let position = positions[i];
                    let orientation = rotations[i].normalized();
                    self.grid_manager.grids[rigid_grid.id].position =
                        position - rigid_grid.center_of_mass_from_grid.rotated_by(orientation);
                    self.grid_manager.grids[rigid_grid.id].orientation = orientation;
                }
                self.grid_manager.update(&mut self.design);
                self.hash_maps_update = true;
                self.update_status = true;
            } else {
                println!("error while solving");
            }
        } else {
            println!("could not make grid system");
        }
    }

    fn make_grid_system(&self, time_span: (f32, f32)) -> Option<GridsSystem> {
        let intervals = self.design.get_intervals();
        let parameters = self.design.parameters.unwrap_or_default();
        let mut selected_grids = HashMap::with_capacity(self.grid_manager.grids.len());
        let mut rigid_grids = Vec::with_capacity(self.grid_manager.grids.len());
        for g_id in 0..self.grid_manager.grids.len() {
            if let Some(rigid_grid) = self.make_rigid_grid(g_id, &intervals, &parameters) {
                selected_grids.insert(g_id, rigid_grids.len());
                rigid_grids.push(rigid_grid);
            }
        }
        if rigid_grids.len() == 0 {
            return None;
        }
        let xovers = self.get_xovers_list();
        let mut springs = Vec::new();
        for (n1, n2) in xovers {
            let h1 = self.design.helices.get(&n1.helix)?;
            let h2 = self.design.helices.get(&n2.helix)?;
            let g_id1 = h1.grid_position.map(|gp| gp.grid);
            let g_id2 = h2.grid_position.map(|gp| gp.grid);
            if let Some((g_id1, g_id2)) = g_id1.zip(g_id2) {
                if g_id1 != g_id2 {
                    let rigid_id1 = selected_grids.get(&g_id1).cloned();
                    let rigid_id2 = selected_grids.get(&g_id2).cloned();
                    if let Some((rigid_id1, rigid_id2)) = rigid_id1.zip(rigid_id2) {
                        let grid1 = &self.grid_manager.grids[g_id1];
                        let grid2 = &self.grid_manager.grids[g_id2];
                        let pos1 = (h1.space_pos(&parameters, n1.position, n1.forward)
                            - grid1.position)
                            .rotated_by(grid1.orientation.reversed());
                        let pos2 = (h2.space_pos(&parameters, n2.position, n2.forward)
                            - grid2.position)
                            .rotated_by(grid2.orientation.reversed());
                        let application_point1 = ApplicationPoint {
                            position_on_grid: pos1
                                - rigid_grids[rigid_id1].center_of_mass_from_grid,
                            grid_id: rigid_id1,
                        };
                        let application_point2 = ApplicationPoint {
                            position_on_grid: pos2
                                - rigid_grids[rigid_id2].center_of_mass_from_grid,
                            grid_id: rigid_id2,
                        };
                        springs.push((application_point1, application_point2));
                    }
                }
            }
        }
        Some(GridsSystem {
            springs,
            grids: rigid_grids,
            time_span,
            last_state: None,
        })
    }

    fn make_rigid_grid(
        &self,
        g_id: usize,
        intervals: &BTreeMap<usize, (isize, isize)>,
        parameters: &Parameters,
    ) -> Option<RigidGrid> {
        let helices: Vec<usize> = self.grids[g_id]
            .read()
            .unwrap()
            .helices()
            .values()
            .cloned()
            .collect();
        let grid = self.grid_manager.grids.get(g_id)?;
        let mut rigid_helices = Vec::with_capacity(helices.len());
        for h in helices {
            if let Some(rigid_helix) = self.make_rigid_helix(h, intervals, parameters) {
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

    fn make_rigid_helix(
        &self,
        h_id: usize,
        intervals: &BTreeMap<usize, (isize, isize)>,
        parameters: &Parameters,
    ) -> Option<RigidHelix> {
        let (x_min, x_max) = intervals.get(&h_id)?;
        let helix = self.design.helices.get(&h_id)?;
        let grid_position = helix.grid_position?;
        let grid = self.grid_manager.grids.get(grid_position.grid)?;
        let position = grid.position_helix(grid_position.x, grid_position.y) - grid.position;
        Some(RigidHelix {
            z_pos: position.x,
            y_pos: position.y,
            x_min: *x_min as f32 * parameters.z_step,
            x_max: *x_max as f32 * parameters.z_step,
        })
    }

    pub(super) fn check_rigid_body(&mut self) {
        if let Some(ptrs) = self.rigid_body_ptr.as_mut() {
            let now = Instant::now();
            if (now - ptrs.instant).as_millis() > 30 {
                let (snd, rcv) = std::sync::mpsc::channel();
                *ptrs.state.lock().unwrap() = Some(snd);
                let state = rcv.recv().unwrap();
                for i in 0..state.ids.len() {
                    let position = state.positions[i];
                    let orientation = state.orientations[i].normalized();
                    self.grid_manager.grids[state.ids[i]].position =
                        position - state.center_of_mass_from_grid[i].rotated_by(orientation);
                    self.grid_manager.grids[state.ids[i]].orientation = orientation;
                }
                self.grid_manager.update(&mut self.design);
                self.hash_maps_update = true;
                self.update_status = true;
                ptrs.instant = now;
                self.hash_maps_update = true;
                self.update_status = true;
            }
        }
    }

    pub fn rigid_body_request(&mut self, request: (f32, f32), computing: Arc<Mutex<bool>>) {
        if self.rigid_body_ptr.is_some() {
            self.stop_rigid_body()
        } else {
            self.start_rigid_body(request, computing)
        }
    }

    fn start_rigid_body(&mut self, request: (f32, f32), computing: Arc<Mutex<bool>>) {
        if let Some(grid_system) = self.make_grid_system(request) {
            let grid_system_thread = GridsSystemThread::new(grid_system);
            let date = Instant::now();
            let (stop, snd) = grid_system_thread.run(computing);
            self.rigid_body_ptr = Some(RigidBodyPtr {
                instant: date,
                stop,
                state: snd,
            });
        }
    }

    fn stop_rigid_body(&mut self) {
        if let Some(rigid_body_ptr) = self.rigid_body_ptr.as_mut() {
            *rigid_body_ptr.stop.lock().unwrap() = true;
        } else {
            println!("design was not performing rigid body simulation");
        }
        self.rigid_body_ptr = None;
    }
}
