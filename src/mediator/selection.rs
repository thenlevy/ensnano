use crate::design::{Design, Nucl};
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy)]
pub enum Selection {
    Nucleotide(u32, Nucl),
    Bound(u32, Nucl, Nucl),
    Design(u32),
    Strand(u32, u32),
    Helix(u32, u32),
    Grid(u32, usize),
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
            Selection::Nothing => None,
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
                vec![b1, b2]
                    .iter()
                    .map(|b| {
                        if *b {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    })
                    .collect()
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
            _ => Vec::new(),
        }
    }
}

pub(super) fn list_of_strands(
    selection: &[Selection],
    design: Arc<RwLock<Design>>,
) -> Option<(usize, Vec<usize>)> {
    let design_id = selection.get(0).and_then(Selection::get_design)?;
    let mut strands = BTreeSet::new();
    for s in selection.iter() {
        match s {
            Selection::Nucleotide(d_id, n) => {
                if *d_id != design_id {
                    return None;
                }
                let s_id = design.read().unwrap().get_strand_nucl(n)?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Grid,
    Nucleotide,
    Design,
    Strand,
    Helix,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                ActionMode::Normal => "Normal",
                ActionMode::Translate => "Translate",
                ActionMode::Rotate => "Rotate",
                ActionMode::Build(_) => "Build",
                ActionMode::BuildHelix { .. } => "BuildHelix",
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
