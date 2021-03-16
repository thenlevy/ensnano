use super::{Flat, HelixVec, ViewPtr};
use crate::design::{Design, Nucl, StrandBuilder};
use crate::mediator::{Selection, SelectionMode};
use std::sync::{Arc, RwLock};
use ultraviolet::Vec2;

mod helix;
pub use helix::{GpuVertex, Helix, HelixModel};
mod strand;
pub use strand::{FreeEnd, Strand, StrandVertex};
mod design;
use super::{CameraPtr, FlatHelix, FlatIdx, FlatNucl};
use crate::consts::*;
use crate::design::{Helix as DesignHelix, Strand as DesignStrand};
use crate::utils::camera2d::FitRectangle;
use ahash::RandomState;
pub use design::FlatTorsion;
use design::{Design2d, Helix2d};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub struct Data {
    view: ViewPtr,
    design: Design2d,
    instance_update: bool,
    instance_reset: bool,
    helices: HelixVec<Helix>,
    selected_helix: Option<FlatIdx>,
    nb_helices_created: usize,
    basis_map: Arc<RwLock<HashMap<Nucl, char, RandomState>>>,
    groups: Arc<RwLock<BTreeMap<usize, bool>>>,
    suggestions: HashMap<FlatNucl, HashSet<FlatNucl, RandomState>, RandomState>,
    selection_mode: SelectionMode,
    pub selection: Vec<Selection>,
    id: u32,
    selection_updated: bool,
}

impl Data {
    pub fn new(view: ViewPtr, design: Arc<RwLock<Design>>, id: u32) -> Self {
        let basis_map = design.read().unwrap().get_basis_map();
        let groups = design.read().unwrap().get_groups();
        Self {
            view,
            design: Design2d::new(design),
            instance_update: true,
            instance_reset: false,
            helices: HelixVec::new(),
            selected_helix: None,
            nb_helices_created: 0,
            basis_map,
            groups,
            suggestions: Default::default(),
            selection_mode: SelectionMode::default(),
            selection: vec![],
            selection_updated: false,
            id,
        }
    }

