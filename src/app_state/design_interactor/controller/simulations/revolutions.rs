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

use super::{SimulationInterface, SimulationReader, SimulationUpdate};
use std::f64::consts::TAU;
use std::sync::{Arc, Mutex, Weak};
use ultraviolet::{Isometry3, Rotor3, Vec3};

use mathru::algebra::linear::vector::vector::Vector;
use mathru::analysis::differential_equation::ordinary::{
    solver::runge_kutta::{explicit::fixed::FixedStepper, ExplicitEuler, Ralston4},
    ExplicitODE,
};

use ensnano_design::{
    CurveDescriptor, CurveDescriptor2D, DVec3, InterpolatedCurveDescriptor,
    InterpolationDescriptor, Parameters as DNAParameters,
};
use ensnano_interactor::{
    EquadiffSolvingMethod, RevolutionSimulationParameters, RevolutionSurfaceDescriptor,
    RevolutionSurfaceSystemDescriptor,
};

use crate::app_state::{
    design_interactor::controller::simulations::revolutions::open_curves::OpenSurfaceTopology,
    ErrOperation,
};

mod closed_curves;
use closed_curves::CloseSurfaceTopology;

mod open_curves;

trait SpringTopology: Send + Sync + 'static {
    fn nb_balls(&self) -> usize;

    fn balls_with_successor(&self) -> &[usize];
    /// Return the identifier of the next ball on the helix,  or `ball_id` if `ball_id` is
    /// the last ball on an open helix.
    fn successor(&self, ball_id: usize) -> usize;

    fn balls_with_predecessor(&self) -> &[usize];
    /// Return the identfier of the previous ball on the helix,  or `ball_id` if `ball_id` is
    /// the first ball on an open helix.
    fn predecessor(&self, ball_id: usize) -> usize;

    fn balls_with_predecessor_and_successor(&self) -> &[usize];

    fn balls_involved_in_spring(&self) -> &[usize];
    fn other_spring_end(&self, ball_id: usize) -> usize;

    fn surface_position(&self, revolution_angle: f64, theta: f64) -> DVec3;
    fn dpos_dtheta(&self, revolution_angle: f64, theta: f64) -> DVec3;
    fn revolution_angle_ball(&self, ball_id: usize) -> f64;

    fn theta_ball_init(&self) -> Vec<f64>;

    fn rescale_section(&mut self, scaling_factor: f64);
    fn rescale_radius(&mut self, scaling_factor: f64);

    fn cloned(&self) -> Box<dyn SpringTopology>;

    fn axis(&self, revolution_angle: f64) -> DVec3;

    fn to_curve_descriptor(&self, thetas: Vec<f64>, finished: bool) -> Vec<CurveDescriptor>;

    fn fixed_points(&self) -> &[usize];

    fn additional_forces(
        &self,
        _thetas: &[f64],
        _parameters: &RevolutionSimulationParameters,
    ) -> Vec<(usize, DVec3)> {
        vec![]
    }

    fn revolution_radius(&self) -> f64;
}

pub struct RevolutionSurfaceSystem {
    topology: Box<dyn SpringTopology>,
    dna_parameters: DNAParameters,
    last_thetas: Option<Vec<f64>>,
    last_dthetas: Option<Vec<f64>>,
    scaffold_len_target: usize,
    current_scaffold_length: Option<usize>,
    simulation_parameters: RevolutionSimulationParameters,
    plane_position: Vec3,
    plane_orientation: Rotor3,
}

impl Clone for RevolutionSurfaceSystem {
    fn clone(&self) -> Self {
        Self {
            topology: self.topology.cloned(),
            dna_parameters: self.dna_parameters,
            last_thetas: self.last_thetas.clone(),
            last_dthetas: self.last_dthetas.clone(),
            scaffold_len_target: self.scaffold_len_target,
            current_scaffold_length: self.current_scaffold_length,
            simulation_parameters: self.simulation_parameters.clone(),
            plane_position: self.plane_position,
            plane_orientation: self.plane_orientation,
        }
    }
}

