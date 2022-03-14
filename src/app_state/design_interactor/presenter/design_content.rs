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
use crate::scene::GridInstance;
use ahash::RandomState;
use ensnano_design::elements::DnaElement;
use ensnano_design::grid::{GridObject, GridPosition, HelixGridPosition};
use ensnano_design::*;
use ensnano_interactor::{
    graphics::{LoopoutBond, LoopoutNucl},
    ObjectType,
};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use ultraviolet::Vec3;

use ensnano_design::grid::GridData;

mod xover_suggestions;
use xover_suggestions::XoverSuggestions;

#[derive(Default, Clone)]
pub struct NuclCollection {
    identifier: HashMap<Nucl, u32, RandomState>,
    virtual_nucl_map: HashMap<VirtualNucl, Nucl, RandomState>,
}

impl super::NuclCollection for NuclCollection {
    fn iter_nucls_ids<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a Nucl, &'a u32)> + 'a> {
        Box::new(self.identifier.iter())
    }

    fn contains_nucl(&self, nucl: &Nucl) -> bool {
        self.identifier.contains_key(nucl)
    }

    fn iter_nucls<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Nucl> + 'a> {
        Box::new(self.identifier.keys())
    }

    fn virtual_to_real(&self, virtual_nucl: &VirtualNucl) -> Option<&Nucl> {
        self.virtual_nucl_map.get(virtual_nucl)
    }
}

impl NuclCollection {
    pub fn get_identifier(&self, nucl: &Nucl) -> Option<&u32> {
        self.identifier.get(nucl)
    }

    pub fn contains_nucl(&self, nucl: &Nucl) -> bool {
        self.identifier.contains_key(nucl)
    }

    pub fn nb_nucls(&self) -> usize {
        self.identifier.len()
    }

    fn insert(&mut self, key: Nucl, id: u32) -> Option<u32> {
        self.identifier.insert(key, id)
    }

    fn insert_virtual(&mut self, virtual_nucl: VirtualNucl, nucl: Nucl) -> Option<Nucl> {
        self.virtual_nucl_map.insert(virtual_nucl, nucl)
    }
}

#[derive(Default, Clone)]
pub(super) struct DesignContent {
    /// Maps identifer of elements to their object type
    pub object_type: HashMap<u32, ObjectType, RandomState>,
    /// Maps identifier of nucleotide to Nucleotide objects
    pub nucleotide: HashMap<u32, Nucl, RandomState>,
    /// Maps identifier of bounds to the pair of nucleotides involved in the bound
    pub nucleotides_involved: HashMap<u32, (Nucl, Nucl), RandomState>,
    /// Maps identifier of element to their position in the Model's coordinates
    pub space_position: HashMap<u32, [f32; 3], RandomState>,
    /// Maps a Nucl object to its identifier
    pub nucl_collection: Arc<NuclCollection>,
    /// Maps a pair of nucleotide forming a bound to the identifier of the bound
    pub identifier_bound: HashMap<(Nucl, Nucl), u32, RandomState>,
    /// Maps the identifier of a element to the identifier of the strands to which it belongs
    pub strand_map: HashMap<u32, usize, RandomState>,
    /// Maps the identifier of a element to the identifier of the helix to which it belongs
    pub helix_map: HashMap<u32, usize, RandomState>,
    /// Maps the identifier of an element to its color
    pub color: HashMap<u32, u32, RandomState>,
    pub basis_map: Arc<HashMap<Nucl, char, RandomState>>,
    pub prime3_set: Vec<Prime3End>,
    pub elements: Vec<DnaElement>,
    pub suggestions: Vec<(Nucl, Nucl)>,
    pub(super) grid_manager: GridData,
    pub loopout_nucls: Vec<LoopoutNucl>,
    pub loopout_bonds: Vec<LoopoutBond>,
    /// Maps bonds identifier to the length of the corresponding insertion.
    pub insertion_length: HashMap<u32, usize, RandomState>,
}

