use super::ViewPtr;
pub struct Data {
    view: ViewPtr,
}

mod helix;

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self { view }
    }
}
