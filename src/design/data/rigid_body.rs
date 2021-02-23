use super::*;
use mathru::algebra::linear::vector::vector::Vector;
use mathru::analysis::differential_equation::ordinary::{ExplicitODE, Kutta3};
use ultraviolet::{Bivec3, Mat3, Rotor3, Vec3};

#[derive(Debug)]
struct HelixSystem {
    springs: Vec<(RigidNucl, RigidNucl)>,
    helices: Vec<RigidHelix>,
    time_span: (f32, f32),
    last_state: Option<Vector<f32>>,
    parameters: Parameters,
    anchors: Vec<(RigidNucl, Vec3)>,
}

#[derive(Debug)]
struct RigidNucl {
    helix: usize,
    position: isize,
    forward: bool,
}

impl HelixSystem {
    fn forces_and_torques(
        &self,
        positions: &[Vec3],
        orientations: &[Rotor3],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut forces = vec![Vec3::zero(); self.helices.len()];
        let mut torques = vec![Vec3::zero(); self.helices.len()];

        const L0: f32 = 0.7;
        const K_SPRING: f32 = 1.;
        const K_ANCHOR: f32 = 1000.;

        let point_conversion = |nucl: &RigidNucl| {
            let position = positions[nucl.helix]
                + self.helices[nucl.helix]
                    .center_to_origin
                    .rotated_by(orientations[nucl.helix]);
            let mut helix = Helix::new(position, orientations[nucl.helix]);
            helix.roll(self.helices[nucl.helix].roll);
            helix.space_pos(&self.parameters, nucl.position, nucl.forward)
        };

        for spring in self.springs.iter() {
            let point_0 = point_conversion(&spring.0);
            let point_1 = point_conversion(&spring.1);
            let len = (point_1 - point_0).mag();
            if !len.is_nan() {
                //println!("spring {:?}, len {}", spring, len);
            }
            let norm = len - L0;

            // The force applied on point 0
            let force = if len > 1e-5 {
                K_SPRING * norm * (point_1 - point_0) / len
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

        for (nucl, position) in self.anchors.iter() {
            let point_0 = point_conversion(&nucl);
            let len = (point_0 - *position).mag();
            if len < 100. {
                println!("point_0: {:?}", point_0);
                println!("position: {:?}", *position);
            }
            let force = if len > 1e-5 {
                K_SPRING * K_ANCHOR * -(point_0 - *position)
            } else {
                Vec3::zero()
            };

            forces[nucl.helix] += 10. * force;

            let torque0 = (point_0 - positions[nucl.helix]).cross(force);

            torques[nucl.helix] += torque0;
        }

        (forces, torques)
    }
}

impl HelixSystem {
    fn read_state(&self, x: &Vector<f32>) -> (Vec<Vec3>, Vec<Rotor3>, Vec<Vec3>, Vec<Vec3>) {
        let mut positions = Vec::with_capacity(self.helices.len());
        let mut rotations = Vec::with_capacity(self.helices.len());
        let mut linear_momentums = Vec::with_capacity(self.helices.len());
        let mut angular_momentums = Vec::with_capacity(self.helices.len());
        let mut iterator = x.iter();
        for _ in 0..self.helices.len() {
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

impl ExplicitODE<f32> for HelixSystem {
    // We read the sytem in the following format. For each grid, we read
    // * 3 f32 for position
    // * 4 f32 for rotation
    // * 3 f32 for linear momentum
    // * 3 f32 for angular momentum

    fn func(&self, _t: &f32, x: &Vector<f32>) -> Vector<f32> {
        let (positions, rotations, linear_momentums, angular_momentums) = self.read_state(x);
        let (forces, torques) = self.forces_and_torques(&positions, &rotations);

        let mut ret = Vec::with_capacity(13 * self.helices.len());
        for i in 0..self.helices.len() {
            let d_position = linear_momentums[i] / self.helices[i].height();
            ret.push(d_position.x);
            ret.push(d_position.y);
            ret.push(d_position.z);
            let omega = self.helices[i].inertia_inverse * angular_momentums[i];
            let d_rotation = 0.5
                * Rotor3::from_quaternion_array([omega.x, omega.y, omega.z, 0f32])
                * rotations[i];

            ret.push(d_rotation.s);
            ret.push(d_rotation.bv.xy);
            ret.push(d_rotation.bv.xz);
            ret.push(d_rotation.bv.yz);

            let d_linear_momentum =
                forces[i] - linear_momentums[i] * 100. / self.helices[i].height();

            ret.push(d_linear_momentum.x);
            ret.push(d_linear_momentum.y);
            ret.push(d_linear_momentum.z);

            let d_angular_momentum =
                torques[i] - angular_momentums[i] * 100. / self.helices[i].height();
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
            let mut ret = Vec::with_capacity(13 * self.helices.len());
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
            Vector::new_row(ret.len(), ret)
        }
    }
}

struct GridsSystem {
    springs: Vec<(ApplicationPoint, ApplicationPoint)>,
    grids: Vec<RigidGrid>,
    time_span: (f32, f32),
    last_state: Option<Vector<f32>>,
    anchors: Vec<(ApplicationPoint, Vec3)>,
}

impl GridsSystem {
    fn forces_and_torques(
        &self,
        positions: &[Vec3],
        orientations: &[Rotor3],
        volume_exclusion: f32,
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
            //println!("len {}", len);
            let norm = len - L0;

            // The force applied on point 0
            let force = K_SPRING * norm * (point_1 - point_0) / len;

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
            let d_position = linear_momentums[i] / self.grids[i].mass;
            ret.push(d_position.x);
            ret.push(d_position.y);
            ret.push(d_position.z);
            let omega = self.grids[i].inertia_inverse * angular_momentums[i];
            let d_rotation = 0.5
                * Rotor3::from_quaternion_array([omega.x, omega.y, omega.z, 0f32])
                * rotations[i];

            ret.push(d_rotation.s);
            ret.push(d_rotation.bv.xy);
            ret.push(d_rotation.bv.xz);
            ret.push(d_rotation.bv.yz);

            let d_linear_momentum = forces[i] - linear_momentums[i] * 100. / self.grids[i].mass;

            ret.push(d_linear_momentum.x);
            ret.push(d_linear_momentum.y);
            ret.push(d_linear_momentum.z);

            let d_angular_momentum = torques[i] - angular_momentums[i];
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
    pub roll: f32,
    pub orientation: Rotor3,
    pub inertia_inverse: Mat3,
    pub center_of_mass: Vec3,
    pub center_to_origin: Vec3,
    pub mass: f32,
    pub id: usize,
}

impl RigidHelix {
    fn new_from_grid(
        y_pos: f32,
        z_pos: f32,
        x_min: f32,
        x_max: f32,
        roll: f32,
        orientation: Rotor3,
    ) -> RigidHelix {
        Self {
            roll,
            orientation,
            center_of_mass: Vec3::new((x_min + x_max) / 2., y_pos, z_pos),
            center_to_origin: -(x_min + x_max) / 2. * Vec3::unit_x(),
            mass: x_max - x_min,
            inertia_inverse: inertia_helix(x_max - x_min, 1.).inversed(),
            // at the moment we do not care for the id when creating a rigid helix for a grid
            id: 0,
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
        id: usize,
    ) -> RigidHelix {
        Self {
            roll,
            orientation,
            center_of_mass: Vec3::new(x_pos, y_pos, z_pos),
            center_to_origin: delta,
            mass,
            inertia_inverse: inertia_helix(mass, 1.).inversed(),
            id,
        }
    }

    fn center_of_mass(&self) -> Vec3 {
        self.center_of_mass
    }

    fn height(&self) -> f32 {
        self.mass
    }
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
    id: usize,
    helices: Vec<RigidHelix>,
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
            helices,
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

struct HelixSystemThread {
    helix_system: HelixSystem,
    /// When the wrapped boolean is set to true, stop the simulation perfomed by self.
    stop: Arc<Mutex<bool>>,
    /// When the wrapped option takes the value of some channel, the thread that performs the
    /// simulation sends the last computed state of the system
    sender: Arc<Mutex<Option<Sender<RigidHelixState>>>>,
}

impl HelixSystemThread {
    fn new(helix_system: HelixSystem) -> Self {
        Self {
            helix_system,
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
        Arc<Mutex<Option<Sender<RigidHelixState>>>>,
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
                if let Ok((_, y)) = solver.solve(&self.helix_system) {
                    self.helix_system.last_state = y.last().cloned();
                }
            }
            *computing.lock().unwrap() = false;
        });
        (stop, sender)
    }

    fn get_state(&self) -> RigidHelixState {
        let state = self.helix_system.init_cond();
        let (positions, orientations, _, _) = self.helix_system.read_state(&state);
        let ids = self.helix_system.helices.iter().map(|g| g.id).collect();
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

pub(super) struct RigidHelixPtr {
    stop: Arc<Mutex<bool>>,
    state: Arc<Mutex<Option<Sender<RigidHelixState>>>>,
    instant: Instant,
}

#[derive(Debug, Clone)]
struct RigidHelixState {
    positions: Vec<Vec3>,
    orientations: Vec<Rotor3>,
    center_of_mass_from_helix: Vec<Vec3>,
    ids: Vec<usize>,
}

impl Data {
    pub fn grid_simulation(&mut self, time_span: (f32, f32)) {
        if let Some(grid_system) = self.make_grid_system(time_span) {
            let solver = Kutta3::new(1e-4f32);
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

    pub fn helices_simulation(&mut self, time_span: (f32, f32)) {
        if let Some(helix_system) = self.make_helices_system(time_span) {
            let solver = Kutta3::new(1e-4f32);
            if let Ok((_, y)) = solver.solve(&helix_system) {
                let last_state = y.last().unwrap();
                let (positions, rotations, _, _) = helix_system.read_state(last_state);
                for (i, rigid_helix) in helix_system.helices.iter().enumerate() {
                    let position = positions[i];
                    let orientation = rotations[i].normalized();
                    let helix = self.design.helices.get_mut(&rigid_helix.id).unwrap();
                    helix.position = position - rigid_helix.center_of_mass;
                    helix.orientation = orientation;
                    helix.end_movement();
                }
                self.hash_maps_update = true;
                self.update_status = true;
            } else {
                println!("error while solving");
            }
        } else {
            println!("could not make grid system");
        }
    }

    fn make_helices_system(&self, time_span: (f32, f32)) -> Option<HelixSystem> {
        let intervals = self.design.get_intervals();
        let parameters = self.design.parameters.unwrap_or_default();
        let mut helix_map = HashMap::with_capacity(self.design.helices.len());
        let mut rigid_helices = Vec::with_capacity(self.design.helices.len());
        for h_id in self.design.helices.keys() {
            if let Some(rigid_helix) =
                self.make_rigid_helix_world_pov(*h_id, &intervals, &parameters)
            {
                helix_map.insert(h_id, rigid_helices.len());
                rigid_helices.push(rigid_helix);
            }
        }
        let xovers = self.get_xovers_list();
        let mut springs = Vec::with_capacity(xovers.len());
        for (n1, n2) in xovers {
            let rigid_1 = RigidNucl {
                helix: helix_map[&n1.helix],
                position: n1.position,
                forward: n1.forward,
            };
            let rigid_2 = RigidNucl {
                helix: helix_map[&n2.helix],
                position: n2.position,
                forward: n2.forward,
            };
            springs.push((rigid_1, rigid_2));
        }
        let nucl_0 = Nucl {
            helix: 1,
            position: 0,
            forward: true,
        };
        let mut anchors = vec![];
        for anchor in self.anchors.iter() {
            if let Some(n_id) = self.identifier_nucl.get(anchor) {
                if let Some(rigid_helix) = helix_map.get(&anchor.helix) {
                    let rigid_nucl = RigidNucl {
                        helix: *rigid_helix,
                        position: anchor.position,
                        forward: anchor.forward,
                    };
                    let position: Vec3 = self.space_position[n_id].into();
                    anchors.push((rigid_nucl, position));
                }
            }
        }
        Some(HelixSystem {
            helices: rigid_helices,
            springs,
            last_state: None,
            time_span,
            parameters,
            anchors,
        })
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
        Some(GridsSystem {
            springs,
            grids: rigid_grids,
            time_span,
            last_state: None,
            anchors: vec![],
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
            if let Some(rigid_helix) = self.make_rigid_helix_grid_pov(h, intervals, parameters) {
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
        Some(RigidHelix::new_from_grid(
            position.y,
            position.z,
            *x_min as f32 * parameters.z_step,
            *x_max as f32 * parameters.z_step,
            helix.roll,
            helix.orientation,
        ))
    }

    fn make_rigid_helix_world_pov(
        &self,
        h_id: usize,
        intervals: &BTreeMap<usize, (isize, isize)>,
        parameters: &Parameters,
    ) -> Option<RigidHelix> {
        let (x_min, x_max) = intervals.get(&h_id)?;
        let helix = self.design.helices.get(&h_id)?;
        let left = helix.axis_position(parameters, *x_min);
        let right = helix.axis_position(parameters, *x_max);
        let position = (left + right) / 2.;
        let position_delta =
            -(*x_max as f32 * parameters.z_step + *x_min as f32 * parameters.z_step) / 2.
                * Vec3::unit_x();
        Some(RigidHelix::new_from_world(
            position.y,
            position.z,
            position.x,
            position_delta,
            (right - left).mag(),
            helix.roll,
            helix.orientation,
            h_id,
        ))
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
                    let grid = &mut self.grid_manager.grids[state.ids[i]];
                    grid.position =
                        position - state.center_of_mass_from_grid[i].rotated_by(orientation);
                    grid.orientation = orientation;
                    grid.end_movement();
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

    pub(super) fn check_rigid_helices(&mut self) {
        if let Some(ptrs) = self.helix_simulation_ptr.as_mut() {
            let now = Instant::now();
            if (now - ptrs.instant).as_millis() > 30 {
                let (snd, rcv) = std::sync::mpsc::channel();
                *ptrs.state.lock().unwrap() = Some(snd);
                let state = rcv.recv().unwrap();
                for i in 0..state.ids.len() {
                    let position = state.positions[i];
                    let orientation = state.orientations[i].normalized();
                    self.design.helices.get_mut(&state.ids[i]).unwrap().position =
                        position + state.center_of_mass_from_helix[i].rotated_by(orientation);
                    self.design
                        .helices
                        .get_mut(&state.ids[i])
                        .unwrap()
                        .orientation = orientation;
                }
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

    pub fn helix_simulation_request(&mut self, request: (f32, f32), computing: Arc<Mutex<bool>>) {
        if self.helix_simulation_ptr.is_some() {
            self.stop_helix_simulation()
        } else {
            self.start_helix_simulation(request, computing)
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

    fn start_helix_simulation(&mut self, request: (f32, f32), computing: Arc<Mutex<bool>>) {
        if let Some(helix_system) = self.make_helices_system(request) {
            let grid_system_thread = HelixSystemThread::new(helix_system);
            let date = Instant::now();
            let (stop, snd) = grid_system_thread.run(computing);
            self.helix_simulation_ptr = Some(RigidHelixPtr {
                instant: date,
                stop,
                state: snd,
            });
        }
    }

    fn stop_helix_simulation(&mut self) {
        if let Some(helix_simulation_ptr) = self.helix_simulation_ptr.as_mut() {
            *helix_simulation_ptr.stop.lock().unwrap() = true;
        } else {
            println!("design was not performing rigid body simulation");
        }
        self.helix_simulation_ptr = None;
    }
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
