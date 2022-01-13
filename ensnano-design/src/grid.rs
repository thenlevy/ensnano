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

use crate::CurveDescriptor;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::{
    curves,
    design_operations::{ErrOperation, MIN_HELICES_TO_MAKE_GRID},
    mutate_in_arc, Axis, Design, Helices, HelicesMut, Helix, HelixCollection, Parameters,
};
use curves::{CurveCache, GridPositionProvider, InstanciatedCurve, InstanciatedCurveDescriptor};
mod hyperboloid;
pub use hyperboloid::*;
use std::sync::Arc;

use ultraviolet::{Rotor3, Vec2, Vec3};

#[derive(Clone, Debug)]
pub struct Grid {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub parameters: Parameters,
    pub grid_type: GridType,
    pub invisible: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GridDescriptor {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub grid_type: GridTypeDescr,
    #[serde(default)]
    pub invisible: bool, // by default grids are visible so we store a "negative attribute"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GridTypeDescr {
    Square,
    Honeycomb,
    Hyperboloid {
        radius: usize,
        shift: f32,
        length: f32,
        radius_shift: f32,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        forced_radius: Option<f32>,
        #[serde(default)]
        nb_turn: f32,
    },
}

impl GridDescriptor {
    pub fn hyperboloid(position: Vec3, orientation: Rotor3, hyperboloid: Hyperboloid) -> Self {
        Self {
            position,
            orientation,
            grid_type: hyperboloid.desc(),
            invisible: false,
        }
    }

    pub fn to_grid(&self, parameters: Parameters) -> Grid {
        Grid {
            position: self.position,
            orientation: self.orientation,
            invisible: self.invisible,
            grid_type: self.grid_type.to_concrete(),
            parameters,
        }
    }
}

impl GridTypeDescr {
    pub fn to_string(&self) -> String {
        match self {
            GridTypeDescr::Square => String::from("Square"),
            GridTypeDescr::Honeycomb => String::from("Honeycomb"),
            GridTypeDescr::Hyperboloid { .. } => String::from("Hyperboloid"),
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            GridTypeDescr::Square => 0u32,
            GridTypeDescr::Honeycomb => 1u32,
            GridTypeDescr::Hyperboloid { .. } => 2u32,
        }
    }

