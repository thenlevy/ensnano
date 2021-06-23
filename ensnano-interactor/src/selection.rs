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
use ensnano_design::grid::GridPosition;
use ensnano_design::{Design, Nucl};
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

pub const PHANTOM_RANGE: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Nucleotide(u32, Nucl),
    Bound(u32, Nucl, Nucl),
    Xover(u32, usize),
    Design(u32),
    Strand(u32, u32),
    Helix(u32, u32),
    Grid(u32, usize),
    Phantom(PhantomElement),
    Nothing,
}

impl Selection {
    pub fn is_strand(&self) -> bool {
        match self {
            Selection::Strand(_, _) => true,
            _ => false,
        }
    }

    pub fn get_design(&self) -> Option<u32> {
        match self {
            Selection::Design(d) => Some(*d),
            Selection::Bound(d, _, _) => Some(*d),
            Selection::Strand(d, _) => Some(*d),
            Selection::Helix(d, _) => Some(*d),
            Selection::Nucleotide(d, _) => Some(*d),
            Selection::Grid(d, _) => Some(*d),
            Selection::Phantom(pe) => Some(pe.design_id),
            Selection::Nothing => None,
            Selection::Xover(d, _) => Some(*d),
        }
    }

    pub fn info(&self) -> String {
        format!("{:?}", self)
    }

    pub fn fetch_values(&self, reader: &dyn DesignReader) -> Vec<String> {
        match self {
            Selection::Grid(_, g_id) => {
                let b1 = reader.has_persistent_phantom(*g_id);
                let b2 = reader.has_small_spheres(*g_id);
                let mut ret: Vec<String> = vec![b1, b2]
                    .iter()
                    .map(|b| {
                        if *b {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    })
                    .collect();
                if let Some(f) = reader.get_hyperboloid_shift(*g_id) {
                    ret.push(f.to_string());
                }
                ret
            }
            Selection::Strand(_, s_id) => vec![
                format!(
                    "{:?}",
                    reader.get_strand_length(*s_id as usize).unwrap_or(0)
                ),
                format!("{:?}", reader.is_scaffold(*s_id as usize)),
                s_id.to_string(),
                reader.formatted_length_decomposition_of_strand(*s_id as usize),
            ],
            Selection::Nucleotide(_, nucl) => {
                vec![format!("{}", reader.is_anchor(*nucl))]
            }
            _ => Vec::new(),
        }
    }
}

pub(super) fn list_of_strands(
    selection: &[Selection],
    designs: &[Arc<RwLock<Design>>],
) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut strands = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Nucleotide(d_id, n) => {
                if *d_id != design_id {
                    return None;
                }
                let s_id = designs[design_id as usize]
                    .read()
                    .unwrap()
                    .get_strand_nucl(n)?;
                strands.insert(s_id);
            }
            Selection::Strand(d_id, s_id) => {
                if *d_id != design_id {
                    return None;
                }
                strands.insert(*s_id as usize);
            }
            _ => return None,
        }
    }
    let strands: Vec<usize> = strands.into_iter().collect();
    Some((design_id as usize, strands))
}

/// Convert a selection of bounds into a list of cross-overs
pub fn list_of_xovers(
    selection: &[Selection],
    reader: &dyn DesignReader,
) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut xovers = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Bound(d_id, n1, n2) => {
                if *d_id != design_id {
                    return None;
                }
                if let Some(id) = reader.get_xover_id(&(*n1, *n2)) {
                    xovers.insert(id);
                }
            }
            Selection::Xover(d_id, xover_id) => {
                if *d_id != design_id {
                    return None;
                }
                xovers.insert(*xover_id);
            }
            _ => return None,
        }
    }
    Some((design_id as usize, xovers.into_iter().collect()))
}

pub fn list_of_helices(selection: &[Selection]) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut helices = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Helix(d_id, h_id) => {
                if *d_id != design_id {
                    return None;
                }
                helices.insert(*h_id as usize);
            }
            s if s.get_design() == Some(design_id) => (),
            _ => return None,
        }
    }
    Some((design_id as usize, helices.into_iter().collect()))
}

/// Return true iff the selection is only made of helices that are not attached to a grid
pub fn all_helices_no_grid(selection: &[Selection], reader: &dyn DesignReader) -> bool {
    let design_id = selection.get(0).and_then(Selection::get_design);
    let mut nb_helices = 0;
    if design_id.is_none() {
        return false;
    }
    let design_id = design_id.unwrap();

    for s in selection.iter() {
        match s {
            Selection::Helix(d_id, h_id) => {
                if *d_id != design_id {
                    return false;
                }
                if reader.get_grid_position_of_helix(*h_id as usize).is_some() {
                    return false;
                }
                nb_helices += 1;
            }
            s if s.get_design() == Some(design_id) => (),
            _ => return false,
        }
    }
    nb_helices >= 4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectionMode {
    Grid,
    Nucleotide,
    Strand,
    Helix,
    Design,
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::Nucleotide
    }
}

