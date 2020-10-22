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
    selected_helix: Option<usize>,
}

impl Data {
    pub fn new(view: ViewPtr, design: Arc<Mutex<Design>>) -> Self {
        Self {
            view,
            design: Design2d::new(design),
            instance_update: true,
            helices: Vec::new(),
            selected_helix: None,
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

    pub fn get_click(&self, x: f32, y: f32) -> Option<Nucl> {
        for (h_id, h) in self.helices.iter().enumerate() {
            let ret = h.get_click(x, y).map(|(position, forward)| Nucl {
                helix: h_id,
                position,
                forward,
            });
            if ret.is_some() {
                return ret;
            }
        }
        None
    }

    pub fn get_pivot_position(&self, helix: usize, position: isize) -> Option<Vec2> {
        self.helices.get(helix).map(|h| h.get_pivot(position))
    }

    pub fn set_selected_helix(&mut self, helix: Option<usize>) {
        if let Some(h) = self.selected_helix {
            self.helices[h].set_color(0);
        }
        self.selected_helix = helix;
        if let Some(h) = helix {
            self.helices[h].set_color(0xFF_00_00);
        }
        self.instance_update = true;
    }

    pub fn translate_helix(&mut self, translation: Vec2) {
        if let Some(h) = self.selected_helix {
            self.helices[h].translate(translation);
            self.instance_update = true;
        }
    }

    pub fn rotate_helix(&mut self, pivot: Vec2, angle: f32) {
        if let Some(h) = self.selected_helix {
            self.helices[h].rotate(pivot, angle);
            self.instance_update = true;
        }
    }

    pub fn end_movement(&mut self) {
        for h in self.helices.iter_mut() {
            h.end_movement()
        }
    }
}