#[derive(Clone)]
pub struct RevolutionSurface {
    curve: CurveDescriptor2D,
    revolution_radius: f64,
    nb_helix_per_half_section: usize,
    half_turns_count: isize,
    shift_per_turn: isize,
    junction_smoothening: f64,
    nb_helices: usize,
    curve_scale_factor: f64,
}

impl RevolutionSurfaceSystem {
    pub fn new(desc: RevolutionSurfaceSystemDescriptor) -> Self {
        let scaffold_len_target = desc.scaffold_len_target;
        let dna_parameters = desc.dna_parameters;
        let simulation_parameters = desc.simulation_parameters.clone();
        let plane_orientation = desc.target.plane_orientation;
        let plane_position = desc.target.plane_position;
        let topology: Box<dyn SpringTopology> = if desc.target.curve.is_open() {
            Box::new(OpenSurfaceTopology::new(desc))
        } else {
            Box::new(CloseSurfaceTopology::new(desc))
        };

        Self {
            topology,
            dna_parameters,
            last_thetas: None,
            last_dthetas: None,
            scaffold_len_target,
            current_scaffold_length: None,
            simulation_parameters,
            plane_position,
            plane_orientation,
        }
    }

    fn one_radius_optimisation_step(
        &mut self,
        first: &mut bool,
        interface: Option<Arc<Mutex<RevolutionSystemInterface>>>,
    ) -> usize {
        let mut current_default;
        for _ in 0..10 {
            if let Some(interface) = interface.as_ref() {
                let mut interface_lock = interface.lock().unwrap();
                if !interface_lock.still_valid() {
                    // no need to continue the computations
                    return 0;
                }
                if interface_lock.finished {
                    break;
                }
                interface_lock.new_state = Some(self.clone());
            }
            //std::thread::sleep_ms(20_000);
            current_default = self.one_simulation_step(first);
            if current_default < 1.01 {
                break;
            }
        }

        let curve_desc = self.to_curve_desc(false).unwrap();

        let thetas = self
            .last_thetas
            .clone()
            .unwrap_or_else(|| self.topology.theta_ball_init());
        let mut total_len = 0;
        for desc in curve_desc {
            let len = desc.compute_length().unwrap();
            println!("length ~= {:?}", len);
            println!("length ~= {:?} nt", len / self.dna_parameters.z_step as f64);
            total_len += (len / self.dna_parameters.z_step as f64).floor() as usize;
        }

        println!("total len {total_len}");
        let len_by_sum =
            (self.total_length(&thetas) / (self.dna_parameters.z_step as f64)).floor() as usize;
        println!("total len by sum {len_by_sum}");
        let rescaling_factor = self.scaffold_len_target as f64 / total_len as f64;
        self.topology.rescale_radius(rescaling_factor);
        total_len
    }

