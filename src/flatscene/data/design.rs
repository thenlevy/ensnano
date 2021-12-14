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
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::super::{FlatHelix, FlatIdx, FlatNucl, Requests};
use super::{Flat, HelixVec, Nucl, Strand};
use ahash::RandomState;
use ensnano_design::{Extremity, Helix as DesignHelix, Strand as StrandDesign};
use ensnano_interactor::{torsion::Torsion, Referential};
use ultraviolet::{Isometry2, Rotor2, Vec2, Vec3};

pub(super) struct Design2d {
    /// The 2d helices
    helices: HelixVec<Helix2d>,
    /// Maps id of helices in design to location in self.helices
    id_map: HashMap<usize, FlatIdx>,
    /// the 2d strands
    strands: Vec<Strand>,
    /// A pointer to the design
    design: Box<dyn DesignReader>,
    /// The strand being pasted,
    pasted_strands: Vec<Strand>,
    last_flip_other: Option<FlatHelix>,
    removed: BTreeSet<FlatIdx>,
    requests: Arc<Mutex<dyn Requests>>,
    known_helices: HashMap<usize, *const DesignHelix>,
    known_map: *const BTreeMap<usize, Arc<DesignHelix>>,
}

impl Design2d {
    pub fn new<R: DesignReader>(design: R, requests: Arc<Mutex<dyn Requests>>) -> Self {
        Self {
            design: Box::new(design),
            helices: HelixVec::new(),
            id_map: HashMap::new(),
            strands: Vec::new(),
            pasted_strands: Vec::new(),
            last_flip_other: None,
            removed: BTreeSet::new(),
            requests,
            known_helices: Default::default(),
            known_map: std::ptr::null(),
        }
    }

