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
use crate::design::{Design, Nucl};
use crate::utils::PhantomElement;
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

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

    pub fn fetch_values(&self, design: Arc<RwLock<Design>>) -> Vec<String> {
        match self {
            Selection::Grid(_, g_id) => {
                let b1 = design.read().unwrap().has_persistent_phantom(g_id);
                let b2 = design.read().unwrap().has_small_spheres(g_id);
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
                if let Some(f) = design.read().unwrap().get_shift(*g_id) {
                    ret.push(f.to_string());
                }
                ret
            }
            Selection::Strand(_, s_id) => vec![
                format!(
                    "{:?}",
                    design
                        .read()
                        .unwrap()
                        .get_strand_length(*s_id as usize)
                        .unwrap_or(0)
                ),
                format!("{:?}", design.read().unwrap().is_scaffold(*s_id as usize)),
                s_id.to_string(),
                design.read().unwrap().decompose_length(*s_id as usize),
            ],
            Selection::Nucleotide(_, nucl) => {
                vec![format!("{}", design.read().unwrap().is_anchor(*nucl))]
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

pub(super) fn list_of_xovers(
    selection: &[Selection],
    designs: &[Arc<RwLock<Design>>],
) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let design = designs[design_id as usize].read().ok()?;
    let mut xovers = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Bound(d_id, n1, n2) => {
                if *d_id != design_id {
                    return None;
                }
                if let Some(id) = design.get_xover_id(&(*n1, *n2)) {
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

pub fn all_helices_no_grid(selection: &[Selection], designs: &[Arc<RwLock<Design>>]) -> bool {
    let design_id = selection.get(0).and_then(Selection::get_design);
    let mut nb_helices = 0;
    if design_id.is_none() {
        return false;
    }
    let design_id = design_id.unwrap();
    let design = designs[design_id as usize].read().unwrap();

    for s in selection.iter() {
        match s {
            Selection::Helix(d_id, h_id) => {
                if *d_id != design_id {
                    return false;
                }
                if design.get_grid_pos_helix(*h_id).is_some() {
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

use iced::image::Handle;
impl SelectionMode {
    pub const ALL: [SelectionMode; 5] = [
        SelectionMode::Nucleotide,
        SelectionMode::Design,
        SelectionMode::Strand,
        SelectionMode::Helix,
        SelectionMode::Grid,
    ];

    pub fn icon_on(&self) -> Handle {
        let bytes = match self {
            Self::Grid { .. } => include_bytes!("../../icons/icons/Grid-on32.png").to_vec(),
            Self::Helix => include_bytes!("../../icons/icons/Helix-on32.png").to_vec(),
            Self::Nucleotide => include_bytes!("../../icons/icons/Nucleotide-on32.png").to_vec(),
            Self::Strand => include_bytes!("../../icons/icons/Strand-on32.png").to_vec(),
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }

    pub fn icon_off(&self) -> Handle {
        let bytes = match self {
            Self::Grid { .. } => include_bytes!("../../icons/icons/Grid-off32.png").to_vec(),
            Self::Helix => include_bytes!("../../icons/icons/Helix-off32.png").to_vec(),
            Self::Nucleotide => include_bytes!("../../icons/icons/Nucleotide-off32.png").to_vec(),
            Self::Strand => include_bytes!("../../icons/icons/Strand-off32.png").to_vec(),
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }
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

    pub fn icon_on(&self) -> Handle {
        let bytes = match self {
            Self::BuildHelix { .. } => {
                include_bytes!("../../icons/icons/NewHelix-on32.png").to_vec()
            }
            Self::Normal => include_bytes!("../../icons/icons/Select-on32.png").to_vec(),
            Self::Translate => include_bytes!("../../icons/icons/Move-on32.png").to_vec(),
            Self::Rotate => include_bytes!("../../icons/icons/Rotate-on32.png").to_vec(),
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }

    pub fn icon_off(&self) -> Handle {
        let bytes = match self {
            Self::BuildHelix { .. } => {
                include_bytes!("../../icons/icons/NewHelix-off32.png").to_vec()
            }
            Self::Normal => include_bytes!("../../icons/icons/Select-off32.png").to_vec(),
            Self::Translate => include_bytes!("../../icons/icons/Move-off32.png").to_vec(),
            Self::Rotate => include_bytes!("../../icons/icons/Rotate-off32.png").to_vec(),
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }
}