    fn one_simulation_step(&mut self, first: &mut bool) -> f64 {
        let total_nb_section = self.topology.nb_balls();
        if *first {
            let mut spring_relaxation_state = SpringRelaxationState::new();
            let mut system = RelaxationSystem {
                thetas: self
                    .last_thetas
                    .clone()
                    .unwrap_or_else(|| self.topology.theta_ball_init()),
                forces: vec![DVec3::zero(); total_nb_section],
                d_thetas: vec![0.; total_nb_section],
                second_derivative_thetas: vec![0.; total_nb_section],
            };
            self.apply_springs(&mut system, Some(&mut spring_relaxation_state));
            println!("initial spring relax state: {:?}", spring_relaxation_state);
            let rescaling_factor = 1. / spring_relaxation_state.avg_ext;
            self.topology.rescale_section(rescaling_factor);
            *first = false;
        }

        let solver = FixedStepper::new(self.simulation_parameters.simulation_step);

        let solver_solution = match self.simulation_parameters.method {
            EquadiffSolvingMethod::Euler => solver.solve(self, &ExplicitEuler::default()),
            EquadiffSolvingMethod::Ralston => solver.solve(self, &Ralston4::default()),
        };

        let mut spring_relaxation_state = SpringRelaxationState::new();
        if let Some(last_state) = solver_solution.ok().and_then(|(_, y)| y.last().cloned()) {
            let mut system = RelaxationSystem::from_mathru(last_state, total_nb_section);
            self.apply_springs(&mut system, Some(&mut spring_relaxation_state));
            self.last_thetas = Some(system.thetas.clone());
            self.last_dthetas = Some(system.d_thetas.clone());
        } else {
            log::error!("error while solving ODE");
        };
        /*self.target.curve_scale_factor /= (spring_relaxation_state.min_ext
        + spring_relaxation_state.max_ext
        + 2. * spring_relaxation_state.avg_ext)
        / 4.;*/

        /*
        self.target.curve_scale_factor /=
            (spring_relaxation_state.min_ext + spring_relaxation_state.max_ext) / 2.;
        */
        let rescaling_factor =
            2. / (spring_relaxation_state.min_ext + spring_relaxation_state.max_ext);
        self.topology.rescale_section(rescaling_factor);

        println!("spring_relax state {:?}", spring_relaxation_state);
        //println!("curve scale {}", self.target.curve_scale_factor);
        spring_relaxation_state
            .max_ext
            .max(1. / spring_relaxation_state.min_ext)
    }

    fn helix_axis(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        (self.position_section(self.topology.successor(section_idx), thetas)
            - self.position_section(self.topology.predecessor(section_idx), thetas))
        .normalized()
    }

    fn total_length(&self, thetas: &[f64]) -> f64 {
        let mut ret = 0.;
        for i in self.topology.balls_with_successor() {
            ret += (self.position_section(self.topology.successor(*i), thetas)
                - self.position_section(*i, thetas))
            .mag()
        }
        ret
    }

    fn position_section(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        let revolution_angle = self.topology.revolution_angle_ball(section_idx);
        let theta = thetas[section_idx];
        self.topology.surface_position(revolution_angle, theta)
    }

    fn apply_springs(
        &self,
        system: &mut RelaxationSystem,
        mut spring_state: Option<&mut SpringRelaxationState>,
    ) {
        let mut nb_spring = 0;
        if let Some(state) = spring_state.as_mut() {
            state.avg_ext = 0.;
        }
        for i in self.topology.balls_involved_in_spring() {
            let i = *i;
            let j = self.topology.other_spring_end(i);
            let pos_i = self.position_section(i, &system.thetas);
            let pos_j = self.position_section(j, &system.thetas);

            let ui = self.helix_axis(i, &system.thetas);
            let uj = self.helix_axis(j, &system.thetas);

            let revolution_angle = self.topology.revolution_angle_ball(i);
            let z = self.topology.axis(revolution_angle);

            let ri = ((self.dna_parameters.helix_radius as f64
                + (self.dna_parameters.inter_helix_gap as f64) / 2.)
                / ui.dot(z))
            .abs();
            let rj = ((self.dna_parameters.helix_radius as f64
                + (self.dna_parameters.inter_helix_gap as f64) / 2.)
                / uj.dot(z))
            .abs();

            let len0_ij = ri + rj;
            let v_ji = pos_i - pos_j;
            let len_ij = v_ji.mag();

            let f_ij = self.simulation_parameters.spring_stiffness * (1. - len0_ij / len_ij) * v_ji;

            if let Some(state) = spring_state.as_mut() {
                let ext = len_ij / len0_ij;
                state.min_ext = state.min_ext.min(ext);
                state.max_ext = state.max_ext.max(ext);
                state.avg_ext += ext;
            }

            system.forces[i] -= f_ij;
            system.forces[j] += f_ij;
            nb_spring += 1;
        }

        if let Some(state) = spring_state.as_mut() {
            state.avg_ext /= nb_spring as f64;
        }
    }

