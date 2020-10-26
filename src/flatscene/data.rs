use super::ViewPtr;
use crate::design::{Design, StrandBuilder};
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
        for (i, helix) in self.helices.iter_mut().enumerate() {
            helix.update(&new_helices[i]);
        }
        for (delta, h) in new_helices[nb_helix..].iter().enumerate() {
            self.helices.push(Helix::new(
                h.left,
                h.right,
                (3. * (delta + nb_helix) as f32) * Vec2::unit_y(),
                (delta + nb_helix) as u32,
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

    /// If (x, y) is on a nucleotide, select, the corresponding helix, and return a pivot on the
    /// corresponding nucleotide. Otherwise, clear the selection and return `None`.
    pub fn request_pivot(&mut self, x: f32, y: f32) -> Option<Vec2> {
        if let Some(nucl) = self.get_click(x, y) {
            self.set_selected_helix(Some(nucl.helix));
            self.get_pivot_position(nucl.helix, nucl.position)
        } else {
            self.set_selected_helix(None);
            None
        }
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

    pub fn move_helix_forward(&mut self) {
        if let Some(helix) = self.selected_helix {
            self.helices[helix].move_forward();
            self.instance_update = true;
        }
    }

    pub fn move_helix_backward(&mut self) {
        if let Some(helix) = self.selected_helix {
            self.helices[helix].move_backward();
            self.instance_update = true;
        }
    }

    pub fn get_builder(&self, nucl: Nucl) -> Option<StrandBuilder> {
        self.design.get_builder(nucl)
    }

    pub fn notify_update(&mut self) {
        self.instance_update = true;
    }
}