impl DesignContent {
    pub(super) fn get_grid_instances(&self) -> BTreeMap<GridId, GridInstance> {
        self.grid_manager.grid_instances(0)
    }

    pub(super) fn get_helices_on_grid(&self, g_id: GridId) -> Option<HashSet<usize>> {
        self.grid_manager.get_helices_on_grid(g_id)
    }
    /// Return the position of an element.
    /// If the element is a nucleotide, return the center of the nucleotide.
    /// If the element is a bound, return the middle of the segment between the two nucleotides
    /// involved in the bound.
    pub(super) fn get_element_position(&self, id: u32) -> Option<Vec3> {
        if let Some(object_type) = self.object_type.get(&id) {
            match object_type {
                ObjectType::Nucleotide(id) => self.space_position.get(&id).map(|x| x.into()),
                ObjectType::Bound(e1, e2) => {
                    let a = self.space_position.get(e1)?;
                    let b = self.space_position.get(e2)?;
                    Some((Vec3::from(*a) + Vec3::from(*b)) / 2.)
                }
            }
        } else {
            None
        }
    }

    pub(super) fn get_helix_grid_position(&self, h_id: usize) -> Option<HelixGridPosition> {
        self.grid_manager.get_helix_grid_position(h_id)
    }

    pub(super) fn get_grid_latice_position(&self, position: GridPosition) -> Option<Vec3> {
        let grid = self.grid_manager.grids.get(&position.grid)?;
        Some(grid.position_helix(position.x, position.y))
    }

    /// Return a list of pairs ((x, y), h_id) of all the used helices on the grid g_id
    pub(super) fn get_helices_grid_key_coord(&self, g_id: GridId) -> Vec<((isize, isize), usize)> {
        self.grid_manager.get_helices_grid_key_coord(g_id)
    }

    pub(super) fn get_used_coordinates_on_grid(&self, g_id: GridId) -> Vec<(isize, isize)> {
        self.grid_manager.get_used_coordinates_on_grid(g_id)
    }

    pub(super) fn get_helix_id_at_grid_coord(&self, position: GridPosition) -> Option<usize> {
        self.grid_manager
            .pos_to_object(position)
            .map(|obj| obj.helix())
    }

    pub(super) fn get_persistent_phantom_helices_id(&self) -> HashSet<u32> {
        self.grid_manager.get_persistent_phantom_helices_id()
    }

    pub(super) fn grid_has_small_spheres(&self, g_id: GridId) -> bool {
        self.grid_manager.small_spheres.contains(&g_id)
    }

    pub(super) fn grid_has_persistent_phantom(&self, g_id: GridId) -> bool {
        !self.grid_manager.no_phantoms.contains(&g_id)
    }

    pub(super) fn get_grid_nb_turn(&self, g_id: GridId) -> Option<f32> {
        self.grid_manager
            .grids
            .get(&g_id)
            .and_then(|g| g.grid_type.get_nb_turn().map(|x| x as f32))
    }

    pub(super) fn get_grid_shift(&self, g_id: GridId) -> Option<f32> {
        self.grid_manager
            .grids
            .get(&g_id)
            .and_then(|g| g.grid_type.get_shift())
    }

