use super::icednano::{Design, Parameters};
use super::{icednano, Data};
use std::collections::HashMap;
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
}

#[derive(Clone, Debug)]
pub enum GridType {
    Square(SquareGrid),
    Honeycomb(HoneyComb),
}

impl GridDivision for GridType {
    fn grid_type(&self) -> GridType {
        match self {
            GridType::Square(SquareGrid) => GridType::Square(SquareGrid),
            GridType::Honeycomb(HoneyComb) => GridType::Honeycomb(HoneyComb),
        }
    }

    fn origin_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        match self {
            GridType::Square(grid) => grid.origin_helix(parameters, x, y),
            GridType::Honeycomb(grid) => grid.origin_helix(parameters, x, y),
        }
    }

    fn interpolate(&self, parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        match self {
            GridType::Square(grid) => grid.interpolate(parameters, x, y),
            GridType::Honeycomb(grid) => grid.interpolate(parameters, x, y),
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

    pub fn descr(&self) -> GridTypeDescr {
        match self {
            GridType::Square(_) => GridTypeDescr::Square,
            GridType::Honeycomb(_) => GridTypeDescr::Honeycomb,
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
        Vec3::unit_z().rotated_by(self.orientation)
    }

    pub fn position_helix(&self, x: isize, y: isize) -> Vec3 {
        let origin = self.grid_type.origin_helix(&self.parameters, x, y);
        let z_vec = Vec3::unit_z().rotated_by(self.orientation);
        let y_vec = Vec3::unit_y().rotated_by(self.orientation);
        self.position + origin.x * z_vec + origin.y * y_vec
    }

    pub fn interpolate_helix(&self, origin: Vec3, axis: Vec3) -> Option<(isize, isize)> {
        let intersection = self.line_intersection(origin, axis)?;
        Some(
            self.grid_type
                .interpolate(&self.parameters, intersection.x, intersection.y),
        )
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
}

pub trait GridDivision {
    fn origin_helix(&self, parameters: &Parameters, x: isize, y: isize) -> Vec2;
    fn interpolate(&self, parameters: &Parameters, x: f32, y: f32) -> (isize, isize);
    fn grid_type(&self) -> GridType;
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

    fn grid_type(&self) -> GridType {
        GridType::Honeycomb(HoneyComb)
    }
}

#[derive(Clone, Serialize, Deserialize, Copy)]
pub struct GridPosition {
    grid: usize,
    x: isize,
    y: isize,
}

pub(super) struct GridManager {
    pub grids: Vec<Grid>,
    helix_to_pos: HashMap<usize, GridPosition>,
    parameters: Parameters,
}

impl GridManager {
    pub fn new(parameters: Parameters) -> Self {
        Self {
            grids: Vec::new(),
            helix_to_pos: HashMap::new(),
            parameters,
        }
    }

    pub fn new_from_design(design: &Design) -> Self {
        let mut grids = Vec::new();
        let mut helix_to_pos = HashMap::new();
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
            }
        }
        for (h_id, h) in design.helices.iter() {
            if let Some(grid_position) = h.grid_position {
                helix_to_pos.insert(*h_id, grid_position);
            }
        }

        Self {
            grids,
            helix_to_pos,
            parameters: design.parameters.unwrap_or_default(),
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
                id: n
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
                let grid = &self.grids[grid_position.grid];
                h.position = grid.position_helix(grid_position.x, grid_position.y);
            }
        }
        design.grids.clear();
        for g in self.grids.iter() {
            design.grids.push(g.desc());
        }
    }

    fn attach_existing(&self, origin: Vec3, direction: Vec3) -> Option<GridPosition> {
        let mut ret = None;
        let mut best_err = f32::INFINITY;
        for (g_id, g) in self.grids.iter().enumerate() {
            let err = g.error_helix(origin, direction);
            if err < best_err {
                let (x, y) = g.interpolate_helix(origin, direction).unwrap();
                best_err = err;
                ret = Some(GridPosition { grid: g_id, x, y })
            }
        }
        ret
    }

    fn find_grid_for_group(&self, group: &Vec<usize>, design: &Design) -> GridDescriptor {
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
            ))));
        }
        ret
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
    helices: HashMap<(isize, isize), usize>,
    grid_type: GridType,
    parameters: Parameters,
    id: usize,
}

impl Grid2D {
    pub fn new(id: usize, grid_type: GridType, parameters: Parameters) -> Self {
        Self {
            helices: HashMap::new(),
            grid_type,
            parameters,
            id,
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

    pub fn helices(&self) -> &HashMap<(isize, isize), usize> {
        &self.helices
    }

    pub fn helix_position(&self, x: isize, y: isize) -> Vec2 {
        self.grid_type.origin_helix(&self.parameters, x, y)
    }
}