    pub fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.selection_mode = selection_mode
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
            self.view
                .borrow_mut()
                .update_strands(&self.design.get_strands(), &self.helices);
            self.view
                .borrow_mut()
                .update_pasted_strand(self.design.get_pasted_strand(), &self.helices);
            self.update_highlight();
        } else if self.selection_updated {
            self.update_highlight();
        }
        self.instance_update = false;
    }

    pub fn id_map(&self) -> &HashMap<usize, FlatIdx> {
        self.design.id_map()
    }

    pub fn update_highlight(&mut self) {
        let mut selected_strands = HashSet::new();
        let mut selected_xovers = HashSet::new();
        for s in self.selection.iter() {
            match s {
                Selection::Strand(_, s_id) => {
                    selected_strands.insert(*s_id as usize);
                }
                Selection::Bound(_, n1, n2) => {
                    selected_xovers.insert((*n1, *n2));
                }
                _ => (),
            }
        }
        let mut highlighted_strands = Vec::new();
        for s in self.design.get_strands().iter() {
            if selected_strands.contains(&s.id) {
                highlighted_strands.push(s.highlighted(SELECTED_COLOR));
            }
        }
        for xover in selected_xovers.iter() {
            highlighted_strands.push(self.design.strand_from_xover(xover));
        }
        self.view
            .borrow_mut()
            .update_highlight(&highlighted_strands, &self.helices);
        self.selection_updated = false;
    }

    fn fetch_helices(&mut self) {
        let removed_helices = self.design.get_removed_helices();
        for h in removed_helices.iter().rev() {
            self.helices.remove(*h);
        }
        self.view.borrow_mut().rm_helices(removed_helices);
        let id_map = self.design.id_map();
        let nb_helix = self.helices.len();
        let new_helices = self.design.get_helices();
        for (i, helix) in self.helices.iter_mut().enumerate() {
            helix.update(&new_helices[i], id_map);
        }
        for h in new_helices[nb_helix..].iter() {
            let flat_helix = FlatHelix::from_real(h.id, id_map);
            self.helices.push(Helix::new(
                h.left,
                h.right,
                h.isometry,
                flat_helix,
                h.id,
                h.visible,
                self.basis_map.clone(),
                self.groups.clone(),
            ));
            self.nb_helices_created += 1;
        }
        let suggestions = self.design.suggestions();
        self.update_suggestion(&suggestions);
        self.view
            .borrow_mut()
            .set_suggestions(self.design.suggestions());
        self.view
            .borrow_mut()
            .set_torsions(self.design.get_torsions());
    }

    fn update_suggestion(&mut self, suggestion: &[(FlatNucl, FlatNucl)]) {
        self.suggestions.clear();
        for (n1, n2) in suggestion.iter() {
            self.suggestions.entry(*n1).or_default().insert(*n2);
            self.suggestions.entry(*n2).or_default().insert(*n1);
        }
    }

    pub fn get_click(&self, x: f32, y: f32, camera: &CameraPtr) -> ClickResult {
        for h in self.helices.iter() {
            if h.click_on_circle(x, y, camera) {
                let translation_pivot = h.get_circle_pivot(camera).unwrap();
                return ClickResult::CircleWidget { translation_pivot };
            }
        }
        for h in self.helices.iter() {
            let ret = h.get_click(x, y).map(|(position, forward)| FlatNucl {
                helix: h.flat_id,
                position,
                forward,
            });
            if let Some(ret) = ret {
                return ClickResult::Nucl(ret);
            }
        }
        ClickResult::Nothing
    }

    pub fn is_suggested(&self, nucl: &FlatNucl) -> bool {
        self.suggestions.contains_key(nucl)
    }

    pub fn get_rotation_pivot(&self, h_id: FlatIdx, camera: &CameraPtr) -> Option<Vec2> {
        self.helices
            .get(h_id)
            .map(|h| h.visible_center(camera).unwrap_or_else(|| h.center()))
    }

    pub fn get_click_unbounded_helix(&self, x: f32, y: f32, helix: FlatHelix) -> FlatNucl {
        let (position, forward) = self.helices[helix.flat].get_click_unbounded(x, y);
        FlatNucl {
            position,
            forward,
            helix,
        }
    }

    #[allow(dead_code)]
    pub fn get_pivot_position(&self, helix: FlatIdx, position: isize) -> Option<Vec2> {
        self.helices.get(helix).map(|h| h.get_pivot(position))
    }

    pub fn set_selected_helices(&mut self, helices: Vec<FlatHelix>) {
        for h in self.helices.iter_mut() {
            h.set_color(HELIX_BORDER_COLOR);
        }
        for h in helices {
            self.helices[h.flat].set_color(SELECTED_HELIX2D_COLOR);
        }
        self.instance_update = true;
    }

    pub fn snap_helix(&mut self, pivot: FlatNucl, translation: Vec2) {
        self.helices[pivot.helix.flat].snap(pivot, translation);
        self.instance_update = true;
    }

    pub fn rotate_helix(&mut self, helix: FlatHelix, pivot: Vec2, angle: f32) {
        self.helices[helix.flat].rotate(pivot, angle);
        self.instance_update = true;
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

    pub fn get_builder(&self, nucl: FlatNucl, stick: bool) -> Option<StrandBuilder> {
        self.design.get_builder(nucl.to_real(), stick)
    }

    pub fn notify_update(&mut self) {
        self.instance_update = true;
    }

    pub fn notify_reset(&mut self) {
        self.instance_reset = true;
    }

    pub fn can_cross_to(&self, from: FlatNucl, to: FlatNucl) -> bool {
        let from = from.to_real();
        let to = to.to_real();
        let prim5 = self.design.prime5_of(from).or(self.design.prime5_of(to));
        let prim3 = self.design.prime3_of(from).or(self.design.prime3_of(to));
        if prim3 != prim5 {
            prim3.zip(prim5).is_some()
        } else {
            let from_end = self.design.prime5_of(from).or(self.design.prime3_of(from));
            let to_end = self.design.prime3_of(to).or(self.design.prime5_of(to));
            from_end.is_some() && to_end.is_some()
        }
    }

    pub fn can_cut_cross_to(&self, from: FlatNucl, to: FlatNucl) -> bool {
        let can_merge = match self.is_strand_end(from) {
            Some(true) => self.is_xover_end(&to) != Some(true),
            Some(false) => self.is_xover_end(&to) != Some(false),
            _ => false,
        };
        can_merge && self.design.has_nucl(to.to_real())
    }

    pub fn has_nucl(&self, nucl: FlatNucl) -> bool {
        self.design.has_nucl(nucl.to_real())
    }

    pub fn get_strand_id(&self, nucl: FlatNucl) -> Option<usize> {
        let nucl = nucl.to_real();
        self.design.get_strand_id(nucl)
    }

    /// Return the strand ids and the value of target_3prime to construct a CrossCut operation
    pub fn cut_cross(&self, from: FlatNucl, to: FlatNucl) -> Option<(usize, usize, bool)> {
        // After the cut, the target will be the 3' end of the merge iff the source nucl is the
        // 3' end of the source strand
        let target_3prime = self.is_strand_end(from) == Some(true);
        let from = self.get_strand_id(from)?;
        let to = self.get_strand_id(to)?;
        Some((from, to, target_3prime))
    }

    /// Return Some(true) if nucl is a 3' end, Some(false) if nucl is a 5' end and None otherwise
    pub fn is_strand_end(&self, nucl: FlatNucl) -> Option<bool> {
        let nucl = nucl.to_real();
        self.design
            .prime3_of(nucl)
            .map(|_| true)
            .or(self.design.prime5_of(nucl).map(|_| false))
    }

    pub fn set_free_end(&mut self, free_end: Option<FreeEnd>) {
        self.view.borrow_mut().set_free_end(free_end);
        self.view
            .borrow_mut()
            .update_strands(&self.design.get_strands(), &self.helices);
    }

    pub fn xover(&self, from: FlatNucl, to: FlatNucl) -> (usize, usize) {
        let nucl1 = from.to_real();
        let nucl2 = to.to_real();

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

    pub fn get_strand(&self, strand_id: usize) -> Option<DesignStrand> {
        self.design.get_strand(strand_id)
    }

    pub fn can_delete_helix(&mut self, helix: FlatHelix) -> Option<(DesignHelix, usize)> {
        if self.design.can_delete_helix(helix) {
            self.design.get_raw_helix(helix).zip(Some(helix.real))
        } else {
            None
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
        for h in self.helices.iter() {
            self.design.set_isometry(h.flat_id, h.isometry);
        }
    }

    pub fn is_xover_end(&self, nucl: &FlatNucl) -> Option<bool> {
        self.design.is_xover_end(&nucl.to_real())
    }

    pub fn flip_visibility(&mut self, h_id: FlatHelix, apply_to_other: bool) {
        self.design.flip_visibility(h_id, apply_to_other)
    }

    pub fn flip_group(&mut self, h_id: FlatHelix) {
        self.design.flip_group(h_id)
    }

    pub fn get_best_suggestion(&self, nucl: FlatNucl) -> Option<FlatNucl> {
        let mut ret = None;
        let mut best_dist = std::f32::INFINITY;
        if let Some(set) = self.suggestions.get(&nucl) {
            for nucl2 in set {
                let dist = self
                    .design
                    .get_dist(nucl.to_real(), nucl2.to_real())
                    .unwrap_or(std::f32::INFINITY);
                if dist < best_dist {
                    ret = Some(*nucl2);
                    best_dist = dist;
                }
            }
        }
        ret
    }

    pub fn get_selection(&self, nucl: FlatNucl, d_id: u32) -> Selection {
        let nucl = nucl.to_real();
        Selection::Nucleotide(d_id, nucl)
    }

    pub fn select_rectangle(
        &mut self,
        c1: Vec2,
        c2: Vec2,
        camera: &CameraPtr,
        adding: bool,
    ) -> (Vec<FlatNucl>, Vec<Vec2>) {
        self.selection_updated = true;
        if self.selection_mode == SelectionMode::Strand {
            self.select_strands_rectangle(camera, c1, c2, adding);
            return (vec![], vec![]);
        } else if self.selection_mode == SelectionMode::Nucleotide {
            self.select_xovers_rectangle(camera, c1, c2, adding);
            return (vec![], vec![]);
        }
        println!("{:?} {:?}", c1, c2);
        let mut translation_pivots = vec![];
        let mut rotation_pivots = vec![];
        let mut selection = Vec::new();
        for h in self.helices.iter_mut() {
            let c = h.get_circle(camera);
            if c.map(|c| c.in_rectangle(&c1, &c2)).unwrap_or(false) {
                let translation_pivot = h.get_circle_pivot(camera).unwrap();
                let rotation_pivot = h.visible_center(camera).unwrap_or_else(|| h.center());
                h.set_color(SELECTED_HELIX2D_COLOR);
                translation_pivots.push(translation_pivot);
                rotation_pivots.push(rotation_pivot);
                selection.push(Selection::Helix(self.id, h.real_id as u32));
            }
        }
        if adding {
            for s in selection.iter() {
                if !self.selection.contains(s) {
                    self.selection.push(*s);
                }
            }
        } else {
            self.selection = selection;
        }
        (translation_pivots, rotation_pivots)
    }

    fn select_xovers_rectangle(&mut self, camera: &CameraPtr, c1: Vec2, c2: Vec2, adding: bool) {
        let (x1, y1) = camera.borrow().world_to_norm_screen(c1.x, c1.y);
        let (x2, y2) = camera.borrow().world_to_norm_screen(c2.x, c2.y);
        let left = x1.min(x2);
        let right = x1.max(x2);
        let top = y1.min(y2);
        let bottom = y1.max(y2);
        println!("{}, {}, {}, {}", left, top, right, bottom);
        let mut selection = BTreeSet::new();
        for (flat_1, flat_2) in self.design.get_xovers_list() {
            let h1 = &self.helices[flat_1.helix.flat];
            let h2 = &self.helices[flat_2.helix.flat];
            if h1.rectangle_has_nucl(flat_1, left, top, right, bottom, camera)
                && h2.rectangle_has_nucl(flat_2, left, top, right, bottom, camera)
            {
                let n1 = flat_1.to_real();
                let n2 = flat_2.to_real();
                selection.insert((n1, n2));
            }
        }
        let selection: Vec<Selection> = selection
            .iter()
            .map(|(n1, n2)| Selection::Bound(self.id, *n1, *n2))
            .collect();
        if adding {
            for s in selection.iter() {
                if !self.selection.contains(s) {
                    self.selection.push(*s);
                }
            }
        } else {
            self.selection = selection;
        }
        println!("selection {:?}", self.selection);
    }

    fn select_strands_rectangle(&mut self, camera: &CameraPtr, c1: Vec2, c2: Vec2, adding: bool) {
        let (x1, y1) = camera.borrow().world_to_norm_screen(c1.x, c1.y);
        let (x2, y2) = camera.borrow().world_to_norm_screen(c2.x, c2.y);
        let left = x1.min(x2);
        let right = x1.max(x2);
        let top = y1.min(y2);
        let bottom = y1.max(y2);
        println!("{}, {}, {}, {}", left, top, right, bottom);
        let mut selection = BTreeSet::new();
        for s in self.design.get_strands().iter() {
            for n in s.points.iter() {
                let h = &self.helices[n.helix.flat];
                if h.rectangle_has_nucl(*n, left, top, right, bottom, camera) {
                    selection.insert(s.id);
                    break;
                }
            }
        }
        let selection: Vec<Selection> = selection
            .iter()
            .map(|s_id| Selection::Strand(self.id, *s_id as u32))
            .collect();
        if adding {
            for s in selection.iter() {
                if !self.selection.contains(s) {
                    self.selection.push(*s);
                }
            }
        } else {
            self.selection = selection;
        }
    }

    pub fn add_selection(&mut self, click_result: ClickResult) {
        match click_result {
            ClickResult::CircleWidget { translation_pivot } => {
                if self.selection_mode == SelectionMode::Helix {
                    let selection = Selection::Helix(self.id, translation_pivot.helix.real as u32);
                    if let Some(pos) = self.selection.iter().position(|x| *x == selection) {
                        self.selection.remove(pos);
                    } else {
                        self.selection.push(selection);
                    }
                }
            }
            ClickResult::Nucl(nucl) => match self.selection_mode {
                SelectionMode::Strand => {
                    if let Some(s_id) = self.design.get_strand_id(nucl.to_real()) {
                        let selection = Selection::Strand(self.id, s_id as u32);
                        if let Some(pos) = self.selection.iter().position(|x| *x == selection) {
                            self.selection.remove(pos);
                        } else {
                            self.selection.push(selection);
                        }
                    }
                }
                _ => {
                    if let Some(xover) = self.xover_containing_nucl(&nucl) {
                        let selection =
                            Selection::Bound(self.id, xover.0.to_real(), xover.1.to_real());
                        if let Some(pos) = self.selection.iter().position(|x| *x == selection) {
                            self.selection.remove(pos);
                        } else {
                            self.selection.push(selection);
                        }
                    }
                }
            },
            ClickResult::Nothing => (),
        }
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = vec![selection];
        self.view
            .borrow_mut()
            .set_selection(super::FlatSelection::from_real(
                Some(&selection),
                self.id_map(),
            ));
    }

    fn xover_containing_nucl(&self, nucl: &FlatNucl) -> Option<(FlatNucl, FlatNucl)> {
        let xovers_list = self.design.get_xovers_list();
        xovers_list.iter().find_map(|(n1, n2)| {
            if *n1 == *nucl {
                Some((*n1, *n2))
            } else if *n2 == *nucl {
                Some((*n1, *n2))
            } else {
                None
            }
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum ClickResult {
    Nucl(FlatNucl),
    CircleWidget { translation_pivot: FlatNucl },
    Nothing,
}

#[derive(Debug)]
pub(super) struct Xover {
    pub source: DesignStrand,
    pub target: DesignStrand,
    pub source_id: usize,
    pub target_id: usize,
    pub source_nucl: Nucl,
    pub target_nucl: Nucl,
    pub design_id: usize,
    pub target_end: Option<bool>,
    pub source_end: Option<bool>,
}
