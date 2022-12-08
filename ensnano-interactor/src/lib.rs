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
//! interact with the design.

use std::path::PathBuf;

use ensnano_design::{
    elements::{DnaAttribute, DnaElementKey},
    grid::{GridDescriptor, GridId, GridObject, GridTypeDescr, HelixGridPosition, Hyperboloid},
    group_attributes::GroupPivot,
    BezierPathId, BezierPlaneDescriptor, BezierPlaneId, BezierVertex, BezierVertexId,
    CurveDescriptor2D, Isometry3, Nucl, Parameters,
};
use serde::{Deserialize, Serialize};
use ultraviolet::{Isometry2, Rotor3, Vec2, Vec3};
pub mod graphics;
mod selection;
pub use selection::*;
pub mod application;
pub use application::CursorIcon;
pub mod operation;
mod strand_builder;
pub use strand_builder::*;
pub mod consts;
pub mod torsion;
use ensnano_organizer::GroupId;
mod operation_labels;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ObjectType {
    /// A nucleotide identified by its identifier
    Nucleotide(u32),
    /// A bound, identified by the identifier of the two nucleotides that it bounds.
    Bound(u32, u32),
}

impl ObjectType {
    pub fn is_nucl(&self) -> bool {
        matches!(self, ObjectType::Nucleotide(_))
    }

