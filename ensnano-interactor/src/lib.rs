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

//! This modules defines types and operation used  by the graphical component of ENSnano to
//! interract with the design.

use ensnano_design::{grid::GridDescriptor, Nucl, Strand};
use ultraviolet::{Rotor3, Vec3};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ObjectType {
    /// A nucleotide identified by its identifier
    Nucleotide(u32),
    /// A bound, identified by the identifier of the two nucleotides that it bounds.
    Bound(u32, u32),
}

impl ObjectType {
    pub fn is_nucl(&self) -> bool {
        match self {
            ObjectType::Nucleotide(_) => true,
            _ => false,
        }
    }

    pub fn is_bound(&self) -> bool {
        match self {
            ObjectType::Bound(_, _) => true,
            _ => false,
        }
    }

    pub fn same_type(&self, other: Self) -> bool {
        self.is_nucl() == other.is_nucl()
    }
}

/// The referential in which one wants to get an element's coordinates
#[derive(Debug, Clone, Copy)]
pub enum Referential {
    World,
    Model,
}

impl Referential {
    pub fn is_world(&self) -> bool {
        match self {
            Referential::World => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
/// An operation that can be perorfed on a design
pub enum DesignOperation {
    /// Rotate an element of the design
    Rotation(DesignRotation),
    /// Translate an element of the design
    Translation(DesignTranslation),
    /// Add an helix on a grid
    AddGridHelix(GridHelixDescriptor, isize, usize),
    /// Remove an helix on a grid
    RmGridHelix(GridHelixDescriptor, isize, usize),
    RawHelixCreation {
        h_id: usize,
    },
    /// Split a strand at a given position
    Cut {
        nucl: Nucl,
        s_id: usize,
    },
    /// Make a cross-over between two nucleotides, spliting the source and target strands if needed
    Xover {
        undo: bool,
        prime5_id: usize,
        prime3_id: usize,
    },
    /// Make a cross over from a strand end to a nucleotide, spliting the target strand if needed.
    CrossCut {
        target_3prime: bool,
        source_id: usize,
        target_id: usize,
        nucl: Nucl,
        undo: bool,
    },
    /// Delete a strand
    RmStrand {
        strand_id: usize,
        undo: bool,
    },
    MakeAllGrids,
    /// Add a grid to the design
    AddGrid(GridDescriptor),
    /// Remove a grid
    RmGrid(usize),
    /// Pick a new color at random for all the strands that are not the scaffold
    RecolorStaples,
}

/// An action performed on the application
pub enum AppOperation {
    /// Adjust the camera so that the design fit the view
    Fit,
}

/// A rotation on an element of a design.
#[derive(Debug, Clone)]
pub struct DesignRotation {
    pub origin: Vec3,
    pub rotation: Rotor3,
    /// The element of the design on which the rotation will be applied
    pub target: IsometryTarget,
}

/// A translation of an element of a design
#[derive(Clone, Debug)]
pub struct DesignTranslation {
    pub translation: Vec3,
    pub target: IsometryTarget,
}

/// A element on which an isometry must be applied
#[derive(Clone, Debug)]
pub enum IsometryTarget {
    /// The view of the whole design
    Design,
    /// An helix of the design
    Helix(u32, bool),
    /// A grid of the desgin
    Grid(u32),
}

/// A stucture that defines an helix on a grid
#[derive(Clone, Debug)]
pub struct GridHelixDescriptor {
    pub grid_id: usize,
    pub x: isize,
    pub y: isize,
}

/// The return type for methods that ask if a nucleotide is the end of a domain/strand/xover
#[derive(Debug, Clone, Copy)]
pub enum Extremity {
    No,
    Prime3,
    Prime5,
}

impl Extremity {
    pub fn is_3prime(&self) -> bool {
        match self {
            Extremity::Prime3 => true,
            _ => false,
        }
    }

    pub fn is_5prime(&self) -> bool {
        match self {
            Extremity::Prime5 => true,
            _ => false,
        }
    }

    pub fn is_end(&self) -> bool {
        match self {
            Extremity::No => false,
            _ => true,
        }
    }

    pub fn to_opt(&self) -> Option<bool> {
        match self {
            Extremity::No => None,
            Extremity::Prime3 => Some(true),
            Extremity::Prime5 => Some(false),
        }
    }
}
