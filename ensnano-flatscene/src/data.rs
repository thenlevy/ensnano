/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::{
    flattypes::{FlatPosition, FlatSelection, HelixSegment},
    view::EditionInfo,
    AppState, Flat, HelixVec, PhantomElement, Requests, ViewPtr,
};
use ensnano_design::{ultraviolet, Nucl};
use ensnano_interactor::{Selection, SelectionMode};
use std::sync::{Arc, Mutex};
use ultraviolet::Vec2;

pub(crate) mod helix;
pub use helix::{GpuVertex, Helix, HelixHandle, HelixModel, Shift};
mod strand;
pub use strand::{FreeEnd, Strand, StrandVertex};
mod design;
use super::{CameraPtr, FlatHelix, FlatIdx, FlatNucl};
use crate::FlatHelixMaps;
use ahash::RandomState;
use design::{Design2d, Helix2d};
pub use design::{DesignReader, FlatTorsion, NuclCollection};
use ensnano_interactor::consts::*;
use ensnano_utils::camera2d::FitRectangle;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub struct Data<R: DesignReader> {
    view: ViewPtr,
    design: Design2d<R>,
    instance_update: bool,
    instance_reset: bool,
    helices: HelixVec<Helix>,
    selected_helix: Option<FlatIdx>,
    nb_helices_created: usize,
    suggestions: HashMap<FlatNucl, HashSet<FlatNucl, RandomState>, RandomState>,
    id: u32,
    requests: Arc<Mutex<dyn Requests>>,
    last_click: LastClick,
}

impl<R: DesignReader> Data<R> {
    pub fn new(view: ViewPtr, design: R, id: u32, requests: Arc<Mutex<dyn Requests>>) -> Self {
        Self {
            view,
            design: Design2d::new(design, requests.clone()),
            instance_update: true,
            instance_reset: false,
            helices: HelixVec::new(),
            selected_helix: None,
            nb_helices_created: 0,
            suggestions: Default::default(),
            id,
            requests,
            last_click: Default::default(),
        }
    }

    pub fn clear_design(&mut self) {
        self.design.clear();
        self.helices = HelixVec::new();
        self.view.borrow_mut().reset();
        self.instance_update = true;
        self.instance_reset = false;
        self.selected_helix = None;
        self.nb_helices_created = 0;
        self.suggestions = Default::default();
        self.last_click = Default::default();
    }

    pub fn perform_update<S: AppState<Reader = R>>(&mut self, new_state: &S, old_state: &S) {
        if self.instance_reset {
            self.view.borrow_mut().reset();
            self.instance_reset = false;
        }
        if new_state.design_was_updated(old_state)
            || new_state.selection_was_updated(old_state)
            || new_state.candidate_was_updated(old_state)
            || self.instance_update
            || self.view.borrow().needs_redraw()
        {
            log::trace!("updating 2d data");
            self.design.update(new_state.get_design_reader());
            self.fetch_helices(new_state.get_design_reader());
            self.view.borrow_mut().update_helices(&self.helices);
            self.view
                .borrow_mut()
                .update_strands(self.design.get_strands(), &self.helices);
            self.view
                .borrow_mut()
                .update_pasted_strand(self.design.get_pasted_strand(), &self.helices);
            self.update_highlight(new_state);
            self.update_strand_building_info(new_state.get_building_state());
        }
        self.instance_update = false;
    }

    pub fn id_map(&self) -> &FlatHelixMaps {
        self.design.id_map()
    }

