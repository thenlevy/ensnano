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
}

impl Hyperboloid {
    pub fn make_helices(&self) -> (Vec<Helix>, usize) {
        let mut ret = Vec::with_capacity(self.radius);
        use std::f32::consts::PI;
        let angle = PI / self.radius as f32;
        let grid_radius =
            (self.parameters.helix_radius + self.parameters.inter_helix_gap / 2.) / angle.sin();
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
}
