use super::super::icednano::{Helix, Parameters};
use super::{Edge, GridDivision, GridType};

use ultraviolet::{Rotor3, Vec2, Vec3};

/// A structure made of helices arranged circularly on two planes.
#[derive(Clone, Debug)]
pub struct Hyperboloid {
    /// The number of helices on each plane
    pub radius: usize,
    /// The angle between the two planes.
    pub shift: f32,
    /// The distance between the planes.
    pub length: f32,
    /// The difference between the actual sheet radius and the radius needed for the helices to
    /// fit perfectly at the tightest point of the hyperboloid
    pub radius_shift: f32,

    /// A forced grid radius, for when user modifies the shift but still wants the radius in the
    /// center to be constant.
    pub forced_radius: Option<f32>,
}

impl GridDivision for Hyperboloid {
    fn origin_helix(&self, parameters: &Parameters, x: isize, _y: isize) -> Vec2 {
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let grid_radius = self.radius(parameters);
        let i = x % (self.radius as isize);
        let theta = 2. * i as f32 * angle;
        let theta_ = theta + self.shift;
        let left_helix = Vec3::new(0., grid_radius * theta.sin(), grid_radius * theta.cos());
        let right_helix = Vec3::new(
            self.length,
            grid_radius * theta_.sin(),
            grid_radius * theta_.cos(),
        );
        let origin = (right_helix + left_helix) / 2.;
        Vec2::new(origin.z, origin.y)
    }

    fn orientation_helix(&self, parameters: &Parameters, x: isize, _y: isize) -> Rotor3 {
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let grid_radius = self.radius(parameters);
        let i = x % (self.radius as isize);
        let theta = 2. * i as f32 * angle;
        let origin = Vec3::new(0., grid_radius * theta.sin(), grid_radius * theta.cos());
        let theta_ = theta + self.shift;
        let dest = Vec3::new(
            self.length,
            grid_radius * theta_.sin(),
            grid_radius * theta_.cos(),
        );
        Rotor3::from_rotation_between(Vec3::unit_x(), (dest - origin).normalized())
    }

    fn interpolate(&self, _parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let plane_angle = y.atan2(x);
        let i = (plane_angle / angle / 2.).round();
        (i as isize, 0)
    }

    fn translation_to_edge(&self, x1: isize, _y1: isize, x2: isize, _y2: isize) -> Edge {
        Edge::Circle((x2 - x1).rem_euclid(self.radius as isize))
    }

    fn translate_by_edge(&self, x1: isize, y1: isize, edge: Edge) -> Option<(isize, isize)> {
        match edge {
            Edge::Circle(x) => Some(((x1 + x).rem_euclid(self.radius as isize), y1)),
            _ => None,
        }
    }

    fn grid_type(&self) -> GridType {
        unimplemented!()
    }
}

impl Hyperboloid {
    pub fn make_helices(&self, parameters: &Parameters) -> (Vec<Helix>, usize) {
        let mut ret = Vec::with_capacity(self.radius);
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let grid_radius = self.radius(parameters);
        let mut nb_nucl = 0;
        for i in 0..self.radius {
            let theta = 2. * i as f32 * angle;
            let theta_ = theta + self.shift;
            let left_helix = Vec3::new(0., grid_radius * theta.sin(), grid_radius * theta.cos());
            let right_helix = Vec3::new(
                self.length,
                grid_radius * theta_.sin(),
                grid_radius * theta_.cos(),
            );
            let origin = (left_helix + right_helix) / 2.;
            let real_length = (right_helix - left_helix).mag();
            nb_nucl = (real_length / parameters.z_step).round() as usize;
            let orientation = Rotor3::from_rotation_between(
                Vec3::unit_x(),
                (right_helix - left_helix).normalized(),
            );
            let helix = Helix::new(origin, orientation);
            ret.push(helix);
        }
        (ret, nb_nucl)
    }

    pub fn modify_shift(&mut self, new_shift: f32, parameters: &Parameters) {
        let grid_radius = self.radius(parameters);
        let theta = 0f32;
        let theta_ = theta + self.shift;
        let left_helix = Vec3::new(0., grid_radius * theta.sin(), grid_radius * theta.cos());
        let right_helix = Vec3::new(
            self.length,
            grid_radius * theta_.sin(),
            grid_radius * theta_.cos(),
        );
        let origin = (left_helix + right_helix) / 2.;
        let center_radius = self
            .forced_radius
            .unwrap_or(Vec2::new(origin.y, origin.z).mag());
        self.shift = new_shift;
        if self.forced_radius.is_none() {
            self.forced_radius = Some(center_radius);
        }
    }

    /// Return the radii of the sheet so that the helices respectively fits perfectly at the center of the
    /// hyperboloid or at the extremity of the hyperboloid
    fn sheet_radii(&self, parameters: &Parameters) -> (f32, f32) {
        // First determine the radius in the center of the hyperboloid.
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let center_radius =
            (parameters.helix_radius + parameters.inter_helix_gap / 2.) / angle.sin();

        // Let R be the radius on the sheets, delta be self.shift and r be the radius of at the
        // center. Then for a point at R( cos(theta), sin(theta), 0) joining a point at R(cos(theta
        // + delta), sin(theta + delta), h), the radius in the center is
        // r =  R * (((cos(theta) + cos(theta + delta)/ 2)^2 + (sin(theta) + sin(theta+delta))/2)^2)
        // this is a constant to we can take theta = 0 which gives
        // r = R * 1/4 (2 + 2cos(delta))
        (
            (2. * center_radius / (2. + 2. * self.shift.cos()).sqrt()),
            center_radius,
        )
    }

    /// Return true iff the grid supporting self contains the point (x, y)
    pub fn contains_point(&self, parameters: &Parameters, x: f32, y: f32) -> bool {
        let r = self.grid_radius(parameters);
        x.abs() <= r && y.abs() <= r
    }

    fn radius(&self, parameters: &Parameters) -> f32 {
        if let Some(center_radius) = self.forced_radius {
            2. * center_radius / (2. + 2. * self.shift.cos()).sqrt()
        } else {
            let (small_r, big_r) = self.sheet_radii(parameters);
            (1. - self.radius_shift) * big_r + self.radius_shift * small_r
        }
    }

    pub fn grid_radius(&self, parameters: &Parameters) -> f32 {
        let grid_radius = self.radius(parameters);
        let r = grid_radius / 2. * (2. + 2. * self.shift.cos()).sqrt();
        self.forced_radius.unwrap_or(r) + parameters.helix_radius + parameters.inter_helix_gap / 2.
    }
}
