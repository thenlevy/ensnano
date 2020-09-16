mod view;
mod data;
mod controller;

use crate::instance::Instance;
use ultraviolet::{Rotor3, Vec3};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use view::View;
use data::Data;
use controller::Controller;
use std::path::PathBuf;

pub struct Design {
    view: Rc<RefCell<View>>,
    controller: Controller,
    data: Rc<RefCell<Data>>,
}

impl Design {

    pub fn new() -> Self {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Rc::new(RefCell::new(Data::new(&view)));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller
        }
    }

    pub fn new_with_path(path: &PathBuf) -> Self {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Rc::new(RefCell::new(Data::new_with_path(&view, path)));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller
        }
    }

    pub fn was_updated(&mut self) -> bool {
        self.data.borrow_mut().was_updated()
    }

    pub fn fit(&self, ratio: f32, fovy: f32) -> (Vec3, Rotor3) {
        self.data.borrow().fit_design(ratio, fovy)
    }

    pub fn spheres(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_spheres().clone()
    }

    pub fn tubes(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_tubes().clone()
    }

    pub fn selected_spheres(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_selected_spheres().clone()
    }

    pub fn selected_tubes(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_selected_tubes().clone()
    }

    pub fn update_selection(&mut self, id: u32) {
        self.data.borrow_mut().update_selection(id);
    }
}
