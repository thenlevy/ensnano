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
//! This modules defines the type `design::Data` which handles the data representing a DNA
//! nanostructure.
//!
//! The element of a design (nucleotides and bounds) have an identifier that is an u32. Only the
//! last 24 bits of of this identifier can be used, the 8 first bits are reserved for the
//! identifier of the design.
//!
//! The `Data` objects can convert these identifier into `Nucl` position or retrieve information
//! about the element such as its position, color etc...
//!
use crate::gui::SimulationRequest;
use crate::utils::id_generator::IdGenerator;
use ahash::RandomState;
use cadnano_format::Cadnano;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use ultraviolet::Vec3;

use std::borrow::Cow;
use std::fmt;
use std::time::Instant;

mod cadnano;
mod codenano;
mod elements;
mod grid;
mod icednano;
mod insertion_replacement;
mod oxdna;
mod rigid_body;
mod roller;
mod scadnano;
mod strand_builder;
mod strand_template;
mod tests;
mod torsion;
use super::utils::*;
use crate::mediator::Selection;
use crate::scene::GridInstance;
use crate::utils::{message, new_color};
pub use elements::*;
use ensnano_organizer::OrganizerTree;
use grid::GridManager;
pub use grid::*;
pub use icednano::Nucl;
pub use icednano::{Axis, Design, Helix, Parameters, Strand};
use icednano::{Domain, DomainJunction, HelixInterval};
pub use rigid_body::{GridSystemState, RigidBodyConstants, RigidHelixState};
use roller::PhysicalSystem;
use std::sync::{mpsc::Sender, Arc, Mutex, RwLock};
use strand_builder::NeighbourDescriptor;
pub use strand_builder::{DomainIdentifier, StrandBuilder};
use strand_template::{TemplateManager, XoverCopyManager};
pub use torsion::Torsion;

#[derive(Clone)]
pub struct StrandState {
    strands: BTreeMap<usize, Strand>,
    xover_ids: IdGenerator<(Nucl, Nucl)>,
}

impl std::fmt::Debug for StrandState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("").finish()
    }
}

/// In addition to its `design` field, the `Data` struct has several hashmaps that are usefull to
/// quickly access information about the design. These hasmaps must be updated when the design is
/// modified.
///
/// At the moment, the hash maps are completely recomputed on every modification of the design. In
/// the future this might be optimised.
pub struct Data {
    design: icednano::Design,
    file_name: PathBuf,
    last_backup_time: Option<Instant>,
    object_type: HashMap<u32, ObjectType, RandomState>,
    /// Maps identifier of nucleotide to Nucleotide objects
    nucleotide: HashMap<u32, Nucl, RandomState>,
    /// Maps identifier of bounds to the pair of nucleotides involved in the bound
    nucleotides_involved: HashMap<u32, (Nucl, Nucl), RandomState>,
    /// Maps identifier of element to their position in the Model's coordinates
    space_position: HashMap<u32, [f32; 3], RandomState>,
    /// Maps a Nucl object to its identifier
    identifier_nucl: HashMap<Nucl, u32, RandomState>,
    /// Maps a pair of nucleotide forming a bound to the identifier of the bound
    identifier_bound: HashMap<(Nucl, Nucl), u32, RandomState>,
    /// Maps the identifier of a element to the identifier of the strands to which it belongs
    strand_map: HashMap<u32, usize, RandomState>,
    /// Maps the identifier of a element to the identifier of the helix to which it belongs
    helix_map: HashMap<u32, usize, RandomState>,
    /// Maps the identifier of an element to its color
    color: HashMap<u32, u32, RandomState>,
    /// Must be set to true when the design is modified, so that its obeservers get notified of the
    /// modification
    update_status: bool,
    /// Must be set to true when a modification that requires an update of the hash maps is
    /// performed
    hash_maps_update: bool,
    /// Maps nucleotides to basis characters
    basis_map: Arc<RwLock<HashMap<Nucl, char, RandomState>>>,
    grid_manager: GridManager,
    grids: Vec<Arc<RwLock<Grid2D>>>,
    color_idx: usize,
    view_need_reset: bool,
    groups: Arc<RwLock<BTreeMap<usize, bool>>>,
    red_cubes: HashMap<(isize, isize, isize), Vec<Nucl>, RandomState>,
    #[allow(dead_code)]
    blue_cubes: HashMap<(isize, isize, isize), Vec<Nucl>, RandomState>,
    blue_nucl: Vec<Nucl>,
    roller_ptrs: Option<(
        Arc<Mutex<bool>>,
        Arc<Mutex<Option<Sender<Vec<Helix>>>>>,
        Instant,
    )>,
    rigid_body_ptr: Option<rigid_body::RigidBodyPtr>,
    helix_simulation_ptr: Option<rigid_body::RigidHelixPtr>,
    hyperboloid_helices: Vec<usize>,
    hyperboloid_draft: Option<GridDescriptor>,
    template_manager: TemplateManager,
    xover_copy_manager: XoverCopyManager,
    anchors: HashSet<Nucl>,
    rigid_helix_simulator: Option<rigid_body::RigidHelixSimulator>,
    elements_update: Option<Vec<DnaElement>>,
    visible: HashMap<Nucl, bool>,
    visibility_sieve: Option<VisibilitySieve>,
    xover_ids: IdGenerator<(Nucl, Nucl)>,
    prime3_set: Vec<(Vec3, Vec3, u32)>,
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data").finish()
    }
}

