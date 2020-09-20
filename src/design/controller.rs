use super::{Data, View};
use std::cell::RefCell;
use std::rc::Rc;
use ultraviolet::{Rotor3, Vec3};

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;

pub struct Controller {
    view: ViewPtr,
    data: DataPtr,
    old_position: Vec3,
    old_rotation: Rotor3,
}

impl Controller {
    pub fn new(view: ViewPtr, data: DataPtr) -> Self {
        Self {
            view,
            data,
            old_position: Vec3::zero(),
            old_rotation: Rotor3::identity(),
        }
    }

    pub fn translate(&mut self, right: Vec3, up: Vec3) {
        self.view
            .borrow_mut()
            .set_origin(self.old_position + right + up)
    }

    pub fn update(&mut self) {
        self.old_position = self.view.borrow().origin();
        self.old_rotation = self.view.borrow().rotor;
    }
}
