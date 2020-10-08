use super::{Data, View};
use std::cell::RefCell;
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3};

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;

pub struct Controller {
    /// The view controlled by self
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    /// A copy of the model_matrix of the view before the current movement
    old_matrix: Mat4,
    /// The forward vector of the current movement
    forward: Vec3,
}

impl Controller {
    pub fn new(view: ViewPtr, data: DataPtr) -> Self {
        Self {
            view,
            data,
            old_matrix: Mat4::identity(),
            forward: Vec3::zero(),
        }
    }

    pub fn translate(&mut self, translation: &Vec3) {
        self.view
            .borrow_mut()
            .set_matrix(self.old_matrix.translated(translation))
    }

    pub fn rotate(&mut self, rotation: &DesignRotation) {
        match rotation.target {
            IsometryTarget::Design => {
                let rotor = rotation.rotation.into_matrix().into_homogeneous();

                let origin = rotation.origin;

                let new_matrix = Mat4::from_translation(origin)
                    * rotor
                    * Mat4::from_translation(-origin)
                    * self.old_matrix;
                self.view.borrow_mut().set_matrix(new_matrix);
            }
            IsometryTarget::Helix(n) => {
                self.data.borrow_mut().rotate_helix_arround(n as usize, rotation.rotation, rotation.origin)
            }
        }
    }

    /// Terminate the movement computed by self
    pub fn terminate_movement(&mut self) {
        self.old_matrix = self.view.borrow().model_matrix;
        self.forward = Vec3::zero();
        self.data.borrow_mut().terminate_movement();
    }
}

pub struct DesignRotation {
    pub origin: Vec3,
    pub rotation: Rotor3,
    pub target: IsometryTarget,
}

pub enum IsometryTarget {
    Design,
    Helix(u32),
}