impl Data {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let design = icednano::Design::new();
        let grid_manager = GridManager::new(Parameters::default());
        let mut file_name = std::env::current_exe().unwrap();
        file_name.set_file_name("unamed_design.json");
        Self {
            design,
            file_name,
            last_backup_time: None,
            object_type: HashMap::default(),
            space_position: HashMap::default(),
            identifier_nucl: HashMap::default(),
            identifier_bound: HashMap::default(),
            nucleotides_involved: HashMap::default(),
            nucleotide: HashMap::default(),
            strand_map: HashMap::default(),
            helix_map: HashMap::default(),
            color: HashMap::default(),
            update_status: false,
            hash_maps_update: false,
            basis_map: Arc::new(RwLock::new(HashMap::default())),
            grid_manager,
            grids: Vec::new(),
            color_idx: 0,
            view_need_reset: false,
            groups: Default::default(),
            red_cubes: HashMap::default(),
            blue_cubes: HashMap::default(),
            blue_nucl: vec![],
            roller_ptrs: None,
            hyperboloid_helices: vec![],
            hyperboloid_draft: None,
            template_manager: Default::default(),
            xover_copy_manager: Default::default(),
            rigid_body_ptr: None,
            helix_simulation_ptr: None,
            anchors: HashSet::new(),
            rigid_helix_simulator: None,
            elements_update: None,
            visible: Default::default(),
            visibility_sieve: None,
            xover_ids: Default::default(),
            prime3_set: Default::default(),
        }
    }

    pub fn add_hyperboloid(
        &mut self,
        position: Vec3,
        orientation: ultraviolet::Rotor3,
        hyperboloid: Hyperboloid,
    ) {
        self.hyperboloid_draft = Some(GridDescriptor {
            position,
            orientation,
            grid_type: GridTypeDescr::Hyperboloid {
                radius: hyperboloid.radius,
                shift: hyperboloid.shift,
                length: hyperboloid.length,
                radius_shift: hyperboloid.radius_shift,
                forced_radius: None,
            },
        });
        self.make_hyperboloid_helices();
    }

    pub fn update_hyperboloid(
        &mut self,
        nb_helix: usize,
        shift: f32,
        length: f32,
        radius_shift: f32,
    ) {
        let old_hyperboloids =
            std::mem::replace(&mut self.hyperboloid_helices, Vec::with_capacity(nb_helix));
        for h_id in old_hyperboloids.iter() {
            self.rm_strand_containing_nucl(&Nucl {
                helix: *h_id,
                position: 0,
                forward: true,
            });
            self.rm_strand_containing_nucl(&Nucl {
                helix: *h_id,
                position: 0,
                forward: false,
            });
            self.design.helices.remove(&h_id);
            self.update_status = true;
            self.hash_maps_update = true;
            self.view_need_reset = true;
        }
        self.hyperboloid_helices.clear();
        if let Some(descr) = self.hyperboloid_draft.as_mut().map(|d| &mut d.grid_type) {
            *descr = GridTypeDescr::Hyperboloid {
                radius: nb_helix,
                shift,
                length,
                radius_shift,
                forced_radius: None,
            };
        }
        self.make_hyperboloid_helices();
    }

    fn make_hyperboloid_helices(&mut self) {
        if let Some(GridTypeDescr::Hyperboloid {
            radius,
            length,
            shift,
            radius_shift,
            forced_radius,
        }) = self.hyperboloid_draft.map(|h| h.grid_type)
        {
            let hyperboloid = Hyperboloid {
                radius,
                length,
                shift,
                radius_shift,
                forced_radius,
            };
            let parameters = self.design.parameters.unwrap_or_default();
            let (helices, nb_nucl) = hyperboloid.make_helices(&parameters);
            let nb_nucl = nb_nucl.min(5000);
            let mut key = self.design.helices.keys().max().map(|m| m + 1).unwrap_or(0);
            let orientation = self.hyperboloid_draft.as_ref().unwrap().orientation;
            for (i, mut h) in helices.into_iter().enumerate() {
                let origin = hyperboloid.origin_helix(&parameters, i as isize, 0);
                let z_vec = Vec3::unit_z().rotated_by(orientation);
                let y_vec = Vec3::unit_y().rotated_by(orientation);
                h.position = self.hyperboloid_draft.as_ref().unwrap().position
                    + origin.x * z_vec
                    + origin.y * y_vec;
                h.orientation = self.hyperboloid_draft.as_ref().unwrap().orientation
                    * hyperboloid.orientation_helix(&parameters, i as isize, 0);
                self.design.helices.insert(key, h);
                for b in [true, false].iter() {
                    let new_key = self.add_strand(key, -(nb_nucl as isize) / 2, *b);
                    if let icednano::Domain::HelixDomain(ref mut dom) =
                        self.design.strands.get_mut(&new_key).unwrap().domains[0]
                    {
                        dom.end = dom.start + nb_nucl as isize;
                    }
                }
                self.hyperboloid_helices.push(key);
                key += 1;
            }
        }

        self.update_status = true;
        self.make_hash_maps();
    }

    pub fn clear_hyperboloid(&mut self) {
        let nb_helix = self.hyperboloid_helices.len();
        let old_hyperboloids =
            std::mem::replace(&mut self.hyperboloid_helices, Vec::with_capacity(nb_helix));
        for h_id in old_hyperboloids.iter() {
            self.rm_strand_containing_nucl(&Nucl {
                helix: *h_id,
                position: 0,
                forward: true,
            });
            self.rm_strand_containing_nucl(&Nucl {
                helix: *h_id,
                position: 0,
                forward: false,
            });
            self.design.helices.remove(&h_id);
            self.update_status = true;
            self.hash_maps_update = true;
            self.view_need_reset = true;
        }
        self.view_need_reset = true;
    }

    pub fn finalize_hyperboloid(&mut self) {
        if let Some(draft) = self.hyperboloid_draft.take() {
            let g_id = self.add_grid(draft);
            for (i, h_id) in self.hyperboloid_helices.iter().enumerate() {
                if let Some(h) = self.design.helices.get_mut(h_id) {
                    h.grid_position = Some(GridPosition {
                        grid: g_id,
                        x: i as isize,
                        y: 0,
                        axis_pos: 0,
                        roll: 0f32,
                    })
                }
            }
        }
        self.hyperboloid_helices.clear();
        self.update_grids();
        self.grid_manager.update(&mut self.design);
        self.hash_maps_update = true;
        self.view_need_reset = true;
        self.update_status = true;
    }

    /// Create a new data by reading a file. At the moment, the supported format are
    /// * codenano
    /// * icednano
    pub fn new_with_path(json_path: &PathBuf) -> Option<Self> {
        let mut xover_ids: IdGenerator<(Nucl, Nucl)> = Default::default();
        let mut design = read_file(json_path)?;
        design.update_version();
        design.remove_empty_domains();
        for s in design.strands.values_mut() {
            s.read_junctions(&mut xover_ids, true);
        }
        for s in design.strands.values_mut() {
            s.read_junctions(&mut xover_ids, false);
        }
        let mut grid_manager = GridManager::new_from_design(&design);
        let mut grids = grid_manager.grids2d();
        for g in grids.iter_mut() {
            g.write().unwrap().update(&design);
        }
        grid_manager.update(&mut design);
        let color_idx = design.strands.keys().len();
        let groups = design.groups.clone();
        let anchors = design.anchors.clone();
        let file_name = real_name(json_path);

        let mut ret = Self {
            design,
            file_name,
            last_backup_time: None,
            object_type: HashMap::default(),
            space_position: HashMap::default(),
            identifier_nucl: HashMap::default(),
            identifier_bound: HashMap::default(),
            nucleotides_involved: HashMap::default(),
            nucleotide: HashMap::default(),
            strand_map: HashMap::default(),
            helix_map: HashMap::default(),
            color: HashMap::default(),
            update_status: false,
            // false because we call make_hash_maps here
            hash_maps_update: false,
            basis_map: Default::default(),
            grid_manager,
            grids,
            color_idx,
            view_need_reset: false,
            groups: Arc::new(RwLock::new(groups)),
            red_cubes: HashMap::default(),
            blue_cubes: HashMap::default(),
            blue_nucl: vec![],
            roller_ptrs: None,
            hyperboloid_helices: vec![],
            hyperboloid_draft: None,
            template_manager: Default::default(),
            xover_copy_manager: Default::default(),
            rigid_body_ptr: None,
            helix_simulation_ptr: None,
            rigid_helix_simulator: None,
            anchors,
            elements_update: None,
            visible: Default::default(),
            visibility_sieve: None,
            xover_ids,
            prime3_set: Default::default(),
        };
        ret.make_hash_maps();
        ret.terminate_movement();
        Some(ret)
    }

    /// Update all the hash maps
    fn make_hash_maps(&mut self) {
        let mut object_type = HashMap::default();
        let mut space_position = HashMap::default();
        let mut identifier_nucl = HashMap::default();
        let mut identifier_bound = HashMap::default();
        let mut nucleotides_involved = HashMap::default();
        let mut nucleotide = HashMap::default();
        let mut strand_map = HashMap::default();
        let mut color_map = HashMap::default();
        let mut helix_map = HashMap::default();
        let mut basis_map = HashMap::default();
        let mut id = 0u32;
        let mut nucl_id;
        let mut old_nucl: Option<Nucl> = None;
        let mut old_nucl_id = None;
        let mut blue_cubes = HashMap::default();
        let mut red_cubes = HashMap::default();
        let mut elements = Vec::new();
        let mut prime3_set = Vec::new();
        self.blue_nucl.clear();
        let groups = self.groups.read().unwrap();
        for (s_id, strand) in self.design.strands.iter_mut() {
            elements.push(elements::DnaElement::Strand { id: *s_id });
            let mut strand_position = 0;
            let strand_seq = strand.sequence.as_ref().filter(|s| s.is_ascii());
            let color = strand.color;
            let mut last_xover_junction: Option<&mut DomainJunction> = None;
            for (i, domain) in strand.domains.iter().enumerate() {
                if let Some((prime5, prime3)) = old_nucl.clone().zip(domain.prime5_end()) {
                    Self::update_junction(
                        &mut self.xover_ids,
                        *last_xover_junction
                            .as_mut()
                            .expect("Broke Invariant [LastXoverJunction]"),
                        (prime5, prime3),
                    );
                    if let Some(id) = self.xover_ids.get_id(&(prime5, prime3)) {
                        elements.push(DnaElement::CrossOver {
                            xover_id: id,
                            helix5prime: prime5.helix,
                            position5prime: prime5.position,
                            forward5prime: prime5.forward,
                            helix3prime: prime3.helix,
                            position3prime: prime3.position,
                            forward3prime: prime3.forward,
                        });
                    }
                }
                if let icednano::Domain::HelixDomain(domain) = domain {
                    let dom_seq = domain.sequence.as_ref().filter(|s| s.is_ascii());
                    for (dom_position, nucl_position) in domain.iter().enumerate() {
                        let position = self.design.helices[&domain.helix].space_pos(
                            self.design.parameters.as_ref().unwrap(),
                            nucl_position,
                            domain.forward,
                        );
                        let nucl = Nucl {
                            position: nucl_position,
                            forward: domain.forward,
                            helix: domain.helix,
                        };
                        elements.push(DnaElement::Nucleotide {
                            helix: nucl.helix,
                            position: nucl.position,
                            forward: nucl.forward,
                        });
                        nucl_id = id;
                        id += 1;
                        object_type.insert(nucl_id, ObjectType::Nucleotide(nucl_id));
                        nucleotide.insert(nucl_id, nucl);
                        identifier_nucl.insert(nucl, nucl_id);
                        strand_map.insert(nucl_id, *s_id);
                        color_map.insert(nucl_id, color);
                        helix_map.insert(nucl_id, nucl.helix);
                        let basis = dom_seq
                            .as_ref()
                            .and_then(|s| s.as_bytes().get(dom_position))
                            .or_else(|| {
                                strand_seq
                                    .as_ref()
                                    .and_then(|s| s.as_bytes().get(strand_position))
                            });
                        if let Some(basis) = basis {
                            basis_map.insert(nucl, *basis as char);
                        } else {
                            basis_map.remove(&nucl);
                        }
                        strand_position += 1;
                        match groups.get(&nucl.helix) {
                            Some(true) => {
                                let cube = space_to_cube(position.x, position.y, position.z);
                                blue_cubes.entry(cube).or_insert(vec![]).push(nucl.clone());
                                self.blue_nucl.push(nucl);
                            }
                            Some(false) => {
                                let cube = space_to_cube(position.x, position.y, position.z);
                                red_cubes.entry(cube).or_insert(vec![]).push(nucl.clone());
                            }
                            None => (),
                        }
                        let position = [position[0] as f32, position[1] as f32, position[2] as f32];
                        space_position.insert(nucl_id, position);
                        if let Some(old_nucl) = old_nucl.take() {
                            let bound_id = id;
                            id += 1;
                            let bound = (old_nucl, nucl);
                            object_type
                                .insert(bound_id, ObjectType::Bound(old_nucl_id.unwrap(), nucl_id));
                            identifier_bound.insert(bound, bound_id);
                            nucleotides_involved.insert(bound_id, bound);
                            color_map.insert(bound_id, color);
                            strand_map.insert(bound_id, *s_id);
                            helix_map.insert(bound_id, nucl.helix);
                        }
                        old_nucl = Some(nucl);
                        old_nucl_id = Some(nucl_id);
                    }
                    if strand.junctions.len() <= i {
                        println!("{:?}", strand.domains);
                        println!("{:?}", strand.junctions);
                    }
                    last_xover_junction = Some(&mut strand.junctions[i]);
                } else if let icednano::Domain::Insertion(n) = domain {
                    strand_position += n;
                    last_xover_junction = Some(&mut strand.junctions[i]);
                }
            }
            if strand.cyclic {
                let nucl = strand.get_5prime().unwrap();
                let prime5_id = identifier_nucl.get(&nucl).unwrap();
                let bound_id = id;
                id += 1;
                let bound = (old_nucl.unwrap(), nucl);
                object_type.insert(
                    bound_id,
                    ObjectType::Bound(old_nucl_id.unwrap(), *prime5_id),
                );
                identifier_bound.insert(bound, bound_id);
                nucleotides_involved.insert(bound_id, bound);
                color_map.insert(bound_id, color);
                strand_map.insert(bound_id, *s_id);
                helix_map.insert(bound_id, nucl.helix);
                println!("adding {:?}, {:?}", bound.0, bound.1);
                Self::update_junction(
                    &mut self.xover_ids,
                    strand
                        .junctions
                        .last_mut()
                        .expect("Broke Invariant [LastXoverJunction]"),
                    (bound.0, bound.1),
                );
                let (prime5, prime3) = bound;
                if let Some(id) = self.xover_ids.get_id(&(prime5, prime3)) {
                    elements.push(DnaElement::CrossOver {
                        xover_id: id,
                        helix5prime: prime5.helix,
                        position5prime: prime5.position,
                        forward5prime: prime5.forward,
                        helix3prime: prime3.helix,
                        position3prime: prime3.position,
                        forward3prime: prime3.forward,
                    });
                }
            } else {
                if let Some(nucl) = old_nucl {
                    let position_start = self.design.helices[&nucl.helix].space_pos(
                        self.design.parameters.as_ref().unwrap(),
                        nucl.position,
                        nucl.forward,
                    );
                    let position_end = self.design.helices[&nucl.helix].space_pos(
                        self.design.parameters.as_ref().unwrap(),
                        nucl.prime3().position,
                        nucl.forward,
                    );
                    let color = strand.color;
                    prime3_set.push((position_start, position_end, color));
                }
            }
            old_nucl = None;
            old_nucl_id = None;
        }
        self.object_type = object_type;
        self.nucleotide = nucleotide;
        self.nucleotides_involved = nucleotides_involved;
        self.identifier_nucl = identifier_nucl;
        self.identifier_bound = identifier_bound;
        self.strand_map = strand_map;
        self.space_position = space_position;
        self.color = color_map;
        self.helix_map = helix_map;
        *self.basis_map.write().unwrap() = basis_map;
        self.red_cubes = red_cubes;
        self.blue_cubes = blue_cubes;
        self.prime3_set = prime3_set;
        for (h_id, h) in self.design.helices.iter() {
            elements.push(DnaElement::Helix {
                id: *h_id,
                group: groups.get(h_id).cloned(),
                visible: h.visible,
            });
        }
        drop(groups);
        for g_id in 0..self.grid_manager.grids.len() {
            elements.push(DnaElement::Grid {
                id: g_id,
                visible: self.grid_manager.get_visibility(g_id),
            })
        }
        self.read_scaffold_seq(self.design.scaffold_shift.unwrap_or(0));
        self.elements_update = Some(elements);
        self.update_visibility();
        if crate::MUST_TEST {
            self.test_named_junction("TEST AFTER MAKE HASH MAP");
        }
    }

    fn update_junction(
        xover_ids: &mut IdGenerator<(Nucl, Nucl)>,
        junction: &mut DomainJunction,
        bound: (Nucl, Nucl),
    ) {
        let is_xover = bound.0.prime3() != bound.1;
        match junction {
            DomainJunction::Adjacent if is_xover => {
                panic!("DomainJunction::Adjacent between non adjacent nucl")
            }
            DomainJunction::UnindentifiedXover | DomainJunction::IdentifiedXover(_)
                if !is_xover =>
            {
                panic!("Xover between adjacent nucls")
            }
            s @ DomainJunction::UnindentifiedXover => {
                let id = xover_ids.insert(bound);
                *s = DomainJunction::IdentifiedXover(id);
            }
            DomainJunction::IdentifiedXover(id) => {
                let old_bound = xover_ids.get_element(*id);
                if old_bound != Some(bound) {
                    xover_ids.update(old_bound.expect("Could not get exisiting id"), bound);
                }
            }
            _ => (),
        }
    }

    fn read_scaffold_seq(&mut self, shift: usize) {
        let nb_skip = if let Some(sequence) = self.design.scaffold_sequence.as_ref() {
            if sequence.len() == 0 {
                return;
            }
            sequence.len() - (shift % sequence.len())
        } else {
            return;
        };
        if let Some(mut sequence) = self
            .design
            .scaffold_sequence
            .as_ref()
            .map(|s| s.chars().cycle().skip(nb_skip))
        {
            let mut basis_map = self.basis_map.read().unwrap().clone();
            if let Some(strand) = self
                .design
                .scaffold_id
                .as_ref()
                .and_then(|s_id| self.design.strands.get(s_id))
            {
                for domain in &strand.domains {
                    if let icednano::Domain::HelixDomain(dom) = domain {
                        for nucl_position in dom.iter() {
                            let nucl = Nucl {
                                helix: dom.helix,
                                position: nucl_position,
                                forward: dom.forward,
                            };
                            let basis = sequence.next();
                            let basis_compl = compl(basis);
                            if let Some((basis, basis_compl)) = basis.zip(basis_compl) {
                                basis_map.insert(nucl, basis);
                                if self.identifier_nucl.contains_key(&nucl.compl()) {
                                    basis_map.insert(nucl.compl(), basis_compl);
                                }
                            }
                        }
                    } else if let icednano::Domain::Insertion(n) = domain {
                        for _ in 0..*n {
                            sequence.next();
                        }
                    }
                }
            }
            *self.basis_map.write().unwrap() = basis_map;
        }
    }

    /// Set the strand that is the scaffold. If the scaffold has changed, the color of the strand
    /// that previously was the scaffold is modified.
    /// The new scaffold's color is set to blue
    pub fn set_scaffold_id(&mut self, scaffold_id: Option<usize>) {
        if let Some(s_id) = self.design.scaffold_id {
            if let Some(strand) = self.design.strands.get_mut(&s_id) {
                let color = new_color(&mut self.color_idx);
                strand.color = color;
            }
        }
        self.design.scaffold_id = scaffold_id;
        if let Some(strand) = scaffold_id
            .as_ref()
            .and_then(|s_id| self.design.strands.get_mut(s_id))
        {
            strand.color = crate::consts::SCAFFOLD_COLOR;
        }
        self.hash_maps_update = true;
        self.update_status = true;
        self.design.scaffold_shift = None;
    }

    /// Set the sequence of the scaffold
    pub fn set_scaffold_sequence(&mut self, sequence: String, shift: usize) {
        self.design.scaffold_sequence = Some(sequence);
        self.design.scaffold_shift = Some(shift);
        self.hash_maps_update = true;
    }

    pub fn set_scaffold_shift(&mut self, shift: usize) {
        self.design.scaffold_shift = Some(shift);
        self.update_status = true;
        if self.design.scaffold_sequence.is_some() {
            self.hash_maps_update = true;
        }
    }

    pub fn request_save(&mut self, path: &PathBuf) -> std::io::Result<()> {
        self.file_name = real_name(path);
        self.save_file(path)
    }

    fn backup_save(&mut self) {
        let name = backup_name(&self.file_name);
        if self.save_file(&name).is_err() {
            println!("could not save backup");
        }
    }

    fn before_simul_save(&mut self) {
        let name = before_simul_name(&self.file_name);
        if self.save_file(&name).is_err() {
            println!("could not save before saving");
        }
    }

    /// Save the design to a file in the `icednano` format
    pub fn save_file(&mut self, path: &PathBuf) -> std::io::Result<()> {
        self.design.anchors = self.anchors.clone();
        self.design.groups = self.groups.read().unwrap().clone();
        self.design.no_phantoms = self.grid_manager.no_phantoms.clone();
        self.design.small_spheres = self.grid_manager.small_spheres.clone();
        let json_content = serde_json::to_string_pretty(&self.design);
        let mut f = std::fs::File::create(path)?;
        f.write_all(json_content.expect("serde_json failed").as_bytes())
    }

    /// Return true if self was updated since the last time this function was called.
    /// This function is meant to be called by the mediator that will notify all the obeservers
    /// that a update took place.
    pub fn was_updated(&mut self) -> bool {
        if !self.read_rigid_helix_update() {
            self.check_rigid_body();
            self.check_rigid_helices();
            let mut stop_rolling = false;
            if let Some((_, snd_ptr, date)) = self.roller_ptrs.as_mut() {
                let now = Instant::now();
                if (now - *date).as_millis() > 30 {
                    let (snd, rcv) = std::sync::mpsc::channel();
                    *snd_ptr.lock().unwrap() = Some(snd);
                    let helices = rcv.recv().unwrap();
                    for (n, h) in self.design.helices.values_mut().enumerate() {
                        *h = helices[n].clone();
                    }
                    *date = now;
                    self.hash_maps_update = true;
                    self.update_status = true;
                    stop_rolling = true;
                }
            }
            if stop_rolling {
                self.stop_rolling()
            }
            if self.hash_maps_update {
                self.make_hash_maps();
                self.hash_maps_update = false;
            }
        } else {
            self.update_status = true;
        }
        let ret = self.update_status;
        self.update_status = false;
        if let Some(time) = self.last_backup_time {
            if (Instant::now() - time).as_secs() > 30 {
                self.last_backup_time = None;
                self.backup_save();
            }
        } else if ret {
            self.last_backup_time = Some(Instant::now());
        }
        ret
    }

    pub fn roll_request(&mut self, request: SimulationRequest, computing: Arc<Mutex<bool>>) {
        if self.roller_ptrs.is_some() {
            self.stop_rolling()
        } else {
            self.start_rolling(request, computing)
        }
    }

    pub fn get_simulation_state(&self) -> SimulationState {
        if self.roller_ptrs.is_some() {
            SimulationState::Rolling
        } else if self.rigid_helix_simulator.is_some() {
            SimulationState::RigidHelices
        } else if self.rigid_body_ptr.is_some() {
            SimulationState::RigidGrid
        } else {
            SimulationState::None
        }
    }

    pub fn get_xovers_list(&self) -> Vec<(usize, (Nucl, Nucl))> {
        self.xover_ids.get_all_elements()
    }

    fn start_rolling(&mut self, request: SimulationRequest, computing: Arc<Mutex<bool>>) {
        let xovers = self.design.get_xovers();
        let helices: Vec<Helix> = self.design.helices.values().cloned().collect();
        let keys: Vec<usize> = self.design.helices.keys().cloned().collect();
        let intervals = self.design.get_intervals();
        let physical_system = PhysicalSystem::from_design(
            keys,
            helices,
            xovers,
            self.design.parameters.unwrap_or_default().clone(),
            intervals,
            request.roll,
            request.springs,
            request.target_helices,
        );
        let date = Instant::now();
        let (stop, snd) = physical_system.run(computing);
        self.roller_ptrs = Some((stop, snd, date));
    }

    fn stop_rolling(&mut self) {
        if let Some((stop, _, _)) = self.roller_ptrs.as_mut() {
            *stop.lock().unwrap() = true;
        } else {
            println!("design was not rolling");
        }
        self.roller_ptrs = None;
    }

    pub fn view_need_reset(&mut self) -> bool {
        std::mem::replace(&mut self.view_need_reset, false)
    }

    /// Return the position of a nucleotide, this function is only used internally. The
    /// corresponding public methods is `Data::get_element_position`.
    fn get_space_pos(&self, nucl: &Nucl) -> Option<[f32; 3]> {
        let id = self.identifier_nucl.get(nucl);
        if let Some(ref id) = id {
            if let Some(position) = self.space_position.get(id) {
                Some(*position)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Return the position of an element.
    /// If the element is a nucleotide, return the center of the nucleotide.
    /// If the element is a bound, return the middle of the segment between the two nucleotides
    /// involved in the bound.
    pub fn get_element_position(&mut self, id: u32) -> Option<Vec3> {
        if let Some(object_type) = self.object_type.get(&id) {
            match object_type {
                ObjectType::Nucleotide(id) => self.space_position.get(&id).map(|x| x.into()),
                ObjectType::Bound(_, _) => {
                    let (nucl_a, nucl_b) = self.nucleotides_involved.get(&id)?;
                    let a = self.get_space_pos(nucl_a)?;
                    let b = self.get_space_pos(nucl_b)?;
                    Some((Vec3::from(a) + Vec3::from(b)) / 2.)
                }
            }
        } else {
            None
        }
    }

    /// Get the position of an element, projected on the Helix on which it lies.
    pub fn get_element_axis_position(&mut self, id: u32) -> Option<Vec3> {
        if let Some(object_type) = self.object_type.get(&id) {
            match object_type {
                ObjectType::Nucleotide(id) => {
                    let nucl = self.nucleotide.get(id)?;
                    self.get_axis_pos(*nucl)
                }
                ObjectType::Bound(_, _) => {
                    let (nucl_a, nucl_b) = self.nucleotides_involved.get(&id)?;
                    let a = self.get_axis_pos(*nucl_a)?;
                    let b = self.get_axis_pos(*nucl_b)?;
                    Some((a + b) / 2.)
                }
            }
        } else {
            None
        }
    }

    fn get_axis_pos(&self, nucl: Nucl) -> Option<Vec3> {
        self.design
            .helices
            .get(&nucl.helix)
            .map(|h| h.axis_position(self.design.parameters.as_ref().unwrap(), nucl.position))
    }

    /// Get the nucleotide corresponding to an identifier
    pub fn get_nucl(&self, e_id: u32) -> Option<Nucl> {
        self.nucleotide.get(&e_id).cloned()
    }

    pub fn get_bound_5prime(&self, e_id: u32) -> Option<Nucl> {
        self.nucleotides_involved.get(&e_id).map(|b| b.0)
    }

    /// Get the position of a nucleotide, eventually projected on the axis of the helix that
    /// supports it.
    pub fn get_helix_nucl(&self, nucl: Nucl, on_axis: bool) -> Option<Vec3> {
        self.design.helices.get(&nucl.helix).map(|h| {
            if on_axis {
                h.axis_position(&self.design.parameters.unwrap(), nucl.position)
            } else {
                h.space_pos(
                    &self.design.parameters.unwrap(),
                    nucl.position,
                    nucl.forward,
                )
            }
        })
    }

    /// Return the ObjectType associated to the identifier `id`
    pub fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.object_type.get(&id).cloned()
    }

    /// Return the color of the element with identifier `id`
    pub fn get_color(&self, id: u32) -> Option<u32> {
        let strand = self.strand_map.get(&id)?;
        self.design.strands.get(strand).map(|s| s.color)
    }

    /// Return an iterator over all the identifier of elements that are nucleotides
    pub fn get_all_nucl_ids<'a>(&'a mut self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotide.keys().copied()
    }

    /// Return an iterator over all the identifier of elements that are nucleotides on a visible
    /// helix
    pub fn get_all_visible_nucl_ids(&self) -> Vec<u32> {
        self.nucleotide
            .iter()
            .filter(|(_, n)| self.is_visible(*n))
            .map(|(k, _)| *k)
            .collect()
    }

    /// Return an iterator over all the identifier of elements that are bounds
    pub fn get_all_bound_ids<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotides_involved.keys().copied()
    }

    /// Return a vector of all the identifier of elements that are bounds between two
    /// nucleotides among who at least one is visible
    pub fn get_all_visible_bound_ids(&self) -> Vec<u32> {
        self.nucleotides_involved
            .iter()
            .filter(|(_, b)| self.is_visible(&b.0) && self.is_visible(&b.1))
            .map(|(k, _)| *k)
            .collect()
    }

    /// Return the identifier of the strand on which an element lies
    pub fn get_strand_of_element(&self, id: u32) -> Option<usize> {
        self.strand_map.get(&id).cloned()
    }

    /// Return the identifier of the helix on which an element lies
    pub fn get_helix_of_element(&self, id: u32) -> Option<usize> {
        self.helix_map.get(&id).cloned()
    }

    /// Return all the elements of a strand
    pub fn get_strand_elements(&self, s_id: usize) -> Vec<u32> {
        let mut ret = Vec::new();
        for elt in self.object_type.keys() {
            if self.strand_map.get(&elt) == Some(&s_id) {
                ret.push(*elt)
            }
        }
        ret
    }

    pub fn get_strand_length(&self, s_id: usize) -> Option<usize> {
        self.design.strands.get(&s_id).map(|s| s.length())
    }

    /// Return all the elements that lie on an helix
    pub fn get_helix_elements(&self, h_id: usize) -> Vec<u32> {
        let mut ret = Vec::new();
        for elt in self.object_type.keys() {
            if self.helix_map.get(&elt) == Some(&h_id) {
                ret.push(*elt)
            }
        }
        ret
    }

    /// Change the color of a strand
    pub fn change_strand_color(&mut self, s_id: usize, color: u32) {
        if let Some(strand) = self.design.strands.get_mut(&s_id) {
            self.color.insert(s_id as u32, color);
            strand.color = color;
        } else {
            println!("Warning tried to change color of removed strand");
        }
        self.update_status = true;
    }

    /// Change the color of a strand
    pub fn change_strand_sequence(&mut self, s_id: usize, sequence: String) {
        self.design
            .strands
            .get_mut(&s_id)
            .expect("wrong s_id in change_strand_color")
            .sequence = Some(std::borrow::Cow::Owned(sequence));
        self.update_status = true;
        self.hash_maps_update = true;
    }

    pub fn get_strand_color(&self, s_id: usize) -> Option<u32> {
        self.design.strands.get(&s_id).map(|s| s.color)
    }

    pub fn get_strand_sequence(&self, s_id: usize) -> Option<String> {
        self.design.strands.get(&s_id).map(|s| {
            s.sequence
                .as_ref()
                .unwrap_or(&std::borrow::Cow::Owned(String::new()))
                .to_string()
        })
    }

    pub fn translate_grid(&mut self, g_id: usize, translation: Vec3) {
        self.grid_manager.translate_grid(g_id, translation);
        self.grid_manager.update(&mut self.design);
        self.hash_maps_update = true;
        self.update_status = true;
    }

    /// Apply `rotation` on helix `h_id` arround `origin`. `rotation` and `origin` must be
    /// expressed in the model coordinates
    pub fn rotate_helix_arround(
        &mut self,
        h_id: usize,
        rotation: ultraviolet::Rotor3,
        origin: Vec3,
    ) {
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.rotate_arround(rotation, origin))
            .unwrap_or_default();
        self.grid_manager
            .reattach_helix(h_id, &mut self.design, false, &self.grids);
        self.grid_manager.update(&mut self.design);
        self.hash_maps_update = true;
        self.update_status = true;
    }

    /// Attempt to translate an helix, and return true iff the translation was effectively
    /// perfored.
    ///
    /// If snap_grid is true, the helix is reattached to its grid after the translation. This
    /// attachement can fail (when there is already an existing helix) in this case no translation
    /// is performed, and this function return false. If the translation is performed return true.
    pub fn translate_helix(&mut self, h_id: usize, translation: Vec3, snap_grid: bool) -> bool {
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.translate(translation));
        let mut ret = true;
        if snap_grid {
            let successfull_reattach =
                self.grid_manager
                    .reattach_helix(h_id, &mut self.design, true, &self.grids);
            if !successfull_reattach {
                //self.design.helices.get_mut(&h_id).map(|h| h.cancel_current_movement());
                ret = false;
            }
        }
        self.grid_manager.update(&mut self.design);
        self.update_grids();
        self.hash_maps_update = true;
        self.update_status = true;
        ret
    }

    pub fn rotate_grid_arround(
        &mut self,
        g_id: usize,
        rotation: ultraviolet::Rotor3,
        origin: Vec3,
    ) {
        self.grid_manager
            .rotate_grid_arround(g_id, rotation, origin);
        self.grid_manager.update(&mut self.design);
        self.hash_maps_update = true;
        self.update_status = true;
    }

    /// End current movement. This means that the old_matrices take the value of the current ones.
    pub fn terminate_movement(&mut self) {
        for helix in self.design.helices.values_mut() {
            helix.end_movement();
        }
        self.grid_manager.terminate_movement();
    }

    /// Return the orientation of an helix. (`None` if the helix id does not exists)
    pub fn get_helix_basis(&self, h_id: usize) -> Option<ultraviolet::Rotor3> {
        self.design.helices.get(&h_id).map(|h| {
            if let Some(grid_pos) = h.grid_position {
                self.get_grid_basis(grid_pos.grid).unwrap()
            } else {
                h.orientation
            }
        })
    }

    /// Return the identifier of the 5' nucleotide of a strand.
    pub fn get_5prime(&self, strand_id: usize) -> Option<u32> {
        let nucl = self
            .design
            .strands
            .get(&strand_id)
            .and_then(|s| s.get_5prime())?;
        self.identifier_nucl.get(&nucl).cloned()
    }

    /// Return the identifier of the 3' nucleotide of a strand.
    pub fn get_3prime(&self, strand_id: usize) -> Option<u32> {
        let nucl = self
            .design
            .strands
            .get(&strand_id)
            .and_then(|s| s.get_3prime())?;
        self.identifier_nucl.get(&nucl).cloned()
    }

    /// Return the identifier of a nucleotide
    pub fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32> {
        self.identifier_nucl.get(nucl).cloned()
    }

    pub fn get_identifier_bound(&self, n1: &Nucl, n2: &Nucl) -> Option<u32> {
        self.identifier_bound.get(&(*n1, *n2)).cloned()
    }

    /// Return a NeighbourDescriptor describing the domain on which a nucleotide lies ; or `None`
    /// if the nucleotide position is empty.
    pub fn get_neighbour_nucl(&self, nucl: Nucl) -> Option<NeighbourDescriptor> {
        self.design.get_neighbour_nucl(nucl)
    }

    /// Move one end of a domain. This function requires that one end of the domain is
    /// `fixed_position`. The other end is moved to `position`.
    pub fn update_strand(
        &mut self,
        identifier: DomainIdentifier,
        position: isize,
        fixed_position: isize,
    ) {
        if self.roller_ptrs.is_some()
            || self.rigid_helix_simulator.is_some()
            || self.rigid_body_ptr.is_some()
        {
            return;
        }
        let start = position.min(fixed_position);
        let end = position.max(fixed_position) + 1;
        let domain = &mut self
            .design
            .strands
            .get_mut(&identifier.strand)
            .unwrap()
            .domains[identifier.domain];
        if let icednano::Domain::HelixDomain(domain) = domain {
            assert!(domain.start == fixed_position || domain.end - 1 == fixed_position);
            domain.start = start;
            domain.end = end;
        }
        self.hash_maps_update = true;
        self.update_status = true;
    }

    /// Return a `StrandBuilder` with moving end `nucl` if possible. To create a
    /// `StrandBuilder` with moving end `nucl` one of the following must be true
    ///
    /// * `nucl` is an end of an existing domain. In this case the `StrandBuilder` will be edditing
    /// that domain.
    ///
    /// * The position `nucl` is empty *and at least one of the neighbour position (`nucl.left()`
    /// or `nucl.right()`) is empty. In this case a new strand is created with one domain, that
    /// will be eddited by the returned `StrandBuilder`.
    ///
    /// If it not possible to create a `StrandBuilder`, `None` is returned.
    pub fn get_strand_builder(&mut self, nucl: Nucl, stick: bool) -> Option<StrandBuilder> {
        if self.roller_ptrs.is_some()
            || self.rigid_helix_simulator.is_some()
            || self.rigid_body_ptr.is_some()
        {
            return None;
        }
        let helix = nucl.helix;
        let position = nucl.position;
        let forward = nucl.forward;
        let left = self.design.get_neighbour_nucl(nucl.left());
        let right = self.design.get_neighbour_nucl(nucl.right());
        let axis = self
            .design
            .helices
            .get(&helix)
            .map(|h| h.get_axis(&self.design.parameters.unwrap()))?;
        if self.identifier_nucl.contains_key(&nucl) {
            if let Some(desc) = self.design.get_neighbour_nucl(nucl) {
                let strand_id = desc.identifier.strand;
                let filter = |d: &NeighbourDescriptor| d.identifier != desc.identifier;
                let neighbour_desc = left.filter(filter).or(right.filter(filter));
                let stick = stick || neighbour_desc.map(|d| d.identifier.strand) == Some(strand_id);
                if left.filter(filter).and(right.filter(filter)).is_some() {
                    // TODO maybe we should do something else ?
                    return None;
                }
                match self.design.strands.get(&strand_id).map(|s| s.length()) {
                    Some(n) if n > 1 => Some(StrandBuilder::init_existing(
                        desc.identifier,
                        nucl,
                        axis,
                        desc.fixed_end,
                        neighbour_desc,
                        stick,
                    )),
                    _ => Some(StrandBuilder::init_empty(
                        DomainIdentifier {
                            strand: strand_id,
                            domain: 0,
                        },
                        nucl,
                        axis,
                        neighbour_desc,
                        false,
                    )),
                }
            } else {
                None
            }
        } else {
            if left.is_some() && right.is_some() {
                return None;
            }
            let new_key = self.add_strand(helix, position, forward);
            Some(StrandBuilder::init_empty(
                DomainIdentifier {
                    strand: new_key,
                    domain: 0,
                },
                nucl,
                axis,
                left.or(right),
                true,
            ))
        }
    }

    fn add_strand(&mut self, helix: usize, position: isize, forward: bool) -> usize {
        let new_key = if let Some(k) = self.design.strands.keys().max() {
            *k + 1
        } else {
            0
        };
        let color = {
            let hue = (self.color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
            let saturation =
                (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
            let value = (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
            let hsv = color_space::Hsv::new(hue, saturation, value);
            let rgb = color_space::Rgb::from(hsv);
            (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
        };
        self.color_idx += 1;

        self.design.strands.insert(
            new_key,
            icednano::Strand::init(helix, position, forward, color),
        );
        self.hash_maps_update = true;
        self.update_status = true;
        new_key
    }

    pub fn remake_strand(&mut self, nucl: Nucl, strand_id: usize, color: u32) {
        self.design.strands.insert(
            strand_id,
            icednano::Strand::init(nucl.helix, nucl.position, nucl.forward, color),
        );
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn get_symbol(&self, e_id: u32) -> Option<char> {
        self.nucleotide.get(&e_id).and_then(|nucl| {
            self.basis_map
                .read()
                .unwrap()
                .get(nucl)
                .cloned()
                .or_else(|| compl(self.basis_map.read().unwrap().get(&nucl.compl()).cloned()))
        })
    }

    pub fn get_symbol_position(&self, e_id: u32) -> Option<Vec3> {
        self.nucleotide
            .get(&e_id)
            .and_then(|nucl| self.get_helix_nucl(*nucl, false))
    }

    pub fn get_strand_points(&self, s_id: usize) -> Option<Vec<Nucl>> {
        let strand = self.design.strands.get(&s_id)?;
        let mut ret = Vec::new();
        for domain in strand.domains.iter() {
            if let icednano::Domain::HelixDomain(domain) = domain {
                if domain.forward {
                    ret.push(Nucl::new(domain.helix, domain.start, domain.forward));
                    ret.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                } else {
                    ret.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                    ret.push(Nucl::new(domain.helix, domain.start, domain.forward));
                }
            }
        }
        if strand.cyclic {
            ret.push(ret[0])
        }
        Some(ret)
    }

    pub fn get_copy_points(&self) -> Vec<Vec<Nucl>> {
        let mut ret = Vec::new();
        for strand in self.template_manager.pasted_strands.iter() {
            let mut points = Vec::new();
            for domain in strand.domains.iter() {
                if let icednano::Domain::HelixDomain(domain) = domain {
                    if domain.forward {
                        points.push(Nucl::new(domain.helix, domain.start, domain.forward));
                        points.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                    } else {
                        points.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                        points.push(Nucl::new(domain.helix, domain.start, domain.forward));
                    }
                }
            }
            ret.push(points)
        }
        ret
    }

    pub fn get_pasted_positions(&self) -> Vec<(Vec<Vec3>, bool)> {
        self.template_manager
            .pasted_strands
            .iter()
            .map(|strand| (strand.nucl_position.clone(), strand.pastable))
            .collect()
    }

    /// Return the identifier of the strand whose nucl is the 5' end of, or `None` if nucl is not
    /// the 5' end of any strand.
    pub fn prime5_of(&self, nucl: &Nucl) -> Option<usize> {
        for (s_id, s) in self.design.strands.iter() {
            if !s.cyclic && s.get_5prime() == Some(*nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    /// Return the identifier of the strand whose nucl is the 3' end of, or `None` if nucl is not
    /// the 3' end of any strand.
    pub fn prime3_of(&self, nucl: &Nucl) -> Option<usize> {
        for (s_id, s) in self.design.strands.iter() {
            if !s.cyclic && s.get_3prime() == Some(*nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    /// Return the xover extremity status of nucl.
    pub fn is_xover_end(&self, nucl: &Nucl) -> Extremity {
        let strand_id = if let Some(id) = self.get_strand_nucl(nucl) {
            id
        } else {
            return Extremity::No;
        };

        let strand = self.design.strands.get(&strand_id).expect("strand");
        let mut prev_helix = None;
        for domain in strand.domains.iter() {
            if domain.prime5_end() == Some(*nucl) && prev_helix != domain.half_helix() {
                return Extremity::Prime5;
            } else if domain.prime3_end() == Some(*nucl) {
                return Extremity::Prime3;
            } else if let Some(_) = domain.has_nucl(nucl) {
                return Extremity::No;
            }
            prev_helix = domain.half_helix();
        }
        return Extremity::No;
    }

    fn is_middle_xover(&self, nucl: &Nucl) -> bool {
        self.is_xover_end(nucl).to_opt().is_some() && self.is_strand_end(nucl).to_opt().is_none()
    }

    /// Return the strand end status of nucl
    pub fn is_strand_end(&self, nucl: &Nucl) -> Extremity {
        for s in self.design.strands.values() {
            if !s.cyclic && s.get_5prime() == Some(*nucl) {
                return Extremity::Prime5;
            } else if !s.cyclic && s.get_3prime() == Some(*nucl) {
                return Extremity::Prime3;
            }
        }
        return Extremity::No;
    }

    /// Merge two strands with identifier prime5 and prime3. The resulting strand will have
    /// identifier prime5.
    pub fn merge_strands(&mut self, prime5: usize, prime3: usize) {
        // We panic, if we can't find the strand, because this means that the program has a bug
        if prime5 != prime3 {
            let strand5prime = self.design.strands.remove(&prime5).expect("strand 5 prime");
            let strand3prime = self.design.strands.remove(&prime3).expect("strand 3 prime");
            let len = strand5prime.domains.len() + strand3prime.domains.len();
            let mut domains = Vec::with_capacity(len);
            let mut junctions = Vec::with_capacity(len);
            for (i, domain) in strand5prime.domains.iter().enumerate() {
                domains.push(domain.clone());
                junctions.push(strand5prime.junctions[i].clone());
            }
            let skip;
            let last_helix = domains.last().and_then(|d| d.half_helix());
            let next_helix = strand3prime
                .domains
                .iter()
                .next()
                .and_then(|d| d.half_helix());
            if last_helix == next_helix && last_helix.is_some() {
                skip = 1;
                domains
                    .last_mut()
                    .as_mut()
                    .unwrap()
                    .merge(strand3prime.domains.iter().next().unwrap());
                junctions.pop();
            } else {
                skip = 0;
                if let Some(j) = junctions.iter_mut().last() {
                    *j = DomainJunction::UnindentifiedXover
                }
            }
            for domain in strand3prime.domains.iter().skip(skip) {
                domains.push(domain.clone());
            }
            for junction in strand3prime.junctions.iter() {
                junctions.push(junction.clone());
            }
            let sequence = if let Some((seq5, seq3)) = strand5prime
                .sequence
                .clone()
                .zip(strand3prime.sequence.clone())
            {
                let new_seq = seq5.into_owned() + &seq3.into_owned();
                Some(Cow::Owned(new_seq))
            } else if let Some(ref seq5) = strand5prime.sequence {
                Some(seq5.clone())
            } else if let Some(ref seq3) = strand3prime.sequence {
                Some(seq3.clone())
            } else {
                None
            };
            let new_strand = icednano::Strand {
                domains,
                color: strand5prime.color,
                sequence,
                junctions,
                cyclic: false,
            };
            self.design.strands.insert(prime5, new_strand);
            self.hash_maps_update = true;
            self.update_status = true;
        } else {
            println!("cycling");
            /*
            self.design
            .strands
            .get_mut(&prime5)
            .as_mut()
            .unwrap()
            .cyclic = true;
            self.hash_maps_update = true;
            self.update_status = true;
            */
        }
        self.view_need_reset = true;
        // TODO UNITTEST
    }

    /// Undo a strand merge
    ///
    /// This methods assumes that the given strand id are those of the strands that were merged
    /// during the operation being undone
    pub fn undo_merge(
        &mut self,
        strand_5prime: Strand,
        strand_3prime: Strand,
        prime5: usize,
        prime3: usize,
    ) {
        self.design.strands.remove(&prime5).expect("undo merge");
        self.design.strands.insert(prime5, strand_5prime);
        self.design.strands.insert(prime3, strand_3prime);
        self.update_status = true;
        self.view_need_reset = true;
        self.hash_maps_update = true;
    }

    /// Make a strand cyclic by linking the 3' and the 5' end, or undo this operation.
    pub fn make_cycle(&mut self, strand_id: usize, cyclic: bool) {
        self.design
            .strands
            .get_mut(&strand_id)
            .expect("Attempt to make non existing strand cyclic")
            .cyclic = cyclic;

        let strand = self.design.strands.get_mut(&strand_id).unwrap();
        if cyclic {
            let first_last_domains = (strand.domains.iter().next(), strand.domains.iter().last());
            let merge_insertions = if let (
                Some(icednano::Domain::Insertion(n1)),
                Some(icednano::Domain::Insertion(n2)),
            ) = first_last_domains
            {
                Some(n1 + n2)
            } else {
                None
            };
            if let Some(n) = merge_insertions {
                // If the strand starts and finishes by an Insertion, merge the insertions.
                // TODO UNITTEST for this specific case
                *strand.domains.last_mut().unwrap() = icednano::Domain::Insertion(n);
                // remove the first insertions
                strand.domains.remove(0);
                strand.junctions.remove(0);
            }

            let first_last_domains = (strand.domains.iter().next(), strand.domains.iter().last());
            let skip_last = if let (_, Some(icednano::Domain::Insertion(_))) = first_last_domains {
                1
            } else {
                0
            };
            let skip_first = if let (Some(icednano::Domain::Insertion(_)), _) = first_last_domains {
                1
            } else {
                0
            };
            let last_first_intervals = (
                strand.domains.iter().rev().skip(skip_last).next(),
                strand.domains.get(skip_first),
            );
            if let (
                Some(icednano::Domain::HelixDomain(i1)),
                Some(icednano::Domain::HelixDomain(i2)),
            ) = last_first_intervals
            {
                use icednano::junction;
                let junction = junction(i1, i2);
                *strand.junctions.last_mut().unwrap() = junction;
            } else {
                panic!("Invariant Violated: SaneDomains")
            }
        } else {
            *strand.junctions.last_mut().unwrap() = DomainJunction::Prime3;
        }

        self.update_status = true;
        self.view_need_reset = true;
        //self.make_hash_maps();
        self.hash_maps_update = true;
    }

    /// Undo a strand split.
    ///
    /// This methods assumes that the strand with highest id was created during the split that is
    /// undone.
    pub fn undo_split(&mut self, strand: Strand, s_id: usize) {
        self.update_status = true;
        self.view_need_reset = true;
        self.design
            .strands
            .remove(&s_id)
            .expect("Removing unexisting strand");
        if !strand.cyclic {
            let other_strand_id = self
                .design
                .strands
                .keys()
                .max()
                .expect("other strand id")
                .clone();
            self.design.strands.remove(&other_strand_id).unwrap();
        }
        self.design.strands.insert(s_id, strand);
        self.make_hash_maps();
    }

    /// Split a strand at nucl, and return the id of the newly created strand
    ///
    /// The part of the strand that contains nucl is given the original
    /// strand's id, the other part is given a new id.
    ///
    /// If `force_end` is `Some(true)`, nucl will be on the 3 prime half of the split.
    /// If `force_end` is `Some(false)` nucl will be on the 5 prime half of the split.
    /// If `force_end` is `None`, nucl will be on the 5 prime half of the split unless nucl is the 3
    /// prime extremity of a crossover, in which case nucl will be on the 3 prime half of the
    /// split.
    pub fn split_strand(&mut self, nucl: &Nucl, force_end: Option<bool>) -> Option<usize> {
        self.update_status = true;
        self.hash_maps_update = true;
        self.view_need_reset = true;
        let id = self.get_strand_nucl(nucl);

        if id.is_none() {
            return None;
        }
        let id = id.unwrap();

        let strand = self.design.strands.remove(&id).expect("strand");
        if strand.cyclic {
            let new_strand = self.break_cycle(strand.clone(), *nucl, force_end);
            self.design.strands.insert(id, new_strand);
            self.clean_domains_one_strand(id);
            //println!("Cutting cyclic strand");
            return Some(id);
        }
        if strand.length() <= 1 {
            // return without putting the strand back
            return None;
        }
        let mut i = strand.domains.len();
        let mut prim5_domains = Vec::new();
        let mut len_prim5 = 0;
        let mut domains = None;
        let mut on_3prime = force_end.unwrap_or(false);
        let mut prev_helix = None;
        let mut prime5_junctions: Vec<DomainJunction> = Vec::new();
        let mut prime3_junctions: Vec<DomainJunction> = Vec::new();
        let mut rm_xover: Option<DomainJunction> = None;

        println!("Spliting");
        println!("{:?}", strand.domains);
        println!("{:?}", strand.junctions);

        for (d_id, domain) in strand.domains.iter().enumerate() {
            if domain.prime5_end() == Some(*nucl)
                && prev_helix != domain.helix()
                && force_end != Some(false)
            {
                // nucl is the 5' end of the next domain so it is the on the 3' end of a xover.
                // nucl is not required to be on the 5' half of the split, so we put it on the 3'
                // half
                on_3prime = true;
                i = d_id;
                if let Some(j) = prime5_junctions.last_mut() {
                    rm_xover = Some(j.clone());
                    *j = DomainJunction::Prime3;
                }
                break;
            } else if domain.prime3_end() == Some(*nucl) && force_end != Some(true) {
                // nucl is the 3' end of the current domain so it is the on the 5' end of a xover.
                // nucl is not required to be on the 3' half of the split, so we put it on the 5'
                // half
                rm_xover = Some(strand.junctions[d_id].clone());
                i = d_id + 1;
                prim5_domains.push(domain.clone());
                len_prim5 += domain.length();
                prime5_junctions.push(DomainJunction::Prime3);
                break;
            } else if let Some(n) = domain.has_nucl(nucl) {
                let n = if force_end == Some(true) { n - 1 } else { n };
                i = d_id;
                len_prim5 += n;
                domains = domain.split(n);
                prime5_junctions.push(DomainJunction::Prime3);
                prime3_junctions.push(strand.junctions[d_id].clone());
                break;
            } else {
                len_prim5 += domain.length();
                prim5_domains.push(domain.clone());
                prime5_junctions.push(strand.junctions[d_id].clone());
            }
            prev_helix = domain.helix();
        }
        if let Some(DomainJunction::IdentifiedXover(id)) = rm_xover {
            self.xover_ids.remove(id);
        }
        let mut prim3_domains = Vec::new();
        if let Some(ref domains) = domains {
            prim5_domains.push(domains.0.clone());
            prim3_domains.push(domains.1.clone());
            i += 1;
        }

        for n in i..strand.domains.len() {
            let domain = &strand.domains[n];
            prim3_domains.push(domain.clone());
            prime3_junctions.push(strand.junctions[n].clone());
        }

        let seq_prim5;
        let seq_prim3;
        if let Some(seq) = strand.sequence {
            let seq = seq.into_owned();
            let chars = seq.chars();
            seq_prim5 = Some(Cow::Owned(chars.clone().take(len_prim5).collect()));
            seq_prim3 = Some(Cow::Owned(chars.clone().skip(len_prim5).collect()));
        } else {
            seq_prim3 = None;
            seq_prim5 = None;
        }

        println!("prime5 {:?}", prim5_domains);
        println!("prime5 {:?}", prime5_junctions);

        println!("prime3 {:?}", prim3_domains);
        println!("prime3 {:?}", prime3_junctions);
        let strand_5prime = icednano::Strand {
            domains: prim5_domains,
            color: strand.color,
            junctions: prime5_junctions,
            cyclic: false,
            sequence: seq_prim5,
        };

        let strand_3prime = icednano::Strand {
            domains: prim3_domains,
            color: strand.color,
            cyclic: false,
            junctions: prime3_junctions,
            sequence: seq_prim3,
        };
        let new_id = (*self.design.strands.keys().max().unwrap_or(&0)).max(id) + 1;
        println!("new id {}, ; id {}", new_id, id);
        let (id_5prime, id_3prime) = if !on_3prime {
            (id, new_id)
        } else {
            (new_id, id)
        };
        if strand_5prime.domains.len() > 0 {
            self.design.strands.insert(id_5prime, strand_5prime);
        }
        if strand_3prime.domains.len() > 0 {
            self.design.strands.insert(id_3prime, strand_3prime);
        }
        self.update_status = true;
        //self.make_hash_maps();
        self.hash_maps_update = true;
        self.view_need_reset = true;

        if crate::MUST_TEST {
            self.test_named_junction("TEST AFTER SPLIT STRAND");
        }
        Some(new_id)
        // TODO UNITTEST
    }

    /// Split a cyclic strand at nucl
    ///
    /// If `force_end` is `Some(true)`, nucl will be the new 5' end of the strand.
    /// If `force_end` is `Some(false)` nucl will be the new 3' end of the strand.
    /// If `force_end` is `None`, nucl will be the new 3' end of the strand unless nucl is the 3'
    /// prime extremity of a crossover, in which case nucl will be the new 5' end of the strand
    fn break_cycle(&mut self, mut strand: Strand, nucl: Nucl, force_end: Option<bool>) -> Strand {
        let mut last_dom = None;
        let mut replace_last_dom = None;
        let mut prev_helix = None;

        let mut junctions: Vec<DomainJunction> = Vec::with_capacity(strand.domains.len());
        let mut rm_xover: Option<DomainJunction> = None;

        for (i, domain) in strand.domains.iter().enumerate() {
            if domain.prime5_end() == Some(nucl)
                && prev_helix != domain.helix()
                && force_end != Some(false)
            {
                last_dom = if i != 0 {
                    Some(i - 1)
                } else {
                    Some(strand.domains.len() - 1)
                };
                rm_xover = Some(strand.junctions[last_dom.unwrap()].clone());

                break;
            } else if domain.prime3_end() == Some(nucl) && force_end != Some(true) {
                last_dom = Some(i);
                rm_xover = Some(strand.junctions[last_dom.unwrap()].clone());
                break;
            } else if let Some(n) = domain.has_nucl(&nucl) {
                let n = if force_end == Some(true) { n - 1 } else { n };
                last_dom = Some(i);
                replace_last_dom = domain.split(n);
            }
            prev_helix = domain.helix();
        }
        let last_dom = last_dom.expect("Could not find nucl in strand");
        let mut new_domains = Vec::new();
        if let Some((_, ref d2)) = replace_last_dom {
            new_domains.push(d2.clone());
            junctions.push(strand.junctions[last_dom].clone());
        }
        for (i, d) in strand.domains.iter().enumerate().skip(last_dom + 1) {
            new_domains.push(d.clone());
            junctions.push(strand.junctions[i].clone());
        }
        for (i, d) in strand.domains.iter().enumerate().take(last_dom) {
            new_domains.push(d.clone());
            junctions.push(strand.junctions[i].clone());
        }

        if let Some((ref d1, _)) = replace_last_dom {
            new_domains.push(d1.clone())
        } else {
            new_domains.push(strand.domains[last_dom].clone())
        }
        junctions.push(DomainJunction::Prime3);

        if let Some(DomainJunction::IdentifiedXover(id)) = rm_xover {
            self.xover_ids.remove(id);
        }

        strand.domains = new_domains;
        strand.cyclic = false;
        strand.junctions = junctions;
        strand
    }

    /// Cut the target strand at nucl and the make a cross over from the source strand to the part
    /// that contains nucl
    pub fn cross_cut(
        &mut self,
        source_strand: usize,
        target_strand: usize,
        nucl: Nucl,
        target_3prime: bool,
    ) {
        let new_id = self.design.strands.keys().max().unwrap() + 1;
        let was_cyclic = self.design.strands.get(&target_strand).unwrap().cyclic;
        //println!("half1 {}, ; half0 {}", new_id, target_strand);
        self.split_strand(&nucl, Some(target_3prime));
        //println!("splitted");

        if !was_cyclic && source_strand != target_strand {
            if target_3prime {
                // swap the position of the two half of the target strands so that the merged part is the
                // new id
                let half0 = self.design.strands.remove(&target_strand).unwrap();
                let half1 = self.design.strands.remove(&new_id).unwrap();
                self.design.strands.insert(new_id, half0);
                self.design.strands.insert(target_strand, half1);
                self.merge_strands(source_strand, new_id);
            } else {
                // if the target strand is the 5' end of the merge, we give the new id to the source
                // strand because it is the one that is lost in the merge.
                let half0 = self.design.strands.remove(&source_strand).unwrap();
                let half1 = self.design.strands.remove(&new_id).unwrap();
                self.design.strands.insert(new_id, half0);
                self.design.strands.insert(source_strand, half1);
                self.merge_strands(target_strand, new_id);
            }
        } else if source_strand == target_strand {
            self.make_cycle(source_strand, true)
        } else {
            if target_3prime {
                self.merge_strands(source_strand, target_strand);
            } else {
                self.merge_strands(target_strand, source_strand);
            }
        }
    }

    /// Undo a cross cut by replacing the strand with id source_id and target id by the original
    /// values
    pub fn undo_cross_cut(
        &mut self,
        source: Strand,
        target: Strand,
        source_id: usize,
        target_id: usize,
    ) {
        let new_id = self.design.strands.keys().max().unwrap().clone();
        if source_id != target_id {
            self.design.strands.insert(source_id, source);
            self.design.strands.insert(target_id, target);
        } else {
            self.design.strands.insert(source_id, source);
            self.design.strands.remove(&new_id);
        }
        self.make_hash_maps();
        self.view_need_reset = true;
    }

    pub fn undoable_rm_strand(&mut self, strand: Strand, strand_id: usize, undo: bool) {
        self.update_status = true;
        self.hash_maps_update = true;
        if undo {
            self.design.strands.insert(strand_id, strand);
        } else {
            for j in strand.junctions.iter() {
                if let DomainJunction::IdentifiedXover(id) = j {
                    self.xover_ids.remove(*id);
                }
            }
            self.design.strands.remove(&strand_id).expect("strand");
        }
        self.view_need_reset = true;
    }

    fn rm_strand(&mut self, strand_id: usize) -> bool {
        let strand = self.design.strands.get(&strand_id).cloned();
        if let Some(strand) = strand {
            for j in strand.junctions.iter() {
                if let DomainJunction::IdentifiedXover(id) = j {
                    self.xover_ids.remove(*id);
                }
            }
            self.design.strands.remove(&strand_id).expect("strand");
            true
        } else {
            false
        }
    }

    pub(super) fn rm_strand_containing_nucl(&mut self, nucl: &Nucl) {
        self.update_status = true;
        self.hash_maps_update = true;
        let id = self
            .identifier_nucl
            .get(nucl)
            .and_then(|id| self.strand_map.get(id));

        if id.is_none() {
            return;
        }
        let id = *id.unwrap();

        self.design.strands.remove(&id).expect("strand");
        self.view_need_reset = true;
    }

    pub fn get_all_strand_ids(&self) -> Vec<usize> {
        self.design.strands.keys().cloned().collect()
    }

    pub fn get_grid_instances(&self, design_id: usize) -> Vec<GridInstance> {
        self.grid_manager.grid_instances(design_id)
    }

    pub fn create_grids(&mut self) {
        let groups = self.find_parallel_helices();
        for g in groups.values() {
            self.grid_manager
                .make_grid_from_helices(&mut self.design, g);
        }
        self.grid_manager.update(&mut self.design);
        self.update_grids();
        self.update_status = true;
        self.hash_maps_update = true;
    }

    pub fn make_grid_from_helices(&mut self, group: &[usize]) {
        self.grid_manager
            .make_grid_from_helices(&mut self.design, group);
        self.grid_manager.update(&mut self.design);
        self.update_grids();
        self.update_status = true;
        self.hash_maps_update = true;
    }

    fn update_grids(&mut self) {
        let mut grids = self.grid_manager.grids2d();
        for g in grids.iter_mut() {
            g.write().unwrap().update(&self.design);
        }
        self.grids = grids;
    }

    pub fn get_grid(&self, id: usize) -> Option<Arc<RwLock<Grid2D>>> {
        self.grids.get(id).cloned()
    }

    pub fn get_helices_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
        self.grids.get(g_id).map(|g| {
            g.read()
                .unwrap()
                .helices()
                .values()
                .cloned()
                .map(|x| x)
                .collect()
        })
    }

    pub fn get_helices_grid_coord(&self, g_id: usize) -> Option<Vec<(isize, isize)>> {
        self.grids
            .get(g_id)
            .map(|g| g.read().unwrap().helices().keys().cloned().collect())
    }

    pub fn get_helices_grid_key_coord(&self, g_id: usize) -> Option<Vec<((isize, isize), usize)>> {
        self.grids.get(g_id).map(|g| {
            g.read()
                .unwrap()
                .helices()
                .iter()
                .map(|(a, b)| (*a, *b))
                .collect()
        })
    }

    pub fn get_helix_grid(&self, g_id: usize, x: isize, y: isize) -> Option<u32> {
        self.grids
            .get(g_id)
            .and_then(|g| g.read().unwrap().helices().get(&(x, y)).map(|x| *x as u32))
    }

    pub fn get_grid_basis(&self, g_id: usize) -> Option<ultraviolet::Rotor3> {
        self.grid_manager
            .grids
            .get(g_id as usize)
            .map(|g| g.orientation)
    }

    pub fn get_grid_position(&self, g_id: usize) -> Option<Vec3> {
        self.grid_manager.grids.get(g_id).map(|g| g.position)
    }

    pub fn get_grid_latice_position(&self, g_id: usize, x: isize, y: isize) -> Option<Vec3> {
        self.grid_manager
            .grids
            .get(g_id)
            .map(|g| g.position_helix(x, y))
    }

    pub fn build_helix_grid(
        &mut self,
        g_id: usize,
        x: isize,
        y: isize,
        position: isize,
        length: usize,
    ) {
        if let Some(grid) = self.grid_manager.grids.get(g_id) {
            if !self.grids[g_id]
                .read()
                .unwrap()
                .helices()
                .contains_key(&(x, y))
            {
                let helix = icednano::Helix::new_on_grid(grid, x, y, g_id);
                let helix_id = self.design.helices.keys().last().unwrap_or(&0) + 1;
                self.design.helices.insert(helix_id, helix);
                if length > 0 {
                    for b in [false, true].iter() {
                        let new_key = self.add_strand(helix_id, position, *b);
                        if let icednano::Domain::HelixDomain(ref mut dom) =
                            self.design.strands.get_mut(&new_key).unwrap().domains[0]
                        {
                            dom.end = dom.start + length as isize;
                        }
                    }
                }
                self.update_status = true;
                self.hash_maps_update = true;
                self.grid_manager.update(&mut self.design);
                self.update_grids();
            }
        }
    }

    /// Add an helix to the design.
    pub fn add_helix(&mut self, helix: &Helix, h_id: usize) {
        if self.design.helices.contains_key(&h_id) {
            println!("WARNING: helix key already exists {}", h_id);
        }
        self.design.helices.insert(h_id, helix.clone());
        self.update_status = true;
        self.hash_maps_update = true;
        self.grid_manager.update(&mut self.design);
        self.update_grids();
    }

    pub fn get_helix(&self, h_id: usize) -> Option<Helix> {
        self.design.helices.get(&h_id).cloned()
    }

    pub fn get_strand(&self, s_id: usize) -> Option<Strand> {
        self.design.strands.get(&s_id).cloned()
    }

    /// Remove an helix containing only two filling strands.
    pub fn rm_full_helix_grid(&mut self, g_id: usize, x: isize, y: isize, position: isize) {
        let h = self.grids[g_id]
            .read()
            .unwrap()
            .helices()
            .get(&(x, y))
            .cloned();
        if let Some(h_id) = h {
            self.rm_strand_containing_nucl(&Nucl {
                helix: h_id,
                position,
                forward: true,
            });
            self.rm_strand_containing_nucl(&Nucl {
                helix: h_id,
                position,
                forward: false,
            });
            self.design.helices.remove(&h_id);
            self.grid_manager.remove_helix(h_id);
            self.update_status = true;
            self.hash_maps_update = true;
            self.grid_manager.update(&mut self.design);
            self.update_grids();
            self.view_need_reset = true;
        }
    }

    pub fn rm_helix_grid(&mut self, g_id: usize, x: isize, y: isize) {
        let h = self.grids[g_id]
            .read()
            .unwrap()
            .helices()
            .get(&(x, y))
            .cloned();
        if let Some(h_id) = h {
            self.design.helices.remove(&h_id);
            self.grid_manager.remove_helix(h_id);
            self.update_status = true;
            self.hash_maps_update = true;
            self.grid_manager.update(&mut self.design);
            self.update_grids();
            self.view_need_reset = true;
        }
    }

    pub fn remove_helix(&mut self, h_id: usize) {
        self.update_status = true;
        self.hash_maps_update = true;
        if !self.helix_is_empty(h_id) {
            println!("WARNING REMOVING HELIX THAT IS NOT EMPTY");
        }
        if let Some(h) = self.design.helices.get(&h_id) {
            if let Some(grid_position) = h.grid_position {
                self.rm_helix_grid(grid_position.grid, grid_position.x, grid_position.y);
                return;
            }
        }
        self.design.helices.remove(&h_id);
        self.view_need_reset = true;
    }

    /// Return false if there exists at least one strand with a domain on helix `h_id`, and false
    /// otherwise.
    pub fn helix_is_empty(&self, h_id: usize) -> bool {
        for strand in self.design.strands.values() {
            for domain in strand.domains.iter() {
                if let icednano::Domain::HelixDomain(domain) = domain {
                    if domain.helix == h_id && domain.start < domain.end {
                        return false;
                    }
                }
            }
        }
        self.design.helices.contains_key(&h_id)
    }

    /// Delete the last grid that was added to the grid manager. This can only be done
    /// if the last grid is empty.
    ///
    /// At the moment this method is only called when the user undo the creation of a grid.
    pub fn delete_last_grid(&mut self) {
        self.grid_manager.delete_last_grid();
        self.update_status = true;
        self.hash_maps_update = true;
        self.grid_manager.update(&mut self.design);
        self.update_grids();
    }

    pub fn add_grid(&mut self, desc: GridDescriptor) -> usize {
        let n = self.grid_manager.add_grid(desc);
        self.update_status = true;
        self.hash_maps_update = true;
        self.grid_manager.update(&mut self.design);
        self.update_grids();
        n
    }

    pub fn get_persistent_phantom_helices(&self) -> HashSet<u32> {
        let mut ret = HashSet::new();
        for g in self.grids.iter() {
            if g.read().unwrap().persistent_phantom {
                for x in g.read().unwrap().helices().values() {
                    ret.insert(*x as u32);
                }
            }
        }
        ret
    }

    pub fn has_persistent_phantom(&self, g_id: &usize) -> bool {
        !self.grid_manager.no_phantoms.contains(g_id)
    }

    pub fn set_persistent_phantom(&mut self, g_id: &usize, persistent: bool) {
        if persistent {
            self.grid_manager.no_phantoms.remove(g_id);
        } else {
            self.grid_manager.no_phantoms.insert(*g_id);
        }
        self.update_grids();
        self.update_status = true;
    }

    pub fn helix_has_small_spheres(&mut self, h_id: &usize) -> bool {
        let helix = self.design.helices.get(h_id);
        if let Some(gp) = helix.and_then(|h| h.grid_position) {
            self.grids[gp.grid].read().unwrap().small_spheres
        } else {
            false
        }
    }

    pub fn has_small_spheres(&mut self, g_id: &usize) -> bool {
        self.grid_manager.small_spheres.contains(g_id)
    }

    pub fn set_small_spheres(&mut self, g_id: &usize, small: bool) {
        if small {
            self.grid_manager.small_spheres.insert(*g_id);
        } else {
            self.grid_manager.small_spheres.remove(g_id);
        }
        self.grids[*g_id as usize].write().unwrap().small_spheres = small;
        self.update_grids();
        self.update_status = true;
    }

    pub fn get_grid_pos_helix(&self, h_id: u32) -> Option<GridPosition> {
        self.design
            .helices
            .get(&(h_id as usize))
            .and_then(|h| h.grid_position)
    }

    pub fn get_isometry_2d(&self, h_id: usize) -> Option<ultraviolet::Isometry2> {
        self.design.helices.get(&h_id).and_then(|h| h.isometry2d)
    }

    pub fn set_isometry_2d(&mut self, h_id: usize, isometry2d: ultraviolet::Isometry2) {
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.isometry2d = Some(isometry2d));
    }

    pub fn get_strand_nucl(&self, nucl: &Nucl) -> Option<usize> {
        self.design.get_strand_nucl(nucl)
    }

    pub fn get_visibility_helix(&self, h_id: usize) -> Option<bool> {
        self.design.helices.get(&h_id).map(|h| h.visible)
    }

    pub fn set_visibility_helix(&mut self, h_id: usize, visibility: bool) {
        let update = self.get_visibility_helix(h_id) != Some(visibility);
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.visible = visibility);
        self.update_status = update;
        self.hash_maps_update = update;
    }

    pub fn set_visibility_grid(&mut self, g_id: usize, visibility: bool) {
        let update = self.grid_manager.get_visibility(g_id) != visibility;
        self.grid_manager.set_visibility(g_id, visibility);
        self.update_status = update;
        self.hash_maps_update = update;
    }

    pub fn has_helix(&self, h_id: usize) -> bool {
        self.design.helices.contains_key(&h_id)
    }

    pub fn get_basis_map(&self) -> Arc<RwLock<HashMap<Nucl, char, RandomState>>> {
        self.basis_map.clone()
    }

    pub fn is_scaffold(&self, s_id: usize) -> bool {
        self.design.scaffold_id == Some(s_id)
    }

    pub fn scaffold_is_set(&self) -> bool {
        self.design.scaffold_id.is_some()
    }

    pub fn scaffold_sequence_set(&self) -> bool {
        self.design.scaffold_sequence.is_some()
    }

    pub fn get_stapple_mismatch(&self) -> Option<Nucl> {
        let basis_map = self.basis_map.read().unwrap();
        for strand in self.design.strands.values() {
            for domain in &strand.domains {
                if let icednano::Domain::HelixDomain(dom) = domain {
                    for position in dom.iter() {
                        let nucl = Nucl {
                            position,
                            forward: dom.forward,
                            helix: dom.helix,
                        };
                        if !basis_map.contains_key(&nucl) {
                            return Some(nucl);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_scaffold_sequence_len(&self) -> Option<usize> {
        self.design.scaffold_sequence.as_ref().map(|s| s.len())
    }

    pub fn get_scaffold_len(&self) -> Option<usize> {
        self.design
            .scaffold_id
            .as_ref()
            .and_then(|s_id| self.design.strands.get(s_id))
            .map(|s| s.length())
    }

    /// Return a vector of all the stapples.
    /// This function will panic if all the sapples are not matched.
    pub fn get_stapples(&self) -> Vec<Stapple> {
        let mut ret = Vec::new();
        let mut sequences: BTreeMap<(usize, isize, usize, isize), (usize, String)> =
            Default::default();
        let basis_map = self.basis_map.read().unwrap();
        for (s_id, strand) in self.design.strands.iter() {
            if strand.length() == 0 || self.design.scaffold_id == Some(*s_id) {
                continue;
            }
            let mut sequence = String::new();
            let mut first = true;
            for domain in &strand.domains {
                if !first {
                    sequence.push(' ');
                }
                first = false;
                if let icednano::Domain::HelixDomain(dom) = domain {
                    for position in dom.iter() {
                        let nucl = Nucl {
                            position,
                            forward: dom.forward,
                            helix: dom.helix,
                        };
                        sequence.push(*basis_map.get(&nucl).unwrap_or(&'?'));
                    }
                }
            }
            let key = if let Some((prim5, prim3)) = strand.get_5prime().zip(strand.get_3prime()) {
                (prim5.helix, prim5.position, prim3.helix, prim5.position)
            } else {
                println!("WARNING, STAPPLE WITH NO KEY !!!");
                (0, 0, 0, 0)
            };
            sequences.insert(key, (*s_id, sequence));
        }
        for (n, ((h5, nt5, h3, nt3), (s_id, sequence))) in sequences.iter().enumerate() {
            let plate = n / 96 + 1;
            let row = (n % 96) / 8 + 1;
            let column = match (n % 96) % 8 {
                0 => 'A',
                1 => 'B',
                2 => 'C',
                3 => 'D',
                4 => 'E',
                5 => 'F',
                6 => 'G',
                7 => 'H',
                _ => unreachable!(),
            };
            ret.push(Stapple {
                plate,
                well: format!("{}{}", column, row.to_string()),
                sequence: sequence.clone(),
                name: format!(
                    "Stapple {:04}; 5':h{}:nt{}>3':h{}:nt{}",
                    s_id, *h5, *nt5, *h3, *nt3
                ),
            });
        }
        ret
    }

    /// Shift the scaffold at an optimized poisition and return the corresponding score
    pub fn optimize_shift(&mut self, channel: std::sync::mpsc::Sender<f32>) -> (usize, String) {
        let mut best_score = 10000;
        let mut best_shfit = 0;
        let mut best_result = String::new();
        let len = self
            .design
            .scaffold_sequence
            .as_ref()
            .map(|s| s.len())
            .unwrap_or(0);
        for shift in 0..len {
            if shift % 10 == 0 {
                channel.send(shift as f32 / len as f32).unwrap();
            }
            self.read_scaffold_seq(shift);
            let (score, result) = self.evaluate_shift();
            if score < best_score {
                println!("shift {} score {}", shift, score);
                best_score = score;
                best_shfit = shift;
                best_result = result;
            }
            if score == 0 {
                break;
            }
        }
        self.design.scaffold_shift = Some(best_shfit);
        self.read_scaffold_seq(best_shfit);
        self.update_status = true;
        self.hash_maps_update = true;
        (best_shfit, best_result)
    }

    /// Evaluate a scaffold position. The score of the position is given by
    /// score = nb((A|T)^7) + 10 nb(G^4 | C ^4) + 100 nb (G^5 | C^5) + 1000 nb (G^6 | C^6)
    fn evaluate_shift(&self) -> (usize, String) {
        use std::fmt::Write;
        let basis_map = self.basis_map.read().unwrap();
        let mut ret = 0;
        let mut shown = false;
        let bad = regex::Regex::new(r"[AT]{7,}?").unwrap();
        let verybad = regex::Regex::new(r"G{4,}?|C{4,}?").unwrap();
        let ultimatelybad = regex::Regex::new(r"G{5,}|C{5,}").unwrap();
        let ultimatelybad2 = regex::Regex::new(r"G{6,}|C{6,}").unwrap();
        for (s_id, strand) in self.design.strands.iter() {
            if strand.length() == 0 || self.design.scaffold_id == Some(*s_id) {
                continue;
            }
            let mut sequence = String::with_capacity(10000);
            for domain in &strand.domains {
                if let icednano::Domain::HelixDomain(dom) = domain {
                    for position in dom.iter() {
                        let nucl = Nucl {
                            position,
                            forward: dom.forward,
                            helix: dom.helix,
                        };
                        sequence.push(*basis_map.get(&nucl).unwrap_or(&'?'));
                    }
                }
            }
            let mut matches = bad.find_iter(&sequence);
            while matches.next().is_some() {
                if !shown {
                    shown = true;
                }
                ret += 1;
            }
            let mut matches = verybad.find_iter(&sequence);
            while matches.next().is_some() {
                if !shown {
                    shown = true;
                }
                ret += 100;
            }
            let mut matches = ultimatelybad.find_iter(&sequence);
            while matches.next().is_some() {
                if !shown {
                    shown = true;
                }
                ret += 10_000;
            }
            let mut matches = ultimatelybad2.find_iter(&sequence);
            while matches.next().is_some() {
                if !shown {
                    shown = true;
                }
                ret += 1_000_000;
            }
        }
        let result = if ret == 0 {
            "No bad pattern".to_owned()
        } else {
            let mut result = String::new();
            if ret >= 1_000_000 {
                writeln!(&mut result, "{} times G^6 or C^6", ret / 1_000_000).unwrap();
            }
            if (ret % 1_000_000) >= 10_000 {
                writeln!(
                    &mut result,
                    "{} times G^5 or C^5",
                    (ret % 1_000_000) / 10_000
                )
                .unwrap();
            }
            if (ret % 10_000) >= 100 {
                writeln!(&mut result, "{} times G^4 or C^4", (ret % 10_000) / 100).unwrap();
            }
            if ret % 100 > 0 {
                writeln!(&mut result, "{} times (A or T)^7", (ret % 100)).unwrap();
            }
            result
        };
        (ret, result)
    }

    pub fn get_groups(&self) -> Arc<RwLock<BTreeMap<usize, bool>>> {
        self.groups.clone()
    }

    pub fn flip_group(&mut self, h_id: usize) {
        let new_group = match self.groups.read().unwrap().get(&h_id) {
            None => Some(true),
            Some(true) => Some(false),
            Some(false) => None,
        };
        if let Some(b) = new_group {
            self.groups.write().unwrap().insert(h_id, b);
        } else {
            self.groups.write().unwrap().remove(&h_id);
        }
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn set_group(&mut self, h_id: usize, new_group: Option<bool>) {
        if let Some(b) = new_group {
            self.groups.write().unwrap().insert(h_id, b);
        } else {
            self.groups.write().unwrap().remove(&h_id);
        }
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn get_suggestions(&self) -> Vec<(Nucl, Nucl)> {
        let mut ret = vec![];
        for blue_nucl in self.blue_nucl.iter() {
            let neighbour = self.get_possible_cross_over(blue_nucl);
            for (red_nucl, dist) in neighbour {
                ret.push((*blue_nucl, red_nucl, dist))
            }
        }
        ret.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        self.trimm_suggestion(&ret)
    }

    pub fn trimm_suggestion(&self, suggestion: &Vec<(Nucl, Nucl, f32)>) -> Vec<(Nucl, Nucl)> {
        let mut used = HashSet::new();
        let mut ret = vec![];
        for (a, b, _) in suggestion {
            if !used.contains(a) && !used.contains(b) {
                ret.push((*a, *b));
                used.insert(a);
                used.insert(b);
            }
        }
        ret
    }

    pub fn get_possible_cross_over(&self, nucl: &Nucl) -> Vec<(Nucl, f32)> {
        let mut ret = Vec::new();
        let positions = self.get_space_pos(nucl).unwrap();
        let cube0 = space_to_cube(positions[0], positions[1], positions[2]);

        let len_crit = 1.2;
        for i in vec![-1, 0, 1].iter() {
            for j in vec![-1, 0, 1].iter() {
                for k in vec![-1, 0, 1].iter() {
                    let cube = (cube0.0 + i, cube0.1 + j, cube0.2 + k);
                    if let Some(v) = self.red_cubes.get(&cube) {
                        for red_nucl in v {
                            if red_nucl.helix != nucl.helix {
                                let red_position = self.get_space_pos(&red_nucl).unwrap();
                                let dist = (0..3)
                                    .map(|i| (positions[i], red_position[i]))
                                    .map(|(x, y)| (x - y) * (x - y))
                                    .sum::<f32>()
                                    .sqrt();
                                if dist < len_crit
                                    && self.get_strand_nucl(nucl) != self.design.scaffold_id
                                    && self.get_strand_nucl(red_nucl) != self.design.scaffold_id
                                    && self.get_strand_nucl(nucl) != self.get_strand_nucl(red_nucl)
                                {
                                    ret.push((*red_nucl, dist));
                                }
                            }
                        }
                    }
                }
            }
        }
        ret
    }

    /// Return a string describing the decomposition of the length of the strand `s_id` into the
    /// sum of the length of its domains
    pub fn decompose_length(&self, s_id: usize) -> String {
        let mut ret = String::new();
        if let Some(strand) = self.design.strands.get(&s_id) {
            ret.push_str(&strand.length().to_string());
            let mut first = true;
            for d in strand.domains.iter() {
                let sign = if first { '=' } else { '+' };
                ret.push_str(&format!(" {} {}", sign, d.length()));
                first = false;
            }
        }
        ret
    }

    pub fn recolor_stapples(&mut self) {
        self.hash_maps_update = true;
        self.update_status = true;
        for (s_id, strand) in self.design.strands.iter_mut() {
            if Some(*s_id) != self.design.scaffold_id {
                let color = {
                    let hue = (self.color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
                    let saturation =
                        (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
                    let value =
                        (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
                    let hsv = color_space::Hsv::new(hue, saturation, value);
                    let rgb = color_space::Rgb::from(hsv);
                    (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
                };
                self.color_idx += 1;
                strand.color = color;
            }
        }
    }

    pub fn clean_up_domains(&mut self) {
        let ids: Vec<usize> = self.design.strands.keys().cloned().collect();
        for s_id in ids {
            self.clean_domains_one_strand(s_id)
        }
    }

    fn clean_domains_one_strand(&mut self, s_id: usize) {
        if !self.design.strands.contains_key(&s_id) {
            return;
        }
        let mut strand = self.design.strands.get(&s_id).cloned().unwrap();
        self.rm_strand(s_id);
        strand.merge_consecutive_domains();
        strand.junctions.clear();
        strand.read_junctions(&mut self.xover_ids, true);
        strand.read_junctions(&mut self.xover_ids, false);
        self.design.strands.insert(s_id, strand);
        self.update_status = true;
        self.hash_maps_update = true;
        self.view_need_reset = true;
    }

    /// Return the infomation necessary to make a crossover from source_nucl to target_nucl
    pub fn get_xover_info(
        &self,
        source_nucl: Nucl,
        target_nucl: Nucl,
        design_id: usize,
    ) -> Option<XoverInfo> {
        let source_id = self.get_strand_nucl(&source_nucl)?;
        let target_id = self.get_strand_nucl(&target_nucl)?;

        let source = self.design.strands.get(&source_id).cloned()?;
        let target = self.design.strands.get(&target_id).cloned()?;

        let source_strand_end = self.is_strand_end(&source_nucl);
        let target_strand_end = self.is_strand_end(&target_nucl);

        Some(XoverInfo {
            source,
            target,
            source_id,
            target_id,
            source_nucl,
            target_nucl,
            design_id,
            target_strand_end,
            source_strand_end,
        })
    }

    pub fn notify_death(&mut self) {
        self.stop_rolling();
        self.stop_simulations();
    }

    pub fn roll_helix(&mut self, h_id: usize, roll: f32) {
        self.design.helices.get_mut(&h_id).map(|h| h.set_roll(roll));
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn get_roll_helix(&self, h_id: usize) -> Option<f32> {
        self.design.helices.get(&h_id).map(|h| h.roll)
    }

    pub fn has_template(&self) -> bool {
        self.template_manager.templates.len() > 0
    }

    fn can_add_domains(&self, domains: &[icednano::Domain]) -> bool {
        for s in self.design.strands.values() {
            if s.intersect_domains(domains) {
                return false;
            }
        }
        true
    }

    pub(super) fn get_strand_state(&self) -> StrandState {
        StrandState {
            strands: self.design.strands.clone(),
            xover_ids: self.xover_ids.clone(),
        }
    }

    pub fn general_cross_over(
        &mut self,
        source_nucl: Nucl,
        target_nucl: Nucl,
    ) -> Option<(StrandState, StrandState)> {
        if source_nucl.helix == target_nucl.helix {
            return None;
        }
        let init = self.get_strand_state();
        println!("cross over between {:?} and {:?}", source_nucl, target_nucl);
        let source_id = self.get_strand_nucl(&source_nucl);
        let target_id = self.get_strand_nucl(&target_nucl);

        let source = source_id
            .as_ref()
            .and_then(|source_id| self.design.strands.get(source_id).cloned());
        let target = target_id
            .as_ref()
            .and_then(|target_id| self.design.strands.get(target_id).cloned());

        let source_strand_end = self.is_strand_end(&source_nucl);
        let target_strand_end = self.is_strand_end(&target_nucl);
        println!(
            "source strand {:?}, target strand {:?}",
            source_id, target_id
        );
        println!(
            "source end {:?}, target end {:?}",
            source_strand_end.to_opt(),
            target_strand_end.to_opt()
        );
        if let (Some(source_id), Some(target_id), Some(source), Some(_target)) =
            (source_id, target_id, source, target)
        {
            match (source_strand_end.to_opt(), target_strand_end.to_opt()) {
                (Some(true), Some(true)) | (Some(false), Some(false)) => return None, // xover can't be done,
                (Some(true), Some(false)) => {
                    // We can xover directly
                    if source_id == target_id {
                        self.make_cycle(source_id, true);
                    } else {
                        self.merge_strands(source_id, target_id);
                    }
                }
                (Some(false), Some(true)) => {
                    // We can xover directly but we must reverse the xover
                    if source_id == target_id {
                        self.make_cycle(target_id, true);
                    } else {
                        self.merge_strands(target_id, source_id);
                    }
                }
                (Some(b), None) => {
                    // We can cut cross directly, but only if the target and source's helices are
                    // different
                    println!("2324");
                    let target_3prime = b;
                    if source_nucl.helix != target_nucl.helix {
                        self.cross_cut(source_id, target_id, target_nucl, target_3prime);
                    }
                }
                (None, Some(b)) => {
                    // We can cut cross directly but we need to reverse the xover
                    println!("2331");
                    let target_3prime = b;
                    if source_nucl.helix != target_nucl.helix {
                        self.cross_cut(target_id, source_id, source_nucl, target_3prime);
                    }
                }
                (None, None) => {
                    if source_nucl.helix != target_nucl.helix {
                        if source_id != target_id {
                            self.split_strand(&source_nucl, None);
                            self.cross_cut(source_id, target_id, target_nucl, true);
                        } else if source.cyclic {
                            self.split_strand(&source_nucl, Some(false));
                            self.cross_cut(source_id, target_id, target_nucl, true);
                        } else {
                            // if the two nucleotides are on the same strand care must be taken
                            // because one of them might be on the newly crated strand after the
                            // split
                            let pos1 = source.find_nucl(&source_nucl);
                            let pos2 = source.find_nucl(&target_nucl);
                            if let Some((pos1, pos2)) = pos1.zip(pos2) {
                                if pos1 > pos2 {
                                    // the source nucl will be on the 5' end of the split and the
                                    // target nucl as well
                                    self.split_strand(&source_nucl, Some(false));
                                    self.cross_cut(source_id, target_id, target_nucl, true);
                                } else {
                                    let new_id = self.split_strand(&source_nucl, Some(false));
                                    if let Some(new_id) = new_id {
                                        self.cross_cut(source_id, new_id, target_nucl, true);
                                    } else {
                                        println!("WARNING COULD NOT FIND NEWID");
                                    }
                                }
                            } else {
                                println!("WARNING COULD NOT FIND NUCLS");
                            }
                        }
                    }
                }
            }
            let final_state = self.get_strand_state();
            Some((init, final_state))
        } else {
            None
        }
    }

    pub fn get_xover_id(&self, xover: &(Nucl, Nucl)) -> Option<usize> {
        self.xover_ids.get_id(xover)
    }

    pub fn get_xover_with_id(&self, id: usize) -> Option<(Nucl, Nucl)> {
        self.xover_ids.get_element(id)
    }

    pub fn new_strand_state(&mut self, state: StrandState) {
        self.design.strands = state.strands;
        self.xover_ids = state.xover_ids;
        self.update_status = true;
        self.hash_maps_update = true;
        self.view_need_reset = true;
    }

    pub fn get_insertions(&mut self, s_id: usize) -> Option<Vec<Nucl>> {
        self.design.strands.get(&s_id).map(|s| s.get_insertions())
    }

    pub fn add_anchor(&mut self, anchor: Nucl) {
        if self.anchors.contains(&anchor) {
            self.anchors.remove(&anchor);
        } else {
            self.anchors.insert(anchor);
        }
    }

    pub fn is_anchor(&self, anchor: Nucl) -> bool {
        self.anchors.contains(&anchor)
    }

    pub fn rigid_parameters_update(&mut self, parameters: RigidBodyConstants) {
        if let Some(simulator) = self.rigid_helix_simulator.as_mut() {
            simulator.update_parameters(parameters)
        }
    }

    pub fn shake_nucl(&mut self, nucl: Nucl) {
        if let Some(simulator) = self.rigid_helix_simulator.as_mut() {
            simulator.shake_nucl(nucl)
        }
    }

    /// Set the shift a the hyperboloid grid g_id.
    pub fn set_new_shift(&mut self, g_id: usize, shift: f32) {
        let parameters = self.design.parameters.unwrap_or_default();
        if let Some(grid) = self.grid_manager.grids.get_mut(g_id) {
            grid.grid_type.set_shift(shift, &parameters)
        }
        self.update_grids();
        self.grid_manager.update(&mut self.design);
        self.hash_maps_update = true;
        self.view_need_reset = true;
        self.update_status = true;
    }

    /// Return the shift a the hyperboloid grid g_id.
    pub fn get_shift(&self, g_id: usize) -> Option<f32> {
        self.grid_manager
            .grids
            .get(g_id)
            .and_then(|g| g.grid_type.get_shift())
    }

    pub fn get_new_elements(&mut self) -> Option<Vec<DnaElement>> {
        self.elements_update.take()
    }

    pub fn get_organizer_tree(&self) -> Option<OrganizerTree<DnaElementKey>> {
        self.design.organizer_tree.clone()
    }

    pub fn update_organizer_tree(&mut self, tree: OrganizerTree<DnaElementKey>) {
        self.design.organizer_tree = Some(tree)
    }

    fn is_in_selection(&self, nucl: &Nucl, selection: &[Selection]) -> bool {
        let strand_nucl = self.get_strand_nucl(nucl);
        for s in selection.iter() {
            match s {
                Selection::Bound(_, n1, n2) => {
                    if n1 == nucl || n2 == nucl {
                        return true;
                    }
                }
                Selection::Xover(_, xover_id) => {
                    if let Some((n1, n2)) = self.xover_ids.get_element(*xover_id) {
                        if &n1 == nucl || &n2 == nucl {
                            return true;
                        }
                    }
                }
                Selection::Nucleotide(_, n) => {
                    if n == nucl {
                        return true;
                    }
                }
                Selection::Strand(_, s_id) => {
                    if strand_nucl == Some(*s_id as usize) {
                        return true;
                    }
                }
                Selection::Helix(_, h_id) => {
                    if nucl.helix == *h_id as usize {
                        return true;
                    }
                }
                _ => (),
            }
        }
        false
    }

    fn update_visibility(&mut self) {
        if let Some(VisibilitySieve {
            selection,
            compl,
            visible,
        }) = &self.visibility_sieve
        {
            for nucl in self.nucleotide.values() {
                if self.is_in_selection(nucl, selection) != *compl {
                    self.visible.insert(*nucl, *visible);
                } else {
                    self.visible.remove(nucl);
                }
            }
        } else {
            self.visible = HashMap::new();
        }
        self.update_status = true;
    }

    pub fn set_visibility_sieve(&mut self, selection: Vec<Selection>, compl: bool) {
        let visible = !self.whole_selection_is_visible(&selection, compl);
        self.visibility_sieve = Some(VisibilitySieve {
            selection,
            visible,
            compl,
        });
        self.update_visibility();
    }

    pub fn clear_visibility_sive(&mut self) {
        self.visibility_sieve = None;
        self.update_visibility();
    }

    fn whole_selection_is_visible(&self, selection: &[Selection], compl: bool) -> bool {
        for nucl in self.nucleotide.values() {
            if self.is_in_selection(nucl, selection) != compl {
                if !self.is_visible(nucl) {
                    return false;
                }
            }
        }
        true
    }

    pub fn is_visible(&self, nucl: &Nucl) -> bool {
        *self.visible.get(nucl).unwrap_or(&true)
    }

    pub fn delete_selection(&mut self, selection: Vec<Selection>) -> bool {
        let mut ret = false;
        for s in selection.iter() {
            match s {
                Selection::Strand(_, s_id) => {
                    ret |= self.rm_strand(*s_id as usize);
                }
                _ => (),
            }
        }
        self.update_status |= ret;
        self.hash_maps_update |= ret;
        ret
    }

    pub fn get_scaffold_info(&self) -> Option<super::ScaffoldInfo> {
        let id = self.design.scaffold_id?;
        let length = self.get_strand_length(id)?;
        let shift = self.design.scaffold_shift;
        let starting_nucl = self
            .design
            .strands
            .get(&id)
            .and_then(|s| s.get_nth_nucl(shift.unwrap_or(0)));
        Some(super::ScaffoldInfo {
            id,
            shift,
            length,
            starting_nucl,
        })
    }

    pub fn has_at_least_on_strand_with_insertions(&self) -> bool {
        self.design.has_at_least_on_strand_with_insertions()
    }

    pub fn get_dna_parameters(&self) -> Parameters {
        self.design.parameters.unwrap_or_default()
    }

    pub fn get_prime3_set(&self) -> Vec<(Vec3, Vec3, u32)> {
        self.prime3_set.clone()
    }
}

fn compl(c: Option<char>) -> Option<char> {
    match c {
        Some('T') => Some('A'),
        Some('A') => Some('T'),
        Some('G') => Some('C'),
        Some('C') => Some('G'),
        _ => None,
    }
}

/// Create a design by parsing a file
fn read_file<P: AsRef<Path> + std::fmt::Debug>(path: P) -> Option<icednano::Design> {
    let json_str =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("File not found {:?}", path));

    let design: Result<icednano::Design, _> = serde_json::from_str(&json_str);
    // First try to read icednano format
    if let Ok(design) = design {
        println!("ok icednano");
        Some(design)
    } else {
        // If the file is not in icednano format, try the other supported format
        let cdn_design: Result<codenano::Design<(), ()>, _> = serde_json::from_str(&json_str);

        let scadnano_design: Result<scadnano::ScadnanoDesign, _> = serde_json::from_str(&json_str);

        // Try codenano format
        if let Ok(scadnano) = scadnano_design {
            icednano::Design::from_scadnano(&scadnano)
        } else if let Ok(design) = cdn_design {
            println!("{:?}", scadnano_design.err());
            println!("ok codenano");
            Some(icednano::Design::from_codenano(&design))
        } else if let Ok(cadnano) = Cadnano::from_file(path) {
            println!("ok cadnano");
            Some(icednano::Design::from_cadnano(cadnano))
        } else {
            // The file is not in any supported format
            message("Unrecognized file format".into(), rfd::MessageLevel::Error);
            None
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ObjectType {
    /// A nucleotide identified by its identifier
    Nucleotide(u32),
    /// A bound, identified by the identifier of the two nucleotides that it bounds.
    Bound(u32, u32),
}

impl ObjectType {
    pub fn is_nucl(&self) -> bool {
        match self {
            ObjectType::Nucleotide(_) => true,
            _ => false,
        }
    }

    pub fn is_bound(&self) -> bool {
        match self {
            ObjectType::Bound(_, _) => true,
            _ => false,
        }
    }

    pub fn same_type(&self, other: Self) -> bool {
        self.is_nucl() == other.is_nucl()
    }
}

#[derive(Debug)]
pub struct Stapple {
    pub well: String,
    pub name: String,
    pub sequence: String,
    pub plate: usize,
}

fn space_to_cube(x: f32, y: f32, z: f32) -> (isize, isize, isize) {
    let cube_len = 1.2;
    (
        x.div_euclid(cube_len) as isize,
        y.div_euclid(cube_len) as isize,
        z.div_euclid(cube_len) as isize,
    )
}

fn real_name(path: &PathBuf) -> PathBuf {
    let mut file_name = path.clone();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unamed_design");
    let real_stem = if let Some(prefix) = stem.strip_suffix("_recovry") {
        if prefix.len() > 0 {
            prefix
        } else {
            "unamed_design"
        }
    } else if let Some(prefix) = stem.strip_prefix("_before_simulation") {
        if prefix.len() > 0 {
            prefix
        } else {
            "unamed_design"
        }
    } else {
        stem
    };
    file_name.set_file_name(format!("{}.json", real_stem));
    println!("file name {:?}", file_name);
    file_name
}

fn backup_name(path: &PathBuf) -> PathBuf {
    let mut file_name = path.clone();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{}_recovry", s))
        .unwrap_or(String::from("unamed_design_recovry"));
    file_name.set_file_name(format!("{}.json", stem));
    println!("backup name {:?}", file_name);
    file_name
}

fn before_simul_name(path: &PathBuf) -> PathBuf {
    let mut file_name = path.clone();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{}_before_simulation", s))
        .unwrap_or(String::from("unamed_design_recovry"));
    file_name.set_file_name(format!("{}.json", stem));
    println!("backup name {:?}", file_name);
    file_name
}

struct VisibilitySieve {
    selection: Vec<Selection>,
    compl: bool,
    visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationState {
    None,
    Rolling,
    RigidGrid,
    RigidHelices,
}

impl SimulationState {
    pub fn is_none(&self) -> bool {
        if let Self::None = self {
            true
        } else {
            false
        }
    }

    pub fn is_rolling(&self) -> bool {
        if let Self::Rolling = self {
            true
        } else {
            false
        }
    }

    pub fn simulating_grid(&self) -> bool {
        if let Self::RigidGrid = self {
            true
        } else {
            false
        }
    }

    pub fn simulating_helices(&self) -> bool {
        if let Self::RigidHelices = self {
            true
        } else {
            false
        }
    }
}

impl Default for SimulationState {
    fn default() -> Self {
        Self::None
    }
}
