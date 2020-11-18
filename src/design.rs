//! This modules defines the type [`Design`](Design) which offers an interface to a DNA nanostructure design.
use native_dialog::{Dialog, MessageAlert};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use ultraviolet::{Mat4, Vec3};

use crate::mediator;
use mediator::AppNotification;

mod controller;
mod data;
mod view;
use crate::scene::GridInstance;
use controller::Controller;
pub use controller::{DesignRotation, IsometryTarget, DesignTranslation};
use data::Data;
pub use data::*;
use view::View;

pub struct Design {
    view: Rc<RefCell<View>>,
    #[allow(dead_code)]
    controller: Controller,
    data: Arc<Mutex<Data>>,
    id: usize,
}

impl Design {
    #[allow(dead_code)]
    pub fn new(id: usize) -> Self {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Arc::new(Mutex::new(Data::new()));
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
        let data = Arc::new(Mutex::new(Data::new_with_path(path)?));
        let controller = Controller::new(view.clone(), data.clone());
        Some(Self {
            view,
            data,
            controller,
            id,
        })
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

    /// Return a notification to send to the observer if the data was changed.
    pub fn data_was_updated(&mut self) -> Option<DesignNotification> {
        if self.data.lock().unwrap().was_updated() {
            let notification = DesignNotification {
                content: DesignNotificationContent::InstanceChanged,
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

    /// Get the position of an item of self in a given rerential
    pub fn get_element_position(&self, id: u32, referential: Referential) -> Option<Vec3> {
        if referential.is_world() {
            self.data
                .lock()
                .unwrap()
                .get_element_position(id)
                .map(|x| self.view.borrow().model_matrix.transform_point3(x))
        } else {
            self.data.lock().unwrap().get_element_position(id)
        }
    }

    /// Get the position of an item of self in a given referential
    pub fn get_element_axis_position(&self, id: u32, referential: Referential) -> Option<Vec3> {
        if referential.is_world() {
            self.data
                .lock()
                .unwrap()
                .get_element_axis_position(id)
                .map(|x| self.view.borrow().model_matrix.transform_point3(x))
        } else {
            self.data.lock().unwrap().get_element_axis_position(id)
        }
    }

    /// Get the position of a nucleotide in a given referential. Eventually project the nucleotide
    /// on the it's helix's axis.
    pub fn get_helix_nucl(
        &self,
        nucl: Nucl,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3> {
        if referential.is_world() {
            self.data
                .lock()
                .unwrap()
                .get_helix_nucl(nucl, on_axis)
                .map(|x| self.view.borrow().model_matrix.transform_point3(x))
        } else {
            self.data.lock().unwrap().get_helix_nucl(nucl, on_axis)
        }
    }

    /// Return the `ObjectType` of an element
    pub fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.data.lock().unwrap().get_object_type(id)
    }

    /// Return the color of an element
    pub fn get_color(&self, id: u32) -> Option<u32> {
        self.data.lock().unwrap().get_color(id)
    }

    /// Return all identifier of nucleotides
    pub fn get_all_nucl_ids(&self) -> Vec<u32> {
        self.data.lock().unwrap().get_all_nucl_ids().collect()
    }

    /// Return all identifer of bounds
    pub fn get_all_bound_ids(&self) -> Vec<u32> {
        self.data.lock().unwrap().get_all_bound_ids().collect()
    }

    /// Notify the design of a notification. This is how applications communicate their
    /// modification request to the design
    pub fn on_notify(&mut self, notification: AppNotification) {
        match notification {
            AppNotification::MovementEnded => self.terminate_movement(),
            AppNotification::Rotation(rotation) => self.apply_rotation(&rotation),
            AppNotification::Translation(translation) => self.apply_translation(&translation),
            AppNotification::MakeGrids => self.data.lock().unwrap().create_grids(),
        }
    }

    /// The identifier of the design
    pub fn get_id(&self) -> usize {
        self.id
    }

    /// Return the identifier of the strand on which an element lies
    pub fn get_strand(&self, element_id: u32) -> Option<usize> {
        self.data.lock().unwrap().get_strand(element_id)
    }

    /// Return the identifier of the helix on which an element lies
    pub fn get_helix(&self, element_id: u32) -> Option<usize> {
        self.data.lock().unwrap().get_helix(element_id)
    }

    /// Return all the identifier of the elements that lie on a strand
    pub fn get_strand_elements(&self, strand_id: usize) -> Vec<u32> {
        self.data.lock().unwrap().get_strand_elements(strand_id)
    }

    /// Return all the identifier of the elements that lie on an helix
    pub fn get_helix_elements(&self, helix_id: usize) -> Vec<u32> {
        self.data.lock().unwrap().get_helix_elements(helix_id)
    }

    /// Save the design in icednano format
    pub fn save_to(&self, path: &PathBuf) {
        let result = self.data.lock().unwrap().save_file(path);
        if result.is_err() {
            let text = format!("Could not save_file {:?}", result);
            std::thread::spawn(move || {
                let error_msg = MessageAlert {
                    title: "Error",
                    text: &text,
                    typ: native_dialog::MessageType::Error,
                };
                error_msg.show().unwrap_or(());
            });
        }
    }

    /// Change the collor of a strand
    pub fn change_strand_color(&mut self, strand_id: usize, color: u32) {
        self.data
            .lock()
            .unwrap()
            .change_strand_color(strand_id, color);
    }

    /// Change the sequence of a strand
    pub fn change_strand_sequence(&mut self, strand_id: usize, sequence: String) {
        self.data
            .lock()
            .unwrap()
            .change_strand_sequence(strand_id, sequence);
    }

    pub fn get_strand_color(&self, strand_id: usize) -> Option<u32> {
        self.data.lock().unwrap().get_strand_color(strand_id)
    }

    pub fn get_strand_sequence(&self, strand_id: usize) -> Option<String> {
        self.data.lock().unwrap().get_strand_sequence(strand_id)
    }

    /// Get the basis of the model in the world's coordinates
    pub fn get_basis(&self) -> ultraviolet::Rotor3 {
        let mat4 = self.view.borrow().get_model_matrix();
        let mat3 = ultraviolet::Mat3::new(
            mat4.transform_vec3(Vec3::unit_x()),
            mat4.transform_vec3(Vec3::unit_y()),
            mat4.transform_vec3(Vec3::unit_z()),
        );
        mat3.into_rotor3()
    }

    /// Return the basis of an helix in the world's coordinates
    pub fn get_helix_basis(&self, h_id: u32) -> Option<ultraviolet::Rotor3> {
        self.data
            .lock()
            .unwrap()
            .get_helix_basis(h_id as usize)
            .map(|r| self.get_basis() * r)
    }

    /// Return the identifier of the 5' end of the strand on which an element lies.
    pub fn get_element_5prime(&self, element: u32) -> Option<u32> {
        let strand = self.get_strand(element)?;
        self.data.lock().unwrap().get_5prime(strand)
    }

    /// Return the identifier of the 3' end of the strand on which an element lies.
    pub fn get_element_3prime(&self, element: u32) -> Option<u32> {
        let strand = self.get_strand(element)?;
        self.data.lock().unwrap().get_3prime(strand)
    }

    /// Return a `StrandBuilder` with moving end `nucl` if possibile (see
    /// [`Data::get_strand_builder`](data::Data::get_strand_builder)).
    pub fn get_builder(&mut self, nucl: Nucl) -> Option<StrandBuilder> {
        self.data.lock().unwrap().get_strand_builder(nucl).map(|b| {
            b.transformed(&self.view.borrow().get_model_matrix())
                .given_data(self.data.clone(), self.id as u32)
        })
    }

    /// Return a `StrandBuilder` whose moving end is given by an element, if possible ( see
    /// [`Data::get_strand_builder`](data::Data::get_strand_builder) )
    pub fn get_builder_element(&mut self, element_id: u32) -> Option<StrandBuilder> {
        let nucl = self.data.lock().unwrap().get_nucl(element_id)?;
        self.get_builder(nucl)
    }

    /// If element_id is the identifier of a nucleotide, return the position on which the
    /// nucleotide's symbols must be displayed
    pub fn get_symbol_position(&self, element_id: u32) -> Option<Vec3> {
        self.data.lock().unwrap().get_symbol_position(element_id)
    }

    /// If element_id is the identifier of a nucleotide, return the eventual corresponding
    /// symbols
    pub fn get_symbol(&self, element_id: u32) -> Option<char> {
        self.data.lock().unwrap().get_symbol(element_id)
    }

    pub fn get_strand_points(&self, s_id: usize) -> Option<Vec<Nucl>> {
        self.data.lock().unwrap().get_strand_points(s_id)
    }

    pub fn merge_strands(&mut self, prime5: usize, prime3: usize) {
        self.data.lock().unwrap().merge_strands(prime5, prime3)
    }

    pub fn get_all_strand_ids(&self) -> Vec<usize> {
        self.data.lock().unwrap().get_all_strand_ids()
    }

    pub fn prime3_of(&self, nucl: Nucl) -> Option<usize> {
        self.data.lock().unwrap().prime3_of(&nucl)
    }

    pub fn prime5_of(&self, nucl: Nucl) -> Option<usize> {
        self.data.lock().unwrap().prime5_of(&nucl)
    }

    pub fn split_strand(&self, nucl: Nucl) {
        self.data.lock().unwrap().split_strand(&nucl)
    }

    pub fn get_grid_instance(&self) -> Vec<GridInstance> {
        self.data.lock().unwrap().get_grid_instances(self.id)
    }

    pub fn get_grid2d(&self, id: usize) -> Option<Arc<RwLock<Grid2D>>> {
        self.data.lock().unwrap().get_grid(id)
    }

    pub fn get_grid_basis(&self, g_id: u32) -> Option<ultraviolet::Rotor3> {
        self.data.lock().unwrap().get_grid_basis(g_id)
    }

    pub fn get_helices_grid(&self, g_id: u32) -> Option<HashSet<u32>> {
        self.data.lock().unwrap().get_helices_grid(g_id)
    }

    pub fn get_grid_position(&self, g_id: u32) -> Option<ultraviolet::Vec3> {
        self.data.lock().unwrap().get_grid_position(g_id)
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
    /// The design was modified
    InstanceChanged,
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
