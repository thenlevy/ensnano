use super::{Data, View};
use std::cell::RefCell;
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3};

use std::f32::consts::FRAC_PI_4;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;

pub struct Controller {
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    old_matrix: Mat4,
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

    pub fn translate(&mut self, right: Vec3, up: Vec3, forward: Vec3) {
        self.forward += forward;
        self.view
            .borrow_mut()
            .set_matrix(self.old_matrix.translated(&(right + up + self.forward)))
    }

    pub fn rotate(&mut self, cam_right: Vec3, cam_up: Vec3, x: f64, y: f64, origin: Vec3) {
        let angle_yz = y.min(1.).max(-1.) as f32 * FRAC_PI_4;
        let angle_xz = -(x.min(1.).max(-1.)) as f32 * FRAC_PI_4;

        let plane_xz = ultraviolet::Bivec3::from_normalized_axis(cam_up).normalized();
        let plane_yz = ultraviolet::Bivec3::from_normalized_axis(cam_right).normalized();

        let rotation = Rotor3::from_angle_plane(angle_yz, plane_yz).normalized()
            * Rotor3::from_angle_plane(angle_xz, plane_xz).normalized();

        let new_matrix = Mat4::from_translation(origin)
            * rotation.normalized().into_matrix().into_homogeneous()
            * Mat4::from_translation(-origin)
            * self.old_matrix;

        self.view.borrow_mut().set_matrix(new_matrix);
    }

    pub fn update(&mut self) {
        self.old_matrix = self.view.borrow().model_matrix;
        self.forward = Vec3::zero();
    }
}