    pub fn is_bound(&self) -> bool {
        matches!(self, ObjectType::Bound(_, _))
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
        matches!(self, Referential::World)
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
        position: HelixGridPosition,
        start: isize,
        length: usize,
    },
    AddTwoPointsBezier {
        start: HelixGridPosition,
        end: HelixGridPosition,
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
        grid_ids: Vec<GridId>,
        persistant: bool,
    },
    UpdateAttribute {
        attribute: DnaAttribute,
        elements: Vec<DnaElementKey>,
    },
    SetSmallSpheres {
        grid_ids: Vec<GridId>,
        small: bool,
    },
    /// Apply a translation to the 2d representation of helices holding each pivot
    SnapHelices {
        pivots: Vec<(Nucl, usize)>,
        translation: Vec2,
    },
    RotateHelices {
        helices: Vec<usize>,
        center: Vec2,
        angle: f32,
    },
    ApplySymmetryToHelices {
        helices: Vec<usize>,
        centers: Vec<Vec2>,
        symmetry: Vec2,
    },
    SetIsometry {
        helix: usize,
        segment: usize,
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
    AttachObject {
        object: GridObject,
        grid: GridId,
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
        pivot_position: Option<Vec3>,
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
        grid_id: GridId,
        position: Vec3,
    },
    SetGridOrientation {
        grid_id: GridId,
        orientation: Rotor3,
    },
    SetGridNbTurn {
        grid_id: GridId,
        nb_turn: f32,
    },
    MakeSeveralXovers {
        xovers: Vec<(Nucl, Nucl)>,
        doubled: bool,
    },
    CheckXovers {
        xovers: Vec<usize>,
    },
    SetRainbowScaffold(bool),
    SetDnaParameters {
        parameters: Parameters,
    },
    SetInsertionLength {
        length: usize,
        insertion_point: InsertionPoint,
    },
    AddBezierPlane {
        desc: BezierPlaneDescriptor,
    },
    CreateBezierPath {
        first_vertex: BezierVertex,
    },
    AppendVertexToPath {
        path_id: BezierPathId,
        vertex: BezierVertex,
    },
    /// Move the first vertex to `position` and apply the same translation to the other vertices
    MoveBezierVertex {
        vertices: Vec<BezierVertexId>,
        position: Vec2,
    },
    SetBezierVertexPosition {
        vertex_id: BezierVertexId,
        position: Vec2,
    },
    TurnPathVerticesIntoGrid {
        path_id: BezierPathId,
        grid_type: GridTypeDescr,
    },
    ApplyHomothethyOnBezierPlane {
        homothethy: BezierPlaneHomothethy,
    },
    SetVectorOfBezierTengent(NewBezierTengentVector),
    MakeBezierPathCyclic {
        path_id: BezierPathId,
        cyclic: bool,
    },
    RmFreeGrids {
        grid_ids: Vec<usize>,
    },
    RmBezierVertices {
        vertices: Vec<BezierVertexId>,
    },
    Add3DObject {
        file_path: PathBuf,
        design_path: PathBuf,
    },
    ImportSvgPath {
        path: PathBuf,
    },
}

#[derive(Clone, Debug, Copy)]
pub struct NewBezierTengentVector {
    pub vertex_id: BezierVertexId,
    /// Wether `new_vector` is the vector of the inward or outward tengent
    pub tengent_in: bool,
    pub full_symetry_other_tengent: bool,
    pub new_vector: Vec2,
}

#[derive(Clone, Debug, Copy)]
pub struct InsertionPoint {
    pub nucl: Nucl,
    pub nucl_is_prime5_of_insertion: bool,
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
    Grids(Vec<GridId>),
    /// The pivot of a group
    GroupPivot(GroupId),
    /// The control points of bezier curves
    ControlPoint(Vec<(usize, BezierControlPoint)>),
}

impl ToString for IsometryTarget {
    fn to_string(&self) -> String {
        match self {
            Self::Design => "Design".into(),
            Self::Helices(hs, _) => format!("Helices {:?}", hs),
            Self::Grids(gs) => format!("Grids {:?}", gs),
            Self::GroupPivot(_) => "Group pivot".into(),
            Self::ControlPoint(_) => "Bezier control point".into(),
        }
    }
}

/// A stucture that defines an helix on a grid
#[derive(Clone, Debug)]
pub struct GridHelixDescriptor {
    pub grid_id: GridId,
    pub x: isize,
    pub y: isize,
}

#[derive(Debug, Clone)]
pub struct HyperboloidRequest {
    pub radius: usize,
    pub length: f32,
    pub shift: f32,
    pub radius_shift: f32,
    pub nb_turn: f64,
}

impl HyperboloidRequest {
    pub fn to_grid(self) -> Hyperboloid {
        Hyperboloid {
            radius: self.radius,
            length: self.length,
            shift: self.shift,
            radius_shift: self.radius_shift,
            forced_radius: None,
            nb_turn_per_100_nt: self.nb_turn,
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
    Twisting { grid_id: GridId },
    Relaxing,
}

#[derive(Debug, Clone)]
pub struct RevolutionSurfaceSystemDescriptor {
    pub scaffold_len_target: usize,
    pub target: RevolutionSurfaceDescriptor,
    pub dna_parameters: Parameters,
    pub simulation_parameters: RevolutionSimulationParameters,
}

#[derive(Debug, Clone)]
pub struct RevolutionSurfaceDescriptor {
    pub curve: CurveDescriptor2D,
    pub revolution_radius: f64,
    pub nb_helix_per_half_section: usize,
    pub half_turns_count: isize,
    pub shift_per_turn: isize,
    pub junction_smoothening: f64,
    pub dna_paramters: Parameters,
    pub plane_position: Vec3,
    pub plane_orientation: Rotor3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnrootedRevolutionSurfaceDescriptor {
    pub curve: CurveDescriptor2D,
    pub revolution_radius: RevolutionSurfaceRadius,
    pub half_turn_count: isize,
    pub curve_plane_position: Vec3,
    pub curve_plane_orientation: Rotor3,
}

impl UnrootedRevolutionSurfaceDescriptor {
    pub fn get_frame(&self) -> Isometry3 {
        let mut ret = CurveDescriptor2D::get_frame_3d();

        // Then convert into the plane's frame
        ret.append_translation(self.curve_plane_position);
        ret.append_rotation(self.curve_plane_orientation);

        // Center on the rotation axis as drawn on the plane
        let rotation_axis_translation = (Vec3::unit_z()
            * self.get_revolution_axis_position() as f32)
            .rotated_by(self.curve_plane_orientation);
        ret.append_translation(rotation_axis_translation);
        ret
    }

    pub fn get_revolution_axis_position(&self) -> f64 {
        use RevolutionSurfaceRadius::*;
        match self.revolution_radius {
            Left(x) => self.curve.min_x() - x,
            Right(x) => x + self.curve.max_x(),
            Inside(x) => x,
        }
    }

    pub fn set_axis_position(&mut self, position: f64) {
        let min_x = self.curve.min_x();
        let max_x = self.curve.max_x();
        let new_radius = if position <= min_x {
            RevolutionSurfaceRadius::Left(min_x - position)
        } else if position >= max_x {
            RevolutionSurfaceRadius::Right(position - max_x)
        } else {
            RevolutionSurfaceRadius::Inside(position)
        };
        self.revolution_radius = new_radius;
    }

    /// Approximate the area of the surface by slicing it into strips of triangles.
    ///
    /// The surface is split into `nb_strip` strips of 2 * `nb_section_per_strip` triangles
    pub fn approx_surface_area(&self, nb_strip: usize, nb_section_per_strip: usize) -> Option<f64> {
        use ensnano_design::PointOnSurface;
        use rayon::prelude::*;

        if matches!(self.revolution_radius, RevolutionSurfaceRadius::Inside(_)) {
            return None;
        }

        let ret = (0..nb_strip)
            .into_par_iter()
            .map(|strip_idx| {
                // Parameters along the section for the top and bottom of the strip
                let s_high = strip_idx as f64 / nb_strip as f64;
                let s_low = s_high + 1. / nb_strip as f64;

                let vertices = (0..nb_section_per_strip)
                    .flat_map(|section_idx| {
                        [s_high, s_low].into_iter().map(move |s| {
                            use std::f64::consts::TAU;
                            let revolution_fract = section_idx as f64 / nb_section_per_strip as f64;

                            let revolution_angle = TAU * revolution_fract;

                            let surface_point = PointOnSurface {
                                revolution_angle,
                                section_parameter: s,
                                revolution_axis_position: self.get_revolution_axis_position(),
                                section_half_turn_per_revolution: self.half_turn_count,
                            };
                            self.curve.point_on_surface(&surface_point)
                        })
                    })
                    .cycle();

                // Compute the area of the triangles of the strip using the formula
                // area(ABC) = 1/2 * mag(AB cross AC)
                vertices
                    .clone()
                    .zip(vertices.clone().skip(1))
                    .zip(vertices.skip(2))
                    .take(2 * nb_section_per_strip)
                    .map(|((a, b), c)| 0.5 * (b - a).cross(c - a).mag())
                    .sum::<f64>()
            })
            .sum::<f64>();
        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(non_snake_case)]
    fn surface_area() {
        let r = 1.0;
        let R = 3.0;
        let surface = UnrootedRevolutionSurfaceDescriptor {
            curve: CurveDescriptor2D::Ellipse {
                semi_minor_axis: r.into(),
                semi_major_axis: r.into(),
            },
            revolution_radius: RevolutionSurfaceRadius::Left(R - r),
            half_turn_count: 0,
            curve_plane_position: Vec3::zero(),
            curve_plane_orientation: Rotor3::identity(),
        };

        let expected = 4. * std::f64::consts::PI * std::f64::consts::PI * r * R;

        let actual = surface.approx_surface_area(1_000, 1_000).unwrap();

        assert!(
            (expected - actual).abs() < 1e-3,
            "exptected {expected},  actual {actual}"
        );
    }
}

/*
 * let q be the total shift and n be the number of sections
 * Helices seen as set of section are class of equivalence for the relation ~
 * where a ~ b iff there exists k1, k2 st a = b  + k1 q + k2 n
 *
 * let d = gcd(q, n). If a ~ b then a = b (mod d)
 *
 * Recp. if a = b (mod d) there exists x y st xq + yn = d
 *
 * a = k (xq + yn) + b
 * so a ~ b
 *
 * So ~ is the relation of equivalence modulo d and has d classes.
 */

fn gcd(a: isize, b: isize) -> usize {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();

    if a < b {
        std::mem::swap(&mut a, &mut b);
    }

    while b > 0 {
        let b_ = b;
        b = a % b;
        a = b_;
    }

    a
}

impl RevolutionSurfaceDescriptor {
    pub fn nb_helices(&self) -> usize {
        let additional_shift = if self.half_turns_count % 2 == 1 {
            self.nb_helix_per_half_section / 2
        } else {
            0
        };
        let total_shift = self.shift_per_turn + additional_shift as isize;
        gcd(total_shift, self.nb_helix_per_half_section as isize * 2)
    }
}

impl SimulationState {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_rolling(&self) -> bool {
        matches!(self, Self::Rolling)
    }

    pub fn simulating_grid(&self) -> bool {
        matches!(self, Self::RigidGrid)
    }

    pub fn simulating_helices(&self) -> bool {
        matches!(self, Self::RigidHelices)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    pub fn is_runing(&self) -> bool {
        !matches!(self, Self::Paused | Self::None)
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
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckXoversParameter {
    None,
    Checked,
    Unchecked,
    Both,
}

impl Default for CheckXoversParameter {
    fn default() -> Self {
        Self::None
    }
}

impl ToString for CheckXoversParameter {
    fn to_string(&self) -> String {
        match self {
            Self::None => String::from("None"),
            Self::Checked => String::from("Checked"),
            Self::Unchecked => String::from("Unchecked"),
            Self::Both => String::from("Both"),
        }
    }
}

impl CheckXoversParameter {
    pub const ALL: &'static [Self] = &[Self::None, Self::Checked, Self::Unchecked, Self::Both];

    pub fn wants_checked(&self) -> bool {
        match self {
            Self::Checked | Self::Both => true,
            Self::None | Self::Unchecked => false,
        }
    }

    pub fn wants_unchecked(&self) -> bool {
        match self {
            Self::Unchecked | Self::Both => true,
            Self::None | Self::Checked => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BezierPlaneHomothethy {
    pub plane_id: BezierPlaneId,
    pub fixed_corner: Vec2,
    pub origin_moving_corner: Vec2,
    pub moving_corner: Vec2,
}

#[derive(Debug, Clone, Copy)]
/// One of the standard scaffold sequence shipped with ENSnano
pub enum StandardSequence {
    P7259,
    P7560,
    P8064,
    PUC19,
}

impl StandardSequence {
    pub fn description(&self) -> &'static str {
        match self {
            Self::P7259 => "m13 p7259",
            Self::P7560 => "m13 p7560",
            Self::P8064 => "m13 p8064",
            Self::PUC19 => "pUC19 (2686 nt)",
        }
    }

    pub fn sequence(&self) -> &'static str {
        match self {
            Self::P7259 => include_str!("../p7249-Tilibit.txt"),
            Self::P7560 => include_str!("../p7560.txt"),
            Self::P8064 => include_str!("../m13-p8064.txt"),
            Self::PUC19 => include_str!("../pUC19.txt"),
        }
    }

    /// Return the variant of Self whose associated sequence length is closest to n
    pub fn from_length(n: usize) -> Self {
        let mut best_score = isize::MAX;
        let mut ret = Self::default();
        for candidate in [Self::P7259, Self::P7560, Self::P8064] {
            let score = (candidate.sequence().len() as isize - (n as isize)).abs();
            if score < best_score {
                best_score = score;
                ret = candidate;
            }
        }
        ret
    }
}

impl Default for StandardSequence {
    fn default() -> Self {
        Self::P7259
    }
}

#[derive(Debug, Clone)]
pub struct RevolutionSimulationParameters {
    pub nb_section_per_segment: usize,
    pub spring_stiffness: f64,
    pub torsion_stiffness: f64,
    pub fluid_friction: f64,
    pub ball_mass: f64,
    pub time_span: f64,
    pub simulation_step: f64,
    pub method: EquadiffSolvingMethod,
    pub rescaling: f64,
}

impl Default for RevolutionSimulationParameters {
    fn default() -> Self {
        consts::DEFAULT_REVOLUTION_SIMULATION_PARAMETERS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquadiffSolvingMethod {
    Euler,
    Ralston,
}

impl EquadiffSolvingMethod {
    pub const ALL_METHODS: &'static [Self] = &[Self::Euler, Self::Ralston];
}

impl ToString for EquadiffSolvingMethod {
    fn to_string(&self) -> String {
        match self {
            Self::Euler => "Euler".to_string(),
            Self::Ralston => "Ralston".to_string(),
        }
    }
}

/// The position of the axis of revolution of a revolution surface
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum RevolutionSurfaceRadius {
    /// The axis is on the left of the leftmost point of the section
    Left(f64),
    /// The axis is on the right of the rightmost point of the section
    Right(f64),
    /// The axis is inside the section.
    Inside(f64),
}

impl Default for RevolutionSurfaceRadius {
    fn default() -> Self {
        Self::Left(0.)
    }
}

impl RevolutionSurfaceRadius {
    /// Convert to Self from an f64. The sign indicate the position of the revolution axis relative
    /// to the section.
    ///
    /// Positive value indicate that the axis of revolution is on the right of the section
    /// Negative value indicate that the axis of revolution is on the left of the section
    pub fn from_signed_f64(radius: f64) -> Self {
        if radius.is_sign_positive() {
            Self::Right(radius)
        } else {
            Self::Left(-radius)
        }
    }

    /// Convert self to an f64. The sign indicate the position of the revolution axis relative to
    /// the section.
    ///
    /// Positive value indicate that the axis of revolution is on the right of the section
    /// Negative value indicate that the axis of revolution is on the left of the section
    pub fn to_signed_f64(self) -> Option<f64> {
        match self {
            Self::Left(x) => Some(-x),
            Self::Right(x) => Some(x),
            Self::Inside(_) => None,
        }
    }
}
