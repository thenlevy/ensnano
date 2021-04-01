use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

use super::super::{FlatHelix, FlatIdx, FlatNucl};
use super::{Flat, HelixVec, Nucl, Strand};
use crate::design::{Design, Helix as DesignHelix, Strand as StrandDesign, StrandBuilder, Torsion};
use ultraviolet::{Isometry2, Rotor2, Vec2};

pub(super) struct Design2d {
    /// The 2d helices
    helices: HelixVec<Helix2d>,
    /// Maps id of helices in design to location in self.helices
    id_map: HashMap<usize, FlatIdx>,
    /// the 2d strands
    strands: Vec<Strand>,
    /// A pointer to the design
    design: Arc<RwLock<Design>>,
    /// The strand being pasted,
    pasted_strands: Vec<Strand>,
    last_flip_other: Option<FlatHelix>,
    removed: BTreeSet<FlatIdx>,
}

impl Design2d {
    pub fn new(design: Arc<RwLock<Design>>) -> Self {
        Self {
            design,
            helices: HelixVec::new(),
            id_map: HashMap::new(),
            strands: Vec::new(),
            pasted_strands: Vec::new(),
            last_flip_other: None,
            removed: BTreeSet::new(),
        }
    }

    /// Re-read the design and update the 2d data accordingly
    pub fn update(&mut self) {
        // At the moment we rebuild the strands from scratch. If needed, this might be an optimisation
        // target
        self.strands = Vec::new();
        self.fetch_empty_helices();
        self.rm_deleted_helices();
        let strand_ids = self.design.read().unwrap().get_all_strand_ids();
        for strand_id in strand_ids.iter() {
            let strand_opt = self.design.read().unwrap().get_strand_points(*strand_id);
            let strand = strand_opt.unwrap();
            let color = self
                .design
                .read()
                .unwrap()
                .get_strand_color(*strand_id)
                .unwrap_or_else(|| {
                    println!("Warning: could not find strand color, this is not normal");
                    0
                });
            for nucl in strand.iter() {
                self.read_nucl(nucl)
            }
            let flat_strand = strand
                .iter()
                .map(|n| FlatNucl::from_real(n, self.id_map()))
                .collect();
            let insertions = self
                .design
                .read()
                .unwrap()
                .get_insertions(*strand_id)
                .unwrap_or_default();
            let insertions = insertions
                .iter()
                .map(|n| FlatNucl::from_real(n, self.id_map()))
                .collect::<Vec<_>>();
            self.strands.push(Strand::new(
                color,
                flat_strand,
                insertions,
                *strand_id,
                false,
            ));
        }
        let nucls_opt = self.design.read().unwrap().get_copy_points();

        self.pasted_strands = nucls_opt
            .iter()
            .map(|nucls| {
                let color = 0xCC_30_30_30;
                for nucl in nucls.iter() {
                    self.read_nucl(nucl)
                }
                let flat_strand = nucls
                    .iter()
                    .map(|n| FlatNucl::from_real(n, self.id_map()))
                    .collect();
                Strand::new(color, flat_strand, vec![], 0, false)
            })
            .collect();

        for h_id in self.id_map.keys() {
            let visibility = self.design.read().unwrap().get_visibility_helix(*h_id);
            let flat_helix = FlatHelix::from_real(*h_id, &self.id_map);
            self.helices[flat_helix.flat].visible = visibility.unwrap_or(false);
        }
    }

    pub fn suggestions(&self) -> Vec<(FlatNucl, FlatNucl)> {
        let suggestions = self.design.read().unwrap().get_suggestions();
        suggestions
            .iter()
            .map(|(n1, n2)| {
                (
                    FlatNucl::from_real(n1, &self.id_map),
                    FlatNucl::from_real(n2, &self.id_map),
                )
            })
            .collect()
    }

    fn rm_deleted_helices(&mut self) {
        let mut to_remove = Vec::new();
        for (h_id, h) in self.id_map.iter() {
            if !self.design.read().unwrap().has_helix(*h_id) {
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
            let iso_opt = self.design.read().unwrap().get_isometry(helix);
            let isometry = if let Some(iso) = iso_opt {
                iso
            } else {
                let iso = Isometry2::new(
                    (5. * helix as f32 - 1.) * Vec2::unit_y(),
                    Rotor2::identity(),
                );
                self.design.read().unwrap().set_isometry(helix, iso);
                iso
            };
            self.helices.push(Helix2d {
                id: helix,
                left: nucl.position - 1,
                right: nucl.position + 1,
                isometry,
                visible: self
                    .design
                    .read()
                    .unwrap()
                    .get_visibility_helix(helix)
                    .unwrap_or(false),
            });
        }
    }

    fn fetch_empty_helices(&mut self) {
        let mut i = 0;
        let mut grid2d = self.design.read().unwrap().get_grid2d(i).clone();
        while let Some(grid) = grid2d {
            for h_id in grid.read().unwrap().helices().values() {
                if !self.id_map.contains_key(h_id) {
                    let iso_opt = self.design.read().unwrap().get_isometry(*h_id);
                    let isometry = if let Some(iso) = iso_opt {
                        iso
                    } else {
                        let iso = Isometry2::new(
                            (5. * *h_id as f32 - 1.) * Vec2::unit_y(),
                            Rotor2::identity(),
                        );
                        self.design.read().unwrap().set_isometry(*h_id, iso);
                        iso
                    };
                    self.id_map.insert(*h_id, FlatIdx(self.helices.len()));
                    self.helices.push(Helix2d {
                        id: *h_id,
                        left: -1,
                        right: 1,
                        isometry,
                        visible: self
                            .design
                            .read()
                            .unwrap()
                            .get_visibility_helix(*h_id)
                            .unwrap_or(false),
                    });
                }
            }
            i += 1;
            grid2d = self.design.read().unwrap().get_grid2d(i).clone();
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
        self.design
            .read()
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
                self.design
                    .write()
                    .unwrap()
                    .set_visibility_helix(*helix, visibility)
            }
        } else {
            self.design
                .write()
                .unwrap()
                .set_visibility_helix(flat_helix.real, !self.helices[flat_helix.flat].visible)
        }
    }

