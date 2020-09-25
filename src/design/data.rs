use super::View;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use ultraviolet::{Rotor3, Vec3};

mod codenano;


type ViewPtr = Rc<RefCell<View>>;
pub struct Data {
    view: ViewPtr,
    design: codenano::Design<(), ()>,
    object_type: HashMap<u32, ObjectType>,
    nucleotide: HashMap<u32, Nucl>,
    nucleotides_involved: HashMap<u32, (Nucl, Nucl)>,
    space_position: HashMap<u32, [f32; 3]>,
    identifier_nucl: HashMap<Nucl, u32>,
    identifier_bound: HashMap<(Nucl, Nucl), u32>,
    nucl_to_strand: HashMap<Nucl, usize>,
    color: HashMap<u32, u32>,
    update_status: bool,
}

impl Data {
    #[allow(dead_code)]
    pub fn new(view: &ViewPtr) -> Self {
        let design = codenano::Design::<(), ()>::new();
        Self {
            view: view.clone(),
            design,
            object_type: HashMap::new(),
            space_position: HashMap::new(),
            identifier_nucl: HashMap::new(),
            identifier_bound: HashMap::new(),
            nucleotides_involved: HashMap::new(),
            nucleotide: HashMap::new(),
            nucl_to_strand: HashMap::new(),
            color: HashMap::new(),
            update_status: false,
        }
    }

    /// Create a new data by reading a file. At the moment only codenano's format is supported
    pub fn new_with_path(view: &ViewPtr, json_path: &PathBuf) -> Self {
        let json_str =
            std::fs::read_to_string(json_path).expect(&format!("File not found {:?}", json_path));
        let design = serde_json::from_str(&json_str).expect("Error in .json file");
        let mut ret = Self {
            view: view.clone(),
            design,
            object_type: HashMap::new(),
            space_position: HashMap::new(),
            identifier_nucl: HashMap::new(),
            identifier_bound: HashMap::new(),
            nucleotides_involved: HashMap::new(),
            nucleotide: HashMap::new(),
            nucl_to_strand: HashMap::new(),
            color: HashMap::new(),
            update_status: true,
        };
        ret.make_hash_maps();
        ret
    }

    fn make_hash_maps(&mut self) {
        let mut object_type = HashMap::new();
        let mut space_position = HashMap::new();
        let mut identifier_nucl = HashMap::new();
        let mut identifier_bound = HashMap::new();
        let mut nucleotides_involved = HashMap::new();
        let mut nucleotide = HashMap::new();
        let mut nucl_to_strand = HashMap::new();
        let mut color_map = HashMap::new();
        let mut id = 0u32;
        let mut nucl_id = 0u32;
        let mut old_nucl = None;
        let mut old_nucl_id = None;
        for (s_id, strand) in self.design.strands.iter().enumerate() {
            let color = strand.color.unwrap_or(strand.default_color()).as_int();
            for domain in &strand.domains {
                for nucl_position in domain.iter() {
                    let position = self.design.helices[domain.helix as usize].space_pos(
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
                    nucl_to_strand.insert(nucl, s_id);
                    color_map.insert(nucl_id, color);
                    let position = [position[0] as f32, position[1] as f32, position[2] as f32];
                    space_position.insert(nucl_id, position);
                    if let Some(old_nucl) = old_nucl.take() {
                        let bound_id = id;
                        id += 1;
                        let bound = (old_nucl, nucl);
                        object_type.insert(bound_id, ObjectType::Bound(old_nucl_id.unwrap(), nucl_id));
                        identifier_bound.insert(bound, bound_id);
                        nucleotides_involved.insert(bound_id, bound);
                        color_map.insert(bound_id, color);
                    }
                    old_nucl = Some(nucl);
                    old_nucl_id = Some(nucl_id);
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
        self.nucl_to_strand = nucl_to_strand;
        self.space_position = space_position;
        self.color = color_map;
    }

    #[allow(dead_code)]
    pub fn read_file(&mut self, file: &PathBuf) {
        let json_str = std::fs::read_to_string(file);
        if let Ok(json_str) = json_str {
            let design = serde_json::from_str(&json_str);
            if let Ok(design) = design {
                self.design = design
            } else {
                println!("could not read the new json file");
            }
        }
        self.make_hash_maps();
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
    pub fn get_element_position(&self, id: u32) -> Option<Vec3> {
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

    /// Return the ObjectType associated to the identifier `id`
    pub fn get_object_type(&self, id:u32) -> Option<ObjectType> {
        self.object_type.get(&id).cloned()
    }

    /// Return the identifier of the nucleotide involved in the bound `id`.
    pub fn get_nucl_involved(&self, id: u32) -> Option<(u32, u32)> {
        if let Some((n1, n2)) = self.nucleotides_involved.get(&id) {
            Some((*self.identifier_nucl.get(n1).unwrap(), *self.identifier_nucl.get(n2).unwrap()))
        } else {
            None
        }
    }

    /// Return the color of the element with identifier `id`
    pub fn get_color(&self, id:u32) -> Option<u32> {
        self.color.get(&id).cloned()
    }

    /// Return an iterator over all the identifier of elements that are nucleotides
    pub fn get_all_nucl_ids<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotide.keys().copied()
    }

    /// Return an iterator over all the identifier of elements that are bounds
    pub fn get_all_bound_ids<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        self.nucleotides_involved.keys().copied()
    }

    /// Return true if `id` is the identifier of a nucleotide
    pub fn is_nucl(&self, id: u32) -> bool {
        self.nucleotide.contains_key(&id)
    }

    /// Return true if `id` is the identifier of a bound
    pub fn is_bound(&self, id: u32) -> bool {
        self.nucleotides_involved.contains_key(&id)
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
            _ => false
        }
    }

    pub fn is_bound(&self) -> bool {
        match self {
            ObjectType::Bound(_, _) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct Nucl {
    position: isize,
    helix: isize,
    forward: bool,
}
