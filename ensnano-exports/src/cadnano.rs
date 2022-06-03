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

//! A good description of the cadnano file format can be found at
//! https://github.com/UC-Davis-molecular-computing/scadnano-python-package/blob/main/misc/cadnano-format-specs/v2.txt

use std::collections::HashMap;

use ensnano_design::{grid::GridData, Collection, Design, Domain, Nucl};

mod parity_graph;

pub fn cadnano_export(design: &Design) -> Result<String, CadnanoError> {
    let mut exporter = init_cadnano_exporter(design)?;

    for s in design.strands.values() {
        let mut strand = exporter.new_strand();
        for d in s.domains.iter() {
            if let Domain::HelixDomain(d) = d {
                for pos in d.iter() {
                    let nucl = Nucl {
                        helix: d.helix,
                        position: pos,
                        forward: d.forward,
                    };

                    strand.add_nucl(nucl)?;
                }
            }
        }
        strand.finish(s.cyclic, s.color)?;
    }

    let mut helices: Vec<_> = exporter.helices.values().map(|h| h.clone()).collect();
    helices.sort_by_key(|h| h.num);

    serde_json::to_string(&ExportedCadnano {
        name: String::from("ENSnano exported design"),
        helices,
    })
    .map_err(|e| CadnanoError::SerdeError(e))
}

fn get_ensnano_bonds(design: &Design) -> EnsnanoBonds {
    let mut min_nt_pos = 0;
    let mut max_nt_pos = 0;
    let mut max_helix_idx = 0;

    for s in design.strands.values() {
        for d in s.domains.iter() {
            if let Domain::HelixDomain(d) = d {
                min_nt_pos = min_nt_pos.min(d.start);
                max_nt_pos = max_nt_pos.max(d.end - 1);
                max_helix_idx = max_helix_idx.max(d.helix);
            }
        }
    }

    EnsnanoBonds {
        min_nt_pos,
        max_nt_pos,
        max_helix_idx,
    }
}

fn get_grid_type(grids: &GridData) -> Result<GridType, CadnanoError> {
    let mut ret: Option<GridType> = None;

    for g in grids.source_free_grids.values() {
        match g.grid_type {
            ensnano_design::grid::GridTypeDescr::Square { .. } => {
                if ret == Some(GridType::HonneyComb) {
                    return Err(CadnanoError::NonHomogeneousGridTypes);
                } else {
                    ret = Some(GridType::Square)
                }
            }
            ensnano_design::grid::GridTypeDescr::Honeycomb { .. } => {
                if ret == Some(GridType::Square) {
                    return Err(CadnanoError::NonHomogeneousGridTypes);
                } else {
                    ret = Some(GridType::HonneyComb)
                }
            }
            t => return Err(CadnanoError::UnhandledGridType(t)),
        }
    }

    Ok(ret.unwrap_or(GridType::Square))
}

fn get_cadnano_bonds(design: &Design, grids: &GridData) -> Result<CadnanoBounds, CadnanoError> {
    let ensnano_bonds = get_ensnano_bonds(design);
    let grid_type = get_grid_type(grids)?;
    Ok(ensnano_bonds.convert_to_cadnanobounds(grid_type))
}