    fn to_concrete(&self) -> GridType {
        match self.clone() {
            Self::Square => GridType::square(),
            Self::Honeycomb => GridType::honneycomb(),
            Self::Hyperboloid {
                radius,
                shift,
                forced_radius,
                length,
                radius_shift,
                nb_turn,
            } => GridType::Hyperboloid(Hyperboloid {
                radius,
                shift,
                forced_radius,
                length,
                radius_shift,
                nb_turn,
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub enum GridType {
    Square(SquareGrid),
    Honeycomb(HoneyComb),
    Hyperboloid(Hyperboloid),
}

impl GridDivision for GridType {
    fn grid_type(&self) -> GridType {
        match self {
            GridType::Square(SquareGrid) => GridType::Square(SquareGrid),
            GridType::Honeycomb(HoneyComb) => GridType::Honeycomb(HoneyComb),
            GridType::Hyperboloid(hyperboloid) => GridType::Hyperboloid(hyperboloid.clone()),
        }
    }

    fn origin_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        match self {
            GridType::Square(grid) => grid.origin_helix(parameters, x, y),
            GridType::Honeycomb(grid) => grid.origin_helix(parameters, x, y),
            GridType::Hyperboloid(grid) => grid.origin_helix(parameters, x, y),
        }
    }

    fn orientation_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Rotor3 {
        match self {
            GridType::Square(grid) => grid.orientation_helix(parameters, x, y),
            GridType::Honeycomb(grid) => grid.orientation_helix(parameters, x, y),
            GridType::Hyperboloid(grid) => grid.orientation_helix(parameters, x, y),
        }
    }

    fn interpolate(&self, parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        match self {
            GridType::Square(grid) => grid.interpolate(parameters, x, y),
            GridType::Honeycomb(grid) => grid.interpolate(parameters, x, y),
            GridType::Hyperboloid(grid) => grid.interpolate(parameters, x, y),
        }
    }

    fn translation_to_edge(&self, x1: isize, y1: isize, x2: isize, y2: isize) -> Edge {
        match self {
            GridType::Square(grid) => grid.translation_to_edge(x1, y1, x2, y2),
            GridType::Honeycomb(grid) => grid.translation_to_edge(x1, y1, x2, y2),
            GridType::Hyperboloid(grid) => grid.translation_to_edge(x1, y1, x2, y2),
        }
    }

    fn translate_by_edge(&self, x1: isize, y1: isize, edge: Edge) -> Option<(isize, isize)> {
        match self {
            GridType::Square(grid) => grid.translate_by_edge(x1, y1, edge),
            GridType::Honeycomb(grid) => grid.translate_by_edge(x1, y1, edge),
            GridType::Hyperboloid(grid) => grid.translate_by_edge(x1, y1, edge),
        }
    }

    fn curve(
        &self,
        x: isize,
        y: isize,
        position: Vec3,
        orientation: Rotor3,
        parameters: &Parameters,
    ) -> Option<Arc<CurveDescriptor>> {
        match self {
            GridType::Hyperboloid(grid) => grid.curve(x, y, position, orientation, parameters),
            _ => None,
        }
    }
}

impl GridType {
    pub fn square() -> Self {
        Self::Square(SquareGrid)
    }

    pub fn honneycomb() -> Self {
        Self::Honeycomb(HoneyComb)
    }

    pub fn hyperboloid(h: Hyperboloid) -> Self {
        Self::Hyperboloid(h)
    }

    pub fn descr(&self) -> GridTypeDescr {
        match self {
            GridType::Square(_) => GridTypeDescr::Square,
            GridType::Honeycomb(_) => GridTypeDescr::Honeycomb,
            GridType::Hyperboloid(h) => GridTypeDescr::Hyperboloid {
                radius: h.radius,
                shift: h.shift,
                length: h.length,
                radius_shift: h.radius_shift,
                forced_radius: h.forced_radius,
                nb_turn: h.nb_turn,
            },
        }
    }

    pub fn get_shift(&self) -> Option<f32> {
        match self {
            GridType::Square(_) => None,
            GridType::Honeycomb(_) => None,
            GridType::Hyperboloid(h) => Some(h.shift),
        }
    }

    pub fn get_nb_turn(&self) -> Option<f32> {
        match self {
            GridType::Square(_) => None,
            GridType::Honeycomb(_) => None,
            GridType::Hyperboloid(h) => Some(h.nb_turn),
        }
    }

    pub fn set_shift(&mut self, shift: f32, parameters: &Parameters) {
        match self {
            GridType::Square(_) => println!("WARNING changing shif of non hyperboloid grid"),
            GridType::Honeycomb(_) => println!("WARNING changing shif of non hyperboloid grid"),
            GridType::Hyperboloid(h) => h.modify_shift(shift, parameters),
        }
    }
}

impl Grid {
    pub fn new(
        position: Vec3,
        orientation: Rotor3,
        parameters: Parameters,
        grid_type: GridType,
    ) -> Self {
        Self {
            position,
            orientation,
            parameters,
            grid_type,
            invisible: false,
        }
    }

    /// Return the angle between the grid's plane and an helix axis.
    pub fn angle_axis(&self, axis: Vec3) -> f32 {
        let normal = Vec3::unit_x().rotated_by(self.orientation);
        axis.normalized().dot(normal).abs().asin()
    }

    /// Return the intersection between the grid's plane and a line given by an origin and a
    /// direction. If the line and the plane are parallels, return None.
    pub fn line_intersection(&self, origin: Vec3, direction: Vec3) -> Option<Vec2> {
        let intersection = self.real_intersection(origin, direction)?;
        let z_vec = Vec3::unit_z().rotated_by(self.orientation);
        let y_vec = Vec3::unit_y().rotated_by(self.orientation);
        Some(Vec2::new(
            (intersection - self.position).dot(z_vec),
            (intersection - self.position).dot(y_vec),
        ))
    }

    /// Return the intersection between self and a ray starting at a given `origin` with a given
    /// `direction`.
    pub fn real_intersection(&self, origin: Vec3, direction: Vec3) -> Option<Vec3> {
        let d = self.ray_intersection(origin, direction)?;
        let intersection = origin + d * direction;
        Some(intersection)
    }

    /// Return `d` so that `origin + d * direction` is a point of self.
    pub fn ray_intersection(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let normal = Vec3::unit_x().rotated_by(self.orientation);
        let denom = direction.dot(normal);
        if denom.abs() < 1e-3 {
            None
        } else {
            let d = (self.position - origin).dot(normal) / denom;
            Some(d)
        }
    }

    pub fn axis_helix(&self) -> Vec3 {
        Vec3::unit_x().rotated_by(self.orientation)
    }

    fn project_point(&self, point: Vec3) -> Vec3 {
        let normal = Vec3::unit_x().rotated_by(self.orientation);
        point + (self.position - point).dot(normal) * normal
    }

    pub fn position_helix(&self, x: isize, y: isize) -> Vec3 {
        let origin = self.grid_type.origin_helix(&self.parameters, x, y);
        let z_vec = Vec3::unit_z().rotated_by(self.orientation);
        let y_vec = Vec3::unit_y().rotated_by(self.orientation);
        self.position + origin.x * z_vec + origin.y * y_vec
    }

    pub fn orientation_helix(&self, x: isize, y: isize) -> Rotor3 {
        self.orientation * self.grid_type.orientation_helix(&self.parameters, x, y)
    }

    pub fn interpolate_helix(&self, origin: Vec3, axis: Vec3) -> Option<(isize, isize)> {
        let intersection = self.line_intersection(origin, axis)?;
        Some(
            self.grid_type
                .interpolate(&self.parameters, intersection.x, intersection.y),
        )
    }

    pub fn find_helix_position(
        &self,
        helix: &super::Helix,
        g_id: usize,
    ) -> Option<HelixGridPosition> {
        if let super::Axis::Line { origin, direction } = helix.get_axis(&self.parameters) {
            let (x, y) = self.interpolate_helix(origin, direction)?;
            let intersection = self.position_helix(x, y);
            // direction is the vector from the origin of the helix to its first axis position
            let axis_intersection =
                ((intersection - origin).dot(direction) / direction.mag_sq()).round() as isize;
            let nucl_intersection = helix.space_pos(&self.parameters, axis_intersection, false);
            let projection_nucl = self.project_point(nucl_intersection);
            let roll = {
                let x = (projection_nucl - intersection)
                    .dot(Vec3::unit_z().rotated_by(self.orientation))
                    / -self.parameters.helix_radius;
                let y = (projection_nucl - intersection)
                    .dot(Vec3::unit_y().rotated_by(self.orientation))
                    / -self.parameters.helix_radius;
                x.atan2(y)
                    - std::f32::consts::PI
                    - axis_intersection as f32 * 2. * std::f32::consts::PI
                        / self.parameters.bases_per_turn
            };
            let roll = (roll + std::f32::consts::PI).rem_euclid(2. * std::f32::consts::PI)
                - std::f32::consts::PI;
            Some(HelixGridPosition {
                grid: g_id,
                x,
                y,
                axis_pos: axis_intersection,
                roll,
            })
        } else {
            None
        }
    }

    pub fn desc(&self) -> GridDescriptor {
        GridDescriptor {
            position: self.position,
            orientation: self.orientation,
            grid_type: self.grid_type.descr(),
            invisible: self.invisible,
        }
    }

    pub fn make_curve(&self, x: isize, y: isize) -> Option<Arc<CurveDescriptor>> {
        self.grid_type
            .curve(x, y, self.position, self.orientation, &self.parameters)
    }
}

pub trait GridDivision {
    /// Maps a vertex of the grid to a coordinate in the plane.
    fn origin_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Vec2;
    /// Find the vertex in the grid that is the closest to a point in the plane.
    fn interpolate(&self, parameters: &Parameters, x: f32, y: f32) -> (isize, isize);
    fn grid_type(&self) -> GridType;
    fn translation_to_edge(&self, x1: isize, y1: isize, x2: isize, y2: isize) -> Edge;
    fn translate_by_edge(&self, x1: isize, y1: isize, edge: Edge) -> Option<(isize, isize)>;

    fn orientation_helix(&self, _parameters: &Parameters, _x: isize, _y: isize) -> Rotor3 {
        Rotor3::identity()
    }

    /// If the helix at position (x, y) should be a curve istead of a cylinder, this method must be
    /// overiden
    fn curve(
        &self,
        _x: isize,
        _y: isize,
        _position: Vec3,
        _orientation: Rotor3,
        _parameters: &Parameters,
    ) -> Option<Arc<CurveDescriptor>> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SquareGrid;

impl GridDivision for SquareGrid {
    fn origin_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        Vec2::new(
            x as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
            -y as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
        )
    }

    fn interpolate(&self, parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        (
            (x / (parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize,
            (y / -(parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize,
        )
    }

    fn translation_to_edge(&self, x1: isize, y1: isize, x2: isize, y2: isize) -> Edge {
        Edge::Square {
            x: x2 - x1,
            y: y2 - y1,
        }
    }

    fn translate_by_edge(&self, x1: isize, y1: isize, edge: Edge) -> Option<(isize, isize)> {
        if let Edge::Square { x, y } = edge {
            Some((x1 + x, y1 + y))
        } else {
            None
        }
    }

    fn grid_type(&self) -> GridType {
        GridType::Square(SquareGrid)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HoneyComb;

impl GridDivision for HoneyComb {
    fn origin_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let upper = -3. * r * y as f32;
        let lower = upper - r;
        Vec2::new(
            x as f32 * r * 3f32.sqrt(),
            if x.abs() % 2 != y.abs() % 2 {
                lower
            } else {
                upper
            },
        )
    }

    fn interpolate(&self, parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let first_guess = (
            (x / (r * 3f32.sqrt())).round() as isize,
            (y / (-3. * r)).floor() as isize,
        );

        let mut ret = first_guess;
        let mut best_dist = (self.origin_helix(parameters, first_guess.0, first_guess.1)
            - Vec2::new(x, y))
        .mag_sq();
        for dx in [-2, -1, 0, 1, 2].iter() {
            for dy in [-2, -1, 0, 1, 2].iter() {
                let guess = (first_guess.0 + dx, first_guess.1 + dy);
                let dist =
                    (self.origin_helix(parameters, guess.0, guess.1) - Vec2::new(x, y)).mag_sq();
                if dist < best_dist {
                    ret = guess;
                    best_dist = dist;
                }
            }
        }
        ret
    }

    fn translation_to_edge(&self, x1: isize, y1: isize, x2: isize, y2: isize) -> Edge {
        let partity = x1.abs() % 2 == y1.abs() % 2;
        Edge::Honney {
            x: x2 - x1,
            y: y2 - y1,
            start_parity: partity,
        }
    }

    fn translate_by_edge(&self, x1: isize, y1: isize, edge: Edge) -> Option<(isize, isize)> {
        if let Edge::Honney { x, y, .. } = edge {
            Some((x1 + x, y1 + y))
        } else {
            None
        }
    }

    fn grid_type(&self) -> GridType {
        GridType::Honeycomb(HoneyComb)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub struct HelixGridPosition {
    /// Identifier of the grid
    pub grid: usize,
    /// x coordinate on the grid
    pub x: isize,
    /// y coordinate on the grid
    pub y: isize,
    /// Position of the axis that intersect the grid
    pub axis_pos: isize,
    /// Roll of the helix with respect to the grid
    pub roll: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Hash, Eq)]
pub struct GridPosition {
    /// Identifier of the grid
    pub grid: usize,
    /// x coordinate on the grid
    pub x: isize,
    /// y coordinate on the grid
    pub y: isize,
}

impl HelixGridPosition {
    pub fn with_roll(self, roll: Option<f32>) -> Self {
        if let Some(roll) = roll {
            Self { roll, ..self }
        } else {
            self
        }
    }

    pub fn from_grid_id_x_y(g_id: usize, x: isize, y: isize) -> Self {
        Self {
            grid: g_id,
            x,
            y,
            roll: 0f32,
            axis_pos: 0,
        }
    }

    pub fn light(&self) -> GridPosition {
        GridPosition {
            grid: self.grid,
            x: self.x,
            y: self.y,
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum Edge {
    Square {
        x: isize,
        y: isize,
    },
    Honney {
        x: isize,
        y: isize,
        start_parity: bool,
    },
    Circle(isize),
}

/// A view of the design's grids, with pre-computed maps.
#[derive(Clone, Default, Debug)]
pub struct GridData {
    // We borrow the grids and helices from the source that was used to build the view. This ensure
    // that the data used to build this view are not modified during the view's lifetime.
    pub(super) source_grids: Arc<Vec<GridDescriptor>>,
    source_helices: Helices,
    pub grids: Vec<Grid>,
    pub helix_to_pos: HashMap<usize, HelixGridPosition>,
    pub pos_to_helix: HashMap<GridPosition, usize>,
    pub parameters: Parameters,
    pub no_phantoms: HashSet<usize>,
    pub small_spheres: HashSet<usize>,
}

impl GridData {
    pub(super) fn is_up_to_date(&self, design: &Design) -> bool {
        Arc::ptr_eq(&self.source_grids, &design.grids)
            && Arc::ptr_eq(&self.source_helices.0, &design.helices.0)
    }

    pub fn get_visibility(&self, g_id: usize) -> bool {
        self.grids.get(g_id).map(|g| !g.invisible).unwrap_or(false)
    }

    pub fn new_by_updating_design(design: &mut Design) -> Self {
        let mut grids = Vec::new();
        let mut helix_to_pos = HashMap::new();
        let mut pos_to_helix = HashMap::new();
        let parameters = design.parameters.unwrap_or_default();
        let source_grids = design.grids.clone();
        for desc in source_grids.iter() {
            let grid = desc.to_grid(parameters.clone());
            grids.push(grid);
        }
        let source_helices = design.helices.clone();
        for (h_id, h) in design.helices.iter() {
            if let Some(grid_position) = h.grid_position {
                helix_to_pos.insert(*h_id, grid_position);
                pos_to_helix.insert(grid_position.light(), *h_id);
            }
        }

        let mut ret = Self {
            source_grids,
            source_helices,
            grids,
            helix_to_pos,
            pos_to_helix,
            parameters: design.parameters.unwrap_or_default(),
            no_phantoms: design.no_phantoms.clone(),
            small_spheres: design.small_spheres.clone(),
        };
        ret.reposition_all_helices();
        ret.update_all_curves(Arc::make_mut(&mut design.cached_curve));
        ret.update_support_helices();
        design.helices = ret.source_helices.clone();
        ret
    }

    #[allow(dead_code)]
    pub fn get_empty_grids_id(&self) -> HashSet<usize> {
        let mut ret: HashSet<usize> = (0..self.grids.len()).collect();
        for position in self.pos_to_helix.keys() {
            ret.remove(&position.grid);
        }
        ret
    }

    /// Reposition all the helices at their correct space position
    fn reposition_all_helices(&mut self) {
        let mut helices_mut = self.source_helices.make_mut();
        for (h_id, h) in helices_mut.iter_mut() {
            if let Some(grid_position) = h.grid_position {
                self.helix_to_pos.insert(*h_id, grid_position);
                self.pos_to_helix.insert(grid_position.light(), *h_id);
                let grid = &self.grids[grid_position.grid];

                h.position = grid.position_helix(grid_position.x, grid_position.y);
                h.orientation = {
                    let orientation = grid.orientation_helix(grid_position.x, grid_position.y);
                    let normal =
                        -self.parameters.helix_radius * Vec3::unit_y().rotated_by(orientation);
                    let actual = -self.parameters.helix_radius
                        * Vec3::unit_y().rotated_by(orientation)
                        * grid_position.roll.cos()
                        - self.parameters.helix_radius
                            * Vec3::unit_z().rotated_by(orientation)
                            * grid_position.roll.sin();
                    let roll = Rotor3::from_rotation_between(normal, actual);
                    (roll * grid.orientation_helix(grid_position.x, grid_position.y)).normalized()
                };
                if let Axis::Line { direction, .. } = h.get_axis(&self.parameters) {
                    h.position -= grid_position.axis_pos as f32 * direction;
                }
                if let Some(curve) = grid.make_curve(grid_position.x, grid_position.y) {
                    log::info!("setting curve");
                    h.curve = Some(curve)
                }
            }
        }
    }

    fn update_all_curves(&mut self, cached_curve: &mut CurveCache) {
        let need_update = self
            .source_helices
            .values()
            .any(|h| h.need_curve_update(&self.source_grids));

        if need_update {
            let mut new_helices = self.source_helices.clone();
            for h in new_helices.make_mut().values_mut() {
                self.update_curve(h, cached_curve);
            }
            self.source_helices = new_helices;
        }
    }

    /// Recompute the position of helix `h_id` on its grid. Return false if there is already an
    /// other helix at that position, otherwise return true.
    pub(super) fn reattach_helix<'a>(
        &mut self,
        h_id: usize,
        preserve_roll: bool,
        authorized_collisions: &[usize],
    ) -> Result<(), ErrOperation> {
        let mut helices = self.source_helices.make_mut();
        let h = helices.get_mut(&h_id);
        if h.is_none() {
            return Err(ErrOperation::HelixDoesNotExists(h_id));
        }
        let h = h.unwrap();
        let axis = h.get_axis(&self.parameters);
        if let Some(old_grid_position) = h.grid_position {
            let g = &self.grids[old_grid_position.grid];
            if let Axis::Line { origin, direction } = axis {
                if g.interpolate_helix(origin, direction).is_some() {
                    let old_roll = h.grid_position.map(|gp| gp.roll).filter(|_| preserve_roll);
                    let candidate_position = g
                        .find_helix_position(h, old_grid_position.grid)
                        .map(|g| g.with_roll(old_roll));
                    if let Some(new_grid_position) = candidate_position {
                        if let Some(helix) = self.pos_to_helix.get(&new_grid_position.light()) {
                            log::info!(
                                "{} collides with {}. Authorized collisions are {:?}",
                                h_id,
                                helix,
                                authorized_collisions
                            );
                            if authorized_collisions.contains(&helix) {
                                h.grid_position = candidate_position;
                                h.position = g
                                    .position_helix(new_grid_position.x, new_grid_position.y)
                                    - h.get_axis(&self.parameters)
                                        .direction()
                                        .unwrap_or(Vec3::zero())
                            } else {
                                return Err(ErrOperation::HelixCollisionDuringTranslation);
                            }
                        } else {
                            h.grid_position = candidate_position;
                            h.position = g.position_helix(new_grid_position.x, new_grid_position.y)
                                - h.get_axis(&self.parameters)
                                    .direction()
                                    .unwrap_or(Vec3::zero())
                                    * new_grid_position.axis_pos as f32
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn attach_to(&self, helix: &Helix, g_id: usize) -> Option<HelixGridPosition> {
        let mut ret = None;
        if let Some(g) = self.grids.get(g_id) {
            ret = g.find_helix_position(helix, g_id)
        }
        ret
    }

    fn find_grid_for_group(&self, group: &[usize]) -> GridDescriptor {
        use std::f32::consts::FRAC_PI_2;
        let parameters = self.parameters;
        let leader = self.source_helices.get(&group[0]).unwrap();
        let orientation = Rotor3::from_rotation_between(
            Vec3::unit_x(),
            leader
                .get_axis(&parameters)
                .direction()
                .unwrap_or(Vec3::zero())
                .normalized(),
        );
        let mut hex_grid = Grid::new(
            leader.position,
            orientation,
            self.parameters,
            GridType::honneycomb(),
        );
        let mut best_err = hex_grid.error_group(&group, &self.source_helices);
        for dx in [-1, 0, 1].iter() {
            for dy in [-1, 0, 1].iter() {
                let position = hex_grid.position_helix(*dx, *dy);
                for i in 0..100 {
                    let angle = i as f32 * FRAC_PI_2 / 100.;
                    let rotor = Rotor3::from_rotation_yz(angle);
                    let grid = Grid::new(
                        position,
                        orientation.rotated_by(rotor),
                        self.parameters,
                        GridType::honneycomb(),
                    );
                    let err = grid.error_group(group, &self.source_helices);
                    if err < best_err {
                        hex_grid = grid;
                        best_err = err
                    }
                }
            }
        }

        let mut square_grid = Grid::new(
            leader.position,
            leader.orientation,
            self.parameters,
            GridType::square(),
        );
        let mut best_square_err = square_grid.error_group(&group, &self.source_helices);
        for i in 0..100 {
            let angle = i as f32 * FRAC_PI_2 / 100.;
            let rotor = Rotor3::from_rotation_yz(angle);
            let grid = Grid::new(
                leader.position,
                orientation.rotated_by(rotor),
                self.parameters,
                GridType::square(),
            );
            let err = grid.error_group(group, &self.source_helices);
            if err < best_square_err {
                square_grid = grid;
                best_square_err = err
            }
        }
        if best_square_err < best_err {
            GridDescriptor {
                position: square_grid.position,
                orientation: square_grid.orientation,
                grid_type: GridTypeDescr::Square,
                invisible: square_grid.invisible,
            }
        } else {
            GridDescriptor {
                position: hex_grid.position,
                orientation: hex_grid.orientation,
                grid_type: GridTypeDescr::Honeycomb,
                invisible: hex_grid.invisible,
            }
        }
    }

    /// Retrun the edge between two grid position. Return None if the position are not in the same
    /// grid.
    pub fn get_edge(&self, pos1: &HelixGridPosition, pos2: &HelixGridPosition) -> Option<Edge> {
        if pos1.grid == pos2.grid {
            self.grids.get(pos1.grid).map(|g| {
                g.grid_type
                    .translation_to_edge(pos1.x, pos1.y, pos2.x, pos2.y)
            })
        } else {
            None
        }
    }

    pub fn translate_by_edge(
        &self,
        pos1: &HelixGridPosition,
        edge: &Edge,
    ) -> Option<HelixGridPosition> {
        let position = self
            .grids
            .get(pos1.grid)
            .and_then(|g| g.grid_type.translate_by_edge(pos1.x, pos1.y, *edge))?;
        Some(HelixGridPosition {
            grid: pos1.grid,
            x: position.0,
            y: position.1,
            roll: 0f32,
            axis_pos: 0,
        })
    }

    pub fn pos_to_helix(&self, position: GridPosition) -> Option<usize> {
        self.pos_to_helix.get(&position).cloned()
    }

    pub fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
        if self.grids.len() > g_id {
            Some(
                self.pos_to_helix
                    .iter()
                    .filter(|(pos, _)| pos.grid == g_id)
                    .map(|(_, h)| h)
                    .cloned()
                    .collect(),
            )
        } else {
            None
        }
    }
}

trait GridApprox {
    fn error_group(&self, group: &[usize], helices: &Helices) -> f32;
    fn error_helix(&self, origin: Vec3, direction: Vec3) -> f32;
}

impl GridApprox for Grid {
    fn error_group(&self, group: &[usize], helices: &Helices) -> f32 {
        let mut ret = 0f32;
        for h_id in group.iter() {
            let helix = helices.get(h_id).unwrap();
            let axis = helix.get_axis(&self.parameters);
            if let Axis::Line { origin, direction } = axis {
                ret += self.error_helix(origin, direction);
            }
        }
        ret
    }

    fn error_helix(&self, origin: Vec3, direction: Vec3) -> f32 {
        let position_descrete = self
            .interpolate_helix(origin, direction)
            .map(|(x, y)| self.position_helix(x, y));
        if let Some(position) = self.real_intersection(origin, direction) {
            (position - position_descrete.unwrap()).mag_sq()
        } else {
            std::f32::INFINITY
        }
    }
}

pub(super) fn make_grid_from_helices(
    design: &mut Design,
    helices: &[usize],
) -> Result<(), ErrOperation> {
    if helices.len() < MIN_HELICES_TO_MAKE_GRID {
        return Err(ErrOperation::NotEnoughHelices {
            actual: helices.len(),
            needed: MIN_HELICES_TO_MAKE_GRID,
        });
    }
    let grid_data = design.get_updated_grid_data();
    let desc = grid_data.find_grid_for_group(helices);
    let mut new_grids = Vec::clone(grid_data.source_grids.as_ref());
    let mut new_helices = grid_data.source_helices.clone();
    new_grids.push(desc);
    let mut helices_mut = new_helices.make_mut();
    for h_id in helices.iter() {
        if let Some(h) = helices_mut.get_mut(h_id) {
            if h.grid_position.is_some() {
                continue;
            }
            if let Some(position) = grid_data.attach_to(h, grid_data.grids.len() - 1) {
                h.grid_position = Some(position)
            }
        }
    }
    drop(helices_mut);
    design.grids = Arc::new(new_grids);
    design.helices = new_helices;
    Ok(())
}

/// A mutable view to a design and it's grid data.
/// When this view is droped, the design's helices are automatically updated.
pub(super) struct HelicesTranslator<'a> {
    design: &'a mut Design,
    grid_data: GridData,
}

impl<'a> Drop for HelicesTranslator<'a> {
    fn drop(&mut self) {
        self.design.helices = self.grid_data.source_helices.clone();
    }
}

impl<'a> HelicesTranslator<'a> {
    pub fn from_design(design: &'a mut Design) -> Self {
        let grid_data = GridData::new_by_updating_design(design);
        Self { design, grid_data }
    }

    pub fn translate_helices(
        &mut self,
        snap: bool,
        helices: Vec<usize>,
        translation: Vec3,
    ) -> Result<(), ErrOperation> {
        let mut new_helices = self.grid_data.source_helices.make_mut();
        for h_id in helices.iter() {
            if let Some(h) = new_helices.get_mut(h_id) {
                h.translate(translation);
            }
        }
        drop(new_helices);
        if snap {
            self.attempt_reattach(&helices)
        } else {
            Ok(())
        }
    }

    pub fn rotate_helices_3d(
        &mut self,
        snap: bool,
        helices: Vec<usize>,
        rotation: Rotor3,
        origin: Vec3,
    ) -> Result<(), ErrOperation> {
        let mut new_helices = self.grid_data.source_helices.make_mut();
        for h_id in helices.iter() {
            if let Some(h) = new_helices.get_mut(h_id) {
                h.rotate_arround(rotation, origin)
            }
        }
        drop(new_helices);
        if snap {
            self.attempt_reattach(&helices)
        } else {
            Ok(())
        }
    }

    fn attempt_reattach(&mut self, helices: &[usize]) -> Result<(), ErrOperation> {
        for h_id in helices.iter() {
            self.grid_data.reattach_helix(*h_id, true, helices)?;
        }
        Ok(())
    }
}

impl GridPositionProvider for GridData {
    fn position(&self, g_id: usize, x: isize, y: isize) -> Vec3 {
        if let Some(grid) = self.grids.get(g_id) {
            grid.position_helix(x, y)
        } else {
            log::error!("Attempt to get position on unexisting grid. This is a bug");
            Vec3::zero()
        }
    }

    fn source(&self) -> Arc<Vec<GridDescriptor>> {
        self.source_grids.clone()
    }
}

impl GridData {
    fn update_instanciated_curve_descriptor(&self, helix: &mut Helix) {
        if let Some(curve) = helix.curve.clone() {
            helix.instanciated_descriptor = Some(Arc::new(
                InstanciatedCurveDescriptor::instanciate(curve, self),
            ))
        } else {
            helix.instanciated_descriptor = None;
        }
    }

    pub(super) fn update_curve(&self, helix: &mut Helix, cached_curve: &mut CurveCache) {
        if helix.need_curve_descriptor_update(&self.source_grids) {
            self.update_instanciated_curve_descriptor(helix)
        }

        if let Some(desc) = helix.instanciated_descriptor.as_ref() {
            let curve = desc.make_curve(&self.parameters, cached_curve);
            helix.instanciated_curve = Some(InstanciatedCurve {
                curve,
                source: desc.clone(),
            })
        }
    }

    fn update_support_helices(&mut self) {
        let old_rolls: Vec<f32> = self.source_helices.values().map(|h| h.roll).collect();
        let mut helices_mut = self.source_helices.make_mut();
        for h in helices_mut.values_mut() {
            if let Some(mother_id) = h.support_helix {
                if let Some(mother_roll) = old_rolls.get(mother_id) {
                    h.roll = *mother_roll;
                }
            }
        }
    }
}