    fn apply_torsions(&self, system: &mut RelaxationSystem) {
        for section_idx in self.topology.balls_with_predecessor_and_successor() {
            let i = self.topology.predecessor(*section_idx);
            let j = *section_idx;
            let k = self.topology.successor(*section_idx);

            let pos_i = self.position_section(i, &system.thetas);
            let pos_j = self.position_section(j, &system.thetas);
            let pos_k = self.position_section(k, &system.thetas);

            let u_ij = pos_j - pos_i;
            let u_jk = pos_k - pos_j;
            let v = u_jk - u_ij;
            let f_ijk = self.simulation_parameters.torsion_stiffness * v / v.mag().max(1.);
            system.forces[i] -= f_ijk / 2.;
            system.forces[j] += f_ijk;
            system.forces[k] -= f_ijk / 2.;
        }
    }

    fn dpos_dtheta(&self, section_idx: usize, thetas: &[f64]) -> DVec3 {
        let revolution_angle = self.topology.revolution_angle_ball(section_idx);
        let theta = thetas[section_idx];
        self.topology.dpos_dtheta(revolution_angle, theta)
    }

    fn apply_forces(&self, system: &mut RelaxationSystem) {
        let total_nb_segment = self.topology.nb_balls();

        for (b_id, f) in self
            .topology
            .additional_forces(&system.thetas, &self.simulation_parameters)
        {
            //println!("b_id {b_id}, force {:.3?}", f);
            system.forces[b_id] += f;
        }

        for section_idx in 0..total_nb_segment {
            let tengent = self.dpos_dtheta(section_idx, &system.thetas);
            let acceleration_without_friction =
                system.forces[section_idx].dot(tengent) / tengent.mag_sq();
            system.second_derivative_thetas[section_idx] += (acceleration_without_friction
                - self.simulation_parameters.fluid_friction * system.d_thetas[section_idx])
                / self.simulation_parameters.ball_mass;
        }

        for idx in self.topology.fixed_points() {
            system.second_derivative_thetas[*idx] = 0.;
        }
    }

    fn to_curve_desc(&self, finished: bool) -> Option<Vec<CurveDescriptor>> {
        self.last_thetas
            .clone()
            .map(|t| self.topology.to_curve_descriptor(t, finished))
    }
}

struct RelaxationSystem {
    thetas: Vec<f64>,
    d_thetas: Vec<f64>,
    second_derivative_thetas: Vec<f64>,
    forces: Vec<DVec3>,
}

#[derive(Default, Debug)]
struct SpringRelaxationState {
    min_ext: f64,
    max_ext: f64,
    avg_ext: f64,
}

impl SpringRelaxationState {
    fn new() -> Self {
        Self {
            min_ext: std::f64::INFINITY,
            max_ext: 0.,
            avg_ext: 0.,
        }
    }
}

impl RelaxationSystem {
    fn into_mathru(self) -> Vector<f64> {
        let mut data = self.d_thetas;
        data.extend(self.second_derivative_thetas);
        Vector::new_row(data)
    }

    fn from_mathru(vec: Vector<f64>, nb_section: usize) -> Self {
        let vec = vec.convert_to_vec();
        let thetas = vec[0..nb_section].to_vec();
        let d_thetas = vec[nb_section..].to_vec();

        Self {
            thetas,
            d_thetas,
            second_derivative_thetas: vec![0.; nb_section],
            forces: vec![DVec3::zero(); nb_section],
        }
    }
}

impl ExplicitODE<f64> for RevolutionSurfaceSystem {
    fn init_cond(&self) -> Vector<f64> {
        let total_nb_section = self.topology.nb_balls();

        let mut data = self
            .last_thetas
            .clone()
            .unwrap_or_else(|| self.topology.theta_ball_init());
        let speeds = self
            .last_dthetas
            .clone()
            .unwrap_or_else(|| vec![0.; total_nb_section]);
        data.extend(speeds);
        Vector::new_row(data)
    }

