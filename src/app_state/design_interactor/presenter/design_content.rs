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
use ensnano_design::grid::GridDescriptor;
use ensnano_design::grid::GridPosition;
use ensnano_design::*;
use ensnano_interactor::ObjectType;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use ultraviolet::Vec3;

use grid_data::GridManager;

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
    /// The position in space of the nucleotides in the Red group
    pub red_cubes: HashMap<(isize, isize, isize), Vec<Nucl>, RandomState>,
    /// The list of nucleotides in the blue group
    pub blue_nucl: Vec<Nucl>,
    pub prime3_set: Vec<Prime3End>,
    pub elements: Vec<DnaElement>,
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
        let mut sequences: BTreeMap<(usize, isize, usize, isize), (usize, String)> =
            Default::default();
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
            ret.push(Staple {
                plate,
                well: format!("{}{}", column, row.to_string()),
                sequence: sequence.clone(),
                name: format!(
                    "Staple {:04}; 5':h{}:nt{}>3':h{}:nt{}",
                    s_id, *h5, *nt5, *h3, *nt3
                ),
            });
        }
        ret
    }
}

#[derive(Debug)]
pub struct Staple {
    pub well: String,
    pub name: String,
    pub sequence: String,
    pub plate: usize,
}

#[derive(Clone)]
pub struct Prime3End {
    pub position_start: Vec3,
    pub position_end: Vec3,
    pub color: u32,
}

impl DesignContent {
    /// Update all the hash maps
    pub(super) fn make_hash_maps(
        mut design: Design,
        groups: &BTreeMap<usize, bool>,
        xover_ids: AddressPointer<JunctionsIds>,
        old_grid_ptr: &mut Option<usize>,
    ) -> (Self, Design, Option<JunctionsIds>) {
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
        let mut red_cubes = HashMap::default();
        let mut elements = Vec::new();
        let mut prime3_set = Vec::new();
        let mut blue_nucl = Vec::new();
        let mut new_junctions: Option<JunctionsIds> = None;
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
                        xover_ids.clone(),
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
                        match groups.get(&nucl.helix) {
                            Some(true) => {
                                blue_nucl.push(nucl);
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
                println!("adding {:?}, {:?}", bound.0, bound.1);
                Self::update_junction(
                    &mut new_junctions,
                    xover_ids.clone(),
                    strand
                        .junctions
                        .last_mut()
                        .expect("Broke Invariant [LastXoverJunction]"),
                    (bound.0, bound.1),
                );
                let (prime5, prime3) = bound;
                if let Some(id) = get_shared(&new_junctions, &xover_ids).get_id(&(prime5, prime3)) {
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
                    let position_start = design.helices[&nucl.helix].space_pos(
                        design.parameters.as_ref().unwrap(),
                        nucl.position,
                        nucl.forward,
                    );
                    let position_end = design.helices[&nucl.helix].space_pos(
                        design.parameters.as_ref().unwrap(),
                        nucl.prime3().position,
                        nucl.forward,
                    );
                    let color = strand.color;
                    prime3_set.push(Prime3End {
                        position_start,
                        position_end,
                        color,
                    });
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
            });
        }
        let ret = Self {
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
            red_cubes,
            prime3_set,
            blue_nucl,
            elements,
            grid_manager,
        };

        drop(groups);

        #[cfg(test)]
        {
            let xover_ids = get_shared(&new_junctions, &xover_ids);
            ret.test_named_junction(&design, xover_ids, "TEST AFTER MAKE HASH MAP");
        }
        (ret, design, new_junctions)
    }

    fn update_junction(
        new_xover_ids: &mut Option<JunctionsIds>,
        old_xover_ids: AddressPointer<JunctionsIds>,
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
                let xover_ids = get_mutable(new_xover_ids, old_xover_ids.clone());
                let id = xover_ids.insert(bound);
                *s = DomainJunction::IdentifiedXover(id);
            }
            DomainJunction::IdentifiedXover(id) => {
                let xover_ids = get_shared(new_xover_ids, &old_xover_ids);
                let old_bound = xover_ids.get_element(*id);
                if old_bound != Some(bound) {
                    let xover_ids = get_mutable(new_xover_ids, old_xover_ids.clone());
                    xover_ids.update(old_bound.expect("Could not get exisiting id"), bound);
                }
            }
            _ => (),
        }
    }

    pub fn get_shift(&self, g_id: usize) -> Option<f32> {
        self.grid_manager
            .grids
            .get(g_id)
            .and_then(|g| g.grid_type.get_shift())
    }
}

fn get_mutable<T: Default + Clone>(new: &mut Option<T>, old: AddressPointer<T>) -> &mut T {
    if new.is_some() {
        new.as_mut().unwrap()
    } else {
        *new = Some(old.clone_inner());
        new.as_mut().unwrap()
    }
}

fn get_shared<'a, T: Default>(new: &'a Option<T>, old: &'a AddressPointer<T>) -> &'a T {
    if let Some(new) = new.as_ref() {
        new
    } else {
        old.as_ref()
    }
}

fn space_to_cube(x: f32, y: f32, z: f32) -> (isize, isize, isize) {
    let cube_len = 1.2;
    (
        x.div_euclid(cube_len) as isize,
        y.div_euclid(cube_len) as isize,
        z.div_euclid(cube_len) as isize,
    )
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
