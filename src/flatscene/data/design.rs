use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use super::{Nucl, Strand};
use crate::design::{Design, Helix as DesignHelix, Strand as StrandDesign, StrandBuilder};
use ultraviolet::{Isometry2, Rotor2, Vec2};

pub(super) struct Design2d {
    /// The 2d helices
    helices: Vec<Helix2d>,
    /// Maps id of helices in design to location in self.helices
    id_map: HashMap<usize, usize>,
    /// the 2d strands
    strands: Vec<Strand>,
    /// A pointer to the design
    design: Arc<Mutex<Design>>,
    last_flip_other: Option<usize>,
    removed: BTreeSet<usize>,
}

impl Design2d {
    pub fn new(design: Arc<Mutex<Design>>) -> Self {
        Self {
            design,
            helices: Vec::new(),
            id_map: HashMap::new(),
            strands: Vec::new(),
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
        let strand_ids = self.design.lock().unwrap().get_all_strand_ids();
        for strand_id in strand_ids.iter() {
            let strand_opt = self.design.lock().unwrap().get_strand_points(*strand_id);
            let strand = strand_opt.unwrap();
            let color = self
                .design
                .lock()
                .unwrap()
                .get_strand_color(*strand_id)
                .unwrap_or_else(|| {
                    println!("Warning: could not find strand color, this is not normal");
                    0
                });
            for nucl in strand.iter() {
                self.read_nucl(nucl)
            }
            self.strands.push(Strand::new(color, strand, *strand_id));
        }
        self.rm_deleted_helices();
        for (h_id, h) in self.id_map.iter() {
            let visibility = self.design.lock().unwrap().get_visibility_helix(*h_id);
            self.helices[*h].visible = visibility.unwrap_or(false);
        }
    }

    pub fn suggestions(&self) -> Vec<(Nucl, Nucl)> {
        let mut suggestions = self.design.lock().unwrap().get_suggestions();
        for (n1, n2) in suggestions.iter_mut() {
            n1.helix = *self.id_map.get(&n1.helix).unwrap();
            n2.helix = *self.id_map.get(&n2.helix).unwrap();
        }
        suggestions
    }

    fn rm_deleted_helices(&mut self) {
        let mut to_remove = Vec::new();
        for (h_id, h) in self.id_map.iter() {
            if !self.design.lock().unwrap().has_helix(*h_id) {
                to_remove.push(*h);
                self.removed.insert(*h);
            }
        }
        to_remove.sort();
        if to_remove.len() > 0 {
            for h in to_remove.iter().rev() {
                self.helices.remove(*h);
            }
            self.remake_id_map();
        }
    }

    pub fn get_removed_helices(&mut self) -> BTreeSet<usize> {
        std::mem::replace(&mut self.removed, BTreeSet::new())
    }

    fn read_nucl(&mut self, nucl: &Nucl) {
        let helix = nucl.helix;
        if let Some(pos) = self.id_map.get(&helix) {
            let helix2d = &mut self.helices[*pos];
            helix2d.left = helix2d.left.min(nucl.position - 1);
            helix2d.right = helix2d.right.max(nucl.position + 1);
        } else {
            self.id_map.insert(helix, self.helices.len());
            let iso_opt = self.design.lock().unwrap().get_isometry(helix);
            let isometry = if let Some(iso) = iso_opt {
                iso
            } else {
                let iso = Isometry2::new(
                    (5. * helix as f32 - 1.) * Vec2::unit_y(),
                    Rotor2::identity(),
                );
                self.design.lock().unwrap().set_isometry(helix, iso);
                iso
            };
            self.helices.push(Helix2d {
                id: helix,
                left: nucl.position - 1,
                right: nucl.position + 1,
                isometry,
                visible: self
                    .design
                    .lock()
                    .unwrap()
                    .get_visibility_helix(helix)
                    .unwrap_or(false),
            });
        }
    }

    fn fetch_empty_helices(&mut self) {
        let mut i = 0;
        let mut grid2d = self.design.lock().unwrap().get_grid2d(i).clone();
        while let Some(grid) = grid2d {
            for h_id in grid.read().unwrap().helices().values() {
                if !self.id_map.contains_key(h_id) {
                    let iso_opt = self.design.lock().unwrap().get_isometry(*h_id);
                    let isometry = if let Some(iso) = iso_opt {
                        iso
                    } else {
                        let iso = Isometry2::new(
                            (5. * *h_id as f32 - 1.) * Vec2::unit_y(),
                            Rotor2::identity(),
                        );
                        self.design.lock().unwrap().set_isometry(*h_id, iso);
                        iso
                    };
                    self.id_map.insert(*h_id, self.helices.len());
                    self.helices.push(Helix2d {
                        id: *h_id,
                        left: -1,
                        right: 1,
                        isometry,
                        visible: self
                            .design
                            .lock()
                            .unwrap()
                            .get_visibility_helix(*h_id)
                            .unwrap_or(false),
                    });
                }
            }
            i += 1;
            grid2d = self.design.lock().unwrap().get_grid2d(i).clone();
        }
    }

    pub fn get_helices(&self) -> &Vec<Helix2d> {
        &self.helices
    }

    pub fn get_strands(&self) -> &Vec<Strand> {
        &self.strands
    }

    pub fn set_isometry(&self, h_id: usize, isometry: Isometry2) {
        let helix = self.helices[h_id].id;
        self.design.lock().unwrap().set_isometry(helix, isometry);
    }

    pub fn flip_visibility(&mut self, h_id: usize, apply_to_other: bool) {
        let helix = self.helices[h_id].id;
        if apply_to_other {
            let visibility = if self.last_flip_other == Some(h_id) {
                self.last_flip_other = None;
                self.helices[h_id].visible
            } else {
                self.last_flip_other = Some(h_id);
                !self.helices[h_id].visible
            };
            for helix in self.id_map.keys().filter(|h| **h != helix) {
                self.design
                    .lock()
                    .unwrap()
                    .set_visibility_helix(*helix, visibility)
            }
        } else {
            self.design
                .lock()
                .unwrap()
                .set_visibility_helix(helix, !self.helices[h_id].visible)
        }
    }

    pub fn flip_group(&mut self, h_id: usize) {
        let helix = self.helices[h_id].id;
        self.design.lock().unwrap().flip_group(helix)
    }

    pub fn get_builder(&self, nucl: Nucl, stick: bool) -> Option<StrandBuilder> {
        self.design.lock().unwrap().get_builder(nucl, stick)
    }

    pub fn merge_strand(&mut self, prime5: usize, prime3: usize) {
        self.design.lock().unwrap().merge_strands(prime5, prime3)
    }

    pub fn prime3_of(&self, nucl: Nucl) -> Option<usize> {
        self.design.lock().unwrap().prime3_of(nucl)
    }

    pub fn prime5_of(&self, nucl: Nucl) -> Option<usize> {
        self.design.lock().unwrap().prime5_of(nucl)
    }

    pub fn split_strand(&self, nucl: Nucl) {
        self.design.lock().unwrap().split_strand(nucl)
    }

    pub fn split_strand_forced_end(&self, nucl: Nucl, forced_end: Option<bool>) {
        self.design
            .lock()
            .unwrap()
            .split_strand_forced_end(nucl, forced_end)
    }

    pub fn rm_strand(&self, nucl: Nucl) {
        self.design.lock().unwrap().rm_strand(nucl)
    }

    pub fn can_delete_helix(&self, helix: usize) -> bool {
        let real_helix = self.helices[helix].id;
        self.design.lock().unwrap().helix_is_empty(real_helix)
    }

    pub fn get_raw_helix(&self, h_id: usize) -> Option<DesignHelix> {
        self.design.lock().unwrap().get_raw_helix(h_id)
    }

    pub fn get_strand(&self, s_id: usize) -> Option<StrandDesign> {
        self.design.lock().unwrap().get_raw_strand(s_id)
    }

    fn remake_id_map(&mut self) {
        self.id_map.clear();
        for (i, h) in self.helices.iter().enumerate() {
            self.id_map.insert(h.id, i);
        }
    }

    pub fn id_map(&self) -> &HashMap<usize, usize> {
        &self.id_map
    }

    pub fn is_xover_end(&self, nucl: &Nucl) -> Option<bool> {
        self.design.lock().unwrap().is_xover_end(nucl)
    }

    pub fn has_nucl(&self, nucl: Nucl) -> bool {
        self.design
            .lock()
            .unwrap()
            .get_identifier_nucl(nucl)
            .is_some()
    }

    pub fn get_strand_id(&self, nucl: Nucl) -> Option<usize> {
        self.design.lock().unwrap().get_strand_nucl(&nucl)
    }

    pub fn get_dist(&self, nucl1: Nucl, nucl2: Nucl) -> Option<f32> {
        use crate::design::Referential;
        let pos1 = self
            .design
            .lock()
            .unwrap()
            .get_helix_nucl(nucl1, Referential::Model, false)?;
        let pos2 = self
            .design
            .lock()
            .unwrap()
            .get_helix_nucl(nucl2, Referential::Model, false)?;
        Some((pos1 - pos2).mag())
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
