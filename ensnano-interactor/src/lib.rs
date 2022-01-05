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

use ensnano_design::{
    elements::{DnaAttribute, DnaElementKey},
    grid::{GridDescriptor, GridPosition, Hyperboloid},
    group_attributes::GroupPivot,
    Nucl,
};
use ultraviolet::{Isometry2, Rotor3, Vec2, Vec3};
pub mod graphics;
mod selection;
pub use selection::*;
pub mod application;
pub mod operation;
mod strand_builder;
pub use strand_builder::*;
pub mod torsion;
use ensnano_organizer::GroupId;

#[macro_use]
extern crate log;

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
    AddGridHelix {
        position: GridPosition,
        start: isize,
        length: usize,
    },
    RmHelices {
        h_ids: Vec<usize>,
    },
    RmXovers {
        xovers: Vec<(Nucl, Nucl)>,
    },
    /// Split a strand at a given position. If the strand containing the nucleotide has length 1,
    /// delete the strand.
    Cut {
        nucl: Nucl,
        s_id: usize,
    },
    /// Make a cross-over between two nucleotides, spliting the source and target strands if needed
    GeneralXover {
        source: Nucl,
        target: Nucl,
    },
    /// Merge two strands by making a cross-over between the 3'end of prime_5 and the 5'end of
    /// prime_3
    Xover {
        prime5_id: usize,
        prime3_id: usize,
    },
    /// Make a cross over from a strand end to a nucleotide, spliting the target strand if needed.
    CrossCut {
        target_3prime: bool,
        source_id: usize,
        target_id: usize,
        nucl: Nucl,
    },
    /// Delete a strand
    RmStrands {
        strand_ids: Vec<usize>,
    },
    /// Add a grid to the design
    AddGrid(GridDescriptor),
    /// Remove a grid
    RmGrid(usize),
    /// Pick a new color at random for all the strands that are not the scaffold
    RecolorStaples,
    /// Set the sequence of a set of strands
    ChangeSequence {
        sequence: String,
        strands: Vec<usize>,
    },
    /// Change the color of a set of strands
    ChangeColor {
        color: u32,
        strands: Vec<usize>,
    },
    /// Set the strand with a given id as the scaffold
    SetScaffoldId(Option<usize>),
    /// Change the shift of the scaffold without changing the sequence
    SetScaffoldShift(usize),
    /// Change the sequence and the shift of the scaffold
    SetScaffoldSequence {
        sequence: String,
        shift: usize,
    },
    HyperboloidOperation(HyperboloidOperation),
    CleanDesign,
    HelicesToGrid(Vec<Selection>),
    SetHelicesPersistance {
        grid_ids: Vec<usize>,
        persistant: bool,
    },
    UpdateAttribute {
        attribute: DnaAttribute,
        elements: Vec<DnaElementKey>,
    },
    SetSmallSpheres {
        grid_ids: Vec<usize>,
        small: bool,
    },
    /// Apply a translation to the 2d representation of helices holding each pivot
    SnapHelices {
        pivots: Vec<Nucl>,
        translation: Vec2,
    },
    RotateHelices {
        helices: Vec<usize>,
        center: Vec2,
        angle: f32,
    },
    SetIsometry {
        helix: usize,
        isometry: Isometry2,
    },
    RequestStrandBuilders {
        nucls: Vec<Nucl>,
    },
    MoveBuilders(isize),
    SetRollHelices {
        helices: Vec<usize>,
        roll: f32,
    },
    SetVisibilityHelix {
        helix: usize,
        visible: bool,
    },
    FlipHelixGroup {
        helix: usize,
    },
    FlipAnchors {
        nucls: Vec<Nucl>,
    },
    AttachHelix {
        helix: usize,
        grid: usize,
        x: isize,
        y: isize,
    },
    SetOrganizerTree(ensnano_design::OrganizerTree<DnaElementKey>),
    SetStrandName {
        s_id: usize,
        name: String,
    },
    SetGroupPivot {
        group_id: GroupId,
        pivot: GroupPivot,
    },
    DeleteCamera(ensnano_design::CameraId),
    CreateNewCamera {
        position: Vec3,
        orientation: Rotor3,
    },
    SetFavouriteCamera(ensnano_design::CameraId),
    UpdateCamera {
        camera_id: ensnano_design::CameraId,
        position: Vec3,
        orientation: Rotor3,
    },
    SetCameraName {
        camera_id: ensnano_design::CameraId,
        name: String,
    },
    SetGridPosition {
        grid_id: usize,
        position: Vec3,
    },
    SetGridOrientation {
        grid_id: usize,
        orientation: Rotor3,
    },
}

/// An action performed on the application
pub enum AppOperation {
    /// Adjust the camera so that the design fit the view
    Fit,
}

#[derive(Debug, Clone)]
pub enum HyperboloidOperation {
    New {
        request: HyperboloidRequest,
        position: Vec3,
        orientation: Rotor3,
    },
    Update(HyperboloidRequest),
    Finalize,
    Cancel,
}

