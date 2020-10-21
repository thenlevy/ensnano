use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Nucl, Strand};
use crate::design::Design;

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
        let mut strand_id = 0;
        let mut strand_opt = self.design.lock().unwrap().get_strand_points(strand_id);
        while strand_opt.is_some() {
            let mut strand = strand_opt.unwrap();
            let color = self
                .design
                .lock()
                .unwrap()
                .get_strand_color(strand_id)
                .unwrap_or_else(|| {
                    println!("Warning: could not find strand color, this is not normal");
                    0
                });
            for nucl in strand.iter() {
                self.read_nucl(nucl)
            }
            for nucl in strand.iter_mut() {
                nucl.helix = self.id_map[&nucl.helix]
            }
            self.strands.push(Strand::new(color, strand));
            strand_id += 1;
            strand_opt = self.design.lock().unwrap().get_strand_points(strand_id);
        }
    }

    fn read_nucl(&mut self, nucl: &Nucl) {
        let helix = nucl.helix;
        if let Some(pos) = self.id_map.get(&helix) {
            let helix2d = &mut self.helices[*pos];
            helix2d.left = helix2d.left.min(nucl.position);
            helix2d.right = helix2d.right.max(nucl.position);
        } else {
            self.id_map.insert(helix, self.helices.len());
            self.helices.push(Helix2d {
                id: helix,
                left: nucl.position,
                right: nucl.position,
            });
        }
    }

    pub fn get_helices(&self) -> &Vec<Helix2d> {
        &self.helices
    }

    pub fn get_strands(&self) -> &Vec<Strand> {
        &self.strands
    }
}

/// Store the informations needed to represent an helix from the design
pub struct Helix2d {
    /// The id of the helix within the design
    id: usize,
    /// The smallest position of a nucleotide of the helix
    pub left: isize,
    /// The largest position of a nucleotide of the the helix
    pub right: isize,
}
