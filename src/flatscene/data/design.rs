use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Nucl, Strand};
use crate::design::{Design, StrandBuilder};
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
}

impl Design2d {
    pub fn new(design: Arc<Mutex<Design>>) -> Self {
        Self {
            design,
            helices: Vec::new(),
            id_map: HashMap::new(),
            strands: Vec::new(),
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

    pub fn rm_strand(&self, nucl: Nucl) {
        self.design.lock().unwrap().rm_strand(nucl)
    }

    pub fn rm_helix(&mut self, helix: usize) {
        let real_helix = self.helices[helix].id;
        self.design.lock().unwrap().rm_helix(real_helix);
        self.helices.remove(helix);
        self.remake_id_map();
    }

    pub fn can_delete_helix(&self, helix: usize) -> bool {
        let real_helix = self.helices[helix].id;
        self.design.lock().unwrap().helix_is_empty(real_helix)
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
}