/// A rotation on an element of a design.
#[derive(Debug, Clone)]
pub struct DesignRotation {
    pub origin: Vec3,
    pub rotation: Rotor3,
    /// The element of the design on which the rotation will be applied
    pub target: IsometryTarget,
    pub group_id: Option<GroupId>,
}

/// A translation of an element of a design
#[derive(Clone, Debug)]
pub struct DesignTranslation {
    pub translation: Vec3,
    pub target: IsometryTarget,
    pub group_id: Option<GroupId>,
}

/// A element on which an isometry must be applied
#[derive(Clone, Debug)]
pub enum IsometryTarget {
    /// The view of the whole design
    Design,
    /// An helix of the design
    Helices(Vec<usize>, bool),
    /// A grid of the desgin
    Grids(Vec<usize>),
    /// The pivot of a group
    GroupPivot(GroupId),
}

/// A stucture that defines an helix on a grid
#[derive(Clone, Debug)]
pub struct GridHelixDescriptor {
    pub grid_id: usize,
    pub x: isize,
    pub y: isize,
}

#[derive(Debug, Clone)]
pub struct HyperboloidRequest {
    pub radius: usize,
    pub length: f32,
    pub shift: f32,
    pub radius_shift: f32,
}

impl HyperboloidRequest {
    pub fn to_grid(self) -> Hyperboloid {
        Hyperboloid {
            radius: self.radius,
            length: self.length,
            shift: self.shift,
            radius_shift: self.radius_shift,
            forced_radius: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RollRequest {
    pub roll: bool,
    pub springs: bool,
    pub target_helices: Option<Vec<usize>>,
}

#[derive(Clone, Debug)]
pub struct RigidBodyConstants {
    pub k_spring: f32,
    pub k_friction: f32,
    pub mass: f32,
    pub volume_exclusion: bool,
    pub brownian_motion: bool,
    pub brownian_rate: f32,
    pub brownian_amplitude: f32,
}

impl Default for RigidBodyConstants {
    fn default() -> Self {
        Self {
            k_friction: 1.,
            k_spring: 1.,
            mass: 1.,
            volume_exclusion: false,
            brownian_amplitude: 1.,
            brownian_rate: 1.,
            brownian_motion: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScaffoldInfo {
    pub id: usize,
    pub shift: Option<usize>,
    pub length: usize,
    pub starting_nucl: Option<Nucl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationState {
    None,
    Rolling,
    RigidGrid,
    RigidHelices,
    Paused,
}

impl SimulationState {
    pub fn is_none(&self) -> bool {
        if let Self::None = self {
            true
        } else {
            false
        }
    }

    pub fn is_rolling(&self) -> bool {
        if let Self::Rolling = self {
            true
        } else {
            false
        }
    }

    pub fn simulating_grid(&self) -> bool {
        if let Self::RigidGrid = self {
            true
        } else {
            false
        }
    }

    pub fn simulating_helices(&self) -> bool {
        if let Self::RigidHelices = self {
            true
        } else {
            false
        }
    }

    pub fn is_paused(&self) -> bool {
        if let Self::Paused = self {
            true
        } else {
            false
        }
    }

    pub fn is_runing(&self) -> bool {
        match self {
            Self::Paused | Self::None => false,
            _ => true,
        }
    }
}

impl Default for SimulationState {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidgetBasis {
    World,
    Object,
}

impl WidgetBasis {
    pub fn toggle(&mut self) {
        if self.is_axis_aligned() {
            *self = WidgetBasis::Object
        } else {
            *self = WidgetBasis::World
        };
    }

    pub fn is_axis_aligned(&self) -> bool {
        match self {
            Self::World => true,
            Self::Object => false,
        }
    }
}

impl Default for WidgetBasis {
    fn default() -> Self {
        Self::World
    }
}

/// Information about the domain being elongated
#[derive(Debug, Clone)]
pub struct StrandBuildingStatus {
    pub nt_length: usize,
    pub nm_length: f32,
    pub prime3: Nucl,
    pub prime5: Nucl,
    pub dragged_nucl: Nucl,
}

/// Parameters of strand suggestions
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SuggestionParameters {
    pub include_scaffold: bool,
    pub include_intra_strand: bool,
    pub include_xover_ends: bool,
    pub ignore_groups: bool,
}

impl Default for SuggestionParameters {
    fn default() -> Self {
        Self {
            include_intra_strand: true,
            include_scaffold: true,
            include_xover_ends: false,
            ignore_groups: false,
        }
    }
}

impl SuggestionParameters {
    pub fn with_include_scaffod(&self, include_scaffold: bool) -> Self {
        let mut ret = self.clone();
        ret.include_scaffold = include_scaffold;
        ret
    }

    pub fn with_intra_strand(&self, intra_strand: bool) -> Self {
        let mut ret = self.clone();
        ret.include_intra_strand = intra_strand;
        ret
    }

    pub fn with_ignore_groups(&self, ignore_groups: bool) -> Self {
        let mut ret = self.clone();
        ret.ignore_groups = ignore_groups;
        ret
    }

    pub fn with_xover_ends(&self, include_xover_ends: bool) -> Self {
        let mut ret = self.clone();
        ret.include_xover_ends = include_xover_ends;
        ret
    }
}