fn init_cadnano_exporter(design: &Design) -> Result<CadnanoExporter, CadnanoError> {
    let mut design_clone = design.clone();
    let grids = design_clone.get_updated_grid_data();
    let bonds = get_cadnano_bonds(&design, &grids)?;
    let parity_helix = parity_graph::get_parity(&design, bonds.max_helix_idx)?;

    let mut shift_x = 0;

    let mut even = 0;
    let mut odd = 1;

    let mut cadnano_helices = HashMap::with_capacity(bonds.max_helix_idx);

    for g_id in grids.grids.keys() {
        let mut shift_y = 0;

        let used_coordinates = grids.get_used_coordinates_on_grid(*g_id);
        let min_x = used_coordinates.iter().map(|(x, _)| *x).min().unwrap_or(0);
        let max_x = used_coordinates.iter().map(|(x, _)| *x).max().unwrap_or(0);
        let min_y = used_coordinates.iter().map(|(_, y)| *y).min().unwrap_or(0);

        let coordinates_with_helice = {
            let mut v = grids.get_helices_grid_key_coord(*g_id);
            v.sort_unstable(); // sorted by lexicographic order on (x, y)
            v
        };

        for ((x, y), h) in coordinates_with_helice.iter() {
            let mut candidate = (shift_x - min_x + x, shift_y - min_y + y);
            if parity(candidate) != parity_helix[*h] {
                shift_y += 1;
                candidate.1 += 1;
            }

            let num = if parity_helix[*h] {
                let ret = even;
                even += 2;
                ret
            } else {
                let ret = odd;
                odd += 2;
                ret
            };

            let cadnano_helix = CadnanoHelix::new(num, candidate, bonds.max_nt_pos);
            cadnano_helices.insert(*h, cadnano_helix);
        }
        shift_x += max_x + 1
    }

    Ok(CadnanoExporter {
        bonds,
        helices: cadnano_helices,
    })
}

fn parity(t: (isize, isize)) -> bool {
    (t.0 + t.1) % 2 == 0
}

struct EnsnanoBonds {
    min_nt_pos: isize,
    max_nt_pos: isize,
    max_helix_idx: usize,
}

struct CadnanoBounds {
    shift: isize,
    max_nt_pos: usize,
    max_helix_idx: usize,
}

impl EnsnanoBonds {
    fn convert_to_cadnanobounds(self, grid_type: GridType) -> CadnanoBounds {
        let max_nt_pos = {
            let value = self.max_nt_pos - self.min_nt_pos;
            match grid_type {
                GridType::Square => ((1 + value / 21) * 21) as usize,
                _ => ((1 + value / 32) * 32) as usize,
            }
        };

        CadnanoBounds {
            shift: self.min_nt_pos,
            max_nt_pos,
            max_helix_idx: self.max_helix_idx,
        }
    }
}

#[derive(Debug)]
pub enum CadnanoError {
    Not2Colorable,
    NonHomogeneousGridTypes,
    UnhandledGridType(ensnano_design::grid::GridTypeDescr),
    ImpossibleBond,
    HelixNotFound(usize),
    SerdeError(serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GridType {
    Square,
    HonneyComb,
}

struct CadnanoExporter {
    helices: HashMap<usize, CadnanoHelix>,
    bonds: CadnanoBounds,
}

impl CadnanoExporter {
    fn make_bond(&mut self, prime5: Nucl, prime3: Nucl) -> Result<(), CadnanoError> {
        let helix_prime5 = self
            .helices
            .get(&prime5.helix)
            .ok_or(CadnanoError::HelixNotFound(prime5.helix))?;
        let helix_prime3 = self
            .helices
            .get(&prime3.helix)
            .ok_or(CadnanoError::HelixNotFound(prime3.helix))?;

        let num_prime5 = helix_prime5.num;
        let num_prime3 = helix_prime3.num;

        if (num_prime5 % 2 == num_prime3 % 2) != (prime5.forward == prime3.forward) {
            return Err(CadnanoError::ImpossibleBond);
        } else {
            let helix_prime5 = self
                .helices
                .get_mut(&prime5.helix)
                .ok_or(CadnanoError::HelixNotFound(prime5.helix))?;

            let cadnano_prime5_nucl = if (helix_prime5.num % 2 == 0) == prime5.forward {
                &mut helix_prime5.scaf[(prime5.position - self.bonds.shift) as usize]
            } else {
                &mut helix_prime5.stap[(prime5.position - self.bonds.shift) as usize]
            };
            cadnano_prime5_nucl.2 = num_prime3;
            cadnano_prime5_nucl.3 = prime3.position - self.bonds.shift;

            let helix_prime3 = self
                .helices
                .get_mut(&prime3.helix)
                .ok_or(CadnanoError::HelixNotFound(prime3.helix))?;

            let cadnano_prime3_nucl = if (helix_prime3.num % 2 == 0) == prime3.forward {
                &mut helix_prime3.scaf[(prime3.position - self.bonds.shift) as usize]
            } else {
                &mut helix_prime3.stap[(prime3.position - self.bonds.shift) as usize]
            };
            cadnano_prime3_nucl.0 = num_prime5;
            cadnano_prime3_nucl.1 = prime5.position - self.bonds.shift;
        }
        Ok(())
    }

    fn set_staple_color(&mut self, prime5_nucl: Nucl, color: u32) {
        // this method will never fail and simply do nothing if it cannot succeed
        if let Some(helix) = self.helices.get_mut(&prime5_nucl.helix) {
            if (helix.num % 2 == 0) != prime5_nucl.forward {
                helix.stap_colors.push((
                    prime5_nucl.position - self.bonds.shift,
                    color % 0x01_00_00_00,
                ))
            }
        }
    }

    fn new_strand<'a>(&'a mut self) -> CadnanoStrand<'a> {
        CadnanoStrand {
            exporter: self,
            previous_nucl: None,
            first_nucl: None,
        }
    }
}

