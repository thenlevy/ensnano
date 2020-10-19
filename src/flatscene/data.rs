use super::ViewPtr;
pub struct Data {
    view: ViewPtr,
}

mod helix;
pub use helix::{Helix, HelixModel, GpuVertex};

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self { view }
    }
}
