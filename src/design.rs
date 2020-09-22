mod controller;
mod data;
mod view;

use crate::instance::Instance;
use controller::Controller;
use data::Data;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3};
use view::View;

pub struct Design {
    view: Rc<RefCell<View>>,
    controller: Controller,
    data: Rc<RefCell<Data>>,
}

impl Design {
    pub fn new(id: u32) -> Self {
        let view = Rc::new(RefCell::new(View::new(id)));
        let data = Rc::new(RefCell::new(Data::new(&view)));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller,
        }
    }

    pub fn new_with_path(path: &PathBuf, id: u32) -> Self {
        let view = Rc::new(RefCell::new(View::new(id)));
        let data = Rc::new(RefCell::new(Data::new_with_path(&view, path)));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller,
        }
    }

    pub fn data_was_updated(&mut self) -> bool {
        self.data.borrow_mut().was_updated()
    }

    pub fn view_was_updated(&mut self) -> bool {
        self.view.borrow_mut().was_updated()
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

    pub fn model_matrix(&self) -> Mat4 {
        self.view.borrow().get_model_matrix()
    }

    pub fn middle_point(&self) -> Vec3 {
        self.data.borrow().middle_point()
    }

    pub fn selected_spheres(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_selected_spheres().clone()
    }

    pub fn selected_tubes(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_selected_tubes().clone()
    }

    pub fn update_selection(&mut self, id: Option<u32>) {
        self.data.borrow_mut().update_selection(id);
    }

    pub fn translate(&mut self, right: Vec3, up: Vec3, forward: Vec3) {
        self.controller.translate(right, up, forward)
    }

    pub fn rotate(&mut self, x: f64, y: f64, cam_right: Vec3, cam_up: Vec3, origin: Vec3) {
        self.controller.rotate(cam_right, cam_up, x, y, origin);
    }

    pub fn update_position(&mut self) {
        self.controller.update()
    }

    pub fn get_element_position(&self, id: u32) -> Option<Vec3> {
        self.data
            .borrow()
            .get_element_position(id)
            .map(|x| self.view.borrow().model_matrix.transform_point3(x))
    }
}
