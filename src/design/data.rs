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
use native_dialog::{Dialog, MessageAlert};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use ultraviolet::Vec3;

use std::borrow::Cow;

mod codenano;
mod grid;
mod icednano;
mod strand_builder;
use crate::scene::GridInstance;
use grid::GridManager;
pub use grid::*;
pub use icednano::Nucl;
pub use icednano::{Axis, Design, Parameters};
use std::sync::{Arc, RwLock};
pub use strand_builder::StrandBuilder;
use strand_builder::{DomainIdentifier, NeighbourDescriptor};

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
    basis_map: HashMap<Nucl, char>,
    grid_manager: GridManager,
    grids: Vec<Arc<RwLock<Grid2D>>>,
    color_idx: usize,
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
            basis_map: HashMap::new(),
            grid_manager,
            grids: Vec::new(),
            color_idx: 0,
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
            basis_map: HashMap::new(),
            grid_manager,
            grids,
            color_idx,
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
        self.basis_map = basis_map;
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

    /// Return an iterator over all the identifier of elements that are bounds
    pub fn get_all_bound_ids<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotides_involved.keys().copied()
    }

    /// Return the identifier of the strand on which an element lies
    pub fn get_strand(&self, id: u32) -> Option<usize> {
        self.strand_map.get(&id).cloned()
    }

    /// Return the identifier of the helix on which an element lies
    pub fn get_helix(&self, id: u32) -> Option<usize> {
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
        self.design
            .strands
            .get_mut(&s_id)
            .expect("wrong s_id in change_strand_color")
            .color = color;
        self.color.insert(s_id as u32, color);
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
            .reattach_helix(h_id, &mut self.design, false);
        self.grid_manager.update(&mut self.design);
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn translate_helix(&mut self, h_id: usize, translation: Vec3) {
        self.design
            .helices
            .get_mut(&h_id)
            .map(|h| h.translate(translation));
        self.grid_manager
            .reattach_helix(h_id, &mut self.design, true);
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
                self.get_grid_basis(grid_pos.grid as u32).unwrap()
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
            let mut new_key = 0usize;
            while self.design.strands.contains_key(&new_key) {
                new_key += 1;
            }
            let color = {
                let hue = (self.color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
                let saturation =
                    (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.6;
                let value =
                    (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.3;
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

    pub fn get_symbol(&self, e_id: u32) -> Option<char> {
        self.nucleotide.get(&e_id).and_then(|nucl| {
            self.basis_map
                .get(nucl)
                .cloned()
                .or_else(|| compl(self.basis_map.get(&nucl.compl()).cloned()))
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
            for domain in strand3prime.domains.iter() {
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
            self.design
                .strands
                .get_mut(&prime5)
                .as_mut()
                .unwrap()
                .cyclic = true;
            self.hash_maps_update = true;
            self.update_status = true;
        }
    }

    pub fn split_strand(&mut self, nucl: &Nucl) {
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

        let strand = self.design.strands.remove(&id).expect("strand");
        if strand.cyclic {
            self.design.strands.insert(id, strand);
            println!("Cutting cyclic strand is not implemented yet");
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
        for (d_id, domain) in strand.domains.iter().enumerate() {
            if domain.prime5_end() == Some(*nucl) {
                i = d_id;
                break;
            } else if domain.prime3_end() == Some(*nucl) {
                i = d_id + 1;
                prim5_domains.push(domain.clone());
                len_prim5 += domain.length();
                break;
            } else if let Some(n) = domain.has_nucl(nucl) {
                i = d_id;
                len_prim5 += n;
                domains = domain.split(n);
                break;
            } else {
                len_prim5 += domain.length();
                prim5_domains.push(domain.clone());
            }
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
        let id_5prime = *self.design.strands.keys().max().unwrap_or(&0) + 1;
        let id_3prime = id_5prime + 1;
        if strand_5prime.domains.len() > 0 {
            self.design.strands.insert(id_5prime, strand_5prime);
        }
        if strand_3prime.domains.len() > 0 {
            self.design.strands.insert(id_3prime, strand_3prime);
        }
        self.update_status = true;
        self.hash_maps_update = true;
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

    pub fn get_helices_grid(&self, g_id: u32) -> Option<HashSet<u32>> {
        self.grids.get(g_id as usize).map(|g| {
            g.read()
                .unwrap()
                .helices()
                .values()
                .cloned()
                .map(|x| x as u32)
                .collect()
        })
    }

    pub fn get_helices_grid_coord(&self, g_id: usize) -> Option<Vec<(isize, isize)>> {
        self.grids
            .get(g_id)
            .map(|g| g.read().unwrap().helices().keys().cloned().collect())
    }

    pub fn get_helix_grid(&self, g_id: u32, x: isize, y: isize) -> Option<u32> {
        self.grids
            .get(g_id as usize)
            .and_then(|g| g.read().unwrap().helices().get(&(x, y)).map(|x| *x as u32))
    }

    pub fn get_grid_basis(&self, g_id: u32) -> Option<ultraviolet::Rotor3> {
        self.grid_manager
            .grids
            .get(g_id as usize)
            .map(|g| g.orientation)
    }

    pub fn get_grid_position(&self, g_id: u32) -> Option<Vec3> {
        self.grid_manager
            .grids
            .get(g_id as usize)
            .map(|g| g.position)
    }

    pub fn build_helix_grid(&mut self, g_id: usize, x: isize, y: isize) {
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
                self.update_status = true;
                self.hash_maps_update = true;
                self.grid_manager.update(&mut self.design);
                self.update_grids();
            }
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
        }
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

    pub fn has_persistent_phantom(&self, g_id: &u32) -> bool {
        self.grids[*g_id as usize]
            .read()
            .unwrap()
            .persistent_phantom
    }

    pub fn set_persistent_phantom(&mut self, g_id: &u32, persistent: bool) {
        self.grids[*g_id as usize]
            .write()
            .unwrap()
            .persistent_phantom = persistent;
        self.update_status = true;
    }

    pub fn get_grid_pos_helix(&self, h_id: u32) -> Option<GridPosition> {
        self.design
            .helices
            .get(&(h_id as usize))
            .and_then(|h| h.grid_position)
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
            let error_msg = MessageAlert {
                title: "Error",
                text: "Unrecognized file format",
                typ: native_dialog::MessageType::Error,
            };
            std::thread::spawn(|| {
                error_msg.show().unwrap_or(());
            });
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
