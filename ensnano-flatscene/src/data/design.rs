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
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use super::super::{FlatHelix, FlatIdx, FlatNucl, HelixSegment, Requests};
use super::{Flat, HelixVec, Nucl, Strand};
use ahash::RandomState;
use ensnano_design::{
    ultraviolet, AbscissaConverter, Extremity, Helix as DesignHelix, HelixCollection,
    Strand as StrandDesign,
};
use ensnano_interactor::consts::{
    CANDIDATE_STRAND_HIGHLIGHT_FACTOR_2D, SELECTED_STRAND_HIGHLIGHT_FACTOR_2D,
};
use ensnano_interactor::{torsion::Torsion, Referential};
use ensnano_utils::full_isometry::FullIsometry;
use ultraviolet::{Isometry2, Rotor2, Vec2, Vec3};

use crate::flattypes::FlatHelixMaps;

pub(super) struct Design2d<R: DesignReader> {
    /// The 2d helices
    helices: HelixVec<Helix2d>,
    /// Maps id of helices in design to location in self.helices
    id_map: FlatHelixMaps,
    /// the 2d strands
    strands: Vec<Strand>,
    /// A pointer to the design
    design: R,
    /// The strand being pasted,
    pasted_strands: Vec<Strand>,
    last_flip_other: Option<FlatHelix>,
    removed: BTreeSet<FlatIdx>,
    requests: Arc<Mutex<dyn Requests>>,
    known_helices: HashMap<usize, *const DesignHelix>,
    known_map: *const ensnano_design::Helices,
}

impl<R: DesignReader> Design2d<R> {
    pub fn new(design: R, requests: Arc<Mutex<dyn Requests>>) -> Self {
        Self {
            design,
            helices: HelixVec::new(),
            id_map: Default::default(),
            strands: Vec::new(),
            pasted_strands: Vec::new(),
            last_flip_other: None,
            removed: BTreeSet::new(),
            requests,
            known_helices: Default::default(),
            known_map: std::ptr::null(),
        }
    }

    pub fn clear(&mut self) {
        self.helices = HelixVec::new();
        self.id_map = Default::default();
        self.strands = Default::default();
        self.pasted_strands = Default::default();
        self.last_flip_other = Default::default();
        self.removed = Default::default();
        self.known_helices = Default::default();
        self.known_map = std::ptr::null();
    }

    /// Re-read the design and update the 2d data accordingly
    pub fn update(&mut self, design: R) {
        self.design = design;
        log::trace!("updating design");
        // At the moment we rebuild the strands from scratch. If needed, this might be an optimisation
        // target
        self.strands = Vec::new();
        self.update_helices();
        self.rm_deleted_helices();
        let strand_ids = self.design.get_all_strand_ids();
        for strand_id in strand_ids.iter() {
            let strand_opt = self.design.get_strand_points(*strand_id);
            log::debug!("strand {strand_id}\n strand points: {:?}", strand_opt);
            // Unwrap: `strand_id` is in the list returned by `get_all_strand_ids` so it
            // corresponds to an existing strand id.
            let strand = strand_opt.unwrap();
            let color = self.design.get_strand_color(*strand_id).unwrap_or_else(|| {
                log::warn!("Warning: could not find strand color, this is not normal");
                0
            });
            for nucl in strand.iter() {
                self.read_nucl(nucl)
            }
            let flat_strand: Vec<_> = strand
                .iter()
                .filter_map(|n| FlatNucl::from_real(n, self.id_map()))
                .collect();
            let insertions = self.design.get_insertions(*strand_id).unwrap_or_default();
            let insertions = insertions
                .iter()
                .filter_map(|n| FlatNucl::from_real(n, self.id_map()))
                .collect::<Vec<_>>();
            self.strands.push(Strand::new(
                color,
                flat_strand,
                insertions,
                *strand_id,
                None,
            ));
        }
        let nucls_opt = self.design.get_copy_points();

        self.pasted_strands = nucls_opt
            .iter()
            .map(|nucls| {
                let color = ensnano_interactor::consts::CANDIDATE_COLOR;
                for nucl in nucls.iter() {
                    self.read_nucl(nucl)
                }
                let flat_strand = nucls
                    .iter()
                    .filter_map(|n| FlatNucl::from_real(n, self.id_map()))
                    .collect();
                Strand::new(
                    color,
                    flat_strand,
                    vec![],
                    0,
                    Some(CANDIDATE_STRAND_HIGHLIGHT_FACTOR_2D),
                )
            })
            .collect();

        for (segment, flat_idx) in self.id_map.iter() {
            let visibility = self.design.get_visibility_helix(segment.helix_idx);
            if let Some(h) = self.helices.get_mut(*flat_idx) {
                h.visible = visibility.unwrap_or(false)
            }
        }

        for h in self.helices.iter_mut() {
            h.force_positive_size();
        }
        log::trace!("done");
    }