    fn func(&self, _t: &f64, x: &Vector<f64>) -> Vector<f64> {
        let total_nb_section = self.topology.nb_balls();
        let mut system = RelaxationSystem::from_mathru(x.clone(), total_nb_section);
        self.apply_springs(&mut system, None);
        self.apply_torsions(&mut system);
        self.apply_forces(&mut system);
        system.into_mathru()
    }

    fn time_span(&self) -> (f64, f64) {
        (0., self.simulation_parameters.time_span)
    }
}

impl RevolutionSurface {
    pub fn new(desc: RevolutionSurfaceDescriptor) -> Self {
        let nb_helices = desc.nb_helices();
        let curve_scale_factor =
            desc.nb_helix_per_half_section as f64 * 2. * DNAParameters::INTER_CENTER_GAP as f64
                / desc.curve.perimeter();

        Self {
            curve: desc.curve,
            revolution_radius: desc.revolution_radius,
            nb_helices,
            nb_helix_per_half_section: desc.nb_helix_per_half_section,
            shift_per_turn: desc.shift_per_turn,
            half_turns_count: desc.half_turns_count,
            curve_scale_factor,
            junction_smoothening: desc.junction_smoothening,
        }
    }

    fn position(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        // must be equal to PI * half_turns when revolution_angle = TAU.
        let section_rotation = revolution_angle * (self.half_turns_count as f64) / 2.;

        let section_point = self.curve.point(section_t);

        let x_2d = self.revolution_radius
            + self.curve_scale_factor
                * (section_point.x * section_rotation.cos()
                    - section_rotation.sin() * section_point.y);

        let y_2d = self.curve_scale_factor
            * (section_point.x * section_rotation.sin() + section_rotation.cos() * section_point.y);

        DVec3 {
            x: revolution_angle.cos() * x_2d,
            y: revolution_angle.sin() * x_2d,
            z: y_2d,
        }
    }

    fn dpos_dtheta(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        let section_rotation = revolution_angle * (self.half_turns_count as f64) / 2.;

        let dpos_curve = self.curve.derivative(section_t);

        let x_2d = self.curve_scale_factor
            * (dpos_curve.x * section_rotation.cos() - section_rotation.sin() * dpos_curve.y);

        let y_2d = self.curve_scale_factor
            * (dpos_curve.x * section_rotation.sin() + section_rotation.cos() * dpos_curve.y);

        DVec3 {
            x: revolution_angle.cos() * x_2d,
            y: revolution_angle.sin() * x_2d,
            z: y_2d,
        }
    }

    fn axis(&self, revolution_angle: f64) -> DVec3 {
        DVec3 {
            x: -revolution_angle.sin(),
            y: revolution_angle.cos(),
            z: 0.,
        }
    }

    fn total_shift(&self) -> isize {
        let additional_shift = if self.half_turns_count % 2 == 1 {
            self.nb_helix_per_half_section
        } else {
            0
        };
        self.shift_per_turn + additional_shift as isize
    }
}

pub struct RevolutionSystemThread {
    interface: Weak<Mutex<RevolutionSystemInterface>>,
    system: RevolutionSurfaceSystem,
}

impl RevolutionSystemThread {
    pub fn start_new(
        system: RevolutionSurfaceSystemDescriptor,
        reader: &mut dyn SimulationReader,
    ) -> Result<Arc<Mutex<RevolutionSystemInterface>>, ErrOperation> {
        let ret = Arc::new(Mutex::new(RevolutionSystemInterface::default()));
        let ret_dyn: Arc<Mutex<dyn SimulationInterface>> = ret.clone();
        reader.attach_state(&ret_dyn);
        let simulation_thread = Self::new(system, &ret);
        simulation_thread.run();
        Ok(ret)
    }

    fn new(
        system_desc: RevolutionSurfaceSystemDescriptor,
        interface: &Arc<Mutex<RevolutionSystemInterface>>,
    ) -> Self {
        let system = RevolutionSurfaceSystem::new(system_desc);
        Self {
            interface: Arc::downgrade(interface),
            system,
        }
    }

