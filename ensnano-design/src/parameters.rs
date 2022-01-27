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
//! DNA geometric parmeters.

use super::codenano;
use std::f32::consts::{PI, SQRT_2, TAU};

/// DNA geometric parameters.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Parameters {
    /// Distance between two consecutive bases along the axis of a
    /// helix, in nanometers.
    pub z_step: f32,
    /// Radius of a helix, in nanometers.
    pub helix_radius: f32,
    /// Number of bases per turn in nanometers.
    pub bases_per_turn: f32,
    /// Minor groove angle. DNA helices have a "minor groove" and a
    /// "major groove", meaning that two paired nucleotides are not at
    /// opposite positions around a double helix (i.e. at an angle of
    /// 180°), but instead have a different angle.
    ///
    /// Strands are directed. The "normal" direction is called "5' to
    /// 3'" (named after parts of the nucleotides). This parameter is
    /// the small angle, which is clockwise from the normal strand to
    /// the reverse strand.
    pub groove_angle: f32,

    /// Gap between two neighbouring helices.
    pub inter_helix_gap: f32,
}

impl Parameters {
    /// Default values for the parameters of DNA, taken from the litterature (Wikipedia, Cargo
    /// sorting paper, Woo 2011).
    pub const DEFAULT: Parameters = Parameters {
        // z-step and helix radius from: Wikipedia
        z_step: 0.332,
        helix_radius: 1.,
        // bases per turn from Woo Rothemund (Nature Chemistry).
        bases_per_turn: 10.44,
        // minor groove 12 Å, major groove 22 Å total 34 Å
        groove_angle: 2. * PI * 12. / 34.,
        // From Paul's paper.
        inter_helix_gap: 0.65,
    };

    pub fn from_codenano(codenano_param: &codenano::Parameters) -> Self {
        Self {
            z_step: codenano_param.z_step as f32,
            helix_radius: codenano_param.helix_radius as f32,
            bases_per_turn: codenano_param.bases_per_turn as f32,
            groove_angle: codenano_param.groove_angle as f32,
            inter_helix_gap: codenano_param.inter_helix_gap as f32,
        }
    }

    pub fn formated_string(&self) -> String {
        use std::fmt::Write;
        let mut ret = String::new();
        writeln!(&mut ret, "  Z step: {:.3} nm", self.z_step).unwrap_or_default();
        writeln!(&mut ret, "  Helix radius: {:.2} nm", self.helix_radius).unwrap_or_default();
        writeln!(&mut ret, "  #Bases per turn: {:.2}", self.bases_per_turn).unwrap_or_default();
        writeln!(
            &mut ret,
            "  Minor groove angle: {:.1}°",
            self.groove_angle.to_degrees()
        )
        .unwrap_or_default();
        writeln!(
            &mut ret,
            "  Inter helix gap: {:.2} nm",
            self.inter_helix_gap
        )
        .unwrap_or_default();
        ret
    }
}

impl std::default::Default for Parameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Parameters {
    /// The angle AOC_2 where
    ///
    /// * A is a base on the helix
    /// * B is the base paired to A
    /// * O is the projection of A on the axis of the helix
    /// * C is the 3' neighbour of A
    /// * C_2 is the projection of C in the AOB plane
    fn angle_aoc2(&self) -> f32 {
        TAU / self.bases_per_turn
    }

    /// The distance |AC| where
    ///
    /// * A is a base on the helix
    /// * C is the 3' neighbour of A
    pub fn dist_ac(&self) -> f32 {
        (self.dist_ac2() * self.dist_ac2() + self.z_step * self.z_step).sqrt()
    }

    /// The distance |AC_2| where
    ///
    /// * A is a base on the helix
    /// * B is the base paired to A
    /// * O is the projection of A on the axis of the helix
    /// * C is the 3' neighbour of A
    /// * C_2 is the projection of C in the AOB plane
    pub fn dist_ac2(&self) -> f32 {
        SQRT_2 * (1. - self.angle_aoc2().cos()).sqrt() * self.helix_radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Helix;
    use ultraviolet::{Rotor3, Vec3};

    #[test]
    fn dist_ac_is_correct() {
        let p = Parameters::DEFAULT;

        let h = Helix::new(Vec3::zero(), Rotor3::identity());
        let n1 = h.space_pos(&p, 0, true);
        let n2 = h.space_pos(&p, 1, true);

        let measured_dist = (n1 - n2).mag();

        assert!((measured_dist - p.dist_ac()).abs() < 1e-4);
    }
}
