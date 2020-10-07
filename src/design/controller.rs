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
        self.view.borrow_mut().set_matrix(
            self.old_matrix
                .translated(translation),
        )
    }

    pub fn rotate(&mut self, rotation: &DesignRotation) {
        let angle_yz = rotation.angle_yz;
        let angle_xz = rotation.angle_xz;

        let plane_xz = ultraviolet::Bivec3::from_normalized_axis(rotation.up_vec).normalized();
        let plane_yz = ultraviolet::Bivec3::from_normalized_axis(rotation.right_vec).normalized();

        let rotor =
            Mat4::from_angle_plane(angle_yz, plane_yz) * Mat4::from_angle_plane(angle_xz, plane_xz);

        //println!("{:?}", rotor.normalized().into_matrix());

        let origin = rotation.origin;

        let new_matrix = Mat4::from_translation(origin)
            * rotor
            * Mat4::from_translation(-origin)
            * self.old_matrix;
        self.view.borrow_mut().set_matrix(new_matrix);
    }

    /// Terminate the movement computed by self
    pub fn terminate_movement(&mut self) {
        self.old_matrix = self.view.borrow().model_matrix;
        self.forward = Vec3::zero();
    }
}

pub struct DesignRotation {
    pub origin: Vec3,
    pub up_vec: Vec3,
    pub right_vec: Vec3,
    pub angle_yz: f32,
    pub angle_xz: f32,
}

