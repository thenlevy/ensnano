use super::ViewPtr;
use crate::design::Design;
use std::sync::{Arc, Mutex};
use ultraviolet::Vec2;

mod helix;
pub use helix::{GpuVertex, Helix, HelixModel};
mod strand;
pub use strand::{Nucl, Strand, StrandVertex};
mod design;
use design::{Design2d, Helix2d};

pub struct Data {
    view: ViewPtr,
    design: Design2d,
    instance_update: bool,
    helices: Vec<Helix>,
}

impl Data {
    pub fn new(view: ViewPtr, design: Arc<Mutex<Design>>) -> Self {
        Self {
            view,
            design: Design2d::new(design),
            instance_update: true,
            helices: Vec::new(),
        }
    }

    pub fn perform_update(&mut self) {
        if self.instance_update {
            self.design.update();
            self.fetch_helices();
            self.view.borrow_mut().update_helices(&self.helices);
            self.view
                .borrow_mut()
                .update_strands(&self.design.get_strands(), &self.helices);
        }
        self.instance_update = false;
    }

    fn fetch_helices(&mut self) {
        let nb_helix = self.helices.len();
        let new_helices = self.design.get_helices();
        for i in 0..nb_helix {
            self.helices[i].update(&new_helices[i]);
        }
        for (delta, h) in new_helices[nb_helix..].iter().enumerate() {
            self.helices.push(Helix::new(
                h.left,
                h.right,
                (3. * (delta + nb_helix) as f32) * Vec2::unit_y(),
            ))
        }
    }
}