    pub fn flip_group(&mut self, helix: FlatHelix) {
        self.design.write().unwrap().flip_group(helix.real)
    }

    pub fn get_builder(&self, nucl: Nucl, stick: bool) -> Option<StrandBuilder> {
        self.design.read().unwrap().get_builder(nucl, stick)
    }

    pub fn prime3_of(&self, nucl: Nucl) -> Option<usize> {
        self.design.read().unwrap().prime3_of(nucl)
    }

    pub fn prime5_of(&self, nucl: Nucl) -> Option<usize> {
        self.design.read().unwrap().prime5_of(nucl)
    }

    pub fn can_delete_helix(&self, helix: FlatHelix) -> bool {
        self.design.read().unwrap().helix_is_empty(helix.real)
    }

    pub fn get_raw_helix(&self, helix: FlatHelix) -> Option<DesignHelix> {
        self.design.read().unwrap().get_raw_helix(helix.real)
    }

    pub fn get_strand(&self, s_id: usize) -> Option<StrandDesign> {
        self.design.read().unwrap().get_raw_strand(s_id)
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
        self.design.read().unwrap().is_xover_end(nucl).to_opt()
    }

    pub fn has_nucl(&self, nucl: Nucl) -> bool {
        self.design
            .read()
            .unwrap()
            .get_identifier_nucl(&nucl)
            .is_some()
    }

    pub fn get_strand_id(&self, nucl: Nucl) -> Option<usize> {
        self.design.read().unwrap().get_strand_nucl(&nucl)
    }

    pub fn get_dist(&self, nucl1: Nucl, nucl2: Nucl) -> Option<f32> {
        use crate::design::Referential;
        let pos1 = self
            .design
            .read()
            .unwrap()
            .get_helix_nucl(nucl1, Referential::Model, false)?;
        let pos2 = self
            .design
            .read()
            .unwrap()
            .get_helix_nucl(nucl2, Referential::Model, false)?;
        Some((pos1 - pos2).mag())
    }

    pub fn get_torsions(&self) -> HashMap<(FlatNucl, FlatNucl), FlatTorsion> {
        let torsions = self.design.read().unwrap().get_torsions();
        let conversion = |((n1, n2), k): (&(Nucl, Nucl), &Torsion)| {
            let flat_1 = FlatNucl::from_real(n1, &self.id_map);
            let flat_2 = FlatNucl::from_real(n2, &self.id_map);
            let torsion = FlatTorsion::from_real(k, &self.id_map);
            ((flat_1, flat_2), torsion)
        };
        torsions.iter().map(conversion).collect()
    }

    pub fn get_xovers_list(&self) -> Vec<(FlatNucl, FlatNucl)> {
        let xovers = self.design.read().unwrap().get_xovers_list();
        xovers
            .iter()
            .map(|(n1, n2)| {
                let flat_1 = FlatNucl::from_real(n1, &self.id_map);
                let flat_2 = FlatNucl::from_real(n2, &self.id_map);
                (flat_1, flat_2)
            })
            .collect()
    }

    pub fn strand_from_xover(&self, xover: &(Nucl, Nucl), color: u32) -> Strand {
        let flat_nucls = [xover.0, xover.1]
            .iter()
            .map(|n| FlatNucl::from_real(n, self.id_map()))
            .collect();
        Strand::new(0, flat_nucls, vec![], 0, false).highlighted(color)
    }

    pub fn get_nucl_id(&self, nucl: Nucl) -> Option<u32> {
        self.design.read().unwrap().get_identifier_nucl(&nucl)
    }

    pub fn get_strand_from_eid(&self, element_id: u32) -> Option<usize> {
        self.design.read().unwrap().get_strand(element_id)
    }

    pub fn get_helix_from_eid(&self, element_id: u32) -> Option<usize> {
        self.design.read().unwrap().get_helix(element_id)
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
            friend: real.friend.map(|(n1, n2)| {
                (
                    FlatNucl::from_real(&n1, id_map),
                    FlatNucl::from_real(&n2, id_map),
                )
            }),
        }
    }
}
