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
use ensnano_design::grid::GridPosition;
use ensnano_design::*;
use ensnano_interactor::ObjectType;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use ultraviolet::Vec3;

use grid_data::GridManager;

mod xover_suggestions;
use xover_suggestions::XoverSuggestions;

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
    pub identifier_nucl: HashMap<Nucl, u32, RandomState>,
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
    pub(super) grid_manager: GridManager,
}

impl DesignContent {
    pub(super) fn get_grid_instances(&self) -> Vec<GridInstance> {
        self.grid_manager.grid_instances(0)
    }

    pub(super) fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
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

    pub(super) fn get_helix_grid_position(&self, h_id: usize) -> Option<GridPosition> {
        self.grid_manager.helix_to_pos.get(&h_id).cloned()
    }

    pub(super) fn get_grid_latice_position(&self, g_id: usize, x: isize, y: isize) -> Option<Vec3> {
        let grid = self.grid_manager.grids.get(g_id)?;
        Some(grid.position_helix(x, y))
    }

    pub(super) fn get_helices_grid_key_coord(&self, g_id: usize) -> Vec<((isize, isize), usize)> {
        self.grid_manager
            .pos_to_helix
            .iter()
            .filter(|t| t.0 .0 == g_id)
            .map(|t| ((t.0 .1, t.0 .2), *t.1))
            .collect()
    }

    pub(super) fn get_used_coordinates_on_grid(&self, g_id: usize) -> Vec<(isize, isize)> {
        self.grid_manager
            .pos_to_helix
            .iter()
            .filter(|t| t.0 .0 == g_id)
            .map(|t| (t.0 .1, t.0 .2))
            .collect()
    }

    pub(super) fn get_helix_id_at_grid_coord(
        &self,
        g_id: usize,
        x: isize,
        y: isize,
    ) -> Option<usize> {
        self.grid_manager.pos_to_helix(g_id, x, y)
    }

    pub(super) fn get_persistent_phantom_helices_id(&self) -> HashSet<u32> {
        self.grid_manager
            .pos_to_helix
            .iter()
            .filter(|(k, _)| !self.grid_manager.no_phantoms.contains(&k.0))
            .map(|(_, v)| *v as u32)
            .collect()
    }

    pub(super) fn grid_has_small_spheres(&self, g_id: usize) -> bool {
        self.grid_manager.small_spheres.contains(&g_id)
    }

    pub(super) fn grid_has_persistent_phantom(&self, g_id: usize) -> bool {
        !self.grid_manager.no_phantoms.contains(&g_id)
    }

    pub(super) fn get_grid_shift(&self, g_id: usize) -> Option<f32> {
        self.grid_manager
            .grids
            .get(g_id)
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

    pub(super) fn get_staples(&self, design: &Design) -> Vec<Staple> {
        let mut ret = Vec::new();
        let mut sequences: BTreeMap<(usize, isize, usize, isize), StapleInfo> = Default::default();
        let basis_map = self.basis_map.as_ref();
        for (s_id, strand) in design.strands.iter() {
            if strand.length() == 0 || design.scaffold_id == Some(*s_id) {
                continue;
            }
            let mut sequence = String::new();
            let mut first = true;
            for domain in &strand.domains {
                if !first {
                    sequence.push(' ');
                }
                first = false;
                if let Domain::HelixDomain(dom) = domain {
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
}

struct StapleInfo {
    s_id: usize,
    sequence: String,
    strand_name: Option<Cow<'static, str>>,
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
        old_grid_ptr: &mut Option<usize>,
        suggestion_parameters: &SuggestionParameters,
    ) -> (Self, Design, JunctionsIds) {
        let groups = design.groups.clone();
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
        let mut elements = Vec::new();
        let mut prime3_set = Vec::new();
        let mut new_junctions: JunctionsIds = Default::default();
        let mut suggestion_maker = XoverSuggestions::default();
        xover_ids.agree_on_next_id(&mut new_junctions);
        let mut grid_manager = GridManager::new_from_design(&design);
        if *old_grid_ptr != Some(Arc::as_ptr(&design.grids) as usize) {
            *old_grid_ptr = Some(Arc::as_ptr(&design.grids) as usize);
            grid_manager.reposition_all_helices(&mut design);
        }
        for (s_id, strand) in design.strands.iter_mut() {
            elements.push(elements::DnaElement::Strand { id: *s_id });
            let mut strand_position = 0;
            let strand_seq = strand.sequence.as_ref().filter(|s| s.is_ascii());
            let color = strand.color;
            let mut last_xover_junction: Option<&mut DomainJunction> = None;
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
                        let position = design.helices[&domain.helix].space_pos(
                            design.parameters.as_ref().unwrap(),
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
                        suggestion_maker.add_nucl(nucl, position, groups.as_ref());
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
                        log::debug!("{:?}", strand.domains);
                        log::debug!("{:?}", strand.junctions);
                    }
                    last_xover_junction = Some(&mut strand.junctions[i]);
                } else if let Domain::Insertion(n) = domain {
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
                if let Some(nucl) = old_nucl {
                    let color = strand.color;
                    prime3_set.push(Prime3End { nucl, color });
                }
            }
            old_nucl = None;
            old_nucl_id = None;
        }
        for g_id in 0..grid_manager.grids.len() {
            elements.push(DnaElement::Grid {
                id: g_id,
                visible: grid_manager.get_visibility(g_id),
            })
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
            identifier_nucl,
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
                panic!("DomainJunction::Adjacent between non adjacent nucl")
            }
            DomainJunction::UnindentifiedXover | DomainJunction::IdentifiedXover(_)
                if !is_xover =>
            {
                panic!("Xover between adjacent nucls")
            }
            s @ DomainJunction::UnindentifiedXover => {
                let id = new_xover_ids.insert(bound);
                *s = DomainJunction::IdentifiedXover(id);
            }
            DomainJunction::IdentifiedXover(id) => {
                new_xover_ids.insert_at(bound, *id);
            }
            _ => (),
        }
    }

    #[allow(dead_code)]
    pub fn get_shift(&self, g_id: usize) -> Option<f32> {
        self.grid_manager
            .grids
            .get(g_id)
            .and_then(|g| g.grid_type.get_shift())
    }

    pub fn read_simualtion_update(&mut self, update: &dyn SimulationUpdate) {
        update.update_positions(&self.identifier_nucl, &mut self.space_position)
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