    pub fn update_highlight<S: AppState>(&mut self, new_state: &S) {
        let mut selected_strands = HashSet::new();
        let mut candidate_strands = HashSet::new();
        let mut selected_xovers = HashSet::new();
        let mut candidate_xovers = HashSet::new();
        let mut selected_helices = Vec::new();
        let mut candidate_helices = Vec::new();
        let mut candidate_nucls = Vec::new();
        let mut selected_nucls = Vec::new();
        let id_map = self.design.id_map();
        for s in new_state.get_selection().iter() {
            match s {
                Selection::Strand(_, s_id) if !new_state.is_changing_color() => {
                    selected_strands.insert(*s_id as usize);
                }
                Selection::Bound(_, n1, n2) => {
                    selected_xovers.insert((*n1, *n2));
                }
                Selection::Xover(_, xover_id) => {
                    if let Some((n1, n2)) = self.design.get_xover_with_id(*xover_id) {
                        selected_xovers.insert((n1, n2));
                    }
                }
                Selection::Helix {
                    helix_id,
                    segment_id,
                    ..
                } => {
                    if let Some(flat_helix) = FlatHelix::from_real(
                        HelixSegment {
                            helix_idx: *helix_id,
                            segment_idx: *segment_id,
                        },
                        id_map,
                    ) {
                        selected_helices.push(flat_helix.flat);
                    }
                }
                Selection::Nucleotide(_, n) => {
                    if let Some(flat_nucl) = FlatNucl::from_real(n, id_map) {
                        selected_nucls.push(flat_nucl);
                    }
                }
                _ => (),
            }
        }
        let mut suggestions = Vec::new();
        for c in new_state.get_candidates().iter() {
            match c {
                Selection::Strand(_, s_id) => {
                    candidate_strands.insert(*s_id as usize);
                }
                Selection::Bound(_, n1, n2) => {
                    candidate_xovers.insert((*n1, *n2));
                }
                Selection::Xover(_, xover_id) => {
                    if let Some((n1, n2)) = self.design.get_xover_with_id(*xover_id) {
                        candidate_xovers.insert((n1, n2));
                    }
                }
                Selection::Helix {
                    helix_id,
                    segment_id,
                    ..
                } => {
                    if let Some(flat_helix) = FlatHelix::from_real(
                        HelixSegment {
                            helix_idx: *helix_id,
                            segment_idx: *segment_id,
                        },
                        id_map,
                    ) {
                        candidate_helices.push(flat_helix.flat);
                    }
                }
                Selection::Nucleotide(_, n) => {
                    if let Some(flat_nucl) = FlatNucl::from_real(n, id_map) {
                        candidate_nucls.push(flat_nucl);
                        let mut other = self.get_best_suggestion(flat_nucl);
                        other = other.or_else(|| self.can_make_auto_xover(flat_nucl));
                        if let Some(other) = other {
                            suggestions.push((flat_nucl, other));
                        }
                    }
                }
                _ => (),
            }
        }
        let mut selection_highlight = Vec::new();
        let mut candidate_highlight = Vec::new();
        for s in self.design.get_strands().iter() {
            if selected_strands.contains(&s.id) {
                selection_highlight
                    .push(s.highlighted(SELECTED_COLOR, SELECTED_STRAND_HIGHLIGHT_FACTOR_2D));
            }
            if candidate_strands.contains(&s.id) {
                candidate_highlight
                    .push(s.highlighted(CANDIDATE_COLOR, CANDIDATE_STRAND_HIGHLIGHT_FACTOR_2D));
            }
        }
        for xover in selected_xovers.iter() {
            selection_highlight.push(self.design.strand_from_xover(xover, SELECTED_COLOR, true));
        }
        for xover in candidate_xovers.iter() {
            candidate_highlight.push(self.design.strand_from_xover(xover, CANDIDATE_COLOR, true));
        }
        self.view
            .borrow_mut()
            .update_selection(&selection_highlight, &self.helices);
        self.view
            .borrow_mut()
            .update_candidate(&candidate_highlight, &self.helices);
        self.view
            .borrow_mut()
            .set_selected_helices(selected_helices);
        self.view
            .borrow_mut()
            .set_candidate_helices(candidate_helices);
        self.view.borrow_mut().set_selected_nucls(selected_nucls);
        self.view.borrow_mut().set_candidate_nucls(candidate_nucls);
        self.view.borrow_mut().set_candidate_suggestion(
            suggestions.last().map(|t| t.0),
            suggestions.last().map(|t| t.1),
        );
    }

    fn update_strand_building_info(&self, info: Option<super::StrandBuildingStatus>) {
        let flat_info = info.and_then(|info| info.to_flat(self.id_map()));
        self.view
            .borrow_mut()
            .update_strand_building_info(flat_info);
    }

