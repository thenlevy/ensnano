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
pub use ensnano_design::BezierControlPoint;
use ensnano_design::{
    grid::{GridId, HelixGridPosition},
    BezierPathId, BezierVertexId,
};
use ensnano_design::{Nucl, Strand};
use std::collections::BTreeSet;

pub const PHANTOM_RANGE: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Selection {
    Nucleotide(u32, Nucl),
    Bound(u32, Nucl, Nucl),
    Xover(u32, usize),
    Design(u32),
    Strand(u32, u32),
    Helix {
        design_id: u32,
        helix_id: usize,
        segment_id: usize,
    },
    Grid(u32, GridId),
    Phantom(PhantomElement),
    BezierControlPoint {
        helix_id: usize,
        bezier_control: BezierControlPoint,
    },
    BezierVertex(BezierVertexId),
    BezierTengent {
        vertex_id: BezierVertexId,
        inward: bool,
    },
    Nothing,
}

/// The object that is foccused in the 3D scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterOfSelection {
    Nucleotide(u32, Nucl),
    Bound(u32, Nucl, Nucl),
    HelixGridPosition {
        design: u32,
        grid_id: GridId,
        x: isize,
        y: isize,
    },
    BezierControlPoint {
        helix_id: usize,
        bezier_control: BezierControlPoint,
    },
    BezierVertex {
        path_id: BezierPathId,
        vertex_id: usize,
    },
}

impl Selection {
    pub fn is_strand(&self) -> bool {
        matches!(self, Selection::Strand(_, _))
    }

    pub fn get_design(&self) -> Option<u32> {
        match self {
            Selection::Design(d) => Some(*d),
            Selection::Bound(d, _, _) => Some(*d),
            Selection::Strand(d, _) => Some(*d),
            Selection::Helix { design_id, .. } => Some(*design_id as u32),
            Selection::Nucleotide(d, _) => Some(*d),
            Selection::Grid(d, _) => Some(*d),
            Selection::Phantom(pe) => Some(pe.design_id),
            Selection::Nothing => None,
            Selection::BezierControlPoint { .. } => Some(0),
            Selection::Xover(d, _) => Some(*d),
            Selection::BezierTengent { .. } => Some(0),
            Selection::BezierVertex(_) => Some(0),
        }
    }

    pub fn info(&self) -> String {
        format!("{:?}", self)
    }

    fn get_helices_containing_self(&self, reader: &dyn DesignReader) -> Option<Vec<usize>> {
        match self {
            Self::Design(_) => None,
            Self::Grid(_, _) => None,
            Self::Helix { helix_id, .. } => Some(vec![*helix_id as usize]),
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
            Self::BezierControlPoint { .. } => None,
            Self::BezierTengent { .. } => None,
            Self::BezierVertex(_) => None,
        }
    }

