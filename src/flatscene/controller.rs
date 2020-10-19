use super::{DataPtr, ViewPtr};

pub struct Controller {
    view: ViewPtr,
    data: DataPtr,
}

impl Controller {
    pub fn new(view: ViewPtr, data: DataPtr) -> Self {
        Self { view, data }
    }
}
