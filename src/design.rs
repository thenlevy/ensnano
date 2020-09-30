use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use ultraviolet::{Mat4, Vec3};

use crate::mediator;
use mediator::AppNotification;

mod controller;
mod data;
mod view;
use controller::Controller;
pub use controller::{DesignRotation, DesignTranslation};
use data::Data;
pub use data::{Nucl, ObjectType};
use view::View;

pub struct Design {
    view: Rc<RefCell<View>>,
    #[allow(dead_code)]
    controller: Controller,
    data: Rc<RefCell<Data>>,
    id: usize,
}

impl Design {
    #[allow(dead_code)]
    pub fn new(id: usize) -> Self {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Rc::new(RefCell::new(Data::new()));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller,
            id,
        }
    }

    /// Create a new design by reading a file. At the moment only codenano format is supported
    pub fn new_with_path(id: usize, path: &PathBuf) -> Option<Self> {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Rc::new(RefCell::new(Data::new_with_path(path)?));
        let controller = Controller::new(view.clone(), data.clone());
        Some(Self {
            view,
            data,
            controller,
            id,
        })
    }

    /// `true` if the data has been updated since the last time this function was called
    #[allow(dead_code)]
    pub fn data_was_updated(&mut self) -> bool {
        self.data.borrow_mut().was_updated()
    }

    /// `true` if the view has been updated since the last time this function was called
    pub fn view_was_updated(&mut self) -> Option<DesignNotification> {
        if self.view.borrow_mut().was_updated() {
            let notification = DesignNotification {
                content: DesignNotificationContent::ModelChanged(self.get_model_matrix()),
                design_id: self.id as usize,
            };
            Some(notification)
        } else {
            None
        }
    }

    /// Return the model matrix used to display the design
    pub fn get_model_matrix(&self) -> Mat4 {
        self.view.borrow().get_model_matrix()
    }

    /// Translate the representation of self
    pub fn apply_translation(&mut self, translation: &DesignTranslation) {
        self.controller.translate(translation);
    }

    /// Rotate the representation of self arround `origin`
    pub fn apply_rotation(&mut self, rotation: &DesignRotation) {
        self.controller.rotate(rotation);
    }

    /// Terminate the movement performed by self.
    pub fn terminate_movement(&mut self) {
        self.controller.terminate_movement()
    }

    /// Get the position of an item of self in the world coordinates
    pub fn get_element_position(&self, id: u32, referential: Referential) -> Option<Vec3> {
        if referential.is_world() {
            self.data
                .borrow()
                .get_element_position(id)
                .map(|x| self.view.borrow().model_matrix.transform_point3(x))
        } else {
            self.data.borrow().get_element_position(id)
        }
    }

    pub fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.data.borrow().get_object_type(id)
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

    pub fn on_notify(&mut self, notification: AppNotification) {
        match notification {
            AppNotification::MovementEnded => self.terminate_movement(),
            AppNotification::Rotation(rotation) => self.apply_rotation(rotation),
            AppNotification::Translation(translation) => self.apply_translation(translation),
        }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_strand(&self, element_id: u32) -> Option<usize> {
        self.data.borrow().get_strand(element_id)
    }

    pub fn get_strand_elements(&self, strand_id: usize) -> Vec<u32> {
        self.data.borrow().get_strand_elements(strand_id)
    }
}

#[derive(Clone)]
pub struct DesignNotification {
    pub design_id: usize,
    pub content: DesignNotificationContent,
}

/// A modification to the design that must be notified to the applications
#[derive(Clone)]
pub enum DesignNotificationContent {
    /// The model matrix of the design has been modified
    ModelChanged(Mat4),
}

/// The referential in which one wants to get an element's coordinates
#[derive(Debug, Clone, Copy)]
pub enum Referential {
    World,
    Model,
}

impl Referential {
    pub fn is_world(&self) -> bool {
        match self {
            Referential::World => true,
            _ => false,
        }
    }
}