    pub fn suggestions(&self) -> Vec<(FlatNucl, FlatNucl)> {
        let suggestions = self.design.get_suggestions();
        suggestions
            .iter()
            .filter_map(|(n1, n2)| {
                FlatNucl::from_real(n1, &self.id_map).zip(FlatNucl::from_real(n2, &self.id_map))
            })
            .collect()
    }

    fn rm_deleted_helices(&mut self) {
        let mut to_remove = Vec::new();
        for (segment, h) in self.id_map.iter() {
            if !self.design.has_helix(segment.helix_idx) {
                let flat_helix = FlatHelix {
                    flat: *h,
                    segment: *segment,
                    segment_left: None,
                };
                to_remove.push(flat_helix);
                self.removed.insert(*h);
            }
        }
        to_remove.sort();
        if !to_remove.is_empty() {
            for h in to_remove.iter().rev() {
                self.helices.remove(h.flat);
                self.known_helices.remove(&h.segment.helix_idx);
            }
            self.remake_id_map();
        }
    }

    pub fn get_removed_helices(&mut self) -> BTreeSet<FlatIdx> {
        std::mem::take(&mut self.removed)
    }

    pub fn update_helix(&mut self, helix: FlatHelix, left: isize, right: isize) {
        let helix2d = &mut self.helices[helix.flat];
        helix2d.left = left;
        helix2d.right = right;
        if let Some(min_left) = helix2d.min_left {
            helix2d.right = helix2d.left.max(min_left);
        }
        if let Some(max_right) = helix2d.max_right {
            helix2d.right = helix2d.right.min(max_right);
        }
    }

    /// Add a nucleotide to self and increase the left/right bond the the 2d segment representation
    fn read_nucl(&mut self, nucl: &Nucl) {
        if let Some(flat_nucl) = FlatNucl::from_real(nucl, &self.id_map) {
            let helix2d = &mut self.helices[flat_nucl.helix.flat];
            helix2d.left = helix2d.left.min(nucl.position - 1);
            helix2d.right = helix2d.right.max(nucl.position + 1);
            if let Some(min_left) = helix2d.min_left {
                helix2d.left = helix2d.left.max(min_left);
            }
            if let Some(max_right) = helix2d.max_right {
                helix2d.right = helix2d.right.min(max_right);
            }
        } else {
            log::error!("Could not convert {nucl} to flat nucl")
        }
    }

    fn update_helices(&mut self) {
        if std::ptr::eq(self.known_map, self.design.get_helices_map()) {
            // Nothing to do
            return;
        }
        let helices = self.design.get_helices_map();
        self.known_map = helices;
        let helices = self.design.get_helices_map().clone();
        for (h_id, helix) in helices.iter() {
            // update helix only if necessary
            if self
                .known_helices
                .get(h_id)
                .filter(|ptr| std::ptr::eq(**ptr, helix))
                .is_none()
            {
                self.known_helices.insert(*h_id, helix);
                let segments: Vec<isize> =
                    helix.additonal_isometries.iter().map(|i| i.left).collect();
                let nb_segments = segments.len();
                self.id_map.insert_segments(*h_id, segments);
                for segment_idx in 0..=nb_segments {
                    self.read_helix_segment(HelixSegment {
                        helix_idx: *h_id,
                        segment_idx,
                    })
                }
            }
        }
    }

