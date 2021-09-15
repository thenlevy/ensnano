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

use super::Parameters;
mod hyperboloid;
pub use hyperboloid::*;

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
            } => GridType::Hyperboloid(Hyperboloid {
                radius,
                shift,
                forced_radius,
                length,
                radius_shift,
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

    pub fn find_helix_position(&self, helix: &super::Helix, g_id: usize) -> Option<GridPosition> {
        let super::Axis { origin, direction } = helix.get_axis(&self.parameters);
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

    pub fn desc(&self) -> GridDescriptor {
        GridDescriptor {
            position: self.position,
            orientation: self.orientation,
            grid_type: self.grid_type.descr(),
            invisible: self.invisible,
        }
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

    pub fn from_grid_id_x_y(g_id: usize, x: isize, y: isize) -> Self {
        Self {
            grid: g_id,
            x,
            y,
            roll: 0f32,
            axis_pos: 0,
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