    fn fetch_helices(&mut self, design: R) {
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
            let segment = HelixSegment {
                helix_idx: h.id,
                segment_idx: h.segment_idx,
            };
            if let Some(flat_helix) = FlatHelix::from_real(segment, id_map) {
                self.helices.push(Helix::new(
                    h.left,
                    h.right,
                    h.isometry,
                    flat_helix,
                    h.id,
                    h.visible,
                    h.abscissa_converter.clone(),
                ));
                self.nb_helices_created += 1;
            }
        }
        let suggestions = self.design.suggestions();
        self.update_suggestion(&suggestions);
        self.view
            .borrow_mut()
            .set_suggestions(self.design.suggestions());
        self.view
            .borrow_mut()
            .set_torsions(self.design.get_torsions());
        self.view.borrow_mut().update_maps(
            design.get_group_map(),
            design.get_basis_map(),
            design.get_nucl_collection(),
        );
    }

    fn update_suggestion(&mut self, suggestion: &[(FlatNucl, FlatNucl)]) {
        self.suggestions.clear();
        for (n1, n2) in suggestion.iter() {
            self.suggestions.entry(*n1).or_default().insert(*n2);
            self.suggestions.entry(*n2).or_default().insert(*n1);
        }
    }

    pub fn get_click(&self, x: f32, y: f32, camera: &CameraPtr) -> ClickResult {
        self.get_click_(x, y, camera, true)
    }

    pub fn get_click_unbounded(&self, x: f32, y: f32, camera: &CameraPtr) -> ClickResult {
        self.get_click_(x, y, camera, false)
    }

    fn get_click_(&self, x: f32, y: f32, camera: &CameraPtr, bounded: bool) -> ClickResult {
        for h in self.helices.iter() {
            if h.click_on_circle(x, y, camera) {
                let translation_pivot = h.get_circle_pivot(camera).unwrap();
                return ClickResult::CircleWidget { translation_pivot };
            }
        }
        for h in self.helices.iter() {
            if let Some(handle) = h.click_on_handle(x, y) {
                return ClickResult::HelixHandle {
                    h_id: h.flat_id,
                    handle,
                };
            }
        }
        for h in self.helices.iter() {
            let ret = h
                .get_click(x, y, bounded)
                .map(|(position, forward)| FlatNucl {
                    helix: h.flat_id,
                    flat_position: position,
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

    pub(super) fn add_helix_selection<S: AppState>(
        &mut self,
        click_result: ClickResult,
        camera: &CameraPtr,
        app_state: &S,
    ) -> GraphicalSelection {
        let mut new_selection = app_state.get_selection().to_vec();
        self.add_selection(
            click_result,
            true,
            &mut new_selection,
            app_state.get_selection_mode(),
        );
        let pivots_opt = self.get_pivot_of_selected_helices(camera, &new_selection);
        self.requests
            .lock()
            .unwrap()
            .new_selection(new_selection.clone());
        if let Some((translation_pivots, rotation_pivots)) = pivots_opt {
            GraphicalSelection {
                translation_pivots,
                rotation_pivots,
                new_selection,
            }
        } else {
            GraphicalSelection::selection_only(new_selection)
        }
    }

    pub(super) fn set_helix_selection<S: AppState>(
        &mut self,
        click_result: ClickResult,
        camera: &CameraPtr,
        app_state: &S,
    ) -> GraphicalSelection {
        let mut new_selection = app_state.get_selection().to_vec();
        self.add_selection(
            click_result,
            false,
            &mut new_selection,
            app_state.get_selection_mode(),
        );
        let pivots_opt = self.get_pivot_of_selected_helices(camera, &new_selection);
        self.requests
            .lock()
            .unwrap()
            .new_selection(new_selection.clone());
        if let Some((translation_pivots, rotation_pivots)) = pivots_opt {
            GraphicalSelection {
                translation_pivots,
                rotation_pivots,
                new_selection,
            }
        } else {
            GraphicalSelection::selection_only(new_selection)
        }
    }

    pub fn get_click_unbounded_helix(&self, x: f32, y: f32, helix: FlatHelix) -> FlatNucl {
        let (flat_position, forward) = self.helices[helix.flat].get_click_unbounded(x, y);
        FlatNucl {
            flat_position,
            forward,
            helix,
        }
    }

    #[allow(dead_code)]
    pub fn get_pivot_position(&self, helix: FlatIdx, position: FlatPosition) -> Option<Vec2> {
        self.helices.get(helix).map(|h| h.get_pivot(position))
    }

    pub fn set_selected_helices(&mut self, helices: Vec<FlatHelix>) {
        /*
        for h in self.helices.iter_mut() {
            h.set_color(HELIX_BORDER_COLOR);
        }
        for h in helices {
            self.helices[h.flat].set_color(SELECTED_HELIX2D_COLOR);
        }
        self.instance_update = true;
        */
        let new_selection = helices
            .into_iter()
            .map(|flat| Selection::Helix {
                design_id: 0,
                helix_id: flat.segment.helix_idx,
                segment_id: flat.segment.segment_idx,
            })
            .collect();
        self.requests.lock().unwrap().new_selection(new_selection);
    }

    /*
    pub fn snap_helix(&mut self, pivot: FlatNucl, translation: Vec2) {
        self.helices[pivot.helix.flat].snap(pivot, translation);
        self.instance_update = true;
    }*/

    pub fn move_handle(&mut self, helix: FlatHelix, handle: HelixHandle, position: Vec2) {
        let (left, right) = self.helices[helix.flat].move_handle(handle, position);
        self.design.update_helix(helix, left, right);
        self.instance_update = true;
    }

    pub fn auto_redim_helix(&mut self, helix: FlatHelix, handle: HelixHandle) {
        let (left, right) = self.helices[helix.flat].reset_handle(handle);
        self.design.update_helix(helix, left, right);
    }

    /// Shrink the selected helices if selection is Some, or all helices if selection is None.
    pub fn redim_helices(&mut self, selection: Option<&[Selection]>) {
        if let Some(selection) = selection {
            let mut ids = Vec::new();
            for s in selection.iter() {
                if let Selection::Helix {
                    helix_id,
                    segment_id,
                    ..
                } = s
                {
                    let segment = HelixSegment {
                        helix_idx: *helix_id,
                        segment_idx: *segment_id,
                    };
                    if let Some(h) = self.design.id_map().get_segment_idx(segment) {
                        ids.push(h)
                    }
                }
            }
            for h_id in ids.iter() {
                if let Some(h) = self.helices.get_mut(*h_id) {
                    let (left, right) = h.redim_zero();
                    self.design.update_helix(h.flat_id, left, right);
                }
            }
        } else {
            for h in self.helices.iter_mut() {
                let (left, right) = h.redim_zero();
                self.design.update_helix(h.flat_id, left, right);
            }
        }
        self.notify_update();
    }

    /*
    pub fn rotate_helix(&mut self, helix: FlatHelix, pivot: Vec2, angle: f32) {
        self.helices[helix.flat].rotate(pivot, angle);
        self.instance_update = true;
    }*/

    pub fn end_movement(&mut self) {
        /*
        for h in self.helices.iter_mut() {
            h.end_movement()
        }*/
        self.requests.lock().unwrap().suspend_op()
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

    pub fn can_start_builder_at(&self, nucl: FlatNucl) -> bool {
        self.design.can_start_builder_at(nucl.to_real())
    }

    pub fn notify_update(&mut self) {
        self.instance_update = true;
    }

    pub fn can_cross_to(&self, from: FlatNucl, to: FlatNucl) -> bool {
        let from = from.to_real();
        let to = to.to_real();
        let prim5 = self
            .design
            .prime5_of(from)
            .or_else(|| self.design.prime5_of(to));
        let prim3 = self
            .design
            .prime3_of(from)
            .or_else(|| self.design.prime3_of(to));
        if prim3 != prim5 {
            prim3.zip(prim5).is_some()
        } else {
            let from_end = self
                .design
                .prime5_of(from)
                .or_else(|| self.design.prime3_of(from));
            let to_end = self
                .design
                .prime3_of(to)
                .or_else(|| self.design.prime5_of(to));
            from_end.is_some() && to_end.is_some()
        }
    }

    pub fn attachable_neighbour(&self, nucl: FlatNucl) -> Option<FlatNucl> {
        if self.can_cross_to(nucl, nucl.prime5()) {
            Some(nucl.prime5())
        } else if self.can_cross_to(nucl, nucl.prime3()) {
            Some(nucl.prime3())
        } else {
            None
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
            .or_else(|| self.design.prime5_of(nucl).map(|_| false))
    }

    pub fn set_free_end(&mut self, free_end: Option<FreeEnd>) {
        self.view.borrow_mut().set_free_end(free_end);
        self.instance_update = true;
    }

    pub fn xover(&self, from: FlatNucl, to: FlatNucl) -> (usize, usize) {
        let nucl1 = from.to_real();
        let nucl2 = to.to_real();

        //Handle case where nucl1 is the only nucleotide of the strand
        if let Some(s1) = self
            .design
            .prime5_of(nucl1)
            .and(self.design.prime3_of(nucl1))
        {
            if let Some(s2) = self.design.prime5_of(nucl2) {
                return (s1, s2);
            } else {
                return (self.design.prime3_of(nucl2).unwrap(), s1);
            }
        }

        // The 3 prime strand is the strand whose **5prime** end is in the xover
        let strand_3prime = self
            .design
            .prime5_of(nucl1)
            .or_else(|| self.design.prime5_of(nucl2));

        // The 5 prime strand is the strand whose **3prime** end is in the xover
        let strand_5prime = self
            .design
            .prime3_of(nucl1)
            .or_else(|| self.design.prime3_of(nucl2));

        if strand_3prime.is_none() || strand_5prime.is_none() {
            log::error!("Problem during cross-over attempt. If you are not trying to break a cyclic strand please repport a bug");
        }
        (strand_5prime.unwrap(), strand_3prime.unwrap())
    }

    pub fn get_fit_rectangle(&self) -> FitRectangle {
        let mut ret = FitRectangle::new();
        for h in self.helices.iter() {
            let left = h.get_pivot(h.get_flat_left());
            ret.add_point(Vec2::new(left.x, left.y));
            let right = h.get_pivot(h.get_flat_right());
            ret.add_point(Vec2::new(right.x, right.y));
        }
        ret
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

    pub(super) fn select_rectangle<S: AppState>(
        &mut self,
        c1: Vec2,
        c2: Vec2,
        camera: &CameraPtr,
        adding: bool,
        app_state: &S,
    ) -> GraphicalSelection {
        // Initialize the new selection with the current one. It will be cleared later if `adding`
        // is `false`.
        let mut new_selection = app_state.get_selection().to_vec();
        let selection_mode = app_state.get_selection_mode();
        if selection_mode == SelectionMode::Strand {
            self.select_strands_rectangle(camera, c1, c2, adding, &mut new_selection);
            if !new_selection.is_empty() {
                return GraphicalSelection::selection_only(new_selection);
            }
        } else if selection_mode == SelectionMode::Nucleotide {
            self.select_xovers_rectangle(camera, c1, c2, adding, &mut new_selection);
            if !new_selection.is_empty() {
                return GraphicalSelection::selection_only(new_selection);
            }
        }
        log::debug!("rectangle selection: {:?} {:?}", c1, c2);
        let mut translation_pivots = vec![];
        let mut rotation_pivots = vec![];
        let mut selection = Vec::new();
        for h in self.helices.iter_mut() {
            let c = h.get_circle(camera, &BTreeMap::new());
            if c.map(|c| c.in_rectangle(&c1, &c2)).unwrap_or(false) {
                let translation_pivot = h
                    .get_circle_pivot(camera)
                    .unwrap_or_else(|| h.default_pivot());
                let rotation_pivot = h.visible_center(camera).unwrap_or_else(|| h.center());
                h.set_color(SELECTED_HELIX2D_COLOR);
                translation_pivots.push(translation_pivot);
                rotation_pivots.push(rotation_pivot);
                selection.push(Selection::Helix {
                    segment_id: h.flat_id.segment.segment_idx,
                    helix_id: h.real_id,
                    design_id: self.id,
                });
            }
        }
        if adding {
            if let Some((mut old_translation_pivots, mut old_rotation_pivots)) =
                self.get_pivot_of_selected_helices(camera, &new_selection)
            {
                apply_symetric_difference_to_pivots(
                    &mut old_translation_pivots,
                    &mut old_rotation_pivots,
                    &selection,
                );
                translation_pivots.append(&mut old_translation_pivots);
                rotation_pivots.append(&mut old_rotation_pivots);
            }
            apply_symetric_difference_to_selection(&mut selection, &mut new_selection);
            selection.append(&mut new_selection);
            new_selection = selection;
        } else {
            new_selection = selection;
        }
        GraphicalSelection {
            translation_pivots,
            rotation_pivots,
            new_selection,
        }
    }

    pub fn get_pivot_of_selected_helices(
        &self,
        camera: &CameraPtr,
        selection: &[Selection],
    ) -> Option<(Vec<FlatNucl>, Vec<Vec2>)> {
        let id_map = self.design.id_map();

        let ret: Option<Vec<(FlatNucl, Vec2)>> = selection
            .iter()
            .map(|s| match s {
                Selection::Helix {
                    design_id,
                    helix_id,
                    segment_id,
                } if *design_id == self.id => {
                    let segment = HelixSegment {
                        segment_idx: *segment_id,
                        helix_idx: *helix_id,
                    };
                    if let Some(flat_id) = id_map.get_segment_idx(segment) {
                        if let Some(h) = self.helices.get(flat_id) {
                            let translation_pivot = h
                                .get_circle_pivot(camera)
                                .unwrap_or_else(|| h.default_pivot());
                            let rotation_pivot =
                                h.visible_center(camera).unwrap_or_else(|| h.center());
                            Some((translation_pivot, rotation_pivot))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        ret.map(|v| v.iter().cloned().unzip())
    }

    fn select_xovers_rectangle(
        &mut self,
        camera: &CameraPtr,
        c1: Vec2,
        c2: Vec2,
        adding: bool,
        new_selection: &mut Vec<Selection>,
    ) {
        let (x1, y1) = camera.borrow().world_to_norm_screen(c1.x, c1.y);
        let (x2, y2) = camera.borrow().world_to_norm_screen(c2.x, c2.y);
        let left = x1.min(x2);
        let right = x1.max(x2);
        let top = y1.min(y2);
        let bottom = y1.max(y2);
        log::debug!(
            "rectangle corners: {}, {}, {}, {}",
            left,
            top,
            right,
            bottom
        );
        let mut selection = Vec::new();
        for (xover_id, (flat_1, flat_2)) in self.design.get_xovers_list() {
            let h1 = &self.helices[flat_1.helix.flat];
            let h2 = &self.helices[flat_2.helix.flat];
            let flat_1_in = h1.rectangle_has_nucl(flat_1, left, top, right, bottom, camera);
            let flat_2_in = h2.rectangle_has_nucl(flat_2, left, top, right, bottom, camera);
            if flat_1_in && flat_2_in {
                selection.push(Selection::Xover(self.id, xover_id));
            } else if flat_1_in {
                selection.push(Selection::Nucleotide(self.id, flat_1.to_real()));
            } else if flat_2_in {
                selection.push(Selection::Nucleotide(self.id, flat_2.to_real()));
            }
        }
        for end in self.design.get_strand_ends() {
            let h1 = &self.helices[end.helix.flat];
            if h1.rectangle_has_nucl(end, left, top, right, bottom, camera) {
                selection.push(Selection::Nucleotide(self.id, end.to_real()))
            }
        }
        selection.dedup();
        if selection.is_empty() {
            self.add_long_xover_rectangle(&mut selection, c1, c2);
        }
        if adding {
            for s in selection.iter() {
                if !new_selection.contains(s) {
                    new_selection.push(*s);
                }
            }
        } else {
            *new_selection = selection;
        }
        log::debug!("returned selection {:?}", new_selection);
    }

    fn add_long_xover_rectangle(&self, selection: &mut Vec<Selection>, c1: Vec2, c2: Vec2) {
        let mut selection_set = BTreeSet::new();
        for (xover_id, (flat_1, flat_2)) in self.design.get_xovers_list() {
            let h1 = &self.helices[flat_1.helix.flat];
            let h2 = &self.helices[flat_2.helix.flat];
            let a = h1.get_nucl_position(&flat_1, helix::Shift::No);
            let b = h2.get_nucl_position(&flat_2, helix::Shift::No);
            if helix::rectangle_intersect(c1, c2, a, b) {
                selection_set.insert(xover_id);
            }
        }
        for xover_id in selection_set.into_iter() {
            selection.push(Selection::Xover(self.id, xover_id))
        }
    }

    fn select_strands_rectangle(
        &mut self,
        camera: &CameraPtr,
        c1: Vec2,
        c2: Vec2,
        adding: bool,
        new_selection: &mut Vec<Selection>,
    ) {
        let (x1, y1) = camera.borrow().world_to_norm_screen(c1.x, c1.y);
        let (x2, y2) = camera.borrow().world_to_norm_screen(c2.x, c2.y);
        let left = x1.min(x2);
        let right = x1.max(x2);
        let top = y1.min(y2);
        let bottom = y1.max(y2);
        log::debug!("rectangle corner {}, {}, {}, {}", left, top, right, bottom);
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
                if !new_selection.contains(s) {
                    new_selection.push(*s);
                }
            }
        } else {
            *new_selection = selection;
        }
    }

    pub fn double_click_to_selection(&self, click_result: ClickResult) -> Option<Selection> {
        match click_result {
            ClickResult::CircleWidget { .. } => None,
            ClickResult::Nucl(nucl) => {
                if let Some(xover) = self.xover_containing_nucl(&nucl) {
                    let selection = Selection::Xover(self.id, xover);
                    Some(selection)
                } else {
                    let selection = Selection::Nucleotide(self.id, nucl.to_real());
                    Some(selection)
                }
            }
            ClickResult::HelixHandle { .. } => None,
            ClickResult::Nothing => None,
        }
    }

    pub fn add_selection(
        &mut self,
        click_result: ClickResult,
        adding: bool,
        new_selection: &mut Vec<Selection>,
        selection_mode: SelectionMode,
    ) {
        if !adding {
            new_selection.clear()
        }
        match click_result {
            ClickResult::CircleWidget { translation_pivot } => {
                let selection = Selection::Helix {
                    design_id: self.id,
                    helix_id: translation_pivot.helix.segment.helix_idx,
                    segment_id: translation_pivot.helix.segment.segment_idx,
                };
                if let Some(pos) = new_selection.iter().position(|x| *x == selection) {
                    new_selection.remove(pos);
                } else {
                    new_selection.push(selection);
                }
            }
            ClickResult::Nucl(nucl) => match selection_mode {
                SelectionMode::Strand => {
                    if let Some(s_id) = self.design.get_strand_id(nucl.to_real()) {
                        let selection = Selection::Strand(self.id, s_id as u32);
                        if let Some(pos) = new_selection.iter().position(|x| *x == selection) {
                            new_selection.remove(pos);
                        } else {
                            new_selection.push(selection);
                        }
                    }
                }
                _ => {
                    self.last_click.click_on(nucl);
                    let mut selection_pool = vec![Selection::Nucleotide(self.id, nucl.to_real())];
                    if let Some(xover) = self.xover_containing_nucl(&nucl) {
                        selection_pool.push(Selection::Xover(self.id, xover));
                    }
                    if let Some(s_id) = self.get_strand_id(nucl) {
                        selection_pool.push(Selection::Strand(self.id, s_id as u32));
                    }
                    selection_pool.push(Selection::Nothing);
                    log::info!("selection pool {:?}", selection_pool);
                    let selection = self.last_click.select(&mut selection_pool);
                    log::info!(
                        "selected {:?}, selection_pool {:?}",
                        selection,
                        selection_pool
                    );

                    new_selection.push(selection);
                    new_selection.retain(|s| !selection_pool.contains(s));
                }
            },
            ClickResult::HelixHandle { .. } => (),
            ClickResult::Nothing => (),
        }
    }

    /*
    pub fn set_selection(&mut self, mut selection: Vec<Selection>) {
        self.selection = selection.clone();
        if selection.len() == 1 {
            let xover = if let Some(Selection::Xover(d_id, xover_id)) = selection.get(0) {
                Some(*d_id).zip(self.design.get_xover_with_id(*xover_id))
            } else {
                None
            };
            if let Some((d_id, (n1, n2))) = xover {
                selection[0] = Selection::Bound(d_id, n1, n2);
            }
            self.view
                .borrow_mut()
                .set_selection(super::FlatSelection::from_real(
                    selection.get(0),
                    self.id_map(),
                ));
        }
        self.selection_updated = true;
    }*/

    /*
    pub fn set_candidate(&mut self, candidates: Vec<Selection>) {
        self.candidates = candidates;
        self.selection_updated = true;
    }*/

    pub(super) fn convert_to_flat(&self, selection: Selection) -> FlatSelection {
        FlatSelection::from_real(Some(&selection), self.id_map())
    }

    pub(super) fn xover_to_nuclpair(&self, selection: FlatSelection) -> FlatSelection {
        if let FlatSelection::Xover(d_id, xover_id) = selection {
            if let Some((n1, n2)) = self.design.get_xover_with_id(xover_id) {
                let flat_1 = FlatNucl::from_real(&n1, self.id_map());
                let flat_2 = FlatNucl::from_real(&n2, self.id_map());
                if let Some((flat_1, flat_2)) = flat_1.zip(flat_2) {
                    FlatSelection::Bound(d_id, flat_1, flat_2)
                } else {
                    FlatSelection::Nothing
                }
            } else {
                FlatSelection::Nothing
            }
        } else {
            selection
        }
    }

    fn xover_containing_nucl(&self, nucl: &FlatNucl) -> Option<usize> {
        let xovers_list = self.design.get_xovers_list();
        xovers_list.iter().find_map(|(id, (n1, n2))| {
            if *n1 == *nucl || *n2 == *nucl {
                Some(*id)
            } else {
                None
            }
        })
    }

    pub fn phantom_to_selection(
        &self,
        phantom: PhantomElement,
        selection_mode: SelectionMode,
    ) -> Option<Selection> {
        if let Some(n_id) = self.design.get_nucl_id(phantom.to_nucl()) {
            match selection_mode {
                SelectionMode::Helix => {
                    self.design
                        .get_helix_from_eid(n_id)
                        .map(|h| Selection::Helix {
                            segment_id: 0,
                            helix_id: h,
                            design_id: self.id,
                        })
                }
                SelectionMode::Strand => self
                    .design
                    .get_strand_from_eid(n_id)
                    .map(|s| Selection::Strand(phantom.design_id, s as u32)),
                SelectionMode::Design => None,
                SelectionMode::Nucleotide => {
                    Some(Selection::Nucleotide(phantom.design_id, phantom.to_nucl()))
                }
            }
        } else {
            None
        }
    }

    fn get_xover_nucl(&self, nucl: FlatNucl) -> Option<FlatNucl> {
        for x in self.design.get_xovers_list() {
            if x.1 .0 == nucl {
                return Some(x.1 .1);
            } else if x.1 .1 == nucl {
                return Some(x.1 .0);
            }
        }
        None
    }

    pub fn can_make_auto_xover(&self, nucl: FlatNucl) -> Option<FlatNucl> {
        let strand = self.get_strand_id(nucl)?;

        // Check if the nucleotide on 5' is involved in a crossover ? If so the candidate is the
        // nucleotide on 5' of the nucl crossed to.
        let prime5_nucl = nucl.prime5();
        let strand_of_prime5 = self.get_strand_id(prime5_nucl);

        if let Some(xover_of_prime5) = self.get_xover_nucl(prime5_nucl) {
            let candidate = xover_of_prime5.prime5();
            if strand_of_prime5 == Some(strand) {
                // Special case where auto xover could be closing a cyclic strand
                if self.design.prime5_of(nucl.to_real()) == Some(strand)
                    && self.design.prime3_of(candidate.to_real()) == Some(strand)
                {
                    return Some(candidate);
                }
            } else if strand_of_prime5.is_some() && self.can_cross_to(nucl, candidate) {
                return Some(candidate);
            }
        }

        // Check if the nucleotide on 3' is involved in a crossover ? If so the candidate is the
        // nucleotide on 3' of the nucl crossed to.
        let prime3_nucl = nucl.prime3();
        let strand_of_prime3 = self.get_strand_id(prime3_nucl);
        if let Some(xover_of_prime3) = self.get_xover_nucl(prime3_nucl) {
            let candidate = xover_of_prime3.prime3();
            if strand_of_prime3 == Some(strand) {
                // Special case where auto xover could be closing a cyclic strand
                if self.design.prime3_of(nucl.to_real()) == Some(strand)
                    && self.design.prime5_of(candidate.to_real()) == Some(strand)
                {
                    return Some(candidate);
                }
            } else if strand_of_prime3.is_some() && self.can_cross_to(nucl, candidate) {
                return Some(candidate);
            }
        }

        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClickResult {
    Nucl(FlatNucl),
    CircleWidget {
        translation_pivot: FlatNucl,
    },
    HelixHandle {
        h_id: FlatHelix,
        handle: HelixHandle,
    },
    Nothing,
}

/// A selection made by interacting with the 2D scene.
pub(super) struct GraphicalSelection {
    pub translation_pivots: Vec<FlatNucl>,
    pub rotation_pivots: Vec<Vec2>,
    pub new_selection: Vec<Selection>,
}

impl GraphicalSelection {
    fn selection_only(selection: Vec<Selection>) -> Self {
        Self {
            new_selection: selection,
            translation_pivots: vec![],
            rotation_pivots: vec![],
        }
    }
}

/// Remove the element of `old_translation_pivots` and `old_rotation_pivots` that corresponds to an
/// element existing in `selection`
fn apply_symetric_difference_to_pivots(
    old_translation_pivots: &mut Vec<FlatNucl>,
    old_rotation_pivots: &mut Vec<Vec2>,
    selection: &[Selection],
) {
    if old_translation_pivots.len() != old_rotation_pivots.len() {
        return;
    }

    for i in (0..old_rotation_pivots.len()).rev() {
        let real_helix = old_translation_pivots[i].helix.segment.helix_idx;
        let segment_idx = old_translation_pivots[i].helix.segment.segment_idx;
        if selection
            .iter()
            .any(|s| matches!(s, Selection::Helix{helix_id, segment_id, ..} if *helix_id == real_helix && *segment_id == segment_idx))
        {
            old_translation_pivots.remove(i);
            old_rotation_pivots.remove(i);
        }
    }
}

fn apply_symetric_difference_to_selection(
    old_selection: &mut Vec<Selection>,
    new_selection: &mut Vec<Selection>,
) {
    let mut to_remove = Vec::new();
    for s in old_selection.iter() {
        if new_selection.contains(s) {
            to_remove.push(*s);
        }
    }

    let retain_condition = |s: &Selection| !to_remove.contains(s);

    old_selection.retain(retain_condition);
    new_selection.retain(retain_condition);
}

trait ToFlatInfo {
    fn to_flat(self, id_map: &FlatHelixMaps) -> Option<super::view::EditionInfo>;
}

impl ToFlatInfo for super::StrandBuildingStatus {
    fn to_flat(self, id_map: &FlatHelixMaps) -> Option<super::view::EditionInfo> {
        let flat_nucl = FlatNucl::from_real(&self.dragged_nucl, id_map)?;
        Some(EditionInfo {
            nt_length: self.nt_length,
            nm_length: self.nm_length,
            nucl: flat_nucl,
        })
    }
}

struct LastClick {
    counter: usize,
    last_click_time: std::time::Instant,
    nucl: Option<FlatNucl>,
}

impl Default for LastClick {
    fn default() -> Self {
        Self {
            counter: 0,
            last_click_time: std::time::Instant::now(),
            nucl: None,
        }
    }
}

impl LastClick {
    pub fn click_on(&mut self, nucl: FlatNucl) {
        let now = std::time::Instant::now();
        if self.nucl == Some(nucl)
            && (now - self.last_click_time)
                < std::time::Duration::from_millis(SELECTION_2D_CYCLE_TIME_LIMIT_MS)
        {
            self.counter += 1;
        } else {
            self.counter = 0;
            self.nucl = Some(nucl);
        }
        self.last_click_time = now;
    }

    pub fn select(&self, pool: &mut Vec<Selection>) -> Selection {
        if pool.is_empty() {
            Selection::Nothing
        } else {
            let id = self.counter % pool.len();
            pool.remove(id)
        }
    }
}
