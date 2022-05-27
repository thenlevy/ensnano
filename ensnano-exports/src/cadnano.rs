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

use ensnano_design::{Collection, Design, Domain, Strands};

mod parity_graph;

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

fn get_grid_type(design: &Design) -> Result<GridType, CadnanoError> {
    let mut design = design.clone();
    let mut ret: Option<GridType> = None;

    let grids = design.get_updated_grid_data();
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GridType {
    Square,
    HonneyComb,
}
