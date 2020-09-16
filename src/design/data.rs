use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use super::View;
use std::path::PathBuf;

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
}

impl Data {
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
        }
    }

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
        let mut id = 0u32;
        let mut old_nucl = None;
        for (s_id, strand) in self.design.strands.iter().enumerate() {
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
                    object_type.insert(id, ObjectType::Nucleotide);
                    nucleotide.insert(id, nucl);
                    identifier_nucl.insert(nucl, id);
                    nucl_to_strand.insert(nucl, s_id);
                    let position = [position[0] as f32, position[1] as f32, position[2] as f32];
                    space_position.insert(id, position);
                    id += 1;
                    if let Some(old_nucl) = old_nucl.take() {
                        let bound = (old_nucl, nucl);
                        object_type.insert(id, ObjectType::Bound);
                        identifier_bound.insert(bound, id);
                        nucleotides_involved.insert(id, bound);
                        id += 1;
                    }
                    old_nucl = Some(nucl);
                }
            }
            old_nucl = None;
        }
        self.object_type = object_type;
        self.nucleotide = nucleotide;
        self.nucleotides_involved = nucleotides_involved;
        self.identifier_nucl = identifier_nucl;
        self.identifier_bound = identifier_bound;
        self.nucl_to_strand = nucl_to_strand;
        self.space_position = space_position;
    }

    pub fn get_design(&mut self, file: &PathBuf) {
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

    pub fn update_view(&self) {
        let mut nucleotide = Vec::new();
        let mut covalent_bound = Vec::new();
        for (n_id, nucl) in self.nucleotide.iter() {
            let position = self.space_position.get(n_id).expect("space position");
            let color = if let Some(s_id) = self.nucl_to_strand.get(nucl) {
                self.design.strands[*s_id].default_color().as_int()
            } else {
                0xFF0000
            };
            nucleotide.push((*position, color, *n_id));
        }
        for (b_id, bound) in self.nucleotides_involved.iter() {
            let (nucl1, nucl2) = bound;
            let color = if let Some(s_id) = self.nucl_to_strand.get(nucl1) {
                self.design.strands[*s_id].default_color().as_int()
            } else {
                unreachable!("a bound not on a strand");
            };
            covalent_bound.push((
                self.get_space_pos(nucl1).unwrap(),
                self.get_space_pos(nucl2).unwrap(),
                color,
                *b_id,
            ));
        }
        self.view.borrow_mut().update_spheres(&nucleotide);
        self.view.borrow_mut().update_tubes(&covalent_bound);
    }

    /*
    pub fn update_scene_selection(&self, scene: &mut Scene) {
        if let Some(id) = scene.get_selected_id() {
            if let Some(kind) = self.object_type.get(&id) {
                match kind {
                    ObjectType::Bound => {
                        let (nucl1, nucl2) = self.nucleotides_involved.get(&id).unwrap();
                        let pos1 = self.get_space_pos(nucl1).unwrap();
                        let pos2 = self.get_space_pos(nucl2).unwrap();
                        scene.update_selected_tube(pos1, pos2);
                    }
                    ObjectType::Nucleotide => {
                        scene.update_selected_sphere(*self.space_position.get(&id).unwrap())
                    }
                }
            }
        }
    }
    */

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
}


#[derive(Clone, Copy, Eq, PartialEq)]
enum ObjectType {
    Nucleotide,
    Bound,
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
struct Nucl {
    position: isize,
    helix: isize,
    forward: bool,
}