    fn get_grids_containing_self(&self, reader: &dyn DesignReader) -> Option<Vec<GridId>> {
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

pub fn extract_grids(selection: &[Selection]) -> Vec<GridId> {
    selection.iter().filter_map(extract_one_grid).collect()
}

pub fn extract_only_grids(selection: &[Selection]) -> Option<Vec<GridId>> {
    selection.iter().map(extract_one_grid).collect()
}

fn extract_one_grid(selection: &Selection) -> Option<GridId> {
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

pub fn list_of_grids(selection: &[Selection]) -> Option<(usize, Vec<GridId>)> {
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
    let grids: Vec<_> = grids.into_iter().collect();
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
            // When selecting objects in the 2D view, one often selects strand extremities as well.
            // We do no want this to interfere with the copying of crossovers
            Selection::Nucleotide(_, _) => (),
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
            Selection::Helix {
                design_id: d_id,
                helix_id,
                ..
            } => {
                if *d_id != design_id {
                    return None;
                }
                helices.insert(*helix_id);
            }
            _ => return None,
        }
    }
    Some((design_id as usize, helices.into_iter().collect()))
}

pub fn list_of_free_grids(selection: &[Selection]) -> Option<Vec<usize>> {
    let mut ret = Vec::new();
    for s in selection.iter() {
        match s {
            Selection::Grid(_, GridId::FreeGrid(g_id)) => ret.push(*g_id),
            _ => return None,
        }
    }
    Some(ret)
}

pub fn list_of_bezier_vertices(selection: &[Selection]) -> Option<Vec<BezierVertexId>> {
    selection
        .iter()
        .map(|s| {
            if let Selection::BezierVertex(id) = s {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

pub fn extract_helices(selection: &[Selection]) -> Vec<usize> {
    let mut ret = Vec::new();
    for s in selection.iter() {
        if let Selection::Helix { helix_id, .. } = s {
            ret.push(*helix_id);
        }
    }
    ret.dedup();
    ret
}

pub fn extract_helices_with_controls(selection: &[Selection]) -> Vec<usize> {
    let mut ret = Vec::new();
    for s in selection.iter() {
        if let Selection::Helix { helix_id, .. } = s {
            ret.push(*helix_id);
        } else if let Selection::BezierControlPoint { helix_id, .. } = s {
            ret.push(*helix_id);
        }
    }
    ret.dedup();
    ret
}

pub fn extract_control_points(selection: &[Selection]) -> Vec<(usize, BezierControlPoint)> {
    let mut ret = Vec::new();
    for s in selection.iter() {
        if let Selection::BezierControlPoint {
            helix_id,
            bezier_control,
        } = s
        {
            ret.push((*helix_id, *bezier_control));
        }
    }
    ret.dedup();
    ret
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
) -> Option<Vec<GridId>> {
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
            Selection::Helix {
                design_id: d_id,
                helix_id,
                ..
            } => {
                if *d_id != design_id {
                    return false;
                }
                if reader.get_grid_position_of_helix(*helix_id).is_some() {
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
            ret.push(*nucl)
        }
    }
    ret
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectionMode {
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
                SelectionMode::Design => "Design",
                SelectionMode::Nucleotide => "Nucleotide",
                SelectionMode::Strand => "Strand",
                SelectionMode::Helix => "Helix",
            }
        )
    }
}

impl SelectionMode {
    pub const ALL: [SelectionMode; 4] = [
        SelectionMode::Nucleotide,
        SelectionMode::Design,
        SelectionMode::Strand,
        SelectionMode::Helix,
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
    /// User can cut strands
    Cut,
    /// User is drawing a bezier path
    EditBezierPath,
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
                ActionMode::EditBezierPath { .. } => "Edit path",
            }
        )
    }
}

impl ActionMode {
    pub fn is_build(&self) -> bool {
        matches!(self, Self::Build(_) | Self::BuildHelix { .. })
    }
}

//
// Encoding of phantom element identifier.
// The identifier is an integer of the form helix_id * max_pos_id + pos_id;
//
// helix_id is the identifer of the helix and pos_id is of the form
// position * nb_kind + element_kid
// where element_kind is
// 0 for forward nucl
// 1 for backward nucl
// 2 for forward bond
// 3 for backward bond
//
// and position is a number between -PHANTOM_RANGE and PHANTOM_RANGE that is made positive by
// adding PHANTOM_RANGE to it

/// Generate the identifier of a phantom nucleotide
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

/// Generate the identifier of a phantom bound
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    fn get_grid_position_of_helix(&self, h_id: usize) -> Option<HelixGridPosition>;
    fn get_xover_id(&self, pair: &(Nucl, Nucl)) -> Option<usize>;
    fn get_xover_with_id(&self, id: usize) -> Option<(Nucl, Nucl)>;
    fn get_strand_with_id(&self, id: usize) -> Option<&Strand>;
    fn get_helix_grid(&self, h_id: usize) -> Option<GridId>;
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
                Selection::Grid(_, GridId::FreeGrid(g_id)) => Some(Self::Grid(*g_id)),
                Selection::Grid(_, _) => None,
                Selection::Design(_) => None,
                Selection::Helix { helix_id, .. } => Some(Self::Helix(*helix_id)),
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
                Selection::BezierControlPoint { .. } => None, //TODO make DNAelement out of these
                Selection::BezierVertex(_) => None,
                Selection::BezierTengent { .. } => None,
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
            Self::Helix(h_id) => Selection::Helix {
                design_id: d_id,
                helix_id: *h_id,
                segment_id: 0,
            },
            Self::Strand(s_id) => Selection::Strand(d_id, *s_id as u32),
            Self::Grid(g_id) => Selection::Grid(d_id, GridId::FreeGrid(*g_id)),
        }
    }
}
