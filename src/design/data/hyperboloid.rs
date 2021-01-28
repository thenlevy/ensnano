use super::icednano::{Helix, Parameters};

use ultraviolet::{Rotor3, Vec3};

/// A structure made of helices arranged circularly on two planes.
pub struct Hyperboloid {
    /// The number of helices on each plane
    pub radius: usize,
    /// The angle between the two planes.
    pub shift: f32,
    /// The distance between the planes.
    pub length: f32,
    pub parameters: Parameters,
    /// The difference between the actual sheet radius and the radius needed for the helices to
    /// fit perfectly at the tightest point of the hyperboloid
    pub radius_shift: f32,
}

impl Hyperboloid {
    pub fn make_helices(&self) -> (Vec<Helix>, usize) {
        let mut ret = Vec::with_capacity(self.radius);
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let (small_r, big_r) = self.sheet_radii();
        let grid_radius = (1. - self.radius_shift) * big_r + self.radius_shift * small_r;
        let mut nb_nucl = 0;
        for i in 0..self.radius {
            let theta = 2. * i as f32 * angle;
            let origin = Vec3::new(0., grid_radius * theta.sin(), grid_radius * theta.cos());
            let theta_ = theta + self.shift;
            let dest = Vec3::new(
                self.length,
                grid_radius * theta_.sin(),
                grid_radius * theta_.cos(),
            );
            let real_length = (dest - origin).mag();
            nb_nucl = (real_length / self.parameters.z_step).round() as usize;
            let orientation =
                Rotor3::from_rotation_between(Vec3::unit_x(), (dest - origin).normalized());
            let helix = Helix::new(origin, orientation);
            ret.push(helix);
        }
        (ret, nb_nucl)
    }

    /// Return the radii of the sheet so that the helices respectively fits perfectly at the center of the
    /// hyperboloid or at the extremity of the hyperboloid
    fn sheet_radii(&self) -> (f32, f32) {
        // First determine the radius in the center of the hyperboloid.
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let center_radius =
            (self.parameters.helix_radius + self.parameters.inter_helix_gap / 2.) / angle.sin();

        // Let R be the radius on the sheets, delta be self.shift and r be the radius of at the
        // center. Then for a point at R( cos(theta), sin(theta), 0) joining a point at R(cos(theta
        // + delta), sin(theta + delta), h), the radius in the center is
        // r =  R * (((cos(theta) + cos(theta + delta)/ 2)^2 + (sin(theta) + sin(theta+delta))/2)^2)
        // this is a constant to we can take theta = 0 which gives
        // r = R * 1/4 (2 + 2cos(delta))
        (
            4. * center_radius / (2. + 2. * self.shift.cos()),
            center_radius,
        )
    }
}
