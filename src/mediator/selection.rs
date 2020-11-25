use crate::design::Nucl;

#[derive(Debug, Clone, Copy)]
pub enum Selection {
    Nucleotide(u32, Nucl),
    Design(u32),
    Strand(u32, u32),
    Helix(u32, u32),
    Grid(u32, u32),
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
    /// User can elongate/shorten strands
    Build,
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
                ActionMode::Build => "Build",
                ActionMode::Cut => "Cut",
            }
        )
    }
}
