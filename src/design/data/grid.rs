
use std::marker::PhantomData;
use std::collections::HashMap;
use super::icednano::{Design, Parameters};
use ultraviolet::{Rotor3, Vec3, Vec2};
use std::f32::consts::FRAC_PI_2;
use super::{icednano, Data};

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
        let intersection = self.real_intersection(origin, direction)?;
        let z_vec = Vec3::unit_z().rotated_by(self.orientation);
        let y_vec = Vec3::unit_y().rotated_by(self.orientation);
        Some(Vec2::new((intersection - self.position).dot(z_vec), (intersection - self.position).dot(y_vec)))
    }

    fn real_intersection(&self, origin: Vec3, direction: Vec3) -> Option<Vec3> {
        let normal = Vec3::unit_x().rotated_by(self.orientation);
        let denom = direction.normalized().dot(normal);
        if denom < 1e-3 {
            None
        } else {
            let d = (self.position - origin).dot(normal) / denom;
            let intersection = origin + d * direction;
            Some(intersection)
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

    fn error_group(&self, group: &Vec<usize>, design: &Design) -> f32 {
        let mut ret = 0f32;
        for h_id in group.iter() {
            let helix = design.helices.get(h_id).unwrap();
            let axis = helix.get_axis(&self.parameters);
            ret += self.error_helix(axis.origin, axis.direction);
        }
        ret
    }

    fn error_helix(&self, origin: Vec3, direction: Vec3) -> f32 {
        let position_descrete = self.interpolate_helix(origin, direction).map(|(x, y)| self.position_helix(x, y));
        if let Some(position) = self.real_intersection(origin, direction) {
            (position - position_descrete.unwrap()).mag_sq()
        } else {
            std::f32::INFINITY
        }
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
            -y as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
        )
    }

    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        (
            (x / (parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize,
            (y / -(parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize
        )
    }
}

pub struct HoneyComb; 

impl GridDivision for HoneyComb {
    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let upper = -3. * r * y as f32;
        let lower = upper - r;
        Vec2::new(
            x as f32 * r * 3f32.sqrt(),
            if x.abs() % 2 != y.abs() % 2 {lower} else {upper},
        )
    }

    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let first_guess = (
            (x / (r * 3f32.sqrt())).round() as isize,
            (y / (-3. * r)).floor() as isize
        );

        let mut ret = first_guess;
        let mut best_dist = (Self::origin_helix(parameters, first_guess.0, first_guess.1) - Vec2::new(x, y)).mag_sq();
        for dx in [-2, -1, 0, 1, 2].iter() {
            for dy in [-2, -1, 0, 1, 2].iter() {
                let guess = (first_guess.0 + dx, first_guess.1 + dy);
                let dist = (Self::origin_helix(parameters, guess.0, guess.1) - Vec2::new(x, y)).mag_sq();
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

    pub fn create_grids(&mut self, design: &mut Design) {
        let parameters = design.parameters.unwrap_or_default();
        //let new_square_grids = Vec::new();
        for (h_id, h) in design.helices.iter_mut() {
            if h.grid_position.is_some() {
                continue;
            }
            let axis = h.get_axis(&parameters);
            if let Some(position) = self.attach_existing(axis.origin, axis.direction) {
                h.grid_position = Some(position)
            }
        }
    }

    pub fn guess_grids(&mut self, design: &mut Design, groups: &HashMap<usize, Vec<usize>>) {
        for group in groups.values() {
            if group.len() < 4 {
                continue;
            }
            let desc = self.find_grid_for_group(group, design);
            match desc.grid_type {
                GridType::Square => {
                    let grid: Grid<SquareGrid> = Grid::new(desc.position, desc.orientation, design.parameters.unwrap_or_default());
                    self.square_grids.insert(self.nb_grid, grid);
                    self.nb_grid += 1;
                }
                GridType::Honeycomb => {
                    let grid: Grid<HoneyComb> = Grid::new(desc.position, desc.orientation, design.parameters.unwrap_or_default());
                    self.honeycomb_grids.insert(self.nb_grid, grid);
                    self.nb_grid += 1;
                }
            }
        }
        for (h_id, h) in design.helices.iter_mut() {
            if h.grid_position.is_some() {
                continue;
            }
            let axis = h.get_axis(&design.parameters.unwrap_or_default());
            if let Some(position) = self.attach_existing(axis.origin, axis.direction) {
                h.grid_position = Some(position)
            }
        }
    }

    pub fn update(&mut self, design: &mut Design) {
        for (h_id, h) in design.helices.iter_mut() {
            if let Some(grid_position) = h.grid_position {
                self.helix_to_pos.insert(*h_id, grid_position);
                if let Some(grid) = self.square_grids.get(&grid_position.grid) {
                    h.position = grid.position_helix(grid_position.x, grid_position.y);
                } else {
                    let grid = self.honeycomb_grids.get(&grid_position.grid).unwrap();
                    h.position = grid.position_helix(grid_position.x, grid_position.y);
                }
            }
        }
    }

    fn attach_existing(&self, origin: Vec3, direction: Vec3) -> Option<GridPosition> {
        let mut ret = None;
        let mut best_err = f32::INFINITY;
        for (g_id, g) in self.square_grids.iter() {
            let err = g.error_helix(origin, direction);
            if err < best_err {
                let (x, y) = g.interpolate_helix(origin, direction).unwrap();
                best_err = err;
                ret = Some(GridPosition {
                    grid: *g_id,
                    x,
                    y
                })
            }
        }
        for (g_id, g) in self.honeycomb_grids.iter() {
            let err = g.error_helix(origin, direction);
            if err < best_err {
                best_err = err;
                let (x, y) = g.interpolate_helix(origin, direction).unwrap();
                ret = Some(GridPosition {
                    grid: *g_id,
                    x,
                    y
                })
            }
        }
        ret
    }

    fn find_grid_for_group(&self, group: &Vec<usize>, design: &Design) -> GridDescriptor {
        let parameters = design.parameters.unwrap_or_default();
        let leader = design.helices.get(&group[0]).unwrap();
        let orientation = Rotor3::from_rotation_between(Vec3::unit_x(), leader.get_axis(&parameters).direction.normalized());
        let mut hex_grid: Grid<HoneyComb> = Grid::new(leader.position, orientation, design.parameters.unwrap_or_default());
        let mut best_err = hex_grid.error_group(&group, design);
        for dx in [-1, 0, 1].iter() {
            for dy in [-1, 0, 1].iter() {
                let position = hex_grid.position_helix(*dx, *dy);
                for i in 0..100 {
                    let angle = i as f32 * FRAC_PI_2 / 100.;
                    let rotor = Rotor3::from_rotation_yz(angle);
                    let grid: Grid<HoneyComb> = Grid::new(position, orientation.rotated_by(rotor), design.parameters.unwrap_or_default());
                    let err = grid.error_group(group, design);
                    if err < best_err {
                        hex_grid = grid;
                        best_err = err
                    }
                }
            }
        }

        let mut square_grid: Grid<SquareGrid> = Grid::new(leader.position, leader.orientation, design.parameters.unwrap_or_default());
        let mut best_square_err = square_grid.error_group(&group, design);
        for i in 0..100 {
            let angle = i as f32 * FRAC_PI_2 / 100.;
            let rotor = Rotor3::from_rotation_yz(angle);
            let grid: Grid<SquareGrid> = Grid::new(leader.position, orientation.rotated_by(rotor), design.parameters.unwrap_or_default());
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
                grid_type: GridType::Square
            }
        } else {
            GridDescriptor {
                position: hex_grid.position,
                orientation: hex_grid.orientation,
                grid_type: GridType::Honeycomb
            }
        }
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
                                let nb_cross = candidates.entry((helix.min(new_helix), helix.max(new_helix))).or_insert(0);
                                *nb_cross += 1;
                                if *nb_cross >= 3 {
                                    merger.union(helix, new_helix);
                                }
                            }
                        }
                        current_helix = Some(new_helix);
                    }
                    _ => ()
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
