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
mod hyperboloid;
pub use hyperboloid::*;

use super::icednano::{Design, Parameters};
use super::{icednano, Data};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f32::consts::FRAC_PI_2;
use std::sync::{Arc, RwLock};
use ultraviolet::{Rotor3, Vec2, Vec3};

use crate::scene::GridInstance;

#[derive(Clone, Debug)]
pub struct Grid {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub parameters: Parameters,
    pub grid_type: GridType,
    old_position: Vec3,
    old_orientation: Rotor3,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GridDescriptor {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub grid_type: GridTypeDescr,
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
    },
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
            old_position: position,
            old_orientation: orientation,
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

    fn real_intersection(&self, origin: Vec3, direction: Vec3) -> Option<Vec3> {
        let d = self.ray_intersection(origin, direction)?;
        let intersection = origin + d * direction;
        Some(intersection)
    }

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
        helix: &super::icednano::Helix,
        g_id: usize,
    ) -> Option<GridPosition> {
        let super::icednano::Axis { origin, direction } = helix.get_axis(&self.parameters);
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
        Some(GridPosition {
            grid: g_id,
            x,
            y,
            axis_pos: axis_intersection,
            roll,
        })
    }

    fn error_group(&self, group: &[usize], design: &Design) -> f32 {
        let mut ret = 0f32;
        for h_id in group.iter() {
            let helix = design.helices.get(h_id).unwrap();
            let axis = helix.get_axis(&self.parameters);
            ret += self.error_helix(axis.origin, axis.direction);
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

    pub fn desc(&self) -> GridDescriptor {
        GridDescriptor {
            position: self.position,
            orientation: self.orientation,
            grid_type: self.grid_type.descr(),
        }
    }

    fn append_translation(&mut self, translation: Vec3) {
        self.position += translation;
    }

    fn append_rotation(&mut self, rotation: Rotor3) {
        self.orientation = rotation * self.orientation;
        self.position = rotation * self.position;
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.position = self.old_position;
        self.append_translation(translation)
    }

    pub fn rotate_arround(&mut self, rotation: Rotor3, origin: Vec3) {
        self.orientation = self.old_orientation;
        self.position = self.old_position;
        self.append_translation(-origin);
        self.append_rotation(rotation);
        self.append_translation(origin);
    }

    pub fn end_movement(&mut self) {
        self.old_position = self.position;
        self.old_orientation = self.orientation;
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
pub struct GridPosition {
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

impl GridPosition {
    pub fn with_roll(self, roll: Option<f32>) -> Self {
        if let Some(roll) = roll {
            Self { roll, ..self }
        } else {
            self
        }
    }
}

pub(super) struct GridManager {
    pub grids: Vec<Grid>,
    helix_to_pos: HashMap<usize, GridPosition>,
    pos_to_helix: HashMap<(usize, isize, isize), usize>,
    parameters: Parameters,
    pub no_phantoms: HashSet<usize>,
    pub small_spheres: HashSet<usize>,
    pub visibility: HashMap<usize, bool>,
}

impl GridManager {
    pub fn new(parameters: Parameters) -> Self {
        Self {
            grids: Vec::new(),
            helix_to_pos: HashMap::new(),
            pos_to_helix: HashMap::new(),
            parameters,
            no_phantoms: HashSet::new(),
            small_spheres: HashSet::new(),
            visibility: HashMap::new(),
        }
    }

    pub fn set_visibility(&mut self, g_id: usize, visibility: bool) {
        self.visibility.insert(g_id, visibility);
    }

    pub fn get_visibility(&mut self, g_id: usize) -> bool {
        self.visibility.get(&g_id).cloned().unwrap_or(true)
    }

    /*
    pub fn get_helix_at_pos(&self, grid: usize, x: isize, y: isize) -> Option<usize> {
        for (h, g) in self.helix_to_pos.iter() {
            if let GridPosition {
                grid,
                x,
                y,
                ..} = *g {
                return Some(*h);
            }
        }
        return None;
    }*/

    pub fn remove_helix(&mut self, h_id: usize) {
        let pos = self.helix_to_pos.remove(&h_id);
        if let Some(pos) = pos {
            self.pos_to_helix.remove(&(pos.grid, pos.x, pos.y));
        }
        self.small_spheres.remove(&h_id);
        self.no_phantoms.remove(&h_id);
    }

    pub fn new_from_design(design: &Design) -> Self {
        let mut grids = Vec::new();
        let mut helix_to_pos = HashMap::new();
        let mut pos_to_helix = HashMap::new();
        for desc in design.grids.iter() {
            match desc.grid_type {
                GridTypeDescr::Square => {
                    let grid: Grid = Grid::new(
                        desc.position,
                        desc.orientation,
                        design.parameters.unwrap_or_default(),
                        GridType::square(),
                    );
                    grids.push(grid);
                }
                GridTypeDescr::Honeycomb => {
                    let grid: Grid = Grid::new(
                        desc.position,
                        desc.orientation,
                        design.parameters.unwrap_or_default(),
                        GridType::honneycomb(),
                    );
                    grids.push(grid);
                }
                GridTypeDescr::Hyperboloid {
                    radius,
                    radius_shift,
                    length,
                    shift,
                    forced_radius,
                } => {
                    let grid = Grid::new(
                        desc.position,
                        desc.orientation,
                        design.parameters.unwrap_or_default(),
                        GridType::Hyperboloid(Hyperboloid {
                            radius,
                            shift,
                            length,
                            radius_shift,
                            forced_radius,
                        }),
                    );
                    grids.push(grid);
                }
            }
        }
        for (h_id, h) in design.helices.iter() {
            if let Some(grid_position) = h.grid_position {
                helix_to_pos.insert(*h_id, grid_position);
                pos_to_helix.insert(
                    (grid_position.grid, grid_position.x, grid_position.y),
                    *h_id,
                );
            }
        }

        Self {
            grids,
            helix_to_pos,
            pos_to_helix,
            parameters: design.parameters.unwrap_or_default(),
            no_phantoms: design.no_phantoms.clone(),
            small_spheres: design.small_spheres.clone(),
            visibility: Default::default(),
        }
    }

    pub fn grid_instances(&self, design_id: usize) -> Vec<GridInstance> {
        let mut ret = Vec::new();
        for (n, g) in self.grids.iter().enumerate() {
            let grid = GridInstance {
                grid: g.clone(),
                min_x: -2,
                max_x: 2,
                min_y: -2,
                max_y: 2,
                color: 0x00_00_FF,
                design: design_id,
                id: n,
                fake: false,
                visible: *self.visibility.get(&n).unwrap_or(&true),
            };
            ret.push(grid);
        }
        for grid_position in self.helix_to_pos.values() {
            let grid = grid_position.grid;
            ret[grid].min_x = ret[grid].min_x.min(grid_position.x as i32 - 2);
            ret[grid].max_x = ret[grid].max_x.max(grid_position.x as i32 + 2);
            ret[grid].min_y = ret[grid].min_y.min(grid_position.y as i32 - 2);
            ret[grid].max_y = ret[grid].max_y.max(grid_position.y as i32 + 2);
        }
        ret
    }

    pub fn make_grid_from_helices(&mut self, design: &mut Design, helices: &[usize]) {
        if helices.len() < 4 {
            return;
        }
        let desc = self.find_grid_for_group(helices, design);
        match desc.grid_type {
            GridTypeDescr::Square => {
                let grid: Grid = Grid::new(
                    desc.position,
                    desc.orientation,
                    design.parameters.unwrap_or_default(),
                    GridType::square(),
                );
                self.grids.push(grid);
            }
            GridTypeDescr::Honeycomb => {
                let grid: Grid = Grid::new(
                    desc.position,
                    desc.orientation,
                    design.parameters.unwrap_or_default(),
                    GridType::honneycomb(),
                );
                self.grids.push(grid);
            }
            GridTypeDescr::Hyperboloid {
                radius,
                shift,
                length,
                radius_shift,
                forced_radius,
            } => {
                let grid = Grid::new(
                    desc.position,
                    desc.orientation,
                    design.parameters.unwrap_or_default(),
                    GridType::hyperboloid(Hyperboloid {
                        radius,
                        radius_shift,
                        length,
                        shift,
                        forced_radius,
                    }),
                );
                self.grids.push(grid);
            }
        }
        for h_id in helices.iter() {
            if let Some(h) = design.helices.get_mut(h_id) {
                if h.grid_position.is_some() {
                    continue;
                }
                if let Some(position) = self.attach_to(h, self.grids.len() - 1) {
                    h.grid_position = Some(position)
                }
            }
        }
    }

    pub fn update(&mut self, design: &mut Design) {
        for (h_id, h) in design.helices.iter_mut() {
            if let Some(grid_position) = h.grid_position {
                self.helix_to_pos.insert(*h_id, grid_position);
                self.pos_to_helix.insert(
                    (grid_position.grid, grid_position.x, grid_position.y),
                    *h_id,
                );
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
                h.position -=
                    grid_position.axis_pos as f32 * h.get_axis(&self.parameters).direction;
            }
        }
        design.grids.clear();
        for g in self.grids.iter() {
            design.grids.push(g.desc());
        }
    }

    /// Recompute the position of helix `h_id` on its grid. Return false if there is already an
    /// other helix at that position, otherwise return true.
    pub fn reattach_helix(
        &mut self,
        h_id: usize,
        design: &mut Design,
        preserve_roll: bool,
        grid2ds: &Vec<Arc<RwLock<Grid2D>>>,
    ) -> bool {
        let h = design.helices.get_mut(&h_id).unwrap();
        let axis = h.get_axis(&self.parameters);
        if let Some(grid_position) = h.grid_position {
            let g = &self.grids[grid_position.grid];
            if let Some(_) = g.interpolate_helix(axis.origin, axis.direction) {
                let old_roll = h.grid_position.map(|gp| gp.roll).filter(|_| preserve_roll);
                let candidate_position = g
                    .find_helix_position(h, grid_position.grid)
                    .map(|g| g.with_roll(old_roll));
                if let Some(grid_pos) = candidate_position {
                    if let Some(helix) = grid2ds[grid_pos.grid]
                        .read()
                        .unwrap()
                        .helices
                        .get(&(grid_pos.x, grid_pos.y))
                    {
                        if *helix == h_id {
                            h.grid_position = candidate_position;
                        } else {
                            return false;
                        }
                    } else {
                        h.grid_position = candidate_position;
                    }
                }
            }
        }
        true
    }

    fn attach_to(&self, helix: &icednano::Helix, g_id: usize) -> Option<GridPosition> {
        let mut ret = None;
        if let Some(g) = self.grids.get(g_id) {
            ret = g.find_helix_position(helix, g_id)
        }
        ret
    }

    fn find_grid_for_group(&self, group: &[usize], design: &Design) -> GridDescriptor {
        let parameters = design.parameters.unwrap_or_default();
        let leader = design.helices.get(&group[0]).unwrap();
        let orientation = Rotor3::from_rotation_between(
            Vec3::unit_x(),
            leader.get_axis(&parameters).direction.normalized(),
        );
        let mut hex_grid = Grid::new(
            leader.position,
            orientation,
            design.parameters.unwrap_or_default(),
            GridType::honneycomb(),
        );
        let mut best_err = hex_grid.error_group(&group, design);
        for dx in [-1, 0, 1].iter() {
            for dy in [-1, 0, 1].iter() {
                let position = hex_grid.position_helix(*dx, *dy);
                for i in 0..100 {
                    let angle = i as f32 * FRAC_PI_2 / 100.;
                    let rotor = Rotor3::from_rotation_yz(angle);
                    let grid = Grid::new(
                        position,
                        orientation.rotated_by(rotor),
                        design.parameters.unwrap_or_default(),
                        GridType::honneycomb(),
                    );
                    let err = grid.error_group(group, design);
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
            design.parameters.unwrap_or_default(),
            GridType::square(),
        );
        let mut best_square_err = square_grid.error_group(&group, design);
        for i in 0..100 {
            let angle = i as f32 * FRAC_PI_2 / 100.;
            let rotor = Rotor3::from_rotation_yz(angle);
            let grid = Grid::new(
                leader.position,
                orientation.rotated_by(rotor),
                design.parameters.unwrap_or_default(),
                GridType::square(),
            );
            let err = grid.error_group(group, design);
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
            }
        } else {
            GridDescriptor {
                position: hex_grid.position,
                orientation: hex_grid.orientation,
                grid_type: GridTypeDescr::Honeycomb,
            }
        }
    }

    pub fn grids2d(&self) -> Vec<Arc<RwLock<Grid2D>>> {
        let mut ret = Vec::new();
        for (n, g) in self.grids.iter().enumerate() {
            ret.push(Arc::new(RwLock::new(Grid2D::new(
                n,
                g.grid_type.clone(),
                self.parameters,
                !self.no_phantoms.contains(&n),
                self.small_spheres.contains(&n),
            ))));
        }
        ret
    }

    pub fn rotate_grid_arround(&mut self, g_id: usize, rotation: Rotor3, origin: Vec3) {
        self.grids[g_id].rotate_arround(rotation, origin)
    }

    pub fn translate_grid(&mut self, g_id: usize, translation: Vec3) {
        self.grids[g_id].translate(translation)
    }

    pub fn terminate_movement(&mut self) {
        for g in self.grids.iter_mut() {
            g.end_movement()
        }
    }

    pub fn delete_last_grid(&mut self) {
        self.grids.pop();
    }

    pub fn add_grid(&mut self, desc: GridDescriptor) -> usize {
        match desc.grid_type {
            GridTypeDescr::Square => {
                let grid: Grid = Grid::new(
                    desc.position,
                    desc.orientation,
                    self.parameters,
                    GridType::square(),
                );
                self.grids.push(grid);
            }
            GridTypeDescr::Honeycomb => {
                let grid: Grid = Grid::new(
                    desc.position,
                    desc.orientation,
                    self.parameters,
                    GridType::honneycomb(),
                );
                self.grids.push(grid);
            }
            GridTypeDescr::Hyperboloid {
                radius,
                shift,
                length,
                radius_shift,
                forced_radius,
            } => {
                let grid = Grid::new(
                    desc.position,
                    desc.orientation,
                    self.parameters,
                    GridType::hyperboloid(Hyperboloid {
                        radius,
                        shift,
                        length,
                        radius_shift,
                        forced_radius,
                    }),
                );
                self.grids.push(grid)
            }
        }
        self.grids.len() - 1
    }

    /// Retrun the edge between two grid position. Return None if the position are not in the same
    /// grid.
    pub fn get_edge(&self, pos1: &GridPosition, pos2: &GridPosition) -> Option<Edge> {
        if pos1.grid == pos2.grid {
            self.grids.get(pos1.grid).map(|g| {
                g.grid_type
                    .translation_to_edge(pos1.x, pos1.y, pos2.x, pos2.y)
            })
        } else {
            None
        }
    }

    pub fn translate_by_edge(&self, pos1: &GridPosition, edge: &Edge) -> Option<GridPosition> {
        let position = self
            .grids
            .get(pos1.grid)
            .and_then(|g| g.grid_type.translate_by_edge(pos1.x, pos1.y, *edge))?;
        Some(GridPosition {
            grid: pos1.grid,
            x: position.0,
            y: position.1,
            roll: 0f32,
            axis_pos: 0,
        })
    }

    pub fn pos_to_helix(&self, grid: usize, x: isize, y: isize) -> Option<usize> {
        self.pos_to_helix.get(&(grid, x, y)).cloned()
    }
}

impl Data {
    pub fn find_parallel_helices(&self) -> HashMap<usize, Vec<usize>> {
        let mut ret = HashMap::new();
        let mut merger = GroupMerger::new(self.design.helices.len());
        let mut candidates: HashMap<(usize, usize), usize> = HashMap::new();

        for s in self.design.strands.values() {
            let mut current_helix: Option<usize> = None;
            for d in s.domains.iter() {
                match d {
                    icednano::Domain::HelixDomain(helix_interval) => {
                        let new_helix = helix_interval.helix;
                        if let Some(helix) = current_helix.take() {
                            if helix != new_helix {
                                let nb_cross = candidates
                                    .entry((helix.min(new_helix), helix.max(new_helix)))
                                    .or_insert(0);
                                *nb_cross += 1;
                                if *nb_cross >= 3 {
                                    merger.union(helix, new_helix);
                                }
                            }
                        }
                        current_helix = Some(new_helix);
                    }
                    _ => (),
                }
            }
        }

        for h_id in self.design.helices.keys() {
            let group_id = merger.find(*h_id);
            let group = ret.entry(group_id).or_insert(vec![]);
            group.push(*h_id);
        }
        ret
    }
}

struct GroupMerger {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl GroupMerger {
    pub fn new(nb_element: usize) -> Self {
        Self {
            parent: (0..nb_element).collect(),
            rank: vec![0; nb_element],
        }
    }

    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            let y = self.find(self.parent[x]);
            self.parent[x] = y;
            y
        } else {
            x
        }
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let xroot = self.find(x);
        let yroot = self.find(y);
        if xroot != yroot {
            if self.rank[xroot] < self.rank[yroot] {
                self.parent[xroot] = yroot;
            } else if self.rank[xroot] > self.rank[yroot] {
                self.parent[yroot] = xroot;
            } else {
                self.parent[yroot] = xroot;
                self.rank[xroot] += 1;
            }
        }
    }
}

pub struct Grid2D {
    helices: BTreeMap<(isize, isize), usize>,
    grid_type: GridType,
    parameters: Parameters,
    id: usize,
    pub persistent_phantom: bool,
    pub small_spheres: bool,
}

impl Grid2D {
    pub fn new(
        id: usize,
        grid_type: GridType,
        parameters: Parameters,
        persistent_phantom: bool,
        small_spheres: bool,
    ) -> Self {
        Self {
            helices: BTreeMap::new(),
            grid_type,
            parameters,
            id,
            persistent_phantom,
            small_spheres,
        }
    }

    pub fn update(&mut self, design: &Design) {
        for (h_id, h) in design.helices.iter() {
            if let Some(grid_position) = h.grid_position.filter(|gp| gp.grid == self.id) {
                self.helices
                    .insert((grid_position.x, grid_position.y), *h_id);
            }
        }
    }

    pub fn helices(&self) -> &BTreeMap<(isize, isize), usize> {
        &self.helices
    }

    pub fn helix_position(&self, x: isize, y: isize) -> Vec2 {
        self.grid_type.origin_helix(&self.parameters, x, y)
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
