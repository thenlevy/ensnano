use super::View;
use crate::utils::rotation_to_rotor;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use ultraviolet::{Rotor3, Vec3};

mod codenano;

type Basis = (f32, f64, f64, [f32; 3], u32);

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
    update_status: bool,
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
            update_status: false,
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
            update_status: true,
        };
        ret.make_hash_maps();
        ret.update_view();
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

    pub fn was_updated(&mut self) -> bool {
        let ret = self.update_status;
        self.update_status = false;
        ret
    }

    pub fn update_selection(&mut self, id: Option<u32>) {
        if let Some(id) = id {
            if let Some(kind) = self.object_type.get(&id) {
                match kind {
                    ObjectType::Bound => {
                        let (nucl1, nucl2) = self.nucleotides_involved.get(&id).unwrap();
                        let pos1 = self.get_space_pos(nucl1).unwrap();
                        let pos2 = self.get_space_pos(nucl2).unwrap();
                        self.view
                            .borrow_mut()
                            .update_selected_tubes(&vec![(pos1, pos2)]);
                    }
                    ObjectType::Nucleotide => self
                        .view
                        .borrow_mut()
                        .update_selected_spheres(&vec![*self.space_position.get(&id).unwrap()]),
                }
                self.update_status = true;
            } else {
                println!("not found");
            }
        } else {
            self.view.borrow_mut().update_selected_tubes(&vec![]);
            self.view.borrow_mut().update_selected_spheres(&vec![])
        }
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
    pub fn fit_design(&self, ratio: f32, fovy: f32) -> (Vec3, Rotor3) {
        let mut bases = self.get_bases(ratio);
        let rotation = self.get_fitting_quaternion(&bases);
        let direction = rotation * -Vec3::unit_z();
        let position = self.get_fitting_position(&mut bases, ratio, fovy, &direction);
        (position, rotation)
    }

    pub fn middle_point(&self) -> Vec3 {
        let boundaries = self.boundaries();
        Vec3::new(
            (boundaries[0] + boundaries[1]) as f32 / 2.,
            (boundaries[2] + boundaries[3]) as f32 / 2.,
            (boundaries[4] + boundaries[5]) as f32 / 2.,
        )
    }

    fn boundaries(&self) -> [f64; 6] {
        let mut min_x = std::f64::INFINITY;
        let mut min_y = std::f64::INFINITY;
        let mut min_z = std::f64::INFINITY;
        let mut max_x = std::f64::NEG_INFINITY;
        let mut max_y = std::f64::NEG_INFINITY;
        let mut max_z = std::f64::NEG_INFINITY;

        let param = &self.design.parameters.unwrap();
        for s in &self.design.strands {
            for d in &s.domains {
                let helix = &self.design.helices[d.helix as usize];
                for coord in vec![
                    helix.space_pos(param, d.start, d.forward),
                    helix.space_pos(param, d.end, d.forward),
                ] {
                    if coord[0] < min_x {
                        min_x = coord[0];
                    }
                    if coord[0] > max_x {
                        max_x = coord[0];
                    }
                    if coord[1] < min_y {
                        min_y = coord[1];
                    }
                    if coord[1] > max_y {
                        max_y = coord[1];
                    }
                    if coord[2] < min_z {
                        min_z = coord[2];
                    }
                    if coord[2] > max_z {
                        max_z = coord[2];
                    }
                }
            }
        }
        [min_x, max_x, min_y, max_y, min_z, max_z]
    }

    /// Return the 3 axis sorted by magnitude of instances coordinates
    fn get_bases(&self, ratio: f32) -> Vec<Basis> {
        let [min_x, max_x, min_y, max_y, min_z, max_z] = self.boundaries();
        let delta_x = ((max_x - min_x) * 1.1) as f32;
        let delta_y = ((max_y - min_y) * 1.1) as f32;
        let delta_z = ((max_z - min_z) * 1.1) as f32;

        let mut bases = vec![
            (delta_x, (max_x + min_x) / 2., max_x, [1., 0., 0.], 0),
            (delta_y, (max_y + min_y) / 2., max_y, [0., 1., 0.], 1),
            (delta_z, (max_z + min_z) / 2., max_z, [0., 0., 1.], 2),
        ];

        bases.sort_by(|a, b| (b.0).partial_cmp(&(a.0)).unwrap());

        if ratio < 1. {
            bases.swap(0, 1);
        }

        bases
    }

    fn get_fitting_quaternion(&self, bases: &Vec<Basis>) -> Rotor3 {
        let right: Vec3 = bases[0].3.into();
        let up: Vec3 = bases[1].3.into();
        let reverse_direction = right.cross(up);
        let rotation_matrix = ultraviolet::Mat3::new(right, up, reverse_direction);
        println!("{:?}", rotation_matrix);
        let ret = rotation_to_rotor(&rotation_matrix);
        println!("rotor {:?}", ret.into_matrix());
        ret
    }

    /// Given the orientation of the camera, computes its position so that it can see everything.
    fn get_fitting_position(
        &self,
        bases: &mut Vec<Basis>,
        ratio: f32,
        fovy: f32,
        direction: &Vec3,
    ) -> Vec3 {
        // We want to fit both the x and the y axis.
        let vertical = (bases[1].0).max(bases[0].0 / ratio) + bases[2].0;

        let x_back = vertical / 2. / (fovy / 2.).tan();

        bases.sort_by_key(|b| b.4);
        let coord = Vec3::new(bases[0].1 as f32, bases[1].1 as f32, bases[2].1 as f32);
        coord - *direction * x_back
    }

    pub fn get_element_position(&self, id: u32) -> Option<Vec3> {
        self.space_position.get(&id).map(|x| x.into())
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
