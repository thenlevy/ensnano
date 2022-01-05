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
use ensnano_design::{Nucl, Strand};
use std::collections::BTreeSet;

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

/// The object that is foccused in the 3D scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterOfSelection {
    Nucleotide(u32, Nucl),
    Bound(u32, Nucl, Nucl),
    GridPosition {
        design: u32,
        grid_id: usize,
        x: isize,
        y: isize,
    },
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

    /*
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
    }*/

    fn get_helices_containing_self(&self, reader: &dyn DesignReader) -> Option<Vec<usize>> {
        match self {
            Self::Design(_) => None,
            Self::Grid(_, _) => None,
            Self::Helix(_, h_id) => Some(vec![*h_id as usize]),
            Self::Nucleotide(_, nucl) => Some(vec![nucl.helix]),
            Self::Phantom(pe) => Some(vec![pe.to_nucl().helix]),
            Self::Strand(_, s_id) => {
                let strand = reader.get_strand_with_id(*s_id as usize)?;
                Some(strand.domains.iter().filter_map(|d| d.helix()).collect())
            }
            Self::Xover(_, xover_id) => {
                let (n1, n2) = reader.get_xover_with_id(*xover_id)?;
                Some(vec![n1.helix, n2.helix])
            }
            Self::Bound(_, n1, n2) => Some(vec![n1.helix, n2.helix]),
            Self::Nothing => Some(vec![]),
        }
    }

    fn get_grids_containing_self(&self, reader: &dyn DesignReader) -> Option<Vec<usize>> {
        if let Self::Grid(_, g_id) = self {
            Some(vec![*g_id])
        } else {
            let helices = self.get_helices_containing_self(reader)?;
            Some(
                helices
                    .iter()
                    .filter_map(|h| reader.get_helix_grid(*h))
                    .collect(),
            )
        }
    }
}

pub fn extract_nucls_and_xover_ends(
    selection: &[Selection],
    reader: &dyn DesignReader,
) -> Vec<Nucl> {
    let mut ret = Vec::with_capacity(2 * selection.len());
    for s in selection.iter() {
        match s {
            Selection::Nucleotide(_, n) => ret.push(*n),
            Selection::Bound(_, n1, n2) => {
                ret.push(*n1);
                ret.push(*n2);
            }
            Selection::Xover(_, xover_id) => {
                if let Some((n1, n2)) = reader.get_xover_with_id(*xover_id) {
                    ret.push(n1);
                    ret.push(n2);
                } else {
                    log::error!("No xover with id {}", xover_id);
                }
            }
            Selection::Strand(_, s_id) => {
                if let Some(ends) = reader.get_domain_ends(*s_id as usize) {
                    ret.extend(ends);
                } else {
                    log::error!("No strand with id {}", s_id);
                }
            }
            _ => (),
        }
    }
    ret.dedup();
    ret
}

pub fn extract_strands_from_selection(selection: &[Selection]) -> Vec<usize> {
    selection.iter().filter_map(extract_one_strand).collect()
}

fn extract_one_strand(selection: &Selection) -> Option<usize> {
    if let Selection::Strand(_, s_id) = selection {
        Some(*s_id as usize)
    } else {
        None
    }
}

pub fn extract_grids(selection: &[Selection]) -> Vec<usize> {
    selection.iter().filter_map(extract_one_grid).collect()
}

fn extract_one_grid(selection: &Selection) -> Option<usize> {
    if let Selection::Grid(_, g_id) = selection {
        Some(*g_id)
    } else {
        None
    }
}

pub fn list_of_strands(selection: &[Selection]) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut strands = BTreeSet::new();
    for s in selection.iter() {
        match s {
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

pub fn list_of_grids(selection: &[Selection]) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut grids = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Grid(d_id, g_id) if *d_id == design_id => {
                grids.insert(*g_id);
            }
            _ => return None,
        }
    }
    let grids: Vec<usize> = grids.into_iter().collect();
    Some((design_id as usize, grids))
}

/// Convert a selection of bounds into a list of cross-overs
pub fn list_of_xover_ids(
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

/// Convert a selection of bounds into a list of cross-overs
pub fn list_of_xover_as_nucl_pairs(
    selection: &[Selection],
    reader: &dyn DesignReader,
) -> Option<(usize, Vec<(Nucl, Nucl)>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut xovers = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Bound(d_id, n1, n2) => {
                if *d_id != design_id {
                    return None;
                }
                if reader.get_xover_id(&(*n1, *n2)).is_none() {
                    xovers.insert((*n1, *n2));
                }
            }
            Selection::Xover(d_id, xover_id) => {
                if *d_id != design_id {
                    return None;
                }
                if let Some(pair) = reader.get_xover_with_id(*xover_id) {
                    xovers.insert(pair);
                } else {
                    return None;
                }
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

pub fn set_of_helices_containing_selection(
    selection: &[Selection],
    reader: &dyn DesignReader,
) -> Option<Vec<usize>> {
    let mut ret = Vec::new();
    for s in selection {
        let helices = s.get_helices_containing_self(reader)?;
        ret.extend_from_slice(helices.as_slice());
    }
    ret.sort();
    ret.dedup();
    Some(ret)
}

pub fn set_of_grids_containing_selection(
    selection: &[Selection],
    reader: &dyn DesignReader,
) -> Option<Vec<usize>> {
    let mut ret = Vec::new();
    for s in selection {
        let grids = s.get_grids_containing_self(reader)?;
        ret.extend_from_slice(grids.as_slice());
    }
    ret.sort();
    ret.dedup();
    Some(ret)
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

/// Extract all the elements of the form Selection::Nucl(_) from a slice of selection
pub fn extract_nucls_from_selection(selection: &[Selection]) -> Vec<Nucl> {
    let mut ret = vec![];
    for s in selection.iter() {
        if let Selection::Nucleotide(_, nucl) = s {
            ret.push(nucl.clone())
        }
    }
    ret
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
    fn get_xover_with_id(&self, id: usize) -> Option<(Nucl, Nucl)>;
    fn get_strand_with_id(&self, id: usize) -> Option<&Strand>;
    fn get_helix_grid(&self, h_id: usize) -> Option<usize>;
    fn get_domain_ends(&self, s_id: usize) -> Option<Vec<Nucl>>;
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
