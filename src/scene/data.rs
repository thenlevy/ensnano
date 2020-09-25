use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use super::{View};

use crate::utils::instance::Instance;

type ViewPtr = Rc<RefCell<View>>;

/// A module that handles the instantiation of designs as 3D geometric objects
mod design3d;
use design3d::Design3D;

pub struct Data {
    view: ViewPtr,
    designs: Vec<Design3D>,
    selected: Vec<(u32, u32)>,
    candidates: Vec<(u32, u32)>,
}

impl Data {

    pub fn new(view: ViewPtr) -> Self {
        Self {
            view,
            designs: Vec::new(),
            selected: Vec::new(),
            candidates: Vec::new(),
        }
    }

    pub fn add_design(&mut self, path: &PathBuf) {
        self.designs
            .push(Design3D::new_with_path(path))
    }

    pub fn clear_designs(&mut self) {
        self.designs = Vec::new();
        self.selected = Vec::new();
        self.candidates = Vec::new();
    }

    pub fn get_selected_designs(&self) -> Vec<u32> {
        self.selected.iter().map(|x| x.0).collect()
    }

    /// Return the instances of selected spheres
    pub fn get_selected_spheres(&self) -> Rc<Vec<Instance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for (d_id, id) in self.selected.iter() {
            let d_id = *d_id as usize;
            if self.designs[d_id].is_nucl(*id) {
                ret.push(self.designs[d_id].make_instance(*id))
            }
        }
        Rc::new(ret)
    }

    /// Return the instances of selected tubes
    pub fn get_selected_tubes(&self) -> Rc<Vec<Instance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for (d_id, id) in self.selected.iter() {
            let d_id = *d_id as usize;
            if self.designs[d_id].is_bound(*id) {
                ret.push(self.designs[d_id].make_instance(*id))
            }
        }
        Rc::new(ret)
    }

    /// Return the instances of candidate spheres
    pub fn get_candidate_spheres(&self) -> Rc<Vec<Instance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for (d_id, id) in self.candidates.iter() {
            let d_id = *d_id as usize;
            if self.designs[d_id].is_nucl(*id) {
                ret.push(self.designs[d_id].make_instance(*id))
            }
        }
        Rc::new(ret)
    }

    /// Return the instances of candidate tubes
    pub fn get_candidate_tubes(&self) -> Rc<Vec<Instance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for (d_id, id) in self.candidates.iter() {
            let d_id = *d_id as usize;
            if self.designs[d_id].is_bound(*id) {
                ret.push(self.designs[d_id].make_instance(*id))
            }
        }
        Rc::new(ret)
    }

}

fn last_two_bytes(x: u32) -> u32 {
    (x & 0xFF000000) >> 24
}
