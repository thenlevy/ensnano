use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3};

use crate::utils::instance::Instance;

mod controller;
mod data;
mod view;
use controller::Controller;
use data::Data;
use view::View;

pub struct Design {
    view: Rc<RefCell<View>>,
    #[allow(dead_code)]
    controller: Controller,
    data: Rc<RefCell<Data>>,
}

impl Design {
    #[allow(dead_code)]
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

    /// Create a new design by reading a file. At the moment only codenano format is supported
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

    /// `true` if the data has been updated since the last time this function was called
    pub fn data_was_updated(&mut self) -> bool {
        self.data.borrow_mut().was_updated()
    }

    /// `true` if the view has been updated since the last time this function was called
    pub fn view_was_updated(&mut self) -> bool {
        self.view.borrow_mut().was_updated()
    }

    /// Return a postion and orientation for a camera that would allow the design to fit in the
    /// scene
    pub fn fit(&self, ratio: f32, fovy: f32) -> (Vec3, Rotor3) {
        self.data.borrow().fit_design(ratio, fovy)
    }

    /// Return the list of sphere instances to be displayed to represent the design
    pub fn spheres(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_spheres().clone()
    }

    /// Return the list of tube instances to be displayed to represent the design
    pub fn tubes(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_tubes().clone()
    }

    /// Return the model matrix used to display the design
    pub fn model_matrix(&self) -> Mat4 {
        self.view.borrow().get_model_matrix()
    }

    /// Return the point in the middle of the representation of the design (in the world
    /// coordinates)
    pub fn middle_point(&self) -> Vec3 {
        self.data.borrow().middle_point()
    }

    /// Return the list of instances of selected spheres.
    pub fn selected_spheres(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_selected_spheres().clone()
    }

    /// Return the list of instances of selected tubes.
    pub fn selected_tubes(&self) -> Rc<Vec<Instance>> {
        self.view.borrow().get_selected_tubes().clone()
    }

    /// Select the item with identifier id in self.
    pub fn update_selection(&mut self, id: Option<u32>) {
        self.data.borrow_mut().update_selection(id);
    }

    /// Translate the representation of self
    pub fn translate(&mut self, right: Vec3, up: Vec3, forward: Vec3) {
        self.controller.translate(right, up, forward)
    }

    /// Rotate the representation of self arround `origin`
    pub fn rotate(&mut self, x: f64, y: f64, cam_right: Vec3, cam_up: Vec3, origin: Vec3) {
        self.controller.rotate(cam_right, cam_up, x, y, origin);
    }

    /// Reset the movement performed by self. 
    pub fn reset_movement(&mut self) {
        self.controller.reset_movement()
    }

    /// Get the position of an item of self in the world coordinates
    pub fn get_element_position(&self, id: u32) -> Option<Vec3> {
        self.data
            .borrow()
            .get_element_position(id)
            .map(|x| self.view.borrow().model_matrix.transform_point3(x))
    }
}
