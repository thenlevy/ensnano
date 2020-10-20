use super::ViewPtr;
pub struct Data {
    view: ViewPtr,
}

mod helix;
pub use helix::{GpuVertex, Helix, HelixModel};
mod strand;
pub use strand::{Nucl, Strand, StrandVertex};

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self { view }
    }
}