impl std::fmt::Display for SelectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SelectionMode::Grid => "Grid",
                SelectionMode::Design => "Design",
                SelectionMode::Nucleotide => "Nucleotide",
                SelectionMode::Strand => "Strand",
                SelectionMode::Helix => "Helix",
            }
        )
    }
}

impl SelectionMode {
    pub const ALL: [SelectionMode; 5] = [
        SelectionMode::Nucleotide,
        SelectionMode::Design,
        SelectionMode::Strand,
        SelectionMode::Helix,
        SelectionMode::Grid,
    ];
}

/// Describe the action currently done by the user when they click left
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionMode {
    /// User is moving the camera
    Normal,
    /// User can translate objects and move the camera
    Translate,
    /// User can rotate objects and move the camera
    Rotate,
    /// User can elongate/shorten strands. The boolean attribute indicates if neighbour strands
    /// should "stick"
    Build(bool),
    /// User is creating helices with two strands starting at a given position and with a given
    /// length.
    BuildHelix { position: isize, length: usize },
    /// should "stick"
    /// Use can cut strands
    Cut,
}

impl Default for ActionMode {
    fn default() -> Self {
        ActionMode::Normal
    }
}

impl std::fmt::Display for ActionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ActionMode::Normal => "Select",
                ActionMode::Translate => "Move",
                ActionMode::Rotate => "Rotate",
                ActionMode::Build(_) => "Build",
                ActionMode::BuildHelix { .. } => "Build",
                ActionMode::Cut => "Cut",
            }
        )
    }
}

impl ActionMode {
    pub fn is_build(&self) -> bool {
        match self {
            Self::Build(_) => true,
            Self::BuildHelix { .. } => true,
            _ => false,
        }
    }
}

pub fn phantom_helix_encoder_nucl(
    design_id: u32,
    helix_id: u32,
    position: i32,
    forward: bool,
) -> u32 {
    let pos_id = (position + PHANTOM_RANGE) as u32 * 4 + if forward { 0 } else { 1 };
    let max_pos_id = (2 * PHANTOM_RANGE) as u32 * 4 + 3;
    let helix = helix_id * max_pos_id;
    assert!(helix <= 0xFF_FF_FF);
    (helix + pos_id) | (design_id << 24)
}

pub fn phantom_helix_encoder_bound(
    design_id: u32,
    helix_id: u32,
    position: i32,
    forward: bool,
) -> u32 {
    let pos_id = (position + PHANTOM_RANGE) as u32 * 4 + if forward { 2 } else { 3 };
    let max_pos_id = (2 * PHANTOM_RANGE) as u32 * 4 + 3;
    let helix = helix_id * max_pos_id;
    assert!(helix <= 0xFF_FF_FF);
    (helix + pos_id) | (design_id << 24)
}

pub fn phantom_helix_decoder(id: u32) -> PhantomElement {
    let max_pos_id = (2 * PHANTOM_RANGE) as u32 * 4 + 3;
    let design_id = id >> 24;
    let reminder = id & 0xFF_FF_FF;
    let helix_id = reminder / max_pos_id;
    let reminder = reminder % max_pos_id;
    let bound = reminder & 0b10 > 0;
    let forward = reminder % 2 == 0;
    let nucl_id = reminder / 4;
    let position = nucl_id as i32 - PHANTOM_RANGE;
    PhantomElement {
        design_id,
        helix_id,
        position,
        bound,
        forward,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhantomElement {
    pub design_id: u32,
    pub helix_id: u32,
    pub position: i32,
    pub bound: bool,
    pub forward: bool,
}

impl PhantomElement {
    pub fn to_nucl(&self) -> Nucl {
        Nucl {
            helix: self.helix_id as usize,
            position: self.position as isize,
            forward: self.forward,
        }
    }
}

pub trait DesignReader {
    fn get_grid_position_of_helix(&self, h_id: usize) -> Option<GridPosition>;
    fn get_xover_id(&self, pair: &(Nucl, Nucl)) -> Option<usize>;
    fn has_persistent_phantom(&self, g_id: usize) -> bool;
    fn has_small_spheres(&self, g_id: usize) -> bool;
    /// If g_id is the identifier of an hyperboloid grid, return the shift of the hyperboloid
    fn get_hyperboloid_shift(&self, g_id: usize) -> Option<f32>;
    fn get_strand_length(&self, s_id: usize) -> Option<usize>;
    fn is_scaffold(&self, s_id: usize) -> bool;
    fn formatted_length_decomposition_of_strand(&self, s_id: usize) -> String;
    fn is_anchor(&self, nucl: Nucl) -> bool;
}

pub trait SelectionConversion: Sized {
    fn from_selection(selection: &Selection, d_id: u32) -> Option<Self>;
    fn to_selection(&self, d_id: u32) -> Selection;
}

use ensnano_design::elements::*;
impl SelectionConversion for DnaElementKey {
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
