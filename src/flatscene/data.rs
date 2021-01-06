use super::ViewPtr;
use crate::design::{Design, StrandBuilder};
use std::sync::{Arc, Mutex, RwLock};
use ultraviolet::Vec2;

mod helix;
pub use helix::{GpuVertex, Helix, HelixModel};
mod strand;
pub use strand::{FreeEnd, Nucl, Strand, StrandVertex};
mod design;
use super::CameraPtr;
use crate::consts::*;
use crate::design::{Helix as DesignHelix, Strand as DesignStrand};
use crate::utils::camera2d::FitRectangle;
use design::{Design2d, Helix2d};
use std::collections::HashMap;

pub struct Data {
    view: ViewPtr,
    design: Design2d,
    instance_update: bool,
    instance_reset: bool,
    helices: Vec<Helix>,
    selected_helix: Option<usize>,
    nb_helices_created: usize,
    basis_map: Arc<RwLock<HashMap<Nucl, char>>>,
}

impl Data {
    pub fn new(view: ViewPtr, design: Arc<Mutex<Design>>) -> Self {
        let basis_map = design.lock().unwrap().get_basis_map();
        Self {
            view,
            design: Design2d::new(design),
            instance_update: true,
            instance_reset: false,
            helices: Vec::new(),
            selected_helix: None,
            nb_helices_created: 0,
            basis_map,
        }
    }

    pub fn perform_update(&mut self) {
        if self.instance_reset {
            self.view.borrow_mut().reset();
            self.instance_reset = false;
        }
        if self.instance_update {
            self.design.update();
            self.fetch_helices();
            self.view.borrow_mut().update_helices(&self.helices);
            self.view.borrow_mut().update_strands(
                &self.design.get_strands(),
                &self.helices,
                self.design.id_map(),
            );
        }
        self.instance_update = false;
    }

    pub fn id_map(&self) -> &HashMap<usize, usize> {
        self.design.id_map()
    }

    fn fetch_helices(&mut self) {
        let removed_helices = self.design.get_removed_helices();
        for h in removed_helices.iter().rev() {
            self.helices.remove(*h);
        }
        self.view.borrow_mut().rm_helices(removed_helices);
        let nb_helix = self.helices.len();
        let new_helices = self.design.get_helices();
        for (i, helix) in self.helices.iter_mut().enumerate() {
            helix.update(&new_helices[i]);
            helix.id = i as u32;
        }
        for (delta, h) in new_helices[nb_helix..].iter().enumerate() {
            self.helices.push(Helix::new(
                h.left,
                h.right,
                h.isometry,
                (delta + nb_helix) as u32,
                h.id,
                h.visible,
                self.basis_map.clone(),
            ));
            self.nb_helices_created += 1;
        }
    }

    pub fn get_click(&self, x: f32, y: f32, camera: &CameraPtr) -> ClickResult {
        for h in self.helices.iter() {
            if h.click_on_circle(x, y, camera) {
                let translation_pivot = h.get_circle_pivot(camera).unwrap();
                return ClickResult::CircleWidget { translation_pivot };
            }
        }
        for (h_id, h) in self.helices.iter().enumerate() {
            let ret = h.get_click(x, y).map(|(position, forward)| Nucl {
                helix: h_id,
                position,
                forward,
            });
            if let Some(ret) = ret {
                return ClickResult::Nucl(ret);
            }
        }
        ClickResult::Nothing
    }

    pub fn get_rotation_pivot(&self, h_id: usize, camera: &CameraPtr) -> Option<Vec2> {
        self.helices
            .get(h_id)
            .and_then(|h| h.visible_center(camera))
    }

    pub fn get_click_unbounded_helix(&self, x: f32, y: f32, h_id: usize) -> Nucl {
        let (position, forward) = self.helices[h_id].get_click_unbounded(x, y);
        Nucl {
            position,
            forward,
            helix: h_id,
        }
    }

    pub fn get_pivot_position(&self, helix: usize, position: isize) -> Option<Vec2> {
        self.helices.get(helix).map(|h| h.get_pivot(position))
    }

    pub fn set_selected_helix(&mut self, helix: Option<usize>) {
        if let Some(h) = self.selected_helix {
            self.helices[h].set_color(HELIX_BORDER_COLOR);
        }
        self.selected_helix = helix;
        if let Some(h) = helix {
            self.helices[h].set_color(0xFF_BF_1E_28);
        }
        self.instance_update = true;
    }