    fn read_helix_segment(&mut self, segment: HelixSegment) {
        let iso_opt = self
            .design
            .get_isometry(segment.helix_idx, segment.segment_idx);
        let isometry = if let Some(iso) = iso_opt {
            iso
        } else if let Some(mut iso) = self.design.get_isometry(segment.helix_idx, 0) {
            iso.prepend_translation(5. * self.id_map.len() as f32 * Vec2::unit_y());
            self.requests
                .lock()
                .unwrap()
                .set_isometry(segment.helix_idx, segment.segment_idx, iso);
            iso
        } else {
            let iso = Isometry2::new(
                (5. * self.id_map.len() as f32 - 1.) * Vec2::unit_y(),
                Rotor2::identity(),
            );
            self.requests
                .lock()
                .unwrap()
                .set_isometry(segment.helix_idx, segment.segment_idx, iso);
            iso
        };
        let symmetry = self
            .design
            .get_helix_segment_symmetry(segment.helix_idx, segment.segment_idx)
            .unwrap_or_else(Vec2::one);
        if !self.id_map.contains_segment(segment) {
            let flat_idx = FlatIdx(self.helices.len());
            self.id_map.insert_segment_key(flat_idx, segment);
            let max_right = self.id_map.get_max_right(segment);
            let min_left = self.id_map.get_min_left(segment);
            let left = min_left.unwrap_or(-1);
            self.helices.push(Helix2d {
                id: segment.helix_idx,
                segment_idx: segment.segment_idx,
                left: min_left.map(|x| x - 1).unwrap_or(-1),
                right: left + 2,
                max_right,
                min_left,
                isometry: FullIsometry::from_isommetry_symmetry(isometry, symmetry),
                visible: self
                    .design
                    .get_visibility_helix(segment.helix_idx)
                    .unwrap_or(false),
                abscissa_converter: Arc::new(self.design.get_abcissa_converter(segment.helix_idx)),
            });
        } else {
            // unwrap Ok because we know that the key exists
            let flat = self.id_map.get_segment_idx(segment).unwrap();
            let helix2d = &mut self.helices[flat];
            helix2d.isometry = FullIsometry::from_isommetry_symmetry(isometry, symmetry);
            helix2d.abscissa_converter =
                Arc::new(self.design.get_abcissa_converter(segment.helix_idx));
        }
    }

    pub fn get_helices(&self) -> &[Helix2d] {
        &self.helices
    }

    pub fn get_strands(&self) -> &[Strand] {
        &self.strands
    }

    pub fn get_pasted_strand(&self) -> &[Strand] {
        &self.pasted_strands
    }

    pub fn flip_visibility(&mut self, flat_helix: FlatHelix, apply_to_other: bool) {
        if apply_to_other {
            let visibility = if self.last_flip_other == Some(flat_helix) {
                self.last_flip_other = None;
                self.helices[flat_helix.flat].visible
            } else {
                self.last_flip_other = Some(flat_helix);
                !self.helices[flat_helix.flat].visible
            };
            for (segment, _) in self
                .id_map
                .iter()
                .filter(|(segment, _)| segment.helix_idx != flat_helix.segment.helix_idx)
            {
                self.requests
                    .lock()
                    .unwrap()
                    .set_visibility_helix(segment.helix_idx, visibility)
            }
        } else {
            self.requests.lock().unwrap().set_visibility_helix(
                flat_helix.segment.helix_idx,
                !self.helices[flat_helix.flat].visible,
            )
        }
    }

    pub fn flip_group(&mut self, helix: FlatHelix) {
        self.requests
            .lock()
            .unwrap()
            .flip_group(helix.segment.helix_idx)
    }

    pub fn can_start_builder_at(&self, nucl: Nucl) -> bool {
        self.design.can_start_builder_at(nucl)
    }

    pub fn prime3_of(&self, nucl: Nucl) -> Option<usize> {
        self.design.prime3_of_which_strand(nucl)
    }

    pub fn prime5_of(&self, nucl: Nucl) -> Option<usize> {
        self.design.prime5_of_which_strand(nucl)
    }