    fn run(mut self) {
        std::thread::spawn(move || {
            let mut first = true;
            while let Some(interface_ptr) = self.interface.upgrade() {
                let current_len = self
                    .system
                    .one_radius_optimisation_step(&mut first, Some(interface_ptr.clone()));
                if interface_ptr.lock().unwrap().finished
                    || current_len == self.system.scaffold_len_target
                {
                    if let Some(descs) = self.system.to_curve_desc(true) {
                        interface_ptr.lock().unwrap().curve_descriptor.set(descs);
                    }
                    break;
                } else {
                    self.system.current_scaffold_length = Some(current_len);
                }
            }
        });
    }
}

impl SimulationUpdate for RevolutionSurfaceSystem {
    fn update_design(&self, design: &mut ensnano_design::Design) {
        design.additional_structure = Some(Arc::new(self.clone()))
    }
}

impl ensnano_design::AdditionalStructure for RevolutionSurfaceSystem {
    fn position(&self) -> Vec<ultraviolet::Vec3> {
        use ensnano_design::utils::dvec_to_vec;
        let thetas = self
            .last_thetas
            .clone()
            .unwrap_or_else(|| self.topology.theta_ball_init());
        let total_nb_sections = self.topology.nb_balls();
        (0..total_nb_sections)
            .map(|n| dvec_to_vec(self.position_section(n, &thetas)))
            .collect()
    }

    fn next(&self) -> Vec<(usize, usize)> {
        self.topology
            .balls_involved_in_spring()
            .iter()
            .map(|s| (*s, self.topology.other_spring_end(*s)))
            .collect()
    }

    fn right(&self) -> Vec<(usize, usize)> {
        self.topology
            .balls_with_successor()
            .iter()
            .map(|s| (*s, self.topology.successor(*s)))
            .collect()
    }

    fn nt_path(&self) -> Option<Vec<ultraviolet::Vec3>> {
        let mut ret = Vec::new();
        let curve_desc = self.to_curve_desc(false)?;
        for desc in curve_desc {
            let nts = desc.path()?;
            ret.extend(nts.into_iter().map(ensnano_design::utils::dvec_to_vec));
        }

        for n in 0..1000 {
            ret.push(ensnano_design::utils::dvec_to_vec(
                self.topology
                    .surface_position(-std::f64::consts::FRAC_PI_2, n as f64 / 1000.),
            ));
        }

        Some(ret)
    }

    fn current_length(&self) -> Option<usize> {
        self.current_scaffold_length
    }

    fn frame(&self) -> Isometry3 {
        let mut ret = Isometry3::identity();

        // First inverse the transformation that is performed by the construction of the curve
        ret.append_rotation(Rotor3::from_rotation_yz(-std::f32::consts::FRAC_PI_2));

        // Then convert into the plane's frame
        ret.append_translation(self.plane_position);
        ret.append_rotation(self.plane_orientation);

        // Center on the rotation axis as drawn on the plane
        let rotation_axis_translation = (-Vec3::unit_z() * self.topology.revolution_radius() as f32)
            .rotated_by(self.plane_orientation);
        ret.append_translation(rotation_axis_translation);

        ret
    }
}

impl SimulationInterface for RevolutionSystemInterface {
    fn get_simulation_state(&mut self) -> Option<Box<dyn SimulationUpdate>> {
        if let Some(descs) = self.curve_descriptor.take() {
            Some(Box::new(descs))
        } else {
            let s = self.new_state.take()?;
            Some(Box::new(s))
        }
    }

    fn still_valid(&self) -> bool {
        !matches!(self.curve_descriptor, OptionOnce::Taken)
    }
}

