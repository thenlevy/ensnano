use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use ultraviolet::{Mat4, Vec3};

mod controller;
mod data;
mod view;
use controller::Controller;
use data::Data;
pub use data::{ObjectType, Nucl};
use view::View;

pub struct Design {
    view: Rc<RefCell<View>>,
    #[allow(dead_code)]
    controller: Controller,
    data: Rc<RefCell<Data>>,
}

impl Design {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Rc::new(RefCell::new(Data::new(&view)));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller,
        }
    }

    /// Create a new design by reading a file. At the moment only codenano format is supported
    pub fn new_with_path(path: &PathBuf) -> Self {
        let view = Rc::new(RefCell::new(View::new()));
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


    /// Return the model matrix used to display the design
    pub fn get_model_matrix(&self) -> Mat4 {
        self.view.borrow().get_model_matrix()
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

    pub fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.data.borrow().get_object_type(id)
    }

    pub fn get_nucl_involved(&self, id: u32) -> Option<(u32, u32)> {
        self.data.borrow().get_nucl_involved(id)
    }

    pub fn get_color(&self, id: u32) -> Option<u32> {
        self.data.borrow().get_color(id)
    }

    pub fn get_all_nucl_ids(&self) -> Vec<u32> {
        self.data.borrow().get_all_nucl_ids().collect()
    }

    pub fn get_all_bound_ids(&self) -> Vec<u32> {
        self.data.borrow().get_all_bound_ids().collect()
    }

    pub fn is_nucl(&self, id: u32) -> bool {
        self.data.borrow().is_nucl(id)
    }

    pub fn is_bound(&self, id: u32) -> bool {
        self.data.borrow().is_bound(id)
    }

}
