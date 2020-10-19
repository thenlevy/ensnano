use super::ViewPtr;
pub struct Data {
    view: ViewPtr,
}

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self {
            view
        }
    }
}