impl SimulationUpdate for Vec<CurveDescriptor> {
    fn update_design(&self, design: &mut ensnano_design::Design) {
        use ensnano_design::{Domain, DomainJunction, Helix, HelixInterval, Rotor2, Strand, Vec2};
        let parameters = design.parameters.unwrap_or_default();
        let mut helices = design.helices.make_mut();
        let mut strand_to_be_added = Vec::new();
        for (c_id, c) in self.iter().enumerate() {
            let mut helix = Helix::new_with_curve(c.clone());
            helix.isometry2d = Some(ensnano_design::Isometry2 {
                translation: 5. * c_id as f32 * Vec2::unit_y(),
                rotation: Rotor2::identity(),
            });
            let h_id = helices.push_helix(helix);
            let len = if let CurveDescriptor::InterpolatedCurve(desc) = c {
                desc.objective_number_of_nts.map(|x| x as isize)
            } else {
                None
            };
            if let Some(len_nt) = len.or_else(|| {
                c.compute_length()
                    .map(|len| (len / parameters.z_step as f64).floor() as isize)
            }) {
                strand_to_be_added.push((h_id, len_nt));
            }
        }

        drop(helices);

        let strands = design.mut_strand_and_data().strands;

        // Use "random" integer to determine new strands color
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut now_s = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            % 10_000) as usize
            + 1_000_000;

        for (h_id, len) in strand_to_be_added {
            for forward in [true, false] {
                let domain = Domain::HelixDomain(HelixInterval {
                    helix: h_id,
                    start: 0,
                    end: len,
                    forward,
                    sequence: None,
                });
                let color = ensnano_utils::new_color(&mut now_s);

                strands.push(Strand {
                    color,
                    domains: vec![domain],
                    junctions: vec![DomainJunction::Prime3],
                    name: None,
                    cyclic: false,
                    sequence: None,
                });
            }
        }

        design.additional_structure = None;
    }
}

#[derive(Default)]
pub struct RevolutionSystemInterface {
    new_state: Option<RevolutionSurfaceSystem>,
    curve_descriptor: OptionOnce<Vec<CurveDescriptor>>,
    finished: bool,
}

impl RevolutionSystemInterface {
    pub fn kill(&mut self) {
        self.curve_descriptor = OptionOnce::Taken;
    }

    pub fn finish(&mut self) {
        self.finished = true
    }
}

enum OptionOnce<T> {
    NeverTaken(Option<T>),
    Taken,
}

impl<T> Default for OptionOnce<T> {
    fn default() -> Self {
        Self::NeverTaken(None)
    }
}

impl<T> OptionOnce<T> {
    fn take(&mut self) -> Option<T> {
        match self {
            Self::Taken => None,
            Self::NeverTaken(Some(_)) => {
                if let Self::NeverTaken(ret) = std::mem::replace(self, Self::Taken) {
                    ret
                } else {
                    unreachable!()
                }
            }
            Self::NeverTaken(None) => None,
        }
    }

    fn set(&mut self, ret: T) {
        if let Self::NeverTaken(_) = self {
            *self = Self::NeverTaken(Some(ret))
        }
    }
}

#[cfg(test)]
mod tests {
    const NB_SECTION_PER_SEGMENT: usize = 100;
    use super::*;

    #[test]
    fn relax_hexagon() {
        let surface_desc = RevolutionSurfaceDescriptor {
            curve: CurveDescriptor2D::Ellipse {
                semi_minor_axis: 1f64.into(),
                semi_major_axis: 2f64.into(),
            },
            half_turns_count: 6,
            revolution_radius: 20.,
            junction_smoothening: 0.,
            nb_helix_per_half_section: 7,
            dna_paramters: DNAParameters::GEARY_2014_DNA,
            shift_per_turn: -12,
            plane_orientation: Rotor3::identity(),
            plane_position: Vec3::zero(),
        };
        let system_desc = RevolutionSurfaceSystemDescriptor {
            simulation_parameters: Default::default(),
            dna_parameters: DNAParameters::GEARY_2014_DNA,
            target: surface_desc,
            scaffold_len_target: 7560,
        };
        let mut system = RevolutionSurfaceSystem::new(system_desc);

        let mut current_length = 0;
        let mut first = true;
        while current_length != system.scaffold_len_target {
            current_length = system.one_radius_optimisation_step(&mut first, None);
        }
    }
}