    pub fn remake_id_map(&mut self) {
        self.id_map.clear_maps();
        for (i, h) in self.helices.iter().enumerate() {
            self.id_map.insert_segment_key(
                FlatIdx(i),
                HelixSegment {
                    helix_idx: h.id,
                    segment_idx: h.segment_idx,
                },
            );
        }
    }

    pub fn id_map(&self) -> &FlatHelixMaps {
        &self.id_map
    }

    pub fn is_xover_end(&self, nucl: &Nucl) -> Option<bool> {
        self.design.is_xover_end(nucl).to_opt()
    }

    pub fn has_nucl(&self, nucl: Nucl) -> bool {
        self.design.get_identifier_nucl(&nucl).is_some()
    }

    pub fn get_strand_id(&self, nucl: Nucl) -> Option<usize> {
        self.design.get_id_of_strand_containing_nucl(&nucl)
    }

    pub fn get_dist(&self, nucl1: Nucl, nucl2: Nucl) -> Option<f32> {
        let pos1 = self
            .design
            .get_position_of_nucl_on_helix(nucl1, Referential::Model, false)?;
        let pos2 = self
            .design
            .get_position_of_nucl_on_helix(nucl2, Referential::Model, false)?;
        Some((pos1 - pos2).mag())
    }

    pub fn get_torsions(&self) -> HashMap<(FlatNucl, FlatNucl), FlatTorsion> {
        let torsions = self.design.get_torsions();
        let conversion = |((n1, n2), k): (&(Nucl, Nucl), &Torsion)| {
            let flat_1 = FlatNucl::from_real(n1, &self.id_map);
            let flat_2 = FlatNucl::from_real(n2, &self.id_map);
            let torsion = FlatTorsion::from_real(k, &self.id_map);
            flat_1.zip(flat_2).zip(Some(torsion))
        };
        torsions.iter().filter_map(conversion).collect()
    }

    pub fn get_xovers_list(&self) -> Vec<(usize, (FlatNucl, FlatNucl))> {
        let xovers = self.design.get_xovers_list_with_id();
        xovers
            .iter()
            .filter_map(|(id, (n1, n2))| {
                let flat_1 = FlatNucl::from_real(n1, &self.id_map);
                let flat_2 = FlatNucl::from_real(n2, &self.id_map);
                Some(*id).zip(flat_1.zip(flat_2))
            })
            .collect()
    }

    pub fn strand_from_xover(&self, xover: &(Nucl, Nucl), color: u32, thicker: bool) -> Strand {
        // pretend it's a strand with two size one domains
        let flat_nucls = [xover.0, xover.0, xover.1, xover.1]
            .iter()
            .filter_map(|n| FlatNucl::from_real(n, self.id_map()))
            .collect();

        let width = if thicker {
            SELECTED_STRAND_HIGHLIGHT_FACTOR_2D
        } else {
            1.
        };

        Strand::new(0, flat_nucls, vec![], 0, None).highlighted(color, width)
    }

    pub fn get_nucl_id(&self, nucl: Nucl) -> Option<u32> {
        self.design.get_identifier_nucl(&nucl)
    }

    pub fn get_strand_from_eid(&self, element_id: u32) -> Option<usize> {
        self.design.get_id_of_strand_containing_elt(element_id)
    }

    pub fn get_helix_from_eid(&self, element_id: u32) -> Option<usize> {
        self.design.get_id_of_of_helix_containing_elt(element_id)
    }

    pub fn get_xover_with_id(&self, xover_id: usize) -> Option<(Nucl, Nucl)> {
        self.design.get_xover_with_id(xover_id)
    }

    pub fn get_strand_ends(&self) -> Vec<FlatNucl> {
        self.design
            .get_strand_ends()
            .iter()
            .filter_map(|n| FlatNucl::from_real(n, &self.id_map))
            .collect()
    }
}