struct CadnanoStrand<'a> {
    exporter: &'a mut CadnanoExporter,
    previous_nucl: Option<Nucl>,
    first_nucl: Option<Nucl>,
}

impl CadnanoStrand<'_> {
    fn add_nucl(&mut self, nucl: Nucl) -> Result<(), CadnanoError> {
        self.first_nucl = self.first_nucl.or(Some(nucl));

        if let Some(prime5) = self.previous_nucl.take() {
            self.exporter.make_bond(prime5, nucl)?;
        }
        self.previous_nucl = Some(nucl);
        Ok(())
    }

    fn finish(self, cyclic: bool, color: u32) -> Result<(), CadnanoError> {
        if cyclic {
            if let Some((prime5, prime3)) = self.previous_nucl.zip(self.first_nucl) {
                if prime5 != prime3 {
                    self.exporter.make_bond(prime5, prime3)?;
                }
            }
        }

        if let Some(nucl) = self.first_nucl {
            self.exporter.set_staple_color(nucl, color)
        }

        Ok(())
    }
}

use serde::Serialize;

const NO_CADNANO_NUCL: (isize, isize, isize, isize) = (-1, -1, -1, -1);

#[derive(Serialize, Clone)]
struct CadnanoHelix {
    col: isize,

    #[serde(rename = "loop")]
    loop_: Vec<isize>,
    num: isize,
    row: isize,
    scaf: Vec<(isize, isize, isize, isize)>,
    skip: Vec<isize>,
    stap: Vec<(isize, isize, isize, isize)>,
    /// Each entry is a pair `(prime5_pos, color)` where `prime5_pos` is the position of
    /// the 5' end and color is an u32 of the form 0x00_RR_GG_BB
    stap_colors: Vec<(isize, u32)>,
    /// Unused, can be left empty
    #[serde(rename = "scafLoop")]
    scaf_loop: Vec<isize>,
    /// Unused, can be left empty
    #[serde(rename = "stapLoop")]
    stap_loop: Vec<isize>,
}

impl CadnanoHelix {
    fn new(num: isize, coord: (isize, isize), width: usize) -> Self {
        Self {
            num,
            col: coord.0,
            row: coord.1,
            scaf: vec![NO_CADNANO_NUCL; width],
            skip: vec![0; width],
            stap: vec![NO_CADNANO_NUCL; width],
            stap_colors: Vec::with_capacity(width),
            loop_: vec![0; width],
            // unused
            scaf_loop: vec![],
            // unused
            stap_loop: vec![],
        }
    }
}

#[derive(Serialize)]
struct ExportedCadnano {
    name: String,
    #[serde(rename = "vstrands")]
    helices: Vec<CadnanoHelix>,
}