    /// Re-read the design and update the 2d data accordingly
    pub fn update<R: DesignReader>(&mut self, design: R) {
        self.design = Box::new(design);
        log::trace!("updating design");
        // At the moment we rebuild the strands from scratch. If needed, this might be an optimisation
        // target
        self.strands = Vec::new();
        self.update_helices();
        self.rm_deleted_helices();
        let strand_ids = self.design.get_all_strand_ids();
        for strand_id in strand_ids.iter() {
            let strand_opt = self.design.get_strand_points(*strand_id);
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
            let flat_strand = strand
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
                false,
            ));
        }
        let nucls_opt = self.design.get_copy_points();

        self.pasted_strands = nucls_opt
            .iter()
            .map(|nucls| {
                let color = crate::consts::CANDIDATE_COLOR;
                for nucl in nucls.iter() {
                    self.read_nucl(nucl)
                }
                let flat_strand = nucls
                    .iter()
                    .filter_map(|n| FlatNucl::from_real(n, self.id_map()))
                    .collect();
                Strand::new(color, flat_strand, vec![], 0, true)
            })
            .collect();

        for h_id in self.id_map.keys() {
            let visibility = self.design.get_visibility_helix(*h_id);
            if let Some(flat_helix) = FlatHelix::from_real(*h_id, &self.id_map) {
                self.helices[flat_helix.flat].visible = visibility.unwrap_or(false);
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
        for (h_id, h) in self.id_map.iter() {
            if !self.design.has_helix(*h_id) {
                let flat_helix = FlatHelix {
                    flat: *h,
                    real: *h_id,
                };
                to_remove.push(flat_helix);
                self.removed.insert(*h);
            }
        }
        to_remove.sort();
        if to_remove.len() > 0 {
            for h in to_remove.iter().rev() {
                self.helices.remove(h.flat);
                self.known_helices.remove(&h.real);
            }
            self.remake_id_map();
        }
    }

    pub fn get_removed_helices(&mut self) -> BTreeSet<FlatIdx> {
        std::mem::replace(&mut self.removed, BTreeSet::new())
    }

    pub fn update_helix(&mut self, helix: FlatHelix, left: isize, right: isize) {
        let helix2d = &mut self.helices[helix.flat];
        helix2d.left = left;
        helix2d.right = right;
    }

    /// Add a nucleotide to self.
    /// If the nucleotides lies on an helix that is not known from self, create a new helix.
    fn read_nucl(&mut self, nucl: &Nucl) {
        let helix = nucl.helix;
        if let Some(flat) = self.id_map.get(&helix) {
            let flat_helix = FlatHelix {
                real: helix,
                flat: *flat,
            };
            let helix2d = &mut self.helices[flat_helix.flat];
            helix2d.left = helix2d.left.min(nucl.position - 1);
            helix2d.right = helix2d.right.max(nucl.position + 1);
        } else {
            self.id_map.insert(helix, FlatIdx(self.helices.len()));
            let iso_opt = self.design.get_isometry(helix);
            let isometry = if let Some(iso) = iso_opt {
                iso
            } else {
                let iso = Isometry2::new(
                    (5. * helix as f32 - 1.) * Vec2::unit_y(),
                    Rotor2::identity(),
                );
                self.requests.lock().unwrap().set_isometry(helix, iso);
                iso
            };
            self.helices.push(Helix2d {
                id: helix,
                left: nucl.position - 1,
                right: nucl.position + 1,
                isometry,
                visible: self.design.get_visibility_helix(helix).unwrap_or(false),
            });
        }
    }

    fn update_helices(&mut self) {
        if self.known_map == Arc::as_ptr(&self.design.get_helices_map()) {
            return;
        }
        let helices = self.design.get_helices_map();
        self.known_map = Arc::as_ptr(&helices);
        for (h_id, helix) in helices.iter() {
            if self.known_helices.get(h_id) != Some(&Arc::as_ptr(helix)) {
                self.known_helices.insert(*h_id, Arc::as_ptr(helix));
                let iso_opt = self.design.get_isometry(*h_id);
                let isometry = if let Some(iso) = iso_opt {
                    iso
                } else {
                    let iso = Isometry2::new(
                        (5. * *h_id as f32 - 1.) * Vec2::unit_y(),
                        Rotor2::identity(),
                    );
                    self.requests.lock().unwrap().set_isometry(*h_id, iso);
                    iso
                };
                if !self.id_map.contains_key(h_id) {
                    self.id_map.insert(*h_id, FlatIdx(self.helices.len()));
                    self.helices.push(Helix2d {
                        id: *h_id,
                        left: -1,
                        right: 1,
                        isometry,
                        visible: self.design.get_visibility_helix(*h_id).unwrap_or(false),
                    });
                } else {
                    let flat = self.id_map.get(h_id).unwrap();
                    let helix2d = &mut self.helices[*flat];
                    helix2d.isometry = isometry;
                }
            }
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

    pub fn set_isometry(&self, helix: FlatHelix, isometry: Isometry2) {
        self.requests
            .lock()
            .unwrap()
            .set_isometry(helix.real, isometry);
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
            for helix in self.id_map.keys().filter(|h| **h != flat_helix.real) {
                self.requests
                    .lock()
                    .unwrap()
                    .set_visibility_helix(*helix, visibility)
            }
        } else {
            self.requests
                .lock()
                .unwrap()
                .set_visibility_helix(flat_helix.real, !self.helices[flat_helix.flat].visible)
        }
    }

    pub fn flip_group(&mut self, helix: FlatHelix) {
        self.requests.lock().unwrap().flip_group(helix.real)
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
        self.id_map.clear();
        for (i, h) in self.helices.iter().enumerate() {
            self.id_map.insert(h.id, FlatIdx(i));
        }
    }

    pub fn id_map(&self) -> &HashMap<usize, FlatIdx> {
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

    pub fn strand_from_xover(&self, xover: &(Nucl, Nucl), color: u32) -> Strand {
        // pretend it's a strand with two size one domains
        let flat_nucls = [xover.0, xover.0, xover.1, xover.1]
            .iter()
            .filter_map(|n| FlatNucl::from_real(n, self.id_map()))
            .collect();
        Strand::new(0, flat_nucls, vec![], 0, false).highlighted(color)
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
    /// The smallest position of a nucleotide of the helix
    pub left: isize,
    /// The largest position of a nucleotide of the the helix
    pub right: isize,
    pub isometry: Isometry2,
    pub visible: bool,
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
    pub fn from_real(real: &Torsion, id_map: &HashMap<usize, FlatIdx>) -> Self {
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
    fn get_isometry(&self, h_id: usize) -> Option<Isometry2>;
    fn can_start_builder_at(&self, nucl: Nucl) -> bool;
    fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize>;
    fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize>;
    fn helix_is_empty(&self, h_id: usize) -> Option<bool>;
    fn get_helices_map(&self) -> Arc<BTreeMap<usize, Arc<DesignHelix>>>;
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
    fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>>;
    fn get_basis_map(&self) -> Arc<HashMap<Nucl, char, RandomState>>;
    fn get_group_map(&self) -> Arc<BTreeMap<usize, bool>>;
    fn get_strand_ends(&self) -> Vec<Nucl>;
}
