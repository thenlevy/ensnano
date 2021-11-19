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

use super::Curved;
use std::f32::consts::{TAU, PI};
use ultraviolet::{Rotor3, Vec3, Vec2};

const H: f32 =
    crate::Parameters::DEFAULT.helix_radius + crate::Parameters::DEFAULT.inter_helix_gap / 2.;

/// A torus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torus {
    /// The angle shift a t = 0 along the slice
    pub theta0: f32,
    /// The number of helices on each slice
    pub half_nb_helix: usize,
    /// The radius of the torus
    pub big_radius: f32,
}

impl Torus {
    fn theta(&self, t: f32) -> f32 {
        TAU * (2. * self.half_nb_helix as f32) * t / 2. + self.theta0
    }

    fn theta_dt(&self) -> f32 {
        TAU * (2. * self.half_nb_helix as f32) / 2.
    }

    fn phi(&self, t: f32) -> f32 {
        TAU * t
    }

    fn phi_dt(&self) -> f32 {
        TAU
    }

    fn small_radius(&self) -> f32 {
        4. * H * self.half_nb_helix as f32 / TAU
    }

    // REAL TORUS

    fn position_torus(&self, t: f32) -> Vec3 {
        let theta = self.theta(t);
        let small_radius = self.small_radius();
        let phi = self.phi(t);

        Vec3 {
            z: theta.cos() * (self.big_radius + small_radius * phi.cos()),
            x: theta.sin() * (self.big_radius + small_radius * phi.cos()),
            y: phi.sin() * small_radius,
        }
    }

    fn speed_torus(&self, t: f32) -> Vec3 {
        let theta = self.theta(t);
        let small_radius = self.small_radius();
        let phi = self.phi(t);

        let theta_dt = self.theta_dt();
        let phi_dt = self.phi_dt();

        Vec3 {
            z: theta.cos() * (-phi.sin() * small_radius * phi_dt)
                - theta.sin() * theta_dt * (self.big_radius + small_radius * phi.cos()),
            x: theta.sin() * (-phi.sin() * small_radius * phi_dt)
                + theta.cos() * theta_dt * (self.big_radius + small_radius * phi.cos()),
            y: phi_dt * small_radius * phi.cos(),
        }
    }

    fn acceleration_torus(&self, t: f32) -> Vec3 {
        let theta = self.theta(t);
        let small_radius = self.small_radius();
        let phi = self.phi(t);

        let theta_dt = self.theta_dt();
        let phi_dt = self.phi_dt();

        Vec3 {
            z: (-theta_dt * theta.sin() * (-phi.sin() * small_radius * phi_dt)
                + theta.cos() * (-phi.cos() * small_radius * phi_dt * phi_dt))
                - (theta_dt
                    * theta_dt
                    * theta.cos()
                    * (self.big_radius + small_radius * phi.cos())
                    + theta.sin() * theta_dt * (small_radius * -phi_dt * phi.sin())),
            x: (theta_dt * theta.cos() * (-phi.sin() * small_radius * phi_dt)
                + theta.sin() * (-phi_dt * phi_dt * small_radius * phi.cos()))
                + (-theta_dt
                    * theta_dt
                    * theta.sin()
                    * (self.big_radius + small_radius * phi.cos())
                    + theta.cos() * theta_dt * (small_radius * -phi_dt * phi.sin())),
            y: -phi_dt * phi_dt * small_radius * phi.sin(),
        }
    }

    // MOEBIUS RING

    fn ellipse_perimeter_approximation(a: f32, b: f32) -> f32 {
        let h_ = (a - b)/ (a + b);
        let h = h_ * h_;
        let p = PI * (a + b) * (1. + 3. * h / (10. + (4. - 3. * h).sqrt()));

        return p;
    }

    fn ellipse_parameters(&self, ratio_a_b: f32, n: usize) -> (f32, f32) {
        let b_ = 1 as f32;
        let a_ = ratio_a_b;
        let p = Self::ellipse_perimeter_approximation(a_, b_);

        let wanted_p = 4. * n as f32 * H;
        let x = wanted_p / p;

        return (a_ * x, b_ * x);
    }

    fn position_moebius_ring(&self, t: f32) -> Vec3 {
        let theta = self.theta(t);
        let small_radius = self.small_radius();
        let phi = self.phi(t);

        Vec3 {
            z: theta.cos() * (self.big_radius + small_radius * phi.cos()),
            x: theta.sin() * (self.big_radius + small_radius * phi.cos()),
            y: phi.sin() * small_radius,
        }
    }

    // Rotating rectangle
    fn n1(&self) -> usize { self.half_nb_helix & 255 } // number of helices on the horizontal side
    fn n2(&self) -> usize { self.half_nb_helix >> 8 } // number of helices on the vertical side
    fn n12(&self) -> usize { self.n1() + self.n2() }
    
    fn side1(&self) -> f32 { (self.n1() - 1) as f32 * 2. * H }
    fn side2(&self) -> f32  { (self.n2() - 1) as f32 * 2. * H }
    fn rectangle_perimeter(&self) -> f32 { 2. * (self.side1() + self.side2()) }

    fn t1(&self) -> f32 { self.side1() / self.rectangle_perimeter() }
    fn t2(&self) -> f32 { (self.side1() + self.side2()) / self.rectangle_perimeter() }
    fn t3(&self) -> f32 { (2. * self.side1() + self.side2()) / self.rectangle_perimeter() }

    fn rectangle(&self, t: f32) -> Vec2 {
        let (s1, s2) = (self.side1(), self.side2());
        let mut s = t * 2. * (s1 + s2);

        let c0 = Vec2 { x: s1/2., y: s2/2., };
        let c1 = Vec2 { x: -s1/2., y: s2/2., };
        let c2 = Vec2 { x: -s1/2., y: -s2/2., };
        let c3 = Vec2 { x: s1/2., y: -s2/2., };

        if s < s1 {
            let u = s / s1;
            return c0 * (1. - u) + u * c1;
        } 
        s -= s1;
        if s < s2 {
            let u = s / s2;
            return c1 * (1. - u) + u * c2;
        } 
        s -= s2; 
        if s < s1  {
            let u = s / s1;
            return c2 * (1. - u) + u * c3;
        } 
        s -= s1;
        if s <= s2 {
            let u = s / s2;
            return c3 * (1. - u) + u * c0;
        } 
        return c0
    }

}

impl Curved for Torus {
    fn position(&self, t: f32) -> Vec3 {
        return self.position_torus(t);
    }

    fn speed(&self, t: f32) -> Vec3 {
        return self.speed_torus(t);
    }
    
    fn acceleration(&self, t: f32) -> Vec3 {
        return self.acceleration_torus(t);
    }
}