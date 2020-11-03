
use std::marker::PhantomData;
use std::collections::HashMap;
use super::icednano::{Design, Parameters};
use ultraviolet::{Rotor3, Vec3, Vec2};

use crate::scene::{GridType as GridType_, GridInstance};

pub struct Grid<T: GridDivision> {
    pub position: Vec3,
    pub orientation: Rotor3,
    parameters: Parameters,
    phantom_data: PhantomData<T>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct GridDescriptor {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub grid_type: GridType
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum GridType {
    Square,
    Honeycomb,
}

impl<T: GridDivision> Grid<T> {
    pub fn new(position: Vec3, orientation: Rotor3, parameters: Parameters) -> Self {
        Self {
            position,
            orientation,
            parameters,
            phantom_data: PhantomData
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
        let normal = Vec3::unit_x().rotated_by(self.orientation);
        let denom = direction.normalized().dot(normal);
        if denom < 1e-3 {
            None
        } else {
            let d = (self.position - origin).dot(normal) / denom;
            let intersection = origin + d * direction;
            let z_vec = Vec3::unit_z().rotated_by(self.orientation);
            let y_vec = Vec3::unit_y().rotated_by(self.orientation);
            Some(Vec2::new((intersection - self.position).dot(z_vec), (intersection - self.position).dot(y_vec)))
        }
    }

    pub fn axis_helix(&self) -> Vec3 {
        Vec3::unit_z().rotated_by(self.orientation)
    }

    pub fn position_helix(&self, x: isize, y: isize) -> Vec3 {
        let origin = T::origin_helix(&self.parameters, x, y);
        let z_vec = Vec3::unit_z().rotated_by(self.orientation);
        let y_vec = Vec3::unit_y().rotated_by(self.orientation);
        self.position + origin.x * z_vec + origin.y * y_vec
    }

    pub fn interpolate_helix(&self, origin: Vec3, axis: Vec3) -> Option<(isize, isize)> {
        let intersection = self.line_intersection(origin, axis)?;
        Some(T::interpolate(&self.parameters, intersection.x, intersection.y))
    }
}

pub trait GridDivision {

    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2;
    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize);

}

pub struct SquareGrid;

impl GridDivision for SquareGrid {
    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        Vec2::new(
            x as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
            y as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
        )
    }

    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        (
            (x / (parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize,
            (y / (parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize
        )
    }
}

pub struct HoneyComb; 

impl GridDivision for HoneyComb {
    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let lower = 3. * r * y as f32;
        let upper = lower + r;
        Vec2::new(
            x as f32 * r * 3f32.sqrt(),
            if x % 2 == y % 2 {lower} else {upper},
        )
    }

    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let first_guess = (
            (x / (r * 3f32.sqrt())).round() as isize,
            (y / (3. * r)).floor() as isize
        );

        let mut ret = first_guess;
        let mut best_dist = (Self::origin_helix(parameters, first_guess.0, first_guess.1) - Vec2::new(x, y)).mag();
        for dx in [-1, 0, 1].iter() {
            for dy in [-1, 0, 1].iter() {
                let guess = (first_guess.0 + dx, first_guess.1 + dy);
                let dist = (Self::origin_helix(parameters, guess.0, guess.1) - Vec2::new(x, y)).mag();
                if dist < best_dist {
                    ret = guess;
                    best_dist = dist;
                }
            }
        }
        ret
    }

}

#[derive(Clone, Serialize, Deserialize, Copy)]
pub struct GridPosition {
    grid: usize,
    x: isize,
    y: isize,
}

pub struct GridManager {
    nb_grid: usize,
    square_grids: HashMap<usize, Grid<SquareGrid>>,
    honeycomb_grids: HashMap<usize, Grid<HoneyComb>>,
    helix_to_pos: HashMap<usize, GridPosition>,
}

impl GridManager {
    pub fn new() -> Self {
        Self {
            nb_grid: 0,
            square_grids: HashMap::new(),
            honeycomb_grids: HashMap::new(),
            helix_to_pos: HashMap::new(),
        }
    }

    pub fn new_from_design(design: &Design) -> Self {
        let mut nb_grid = 0;
        let mut square_grids = HashMap::new();
        let mut honeycomb_grids = HashMap::new();
        let mut helix_to_pos = HashMap::new();
        for desc in design.grids.iter() {
            match desc.grid_type {
                GridType::Square => {
                    let grid: Grid<SquareGrid> = Grid::new(desc.position, desc.orientation, design.parameters.unwrap_or_default());
                    square_grids.insert(nb_grid, grid);
                    nb_grid += 1;
                }
                GridType::Honeycomb => {
                    let grid: Grid<HoneyComb> = Grid::new(desc.position, desc.orientation, design.parameters.unwrap_or_default());
                    honeycomb_grids.insert(nb_grid, grid);
                    nb_grid += 1;
                }
            }
        }
        for (h_id, h) in design.helices.iter() {
            if let Some(grid_position) = h.grid_position {
                helix_to_pos.insert(*h_id, grid_position);
            }
        }

        Self {
            nb_grid,
            square_grids,
            honeycomb_grids,
            helix_to_pos
        }
    }

    pub fn grid_instances(&self) -> Vec<GridInstance> {
        let mut ret = Vec::new();
        for i in 0..self.nb_grid {
            let grid = if let Some(original) = self.square_grids.get(&i) {
                GridInstance {
                    position: original.position,
                    orientation: original.orientation,
                    min_x: -2,
                    max_x: 2,
                    min_y: -2,
                    max_y: 2,
                    grid_type: GridType_::Square
                }
            } else {
                let original = self.honeycomb_grids.get(&i).unwrap();
                GridInstance {
                    position: original.position,
                    orientation: original.orientation,
                    min_x: -2,
                    max_x: 2,
                    min_y: -2,
                    max_y: 2,
                    grid_type: GridType_::Honeycomb
                }
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

}
