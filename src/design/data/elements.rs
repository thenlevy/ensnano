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
use super::Nucl;
use crate::mediator::Selection;
pub use ensnano_design::elements::*;

pub trait SelectionConverstion: Sized {
    fn from_selection(selection: &Selection, d_id: u32) -> Option<Self>;
    fn to_selection(&self, d_id: u32) -> Selection;
}

impl SelectionConverstion for DnaElementKey {
    fn from_selection(selection: &Selection, d_id: u32) -> Option<Self> {
        if selection.get_design() == Some(d_id) {
            match selection {
                Selection::Grid(_, g_id) => Some(Self::Grid(*g_id)),
                Selection::Design(_) => None,
                Selection::Helix(_, h_id) => Some(Self::Helix(*h_id as usize)),
                Selection::Strand(_, s_id) => Some(Self::Strand(*s_id as usize)),
                Selection::Nucleotide(_, nucl) => Some(Self::Nucleotide {
                    helix: nucl.helix,
                    position: nucl.position,
                    forward: nucl.forward,
                }),
                Selection::Bound(_, _, _) => None,
                Selection::Xover(_, xover_id) => Some(Self::CrossOver {
                    xover_id: *xover_id,
                }),
                Selection::Phantom(pe) => {
                    if pe.bound {
                        None
                    } else {
                        let nucl = pe.to_nucl();
                        Some(Self::Nucleotide {
                            helix: nucl.helix,
                            position: nucl.position,
                            forward: nucl.forward,
                        })
                    }
                }
                Selection::Nothing => None,
            }
        } else {
            None
        }
    }

    fn to_selection(&self, d_id: u32) -> Selection {
        match self {
            Self::Nucleotide {
                helix,
                position,
                forward,
            } => Selection::Nucleotide(
                d_id,
                Nucl {
                    helix: *helix,
                    position: *position,
                    forward: *forward,
                },
            ),
            Self::CrossOver { xover_id } => Selection::Xover(d_id, *xover_id),
            Self::Helix(h_id) => Selection::Helix(d_id, *h_id as u32),
            Self::Strand(s_id) => Selection::Strand(d_id, *s_id as u32),
            Self::Grid(g_id) => Selection::Grid(d_id, *g_id),
        }
    }
}
