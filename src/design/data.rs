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
use native_dialog::{MessageDialog, MessageType};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use ultraviolet::Vec3;

use std::borrow::Cow;
use std::fmt;

mod codenano;
mod grid;
mod icednano;
mod strand_builder;
use crate::scene::GridInstance;
use grid::GridManager;
pub use grid::*;
pub use icednano::Nucl;
pub use icednano::{Axis, Design, Helix, Parameters, Strand};
use std::sync::{Arc, RwLock};
use strand_builder::NeighbourDescriptor;
pub use strand_builder::{DomainIdentifier, StrandBuilder};

/// In addition to its `design` field, the `Data` struct has several hashmaps that are usefull to
/// quickly access information about the design. These hasmaps must be updated when the design is
/// modified.
///
/// At the moment, the hash maps are completely recomputed on every modification of the design. In
/// the future this might be optimised.
pub struct Data {
    design: icednano::Design,
    object_type: HashMap<u32, ObjectType>,
    /// Maps identifier of nucleotide to Nucleotide objects
    nucleotide: HashMap<u32, Nucl>,
    /// Maps identifier of bounds to the pair of nucleotides involved in the bound
    nucleotides_involved: HashMap<u32, (Nucl, Nucl)>,
    /// Maps identifier of element to their position in the Model's coordinates
    space_position: HashMap<u32, [f32; 3]>,
    /// Maps a Nucl object to its identifier
    identifier_nucl: HashMap<Nucl, u32>,
    /// Maps a pair of nucleotide forming a bound to the identifier of the bound
    identifier_bound: HashMap<(Nucl, Nucl), u32>,
    /// Maps the identifier of a element to the identifier of the strands to which it belongs
    strand_map: HashMap<u32, usize>,
    /// Maps the identifier of a element to the identifier of the helix to which it belongs
    helix_map: HashMap<u32, usize>,
    /// Maps the identifier of an element to its color
    color: HashMap<u32, u32>,
    /// Must be set to true when the design is modified, so that its obeservers get notified of the
    /// modification
    update_status: bool,
    /// Must be set to true when a modification that requires an update of the hash maps is
    /// performed
    hash_maps_update: bool,
    /// Maps nucleotides to basis characters
    basis_map: Arc<RwLock<HashMap<Nucl, char>>>,
    grid_manager: GridManager,
    grids: Vec<Arc<RwLock<Grid2D>>>,
    color_idx: usize,
    view_need_reset: bool,
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
        Self {
            design,
            object_type: HashMap::new(),
            space_position: HashMap::new(),
            identifier_nucl: HashMap::new(),
            identifier_bound: HashMap::new(),
            nucleotides_involved: HashMap::new(),
            nucleotide: HashMap::new(),
            strand_map: HashMap::new(),
            helix_map: HashMap::new(),
            color: HashMap::new(),
            update_status: false,
            hash_maps_update: false,
            basis_map: Arc::new(RwLock::new(HashMap::new())),
            grid_manager,
            grids: Vec::new(),
            color_idx: 0,
            view_need_reset: false,
        }
    }

    /// Create a new data by reading a file. At the moment, the supported format are
    /// * codenano
    /// * icednano
    pub fn new_with_path(json_path: &PathBuf) -> Option<Self> {
        let design = read_file(json_path)?;
        let grid_manager = GridManager::new_from_design(&design);
        let mut grids = grid_manager.grids2d();
        for g in grids.iter_mut() {
            g.write().unwrap().update(&design);
        }
        let color_idx = design.strands.keys().len();
        let mut ret = Self {
            design,
            object_type: HashMap::new(),
            space_position: HashMap::new(),
            identifier_nucl: HashMap::new(),
            identifier_bound: HashMap::new(),
            nucleotides_involved: HashMap::new(),
            nucleotide: HashMap::new(),
            strand_map: HashMap::new(),
            helix_map: HashMap::new(),
            color: HashMap::new(),
            update_status: false,
            // false because we call make_hash_maps here
            hash_maps_update: false,
            basis_map: Default::default(),
            grid_manager,
            grids,
            color_idx,
            view_need_reset: false,
        };
        ret.make_hash_maps();
        ret.terminate_movement();
        Some(ret)
    }

    /// Update all the hash maps
    fn make_hash_maps(&mut self) {
        let mut object_type = HashMap::new();
        let mut space_position = HashMap::new();
        let mut identifier_nucl = HashMap::new();
        let mut identifier_bound = HashMap::new();
        let mut nucleotides_involved = HashMap::new();
        let mut nucleotide = HashMap::new();
        let mut strand_map = HashMap::new();
        let mut color_map = HashMap::new();
        let mut helix_map = HashMap::new();
        let mut basis_map = HashMap::new();
        let mut id = 0u32;
        let mut nucl_id;
        let mut old_nucl = None;
        let mut old_nucl_id = None;
        for (s_id, strand) in self.design.strands.iter() {
            let mut strand_position = 0;
            let strand_seq = strand.sequence.as_ref().filter(|s| s.is_ascii());
            let color = strand.color;
            for domain in &strand.domains {
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
                } else if let icednano::Domain::Insertion(n) = domain {
                    strand_position += n;
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
        self.read_scaffold_seq(self.design.scaffold_shift.unwrap_or(0));
    }

    fn read_scaffold_seq(&mut self, shift: usize) {
        if let Some(mut sequence) = self
            .design
            .scaffold_sequence
            .as_ref()
            .map(|s| s.chars().cycle().skip(shift))
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

    /// Set the strand that is the scaffold
    pub fn set_scaffold_id(&mut self, scaffold_id: Option<usize>) {
        self.design.scaffold_id = scaffold_id;
        self.hash_maps_update = true;
        self.update_status = true;
    }

    /// Set the sequence of the scaffold
    pub fn set_scaffold_sequence(&mut self, sequence: String) {
        self.design.scaffold_sequence = Some(sequence);
        self.design.scaffold_shift = Some(0);
        self.hash_maps_update = true;
    }

    /// Save the design to a file in the `icednano` format
    pub fn save_file(&self, path: &PathBuf) -> std::io::Result<()> {
        let json_content = serde_json::to_string_pretty(&self.design);
        let mut f = std::fs::File::create(path)?;
        f.write_all(json_content.expect("serde_json failed").as_bytes())
    }

    /// Return true if self was updated since the last time this function was called.
    /// This function is meant to be called by the mediator that will notify all the obeservers
    /// that a update took place.
    pub fn was_updated(&mut self) -> bool {
        if self.hash_maps_update {
            self.make_hash_maps();
            self.hash_maps_update = false;
        }
        let ret = self.update_status;
        self.update_status = false;
        ret
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
            .filter(|(_, n)| self.get_visibility_helix(n.helix).unwrap_or(false))
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
            .filter(|(_, b)| {
                self.get_visibility_helix(b.0.helix).unwrap_or(false)
                    || self.get_visibility_helix(b.1.helix).unwrap_or(false)
            })
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

    pub fn translate_helix(&mut self, h_id: usize, translation: Vec3, snap_grid: bool) {
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.translate(translation));
        if snap_grid {
            self.grid_manager
                .reattach_helix(h_id, &mut self.design, true, &self.grids);
        }
        self.grid_manager.update(&mut self.design);
        self.update_grids();
        self.hash_maps_update = true;
        self.update_status = true;
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
    pub fn get_identifier_nucl(&self, nucl: Nucl) -> Option<u32> {
        self.identifier_nucl.get(&nucl).cloned()
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
                (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.6;
            let value = (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.3;
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

    /// Return the identifier of the strand whose nucl is the 5' end of, or `None` if nucl is not
    /// the 5' end of any strand.
    pub fn prime5_of(&self, nucl: &Nucl) -> Option<usize> {
        let id = self.identifier_nucl.get(nucl)?;
        let strand_id = self.strand_map.get(id)?;
        if self.design.strands[strand_id].cyclic {
            None
        } else {
            let real_prime5 = self.get_5prime(*strand_id)?;
            if *id == real_prime5 {
                Some(*strand_id)
            } else {
                None
            }
        }
    }

    /// Return the identifier of the strand whose nucl is the 3' end of, or `None` if nucl is not
    /// the 3' end of any strand.
    pub fn prime3_of(&self, nucl: &Nucl) -> Option<usize> {
        let id = self.identifier_nucl.get(nucl)?;
        let strand_id = self.strand_map.get(id)?;
        if self.design.strands[strand_id].cyclic {
            None
        } else {
            let real_prime3 = self.get_3prime(*strand_id)?;
            if *id == real_prime3 {
                Some(*strand_id)
            } else {
                None
            }
        }
    }

    /// Return Some(true) if nucl is the 3' end of a xover, Some(false) if nucl is the 5' end of a
    /// xover and None in the other cases
    pub fn is_xover_end(&self, nucl: &Nucl) -> Option<bool> {
        let id = self.identifier_nucl.get(nucl)?;
        let strand_id = self.strand_map.get(id)?;
        let strand = self.design.strands.get(strand_id).expect("strand");
        let mut prev_helix = None;
        for domain in strand.domains.iter() {
            if domain.prime5_end() == Some(*nucl) && prev_helix != domain.half_helix() {
                return Some(false);
            } else if domain.prime3_end() == Some(*nucl) {
                return Some(true);
            } else if let Some(_) = domain.has_nucl(nucl) {
                return None;
            }
            prev_helix = domain.half_helix();
        }
        return None;
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
            for domain in strand5prime.domains.iter() {
                domains.push(domain.clone());
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
            } else {
                skip = 0;
            }
            for domain in strand3prime.domains.iter().skip(skip) {
                domains.push(domain.clone());
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
        self.update_status = true;
        self.view_need_reset = true;
        self.make_hash_maps();
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

    /// Split a strand at nucl.
    ///
    /// The part of the strand that contains nucl is given the original
    /// strand's id, the other part is given a new id.
    ///
    /// If `force_end` is `Some(true)`, nucl will be on the 3 prime half of the split.
    /// If `force_end` is `Some(false)` nucl will be on the 5 prime half of the split.
    /// If `force_end` is `None`, nucl will be on the 5 prime half of the split unless nucl is the 3
    /// prime extremity of a crossover, in which case nucl will be on the 3 prime half of the
    /// split.
    pub fn split_strand(&mut self, nucl: &Nucl, force_end: Option<bool>) {
        self.update_status = true;
        self.hash_maps_update = true;
        self.view_need_reset = true;
        let id = self
            .identifier_nucl
            .get(nucl)
            .and_then(|id| self.strand_map.get(id));

        if id.is_none() {
            return;
        }
        let id = *id.unwrap();

        let strand = self.design.strands.remove(&id).expect("strand");
        if strand.cyclic {
            let new_strand = self.break_cycle(strand.clone(), *nucl, force_end);
            self.design.strands.insert(id, new_strand);
            println!("Cutting cyclic strand");
            return;
        }
        if strand.length() <= 1 {
            // return without putting the strand back
            return;
        }
        let mut i = strand.domains.len();
        let mut prim5_domains = Vec::new();
        let mut len_prim5 = 0;
        let mut domains = None;
        let mut on_3prime = force_end.unwrap_or(false);
        let mut prev_helix = None;
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
                break;
            } else if domain.prime3_end() == Some(*nucl) && force_end != Some(true) {
                // nucl is the 3' end of the current domain so it is the on the 5' end of a xover.
                // nucl is not required to be on the 3' half of the split, so we put it on the 5'
                // half
                i = d_id + 1;
                prim5_domains.push(domain.clone());
                len_prim5 += domain.length();
                break;
            } else if let Some(n) = domain.has_nucl(nucl) {
                let n = if force_end == Some(true) { n - 1 } else { n };
                i = d_id;
                len_prim5 += n;
                domains = domain.split(n);
                break;
            } else {
                len_prim5 += domain.length();
                prim5_domains.push(domain.clone());
            }
            prev_helix = domain.helix();
        }
        let mut prim3_domains = Vec::new();
        if let Some(ref domains) = domains {
            prim5_domains.push(domains.0.clone());
            prim3_domains.push(domains.1.clone());
            i += 1;
        }

        for domain in strand.domains.iter().skip(i) {
            prim3_domains.push(domain.clone());
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

        let strand_5prime = icednano::Strand {
            domains: prim5_domains,
            color: strand.color,
            cyclic: false,
            sequence: seq_prim5,
        };

        let strand_3prime = icednano::Strand {
            domains: prim3_domains,
            color: strand.color,
            cyclic: false,
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
        self.make_hash_maps();
        self.view_need_reset = true;
    }

    /// Split a cyclic strand at nucl
    ///
    /// If `force_end` is `Some(true)`, nucl will be the new 5' end of the strand.
    /// If `force_end` is `Some(false)` nucl will be the new 3' end of the strand.
    /// If `force_end` is `None`, nucl will be the new 3' end of the strand unless nucl is the 3'
    /// prime extremity of a crossover, in which case nucl will be the new 5' end of the strand
    fn break_cycle(&self, mut strand: Strand, nucl: Nucl, force_end: Option<bool>) -> Strand {
        let mut last_dom = None;
        let mut replace_last_dom = None;
        let mut prev_helix = None;

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
                break;
            } else if domain.prime3_end() == Some(nucl) && force_end != Some(true) {
                last_dom = Some(i);
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
            new_domains.push(d2.clone())
        }
        for d in strand.domains.iter().skip(last_dom + 1) {
            new_domains.push(d.clone());
        }
        for d in strand.domains.iter().take(last_dom) {
            new_domains.push(d.clone())
        }
        if let Some((ref d1, _)) = replace_last_dom {
            new_domains.push(d1.clone())
        } else {
            new_domains.push(strand.domains[last_dom].clone())
        }
        strand.domains = new_domains;
        strand.cyclic = false;
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
        println!("half1 {}, ; half0 {}", new_id, target_strand);
        self.split_strand(&nucl, Some(target_3prime));
        println!("splitted");

        if self.design.strands.get(&target_strand).unwrap().cyclic {
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
        self.design.strands.insert(source_id, source);
        self.design.strands.insert(target_id, target);
        self.make_hash_maps();
        self.view_need_reset = true;
    }

    pub fn rm_strand(&mut self, nucl: &Nucl) {
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
        self.grid_manager.guess_grids(&mut self.design, &groups);
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
            self.rm_strand(&Nucl {
                helix: h_id,
                position,
                forward: true,
            });
            self.rm_strand(&Nucl {
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

    pub fn add_grid(&mut self, desc: GridDescriptor) {
        self.grid_manager.add_grid(desc);
        self.update_status = true;
        self.hash_maps_update = true;
        self.grid_manager.update(&mut self.design);
        self.update_grids();
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
        self.identifier_nucl
            .get(nucl)
            .and_then(|id| self.strand_map.get(id).cloned())
    }

    pub fn get_visibility_helix(&self, h_id: usize) -> Option<bool> {
        self.design.helices.get(&h_id).map(|h| h.visible)
    }

    pub fn set_visibility_helix(&mut self, h_id: usize, visibility: bool) {
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.visible = visibility);
        self.update_status = true;
    }

    pub fn has_helix(&self, h_id: usize) -> bool {
        self.design.helices.contains_key(&h_id)
    }

    pub fn get_basis_map(&self) -> Arc<RwLock<HashMap<Nucl, char>>> {
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
        let basis_map = self.basis_map.read().unwrap();
        for (n, (s_id, strand)) in self.design.strands.iter().enumerate() {
            if strand.length() == 0 || self.design.scaffold_id == Some(*s_id) {
                continue;
            }
            let mut sequence = String::new();
            for domain in &strand.domains {
                if let icednano::Domain::HelixDomain(dom) = domain {
                    for position in dom.iter() {
                        let nucl = Nucl {
                            position,
                            forward: dom.forward,
                            helix: dom.helix,
                        };
                        sequence.push(*basis_map.get(&nucl).unwrap());
                    }
                }
                sequence.push(' ');
            }
            let plate = n / 96 + 1;
            let row = (n % 96) % 12 + 1;
            let column = match (n % 96) / 12 {
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
                sequence,
                name: format!("Stapple {}", n),
            });
        }
        ret
    }

    /// Shift the scaffold at an optimized poisition and return the corresponding score
    pub fn optimize_shift(&mut self) -> usize {
        let mut best_score = 10000;
        let mut best_shfit = 0;
        let len = self
            .design
            .scaffold_sequence
            .as_ref()
            .map(|s| s.len())
            .unwrap_or(0);
        for shift in 0..len {
            println!("reading {}", shift);
            self.read_scaffold_seq(shift);
            let score = self.evaluate_shift();
            if score < best_score {
                best_score = score;
                best_shfit = shift;
            }
            if score == 0 {
                break;
            }
        }
        self.design.scaffold_shift = Some(best_shfit);
        best_score
    }

    fn evaluate_shift(&self) -> usize {
        let basis_map = self.basis_map.read().unwrap();
        let mut ret = 0;
        let re = regex::Regex::new(r"G{4,}|C{4,}|[AT]{7,}").unwrap();
        for (s_id, strand) in self.design.strands.iter() {
            if strand.length() == 0 || self.design.scaffold_id == Some(*s_id) {
                continue;
            }
            let mut sequence = String::new();
            for domain in &strand.domains {
                if let icednano::Domain::HelixDomain(dom) = domain {
                    for position in dom.iter() {
                        let nucl = Nucl {
                            position,
                            forward: dom.forward,
                            helix: dom.helix,
                        };
                        sequence.push(*basis_map.get(&nucl).unwrap());
                    }
                }
                sequence.push(' ');
            }
            let mut matches = re.find_iter(&sequence);
            while matches.next().is_some() {
                ret += 1;
            }
        }
        ret
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
fn read_file(path: &PathBuf) -> Option<icednano::Design> {
    let json_str =
        std::fs::read_to_string(path).unwrap_or_else(|_| panic!("File not found {:?}", path));

    let design: Result<icednano::Design, _> = serde_json::from_str(&json_str);
    // First try to read icednano format
    if let Ok(design) = design {
        Some(design)
    } else {
        // If the file is not in icednano format, try the other supported format
        let cdn_design: Result<codenano::Design<(), ()>, _> = serde_json::from_str(&json_str);

        // Try codenano format
        if let Ok(design) = cdn_design {
            Some(icednano::Design::from_codenano(&design))
        } else {
            // The file is not in any supported format
            MessageDialog::new()
                .set_type(MessageType::Error)
                .set_text("Unrecognized file format")
                .show_alert()
                .unwrap();
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
