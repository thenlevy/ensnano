use native_dialog::{Dialog, MessageAlert};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use ultraviolet::Vec3;

mod codenano;
mod icednano;
mod strand_builder;
pub use icednano::Nucl;
pub use icednano::{Axis, Design};
pub use strand_builder::StrandBuilder;
use strand_builder::{DomainIdentifier, NeighbourDescriptor};

pub struct Data {
    design: icednano::Design,
    object_type: HashMap<u32, ObjectType>,
    nucleotide: HashMap<u32, Nucl>,
    nucleotides_involved: HashMap<u32, (Nucl, Nucl)>,
    space_position: HashMap<u32, [f32; 3]>,
    identifier_nucl: HashMap<Nucl, u32>,
    identifier_bound: HashMap<(Nucl, Nucl), u32>,
    strand_map: HashMap<u32, usize>,
    helix_map: HashMap<u32, usize>,
    color: HashMap<u32, u32>,
    update_status: bool,
    hash_maps_update: bool,
}

impl Data {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let design = icednano::Design::new();
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
        }
    }

    /// Create a new data by reading a file. At the moment, the supported format are
    /// * codenano
    /// * icednano
    pub fn new_with_path(json_path: &PathBuf) -> Option<Self> {
        let design = read_file(json_path)?;
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
        };
        ret.make_hash_maps();
        ret.terminate_movement();
        Some(ret)
    }

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
        let mut id = 0u32;
        let mut nucl_id;
        let mut old_nucl = None;
        let mut old_nucl_id = None;
        for (s_id, strand) in self.design.strands.iter() {
            let color = strand.color;
            for domain in &strand.domains {
                if let icednano::Domain::HelixDomain(domain) = domain {
                    for nucl_position in domain.iter() {
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
    }

    pub fn save_file(&self, path: &PathBuf) -> std::io::Result<()> {
        let json_content = serde_json::to_string_pretty(&self.design);
        let mut f = std::fs::File::create(path)?;
        f.write_all(json_content.expect("serde_json failed").as_bytes())
    }

    /// Return true if self was updated since the last time this function was called.
    pub fn was_updated(&mut self) -> bool {
        let ret = self.update_status;
        self.update_status = false;
        ret
    }

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
        if self.hash_maps_update {
            self.make_hash_maps();
            self.hash_maps_update = false;
        }
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

    pub fn get_element_axis_position(&mut self, id: u32) -> Option<Vec3> {
        if self.hash_maps_update {
            self.make_hash_maps();
            self.hash_maps_update = false;
        }
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
                    Some((Vec3::from(a) + Vec3::from(b)) / 2.)
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

    pub fn get_nucl(&self, e_id: u32) -> Option<Nucl> {
        self.nucleotide.get(&e_id).cloned()
    }

    pub fn get_helix_nucl(
        &self,
        helix_id: usize,
        nucl: isize,
        forward: bool,
        on_axis: bool,
    ) -> Option<Vec3> {
        self.design.helices.get(&helix_id).map(|h| {
            if on_axis {
                h.axis_position(&self.design.parameters.unwrap(), nucl)
            } else {
                h.space_pos(&self.design.parameters.unwrap(), nucl, forward)
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
    pub fn get_all_nucl_ids<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotide.keys().copied()
    }

    /// Return an iterator over all the identifier of elements that are bounds
    pub fn get_all_bound_ids<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotides_involved.keys().copied()
    }

    pub fn get_strand(&self, id: u32) -> Option<usize> {
        self.strand_map.get(&id).cloned()
    }

    pub fn get_helix(&self, id: u32) -> Option<usize> {
        self.helix_map.get(&id).cloned()
    }

    pub fn get_strand_elements(&self, s_id: usize) -> Vec<u32> {
        let mut ret = Vec::new();
        for elt in self.object_type.keys() {
            if self.strand_map.get(&elt) == Some(&s_id) {
                ret.push(*elt)
            }
        }
        ret
    }

    pub fn get_helix_elements(&self, h_id: usize) -> Vec<u32> {
        let mut ret = Vec::new();
        for elt in self.object_type.keys() {
            if self.helix_map.get(&elt) == Some(&h_id) {
                ret.push(*elt)
            }
        }
        ret
    }

    pub fn change_strand_color(&mut self, s_id: usize, color: u32) {
        self.design
            .strands
            .get_mut(&s_id)
            .expect("wrong s_id in change_strand_color")
            .color = color;
        self.color.insert(s_id as u32, color);
        self.update_status = true;
    }

    pub fn get_strand_color(&self, s_id: usize) -> Option<u32> {
        self.design.strands.get(&s_id).map(|s| s.color)
    }

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
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn terminate_movement(&mut self) {
        for helix in self.design.helices.values_mut() {
            helix.end_movement();
        }
    }

    pub fn get_helix_basis(&self, h_id: usize) -> Option<ultraviolet::Rotor3> {
        self.design
            .helices
            .get(&h_id)
            .as_ref()
            .map(|h| h.orientation)
    }

    pub fn get_5prime(&self, strand_id: usize) -> Option<u32> {
        let nucl = self
            .design
            .strands
            .get(&strand_id)
            .and_then(|s| s.get_5prime())?;
        self.identifier_nucl.get(&nucl).cloned()
    }

    pub fn get_3prime(&self, strand_id: usize) -> Option<u32> {
        let nucl = self
            .design
            .strands
            .get(&strand_id)
            .and_then(|s| s.get_3prime())?;
        self.identifier_nucl.get(&nucl).cloned()
    }

    pub fn get_neighbour_nucl(
        &self,
        helix: usize,
        position: isize,
        forwrard: bool,
    ) -> Option<NeighbourDescriptor> {
        self.design.get_neighbour_nucl(helix, position, forwrard)
    }

    pub fn update_strand(
        &mut self,
        identifier: DomainIdentifier,
        position: isize,
        fixed_position: isize,
    ) {
        let start = position.min(fixed_position);
        let end = position.max(fixed_position) - 1;
        let domain = &mut self
            .design
            .strands
            .get_mut(&identifier.strand)
            .unwrap()
            .domains[identifier.domain];
        match domain {
            icednano::Domain::HelixDomain(domain) => {
                domain.start = start;
                domain.end = end;
            }
            _ => (),
        }
        self.hash_maps_update = true;
    }

    pub fn get_strand_builder(
        &mut self,
        helix: usize,
        position: isize,
        forward: bool,
    ) -> Option<StrandBuilder> {
        let left = self.design.get_neighbour_nucl(helix, position - 1, forward);
        let right = self.design.get_neighbour_nucl(helix, position + 1, forward);
        if left.is_some() && right.is_some() {
            return None;
        }
        let axis = self
            .design
            .helices
            .get(&helix)
            .map(|h| h.get_axis(&self.design.parameters.unwrap()))?;
        if self.identifier_nucl.contains_key(&Nucl {
            helix,
            position,
            forward,
        }) {
            if let Some(desc) = self.design.get_neighbour_nucl(helix, position, forward) {
                Some(StrandBuilder::init_existing(
                    desc.identifier,
                    helix,
                    position,
                    forward,
                    axis,
                    desc.fixed_end,
                    left.or(right),
                ))
            } else {
                None
            }
        } else {
            let mut new_key = 0usize;
            while self.design.strands.contains_key(&new_key) {
                new_key += 1;
            }
            self.design
                .strands
                .insert(new_key, icednano::Strand::init(helix, position, forward));
            self.hash_maps_update = true;
            self.update_status = true;
            Some(StrandBuilder::init_empty(
                DomainIdentifier {
                    strand: new_key,
                    domain: 0,
                },
                helix,
                position,
                forward,
                axis,
                left.or(right),
            ))
        }
    }
}

fn read_file(path: &PathBuf) -> Option<icednano::Design> {
    let json_str = std::fs::read_to_string(path).expect(&format!("File not found {:?}", path));

    let design: Result<icednano::Design, _> = serde_json::from_str(&json_str);
    if design.is_ok() {
        let info_msg = MessageAlert {
            title: "Info",
            text: "Recognized icednano file format",
            typ: native_dialog::MessageType::Info,
        };
        std::thread::spawn(|| {
            info_msg.show().unwrap_or(());
        });
        return Some(design.unwrap());
    } else {
        let cdn_design: Result<codenano::Design<(), ()>, _> = serde_json::from_str(&json_str);
        if cdn_design.is_ok() {
            let info_msg = MessageAlert {
                title: "Info",
                text: "Recognized codenano file format",
                typ: native_dialog::MessageType::Info,
            };
            std::thread::spawn(|| {
                info_msg.show().unwrap_or(());
            });
            return Some(icednano::Design::from_codenano(&cdn_design.unwrap()));
        } else {
            let error_msg = MessageAlert {
                title: "Error",
                text: "Unrecognized file format",
                typ: native_dialog::MessageType::Error,
            };
            std::thread::spawn(|| {
                error_msg.show().unwrap_or(());
            });
            return None;
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ObjectType {
    Nucleotide(u32),
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