/// Store the informations needed to represent an helix from the design
#[derive(Debug)]
pub struct Helix2d {
    /// The id of the helix within the design
    pub id: usize,
    /// The smallest position of a nucleotide represented on this helix segment
    pub left: isize,
    /// The largest position of a nucleotide represented on this helix segment
    pub right: isize,
    /// The largest value that can be taken by self.right
    pub max_right: Option<isize>,
    /// The smallest value that can be taken by self.left
    pub min_left: Option<isize>,
    pub isometry: FullIsometry,
    pub visible: bool,
    pub abscissa_converter: Arc<AbscissaConverter>,
    pub segment_idx: usize,
}

impl Helix2d {
    fn force_positive_size(&mut self) {
        if self.right < self.left + 1 {
            self.left = -1;
            self.right = 1;
        }
    }
}

impl Flat for Helix2d {}

pub struct FlatTorsion {
    pub strength_prime5: f32,
    pub strength_prime3: f32,
    pub friend: Option<(FlatNucl, FlatNucl)>,
}

impl FlatTorsion {
    pub fn from_real(real: &Torsion, id_map: &FlatHelixMaps) -> Self {
        Self {
            strength_prime5: real.strength_prime5,
            strength_prime3: real.strength_prime3,
            friend: real.friend.and_then(|(n1, n2)| {
                FlatNucl::from_real(&n1, id_map).zip(FlatNucl::from_real(&n2, id_map))
            }),
        }
    }
}

pub trait DesignReader: 'static {
    type NuclCollection: NuclCollection;
    fn get_all_strand_ids(&self) -> Vec<usize>;
    /// Return a the list of consecutive domain extremities of strand `s_id`. Return None iff there
    /// is no strand with id `s_id` in the design.
    fn get_strand_points(&self, s_id: usize) -> Option<Vec<Nucl>>;
    fn get_strand_color(&self, s_id: usize) -> Option<u32>;
    fn get_insertions(&self, s_id: usize) -> Option<Vec<Nucl>>;
    fn get_copy_points(&self) -> Vec<Vec<Nucl>>;
    fn get_visibility_helix(&self, h_id: usize) -> Option<bool>;
    fn get_suggestions(&self) -> Vec<(Nucl, Nucl)>;
    fn has_helix(&self, h_id: usize) -> bool;
    fn get_isometry(&self, h_id: usize, segment_idx: usize) -> Option<Isometry2>;
    fn get_helix_segment_symmetry(&self, h_id: usize, segment_idx: usize) -> Option<Vec2>;
    fn can_start_builder_at(&self, nucl: Nucl) -> bool;
    fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize>;
    fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize>;
    fn helix_is_empty(&self, h_id: usize) -> Option<bool>;
    fn get_helices_map(&self) -> &ensnano_design::Helices;
    fn get_raw_helix(&self, h_id: usize) -> Option<Arc<DesignHelix>>;
    fn get_raw_strand(&self, s_id: usize) -> Option<StrandDesign>;
    fn is_xover_end(&self, nucl: &Nucl) -> Extremity;
    fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32>;
    fn get_id_of_strand_containing_nucl(&self, nucl: &Nucl) -> Option<usize>;
    fn get_position_of_nucl_on_helix(
        &self,
        nucl: Nucl,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3>;
    fn get_torsions(&self) -> HashMap<(Nucl, Nucl), Torsion>;
    fn get_xovers_list_with_id(&self) -> Vec<(usize, (Nucl, Nucl))>;
    fn get_id_of_strand_containing_elt(&self, e_id: u32) -> Option<usize>;
    fn get_id_of_of_helix_containing_elt(&self, e_id: u32) -> Option<usize>;
    fn get_xover_with_id(&self, xover_id: usize) -> Option<(Nucl, Nucl)>;
    fn get_basis_map(&self) -> Arc<HashMap<Nucl, char, RandomState>>;
    fn get_group_map(&self) -> Arc<BTreeMap<usize, bool>>;
    fn get_strand_ends(&self) -> Vec<Nucl>;
    fn get_nucl_collection(&self) -> Arc<Self::NuclCollection>;
    fn get_abcissa_converter(&self, h_id: usize) -> AbscissaConverter;
}

pub trait NuclCollection {
    fn contains(&self, nucl: &Nucl) -> bool;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Nucl> + 'a>;
}