    pub fn snap_helix(&mut self, pivot: Nucl, destination: Vec2) {
        if let Some(h) = self.selected_helix {
            self.helices[h].snap(pivot, destination);
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

    pub fn helix_id_design(&self, id: usize) -> usize {
        self.design.get_helices()[id].id
    }

    pub fn get_builder(&self, nucl: Nucl, stick: bool) -> Option<StrandBuilder> {
        let real_helix = self.design.get_helices()[nucl.helix].id;
        self.design.get_builder(
            Nucl {
                helix: real_helix,
                ..nucl
            },
            stick,
        )
    }

    pub fn notify_update(&mut self) {
        self.instance_update = true;
    }

    pub fn notify_reset(&mut self) {
        self.instance_reset = true;
    }

    pub fn can_cross_to(&self, from: Nucl, to: Nucl) -> bool {
        let from = self.to_real(from);
        let to = self.to_real(to);
        let prim5 = self.design.prime5_of(from).or(self.design.prime5_of(to));
        let prim3 = self.design.prime3_of(from).or(self.design.prime3_of(to));
        prim3.zip(prim5).is_some()
    }

    pub fn can_cut_cross_to(&self, from: Nucl, to: Nucl) -> bool {
        let can_merge = match self.is_strand_end(from) {
            Some(true) => self.is_xover_end(&to) != Some(true),
            Some(false) => self.is_xover_end(&to) != Some(false),
            _ => false,
        };
        can_merge && self.design.has_nucl(self.to_real(to))
    }

    pub fn has_nucl(&self, nucl: Nucl) -> bool {
        self.design.has_nucl(self.to_real(nucl))
    }

    pub fn get_strand_id(&self, nucl: Nucl) -> Option<usize> {
        let nucl = self.to_real(nucl);
        self.design.get_strand_id(nucl)
    }

    /// Return the strand ids and the value of target_3prime to construct a CrossCut operation
    pub fn cut_cross(&self, from: Nucl, to: Nucl) -> Option<(usize, usize, bool)> {
        if self.get_strand_id(from) == self.get_strand_id(to) {
            None
        } else {
            // After the cut, the target will be the 3' end of the merge iff the source nucl is the
            // 3' end of the source strand
            let target_3prime = self.is_strand_end(from) == Some(true);
            let from = self.get_strand_id(from)?;
            let to = self.get_strand_id(to)?;
            Some((from, to, target_3prime))
        }
    }

    /// Return Some(true) if nucl is a 3' end, Some(false) if nucl is a 5' end and None otherwise
    pub fn is_strand_end(&self, nucl: Nucl) -> Option<bool> {
        let nucl = self.to_real(nucl);
        self.design
            .prime3_of(nucl)
            .map(|_| true)
            .or(self.design.prime5_of(nucl).map(|_| false))
    }

    pub fn set_free_end(&mut self, free_end: Option<FreeEnd>) {
        self.view.borrow_mut().set_free_end(free_end);
        self.view.borrow_mut().update_strands(
            &self.design.get_strands(),
            &self.helices,
            self.design.id_map(),
        );
    }

    pub fn xover(&self, from: Nucl, to: Nucl) -> (usize, usize) {
        let nucl1 = self.to_real(from);
        let nucl2 = self.to_real(to);

        // The 3 prime strand is the strand whose **5prime** end is in the xover
        let strand_3prime = self
            .design
            .prime5_of(nucl1)
            .or(self.design.prime5_of(nucl2));

        // The 5 prime strand is the strand whose **3prime** end is in the xover
        let strand_5prime = self
            .design
            .prime3_of(nucl1)
            .or(self.design.prime3_of(nucl2));

        if strand_3prime.is_none() || strand_5prime.is_none() {
            println!("Problem during cross-over attempt. If you are not trying to break a cyclic strand please repport a bug");
        }
        (strand_5prime.unwrap(), strand_3prime.unwrap())
    }

    pub fn rm_strand(&mut self, nucl: Nucl) {
        let nucl = self.to_real(nucl);
        self.instance_reset = true;
        self.design.rm_strand(nucl);
    }

    pub fn get_strand(&self, strand_id: usize) -> Option<DesignStrand> {
        self.design.get_strand(strand_id)
    }

    pub fn can_delete_helix(&mut self, h_id: usize) -> Option<(DesignHelix, usize)> {
        let real_helix = self.design.get_helices()[h_id].id;
        println!("can delete {} / {} ?", h_id, real_helix);
        if self.design.can_delete_helix(h_id) {
            self.design
                .get_raw_helix(real_helix as usize)
                .zip(Some(real_helix as usize))
        } else {
            None
        }
    }

    pub fn to_real(&self, nucl: Nucl) -> Nucl {
        let real_helix = self.design.get_helices()[nucl.helix].id;
        Nucl {
            helix: real_helix,
            ..nucl
        }
    }

    pub fn get_fit_rectangle(&self) -> FitRectangle {
        let mut ret = FitRectangle {
            min_x: -5.,
            max_x: 15.,
            min_y: -30.,
            max_y: 5.,
        };
        for h in self.helices.iter() {
            let left = h.get_pivot(h.get_left());
            ret.add_point(Vec2::new(left.x, -left.y));
            let right = h.get_pivot(h.get_right());
            ret.add_point(Vec2::new(right.x, -right.y));
        }
        ret
    }

    pub fn save_isometry(&mut self) {
        for (h_id, h) in self.helices.iter().enumerate() {
            self.design.set_isometry(h_id, h.isometry);
        }
    }

    pub fn is_xover_end(&self, nucl: &Nucl) -> Option<bool> {
        let nucl = self.to_real(*nucl);
        self.design.is_xover_end(&nucl)
    }

    pub fn flip_visibility(&mut self, h_id: usize, apply_to_other: bool) {
        self.design.flip_visibility(h_id, apply_to_other)
    }
}

#[derive(Debug, PartialEq)]
pub enum ClickResult {
    Nucl(Nucl),
    CircleWidget { translation_pivot: Nucl },
    Nothing,
}