    pub(super) fn get_stapple_mismatch(&self, design: &Design) -> Option<Nucl> {
        let basis_map = self.basis_map.as_ref();
        for strand in design.strands.values() {
            for domain in &strand.domains {
                if let Domain::HelixDomain(dom) = domain {
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

    pub(super) fn get_grid_object(&self, position: GridPosition) -> Option<GridObject> {
        self.grid_manager.pos_to_object(position)
    }

    pub(super) fn get_staples(&self, design: &Design, presenter: &Presenter) -> Vec<Staple> {
        let mut ret = Vec::new();
        let mut sequences: BTreeMap<(usize, isize, usize, isize), StapleInfo> = Default::default();
        let basis_map = self.basis_map.as_ref();
        for (s_id, strand) in design.strands.iter() {
            if strand.length() == 0 || design.scaffold_id == Some(*s_id) {
                continue;
            }
            let mut sequence = String::new();
            let mut first = true;
            let mut previous_char_is_basis = None;
            let mut intervals = StapleIntervals {
                staple_id: *s_id,
                intervals: Vec::new(),
            };
            for domain in &strand.domains {
                let mut staple_domain = None;
                let scaffold = design.scaffold_id.and_then(|id| design.strands.get(&id));
                if !first {
                    sequence.push(' ');
                }
                let helices = &design.helices;
                first = false;
                if let Domain::HelixDomain(dom) = domain {
                    for position in dom.iter() {
                        let nucl = Nucl {
                            position,
                            forward: dom.forward,
                            helix: dom.helix,
                        };

                        let next_basis = basis_map.get(&nucl);
                        if let Some(basis) = next_basis {
                            if previous_char_is_basis == Some(false) {
                                sequence.push(' ');
                            }
                            sequence.push(*basis);
                            previous_char_is_basis = Some(true);
                        } else {
                            if previous_char_is_basis == Some(true) {
                                sequence.push(' ');
                            }
                            sequence.push('?');
                            previous_char_is_basis = Some(false);
                        }
                        if let Some(virtual_nucl) = Nucl::map_to_virtual_nucl(nucl, helices) {
                            if let Some(scaffold) = scaffold {
                                let result = scaffold
                                    .locate_virtual_nucl(&virtual_nucl.compl(), helices)
                                    .map(|v| ScaffoldPosition {
                                        domain_id: v.domain_id,
                                        scaffold_position: (v.pos_on_strand + scaffold.length()
                                            - design.scaffold_shift.unwrap_or(0))
                                            % scaffold.length(),
                                    });
                                if staple_domain.is_none() {
                                    staple_domain = Some(StapleDomain::init(result));
                                }
                                let d = staple_domain.take().unwrap();
                                match d.read_position(result) {
                                    ReadResult::Continue(d) => staple_domain = Some(d),
                                    ReadResult::Stop {
                                        interval,
                                        new_reader,
                                    } => {
                                        intervals.intervals.push(interval);
                                        staple_domain = Some(new_reader);
                                    }
                                }
                            }
                        } else {
                            log::error!("Could not map to virtual nucl");
                        }
                    }
                }
                if let Some(d) = staple_domain {
                    intervals.intervals.push(d.finish())
                }
            }
            let key = if let Some((prim5, prim3)) = strand.get_5prime().zip(strand.get_3prime()) {
                (prim5.helix, prim5.position, prim3.helix, prim3.position)
            } else {
                log::warn!("WARNING, STAPPLE WITH NO KEY !!!");
                (0, 0, 0, 0)
            };
            sequences.insert(
                key,
                StapleInfo {
                    s_id: *s_id,
                    sequence,
                    strand_name: strand.name.clone(),
                    domain_decomposition: presenter.decompose_length(*s_id),
                    length: strand.length(),
                    color: strand.color & 0xFFFFFF,
                    group_names: presenter.get_name_of_group_having_strand(*s_id),
                    intervals,
                },
            );
        }
        for (n, ((h5, nt5, h3, nt3), staple_info)) in sequences.iter().enumerate() {
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
            ret.push(Staple {
                plate,
                well: format!("{}{}", column, row.to_string()),
                sequence: staple_info.sequence.clone(),
                name: staple_info.strand_name.clone().unwrap_or_else(|| {
                    format!(
                        "Staple {:04}; 5':h{}:nt{}>3':h{}:nt{}",
                        staple_info.s_id, *h5, *nt5, *h3, *nt3
                    )
                    .into()
                }),
                color_str: format!("{:#08X}", staple_info.color),
                groups_name_str: staple_info.group_names.join(" ; "),
                length_str: staple_info.length.to_string(),
                domain_decomposition: staple_info
                    .domain_decomposition
                    .split_once("=")
                    .map(|split| split.1.to_string())
                    .unwrap_or(staple_info.domain_decomposition.clone()),
                intervals: staple_info.intervals.clone(),
            });
        }
        ret
    }

    pub fn get_all_visible_nucl_ids(
        &self,
        design: &Design,
        invisible_nucls: &HashSet<Nucl>,
    ) -> Vec<u32> {
        let check_visiblity = |&(_, v): &(&u32, &Nucl)| {
            !invisible_nucls.contains(v)
                && design
                    .helices
                    .get(&v.helix)
                    .map(|h| h.visible)
                    .unwrap_or_default()
        };
        self.nucleotide
            .iter()
            .filter(check_visiblity)
            .map(|t| *t.0)
            .collect()
    }

    pub fn get_all_visible_bounds(
        &self,
        design: &Design,
        invisible_nucls: &HashSet<Nucl>,
    ) -> Vec<u32> {
        let check_visiblity = |&(_, bound): &(&u32, &(Nucl, Nucl))| {
            !(invisible_nucls.contains(&bound.0) && invisible_nucls.contains(&bound.1))
                && (design
                    .helices
                    .get(&bound.0.helix)
                    .map(|h| h.visible)
                    .unwrap_or_default()
                    || design
                        .helices
                        .get(&bound.1.helix)
                        .map(|h| h.visible)
                        .unwrap_or_default())
        };
        self.nucleotides_involved
            .iter()
            .filter(check_visiblity)
            .map(|t| *t.0)
            .collect()
    }
}

#[derive(Debug)]
pub struct Staple {
    pub well: String,
    pub name: Cow<'static, str>,
    pub sequence: String,
    pub plate: usize,
    pub color_str: String,
    pub groups_name_str: String,
    pub domain_decomposition: String,
    pub length_str: String,
    pub intervals: StapleIntervals,
}

#[derive(Debug, Serialize, Clone)]
pub struct StapleIntervals {
    pub staple_id: usize,
    pub intervals: Vec<(isize, isize)>,
}

struct StapleInfo {
    s_id: usize,
    sequence: String,
    strand_name: Option<Cow<'static, str>>,
    color: u32,
    group_names: Vec<String>,
    domain_decomposition: String,
    length: usize,
    intervals: StapleIntervals,
}

#[derive(Clone)]
pub struct Prime3End {
    pub nucl: Nucl,
    pub color: u32,
}

impl DesignContent {
    /// Update all the hash maps
    pub(super) fn make_hash_maps(
        mut design: Design,
        xover_ids: &JunctionsIds,
        suggestion_parameters: &SuggestionParameters,
    ) -> (Self, Design, JunctionsIds) {
        let groups = design.groups.clone();
        let mut object_type = HashMap::default();
        let mut space_position = HashMap::default();
        let mut nucl_collection = NuclCollection::default();
        let mut identifier_bound = HashMap::default();
        let mut nucleotides_involved = HashMap::default();
        let mut nucleotide = HashMap::default();
        let mut strand_map = HashMap::default();
        let mut color_map = HashMap::default();
        let mut helix_map = HashMap::default();
        let mut basis_map = HashMap::default();
        let mut loopout_bonds = Vec::new();
        let mut loopout_nucls = Vec::new();
        let mut id = 0u32;
        let mut nucl_id;
        let mut old_nucl: Option<Nucl> = None;
        let mut old_nucl_id: Option<u32> = None;
        let mut elements = Vec::new();
        let mut prime3_set = Vec::new();
        let mut new_junctions: JunctionsIds = Default::default();
        let mut suggestion_maker = XoverSuggestions::default();
        let mut insertion_length = HashMap::default();
        xover_ids.agree_on_next_id(&mut new_junctions);
        let rainbow_strand = design.scaffold_id.filter(|_| design.rainbow_scaffold);
        let grid_manager = design.get_updated_grid_data().clone();

        for (s_id, strand) in design.strands.iter_mut() {
            elements.push(elements::DnaElement::Strand { id: *s_id });
            let parameters = design.parameters.unwrap_or_default();
            strand.update_insertions(&design.helices, &parameters);
            let mut strand_position = 0;
            let strand_seq = strand.sequence.as_ref().filter(|s| s.is_ascii());
            let color = strand.color;
            let mut last_xover_junction: Option<&mut DomainJunction> = None;
            let rainbow_len = if Some(*s_id) == rainbow_strand {
                strand.length()
            } else {
                0
            };
            // If the strand is not the rainbow strand, the rainbow iterator will be empty and the
            // real strand color will be used.
            let mut rainbow_iterator = (0..rainbow_len).map(|i| {
                let hsv = color_space::Hsv::new(i as f64 * 360. / rainbow_len as f64, 1., 1.);
                let rgb = color_space::Rgb::from(hsv);
                (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
            });

            let mut prev_loopout_pos = None;
            for (i, domain) in strand.domains.iter().enumerate() {
                if let Some((prime5, prime3)) = old_nucl.clone().zip(domain.prime5_end()) {
                    Self::update_junction(
                        &mut new_junctions,
                        *last_xover_junction
                            .as_mut()
                            .expect("Broke Invariant [LastXoverJunction]"),
                        (prime5, prime3),
                    );
                    if let Some(id) = xover_ids.get_id(&(prime5, prime3)) {
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
                if let Domain::HelixDomain(domain) = domain {
                    let dom_seq = domain.sequence.as_ref().filter(|s| s.is_ascii());
                    for (dom_position, nucl_position) in domain.iter().enumerate() {
                        let position = design.helices.get(&domain.helix).unwrap().space_pos(
                            design.parameters.as_ref().unwrap(),
                            nucl_position,
                            domain.forward,
                        );
                        let nucl = Nucl {
                            position: nucl_position,
                            forward: domain.forward,
                            helix: domain.helix,
                        };
                        let virtual_nucl = Nucl::map_to_virtual_nucl(nucl, &design.helices);
                        if let Some(v_nucl) = virtual_nucl {
                            let previous = nucl_collection.insert_virtual(v_nucl, nucl);
                            if previous.is_some() && previous != Some(nucl) {
                                log::error!("NUCLEOTIDE CONFLICTS: nucls {:?} and {:?} are mapped to the same virtual postition {:?}", previous, nucl, v_nucl);
                            }
                        } else {
                            log::error!("Could not get virtual nucl corresponding to {:?}", nucl);
                        }

                        elements.push(DnaElement::Nucleotide {
                            helix: nucl.helix,
                            position: nucl.position,
                            forward: nucl.forward,
                        });
                        let color = rainbow_iterator.next().unwrap_or(color);
                        if let Some(prev_pos) = prev_loopout_pos.take() {
                            loopout_bonds.push(LoopoutBond {
                                position_prime5: prev_pos,
                                position_prime3: position.into(),
                                color,
                                repr_bond_identifier: id,
                            });
                        }
                        nucl_id = if let Some(old_nucl) = old_nucl {
                            let bound_id = id;
                            id += 1;
                            let bound = (old_nucl, nucl);
                            object_type
                                .insert(bound_id, ObjectType::Bound(old_nucl_id.unwrap(), id));
                            identifier_bound.insert(bound, bound_id);
                            nucleotides_involved.insert(bound_id, bound);
                            color_map.insert(bound_id, color);
                            strand_map.insert(bound_id, *s_id);
                            helix_map.insert(bound_id, nucl.helix);
                            id
                        } else {
                            id
                        };
                        id += 1;
                        object_type.insert(nucl_id, ObjectType::Nucleotide(nucl_id));
                        nucleotide.insert(nucl_id, nucl);
                        nucl_collection.insert(nucl, nucl_id);
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
                        suggestion_maker.add_nucl(nucl, position, groups.as_ref());
                        let position = [position[0] as f32, position[1] as f32, position[2] as f32];
                        space_position.insert(nucl_id, position);
                        old_nucl = Some(nucl);
                        old_nucl_id = Some(nucl_id);
                    }
                    if strand.junctions.len() <= i {
                        log::debug!("{:?}", strand.junctions);
                    }
                    last_xover_junction = Some(&mut strand.junctions[i]);
                } else if let Domain::Insertion {
                    nb_nucl,
                    instanciation,
                    sequence: dom_seq,
                } = domain
                {
                    if let Some(instanciation) = instanciation.as_ref() {
                        for (dom_position, pos) in instanciation.as_ref().pos().iter().enumerate() {
                            let color = rainbow_iterator.next().unwrap_or(color);
                            let basis = dom_seq
                                .as_ref()
                                .and_then(|s| s.as_bytes().get(dom_position))
                                .or_else(|| {
                                    strand_seq
                                        .as_ref()
                                        .and_then(|s| s.as_bytes().get(strand_position))
                                });
                            loopout_nucls.push(LoopoutNucl {
                                position: *pos,
                                color,
                                repr_bond_identifier: id,
                                basis: basis.and_then(|b| b.clone().try_into().ok()),
                            });
                            if let Some(prev_pos) =
                                prev_loopout_pos.take().or(old_nucl_id
                                    .and_then(|id| space_position.get(&id).map(Vec3::from)))
                            {
                                loopout_bonds.push(LoopoutBond {
                                    position_prime5: prev_pos,
                                    position_prime3: *pos,
                                    color,
                                    repr_bond_identifier: id,
                                });
                            }
                            prev_loopout_pos = Some(*pos);
                            strand_position += 1;
                        }
                    }
                    insertion_length.insert(id, *nb_nucl);
                    last_xover_junction = Some(&mut strand.junctions[i]);
                }
            }
            if strand.cyclic {
                let nucl = strand.get_5prime().unwrap();
                let prime5_id = nucl_collection.get_identifier(&nucl).unwrap();
                let bound_id = id;
                if let Some((prev_pos, position)) =
                    prev_loopout_pos.take().zip(space_position.get(&prime5_id))
                {
                    loopout_bonds.push(LoopoutBond {
                        position_prime5: prev_pos,
                        position_prime3: position.into(),
                        color,
                        repr_bond_identifier: id,
                    });
                }
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
                log::debug!("adding {:?}, {:?}", bound.0, bound.1);
                Self::update_junction(
                    &mut new_junctions,
                    strand
                        .junctions
                        .last_mut()
                        .expect("Broke Invariant [LastXoverJunction]"),
                    (bound.0, bound.1),
                );
                let (prime5, prime3) = bound;
                if let Some(id) = new_junctions.get_id(&(prime5, prime3)) {
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
                if let Some(len) = insertion_length.remove(&id) {
                    insertion_length.insert(id - 1, len);
                    for loopout_nucl in loopout_nucls.iter_mut() {
                        if loopout_nucl.repr_bond_identifier == id {
                            loopout_nucl.repr_bond_identifier = id - 1;
                        }
                    }
                    for loopout_bond in loopout_bonds.iter_mut() {
                        if loopout_bond.repr_bond_identifier == id {
                            loopout_bond.repr_bond_identifier = id - 1;
                        }
                    }
                }
                if let Some(nucl) = old_nucl {
                    let color = strand.color;
                    prime3_set.push(Prime3End { nucl, color });
                }
            }
            old_nucl = None;
            old_nucl_id = None;
        }
        for g_id in grid_manager.grids.keys() {
            if let GridId::FreeGrid(id) = g_id {
                elements.push(DnaElement::Grid {
                    id: *id,
                    visible: grid_manager.get_visibility(*g_id),
                })
            }
        }
        for (h_id, h) in design.helices.iter() {
            elements.push(DnaElement::Helix {
                id: *h_id,
                group: groups.get(h_id).cloned(),
                visible: h.visible,
                locked_for_simualtions: h.locked_for_simulations,
            });
        }
        let mut ret = Self {
            object_type,
            nucleotide,
            nucleotides_involved,
            nucl_collection: Arc::new(nucl_collection),
            identifier_bound,
            strand_map,
            space_position,
            color: color_map,
            helix_map,
            basis_map: Arc::new(basis_map),
            prime3_set,
            elements,
            grid_manager,
            suggestions: vec![],
            loopout_bonds,
            loopout_nucls,
            insertion_length,
        };
        let suggestions = suggestion_maker.get_suggestions(&design, suggestion_parameters);
        ret.suggestions = suggestions;

        drop(groups);

        #[cfg(test)]
        {
            ret.test_named_junction(&design, &mut new_junctions, "TEST AFTER MAKE HASH MAP");
        }
        (ret, design, new_junctions)
    }

    fn update_junction(
        new_xover_ids: &mut JunctionsIds,
        junction: &mut DomainJunction,
        bound: (Nucl, Nucl),
    ) {
        let is_xover = bound.0.prime3() != bound.1;
        match junction {
            DomainJunction::Adjacent if is_xover => {
                let id = new_xover_ids.insert(bound);
                *junction = DomainJunction::IdentifiedXover(id);
            }
            DomainJunction::UnindentifiedXover | DomainJunction::IdentifiedXover(_)
                if !is_xover =>
            {
                *junction = DomainJunction::Adjacent;
            }
            DomainJunction::UnindentifiedXover => {
                let id = new_xover_ids.insert(bound);
                *junction = DomainJunction::IdentifiedXover(id);
            }
            DomainJunction::IdentifiedXover(id) => {
                new_xover_ids.insert_at(bound, *id);
            }
            _ => (),
        }
    }

    #[allow(dead_code)]
    pub fn get_shift(&self, g_id: GridId) -> Option<f32> {
        self.grid_manager
            .grids
            .get(&g_id)
            .and_then(|g| g.grid_type.get_shift())
    }

    pub fn read_simualtion_update(&mut self, update: &dyn SimulationUpdate) {
        update.update_positions(self.nucl_collection.as_ref(), &mut self.space_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    impl DesignContent {
        pub(super) fn test_named_junction(
            &self,
            design: &Design,
            xover_ids: &JunctionsIds,
            fail_msg: &'static str,
        ) {
            let mut xover_cpy = xover_ids.clone();
            for s in design.strands.values() {
                let mut expected_prime5: Option<Nucl> = None;
                let mut expected_prime5_domain: Option<usize> = None;
                let nb_taken = if s.cyclic {
                    2 * s.domains.len()
                } else {
                    s.domains.len()
                };
                for (i, d) in s.domains.iter().enumerate().cycle().take(nb_taken) {
                    if let Some(prime3) = d.prime5_end() {
                        if let Some(prime5) = expected_prime5 {
                            if prime5.prime3() == prime3 {
                                // Expect adjacent
                                if s.junctions[expected_prime5_domain.unwrap()]
                                    != DomainJunction::Adjacent
                                {
                                    panic!(
                                        "In test{} \n
                                        Expected junction {:?}, got {:?}\n
                                        junctions are {:?}",
                                        fail_msg,
                                        DomainJunction::Adjacent,
                                        s.junctions[expected_prime5_domain.unwrap()],
                                        s.junctions,
                                    );
                                }
                            } else {
                                // Expect named xover
                                if let Some(id) = xover_ids.get_id(&(prime5, prime3)) {
                                    xover_cpy.remove(id);
                                    if s.junctions[expected_prime5_domain.unwrap()]
                                        != DomainJunction::IdentifiedXover(id)
                                    {
                                        panic!(
                                            "In test{} \n
                                        Expected junction {:?}, got {:?}\n
                                        junctions are {:?}",
                                            fail_msg,
                                            DomainJunction::IdentifiedXover(id),
                                            s.junctions[expected_prime5_domain.unwrap()],
                                            s.junctions,
                                        );
                                    }
                                } else {
                                    panic!(
                                        "In test{} \n
                                        Could not find xover in xover_ids {:?}
                                        xover_ids: {:?}",
                                        fail_msg,
                                        (prime5, prime3),
                                        xover_ids.get_all_elements(),
                                    );
                                }
                            }
                            if expected_prime5_domain.unwrap() >= i {
                                break;
                            }
                        }
                    }
                    if let Some(nucl) = d.prime3_end() {
                        expected_prime5 = Some(nucl);
                    }
                    expected_prime5_domain = Some(i);
                }
            }
            assert!(
                xover_cpy.is_empty(),
                "In test {}\n
            Remaining xovers {:?}",
                fail_msg,
                xover_cpy.get_all_elements()
            );
        }
    }
}

trait GridInstancesMaker {
    fn grid_instances(&self, design_id: usize) -> BTreeMap<GridId, GridInstance>;
}

impl GridInstancesMaker for GridData {
    fn grid_instances(&self, design_id: usize) -> BTreeMap<GridId, GridInstance> {
        let mut ret = BTreeMap::new();
        for (g_id, g) in self.grids.iter() {
            let grid = GridInstance {
                grid: g.clone(),
                min_x: -2,
                max_x: 2,
                min_y: -2,
                max_y: 2,
                color: 0x00_00_FF,
                design: design_id,
                id: *g_id,
                fake: false,
                visible: !g.invisible,
            };
            ret.insert(*g_id, grid);
        }
        for grid_position in self.get_all_used_grid_positions() {
            if let Some(grid) = ret.get_mut(&grid_position.grid) {
                grid.min_x = grid.min_x.min(grid_position.x as i32 - 2);
                grid.max_x = grid.max_x.max(grid_position.x as i32 + 2);
                grid.min_y = grid.min_y.min(grid_position.y as i32 - 2);
                grid.max_y = grid.max_y.max(grid_position.y as i32 + 2);
            }
        }
        ret
    }
}

enum StapleDomain {
    ScaffoldDomain {
        domain_id: usize,
        first_scaffold_position: usize,
        last_scaffold_position: usize,
    },
    OtherDomain {
        length: usize,
    },
}

#[derive(Clone, Copy)]
struct ScaffoldPosition {
    domain_id: usize,
    scaffold_position: usize,
}

impl StapleDomain {
    fn init(scaffold_position: Option<ScaffoldPosition>) -> Self {
        if let Some(pos) = scaffold_position {
            Self::ScaffoldDomain {
                domain_id: pos.domain_id,
                first_scaffold_position: pos.scaffold_position,
                last_scaffold_position: pos.scaffold_position,
            }
        } else {
            Self::OtherDomain { length: 0 }
        }
    }

    fn reset(scaffold_position: Option<ScaffoldPosition>) -> Self {
        if let Some(pos) = scaffold_position {
            Self::ScaffoldDomain {
                domain_id: pos.domain_id,
                first_scaffold_position: pos.scaffold_position,
                last_scaffold_position: pos.scaffold_position,
            }
        } else {
            Self::OtherDomain { length: 1 }
        }
    }

    fn finish(&self) -> (isize, isize) {
        match self {
            Self::OtherDomain { length } => (-1, -(*length as isize)),
            Self::ScaffoldDomain {
                first_scaffold_position,
                last_scaffold_position,
                ..
            } => (
                *first_scaffold_position as isize,
                *last_scaffold_position as isize,
            ),
        }
    }

    fn read_position(mut self, position: Option<ScaffoldPosition>) -> ReadResult {
        match &mut self {
            Self::OtherDomain { length } => {
                if position.is_none() {
                    *length += 1;
                    ReadResult::Continue(self)
                } else {
                    ReadResult::Stop {
                        interval: self.finish(),
                        new_reader: Self::reset(position),
                    }
                }
            }
            Self::ScaffoldDomain {
                domain_id,
                last_scaffold_position,
                ..
            } => {
                if let Some(pos) = position.filter(|p| p.domain_id == *domain_id) {
                    *last_scaffold_position = pos.scaffold_position;
                    ReadResult::Continue(self)
                } else {
                    ReadResult::Stop {
                        interval: self.finish(),
                        new_reader: Self::reset(position),
                    }
                }
            }
        }
    }
}

enum ReadResult {
    Continue(StapleDomain),
    Stop {
        interval: (isize, isize),
        new_reader: StapleDomain,
    },
}
