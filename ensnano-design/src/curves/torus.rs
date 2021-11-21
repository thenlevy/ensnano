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

    // Moebius

    fn perimeter_ellipse(&self, a: f32, b: f32, nb_steps: usize) -> f32 {
        let mut p = 0f32;
        let mut u = Vec2 { x: a, y: 0. };
        for i in 0..nb_steps+1 {
            let t = TAU * i as f32 / nb_steps as f32;
            let v = Vec2 { x: a * t.cos(), y: b * t.sin() };
            p += (v - u).mag();
            u = v;
        }
        p
    }

    fn t_for_curvilinear_abscissa(&self, s: f32) -> f32 {
        let p = 9.688448061179066_f32;
        let perimeter = 4. * H * self.half_nb_helix as f32;
        let scale = perimeter / p;
        let mut sp = s / scale;        
        let a = 2.;
        let b = 1.;
        while sp < 0. { 
            sp += p;
        }
        while sp > p {
            sp -= p;
        }
        let nb_steps = NB_STEPS;
        let mut u = Vec2 { x: a, y: 0. };
        let mut t = 0f32;
        for i in 0..nb_steps+1 { // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
            if sp <= 0. {
                break;
            }
            t = TAU * i as f32 / nb_steps as f32;
            let v = Vec2 { x: a * t.cos(), y: b * t.sin() };
            sp -= (v - u).mag();
            u = v;
        }
        t
    }

    fn t_for_curvilinear_abscissa_poly(&self, s: f32) -> f32 {
        let p = 9.688448061179066_f32;
        let perimeter = 4. * H * self.half_nb_helix as f32;
        let scale = perimeter / p;
        let coef: [f32; 21] = [0.00012918397789041247, 0.1515814901975501, 0.10450751273807285, -0.6649830252676487, 1.7278194213623754, -2.8339254809794006, 3.1496843695687855, -2.466968925697648, 1.4018887658135728, -0.5905631979287363, 0.18721564526394163, -0.04507183747440176, 0.008268531828443176, -0.0011525981861020874, 0.00012080470580909873, -9.318521725420798e-06, 5.085225494171747e-07, -1.8187171826245956e-08, 3.5515542659526737e-10, -1.404395246708242e-12, -4.930219196343138e-14];

        let mut sp = s / scale;
        while sp < 0. {
            sp += p;
        }
        while sp >= p {
            sp -= p;
        }
        let mut result = 0_f32;
        for i in (0..coef.len()).rev() {
            result = sp * result + coef[i];
        }
        return TAU * result
    }

    fn position_Moebius(&self, t:f32) -> Vec3 {
        let p = 9.688448061179066_f32;
        let perimeter = 4. * H * self.half_nb_helix as f32;
        // println!("p: {}\t P: {}\tφ:{}\tφφ:{}", perimeter, self.perimeter_ellipse(2.,1., NB_STEPS), self.t_for_curvilinear_abscissa_poly(perimeter/2.), self.t_for_curvilinear_abscissa(perimeter/2.));
        let scale = perimeter / p;
        let a = 2. * scale;
        let b = 1. * scale;
        let theta = self.theta(t) - self.theta0;
        let theta_dt = self.theta_dt();
        let s_dtheta = (perimeter / 2. + 4. * H) / TAU; 
        let s = 4. * H * self.theta0 / TAU + s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (pc, ps) = (phi.cos(), phi.sin());
        let (x,y) = (a * phi.cos(), b * phi.sin());
        Vec3 {
            x: (x * t2c - y * t2s + self.big_radius) * theta.cos(),
            y: x * t2s + y * t2c,
            z: (x * t2c - y * t2s + self.big_radius) * theta.sin(),
        }
    }

    fn speed_Moebius(&self, t:f32) -> Vec3 {
        let dt = 1. / NB_STEPS as f32;
        let x = self.position_Moebius(t);
        let x_dx = self.position_Moebius(t+dt);
        return (x_dx - x) / dt;

        let p = 9.688448061179066_f32;
        let perimeter = 4. * H * self.half_nb_helix as f32;
        let scale = perimeter / p;
        let a = 2. * scale;
        let b = 1. * scale;
        let theta = self.theta(t) - self.theta0;
        let theta_dt = self.theta_dt();
        let s_dtheta = H + (perimeter / 2. + 4. * H) / TAU; 
        let s = s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (pc, ps) = (phi.cos(), phi.sin());
        let (x,y) = (a * phi.cos(), b * phi.sin());
        let n_dt = (a*a*ps*ps + b*b*pc*pc).sqrt() / theta_dt / s_dtheta;
        let (x_dt, y_dt) = (- a * ps / n_dt, b * pc / n_dt);
        Vec3 {
            x: theta_dt * ( 
                - (x * t2c - y * t2s + self.big_radius) * theta.sin()
                + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.cos()
            ), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2)+R)*cos(t),t) = 1/2 (-2 R sin(t) + cos(t) (2 cos(t/2) X'(t) + X(t) (-sin(t/2)) - 2 sin(t/2) Y'(t) - Y(t) cos(t/2)) - 2 sin(t) (X(t) cos(t/2) - Y(t) sin(t/2)))
            y: theta_dt * (
                x_dt * t2s + x * t2c / 2. + y_dt * t2c - y * t2s / 2.
            ), // diff((X(t)*sin(t/2)+Y(t)*cos(t/2)),t) = d/dt(X(t) sin(t/2) + Y(t) cos(t/2)) = sin(t/2) X'(t) + 1/2 X(t) cos(t/2) + cos(t/2) Y'(t) - 1/2 Y(t) sin(t/2)
            z: theta_dt * ( 
                (x * t2c - y * t2s) * theta.cos()
                + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.sin()
            ), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2))*sin(t),t) = sin(t) cos(t/2) X'(t) - 1/2 X(t) sin(t/2) sin(t) + X(t) cos(t/2) cos(t) - sin(t/2) sin(t) Y'(t) - Y(t) sin(t/2) cos(t) - 1/2 Y(t) sin(t) cos(t/2)
        }
    }

    fn acceleration_Moebius(&self, t: f32) -> Vec3 {
        Vec3 {
            x: 0.,
            y: 0.,
            z: 1.,
        }
    }
    
}

impl Curved for Torus {
    fn position(&self, t: f32) -> Vec3 {
        return self.position_Moebius(t);
    }

    fn speed(&self, t: f32) -> Vec3 {
        return self.speed_Moebius(t);
    }
    
    fn acceleration(&self, t: f32) -> Vec3 {
        return self.acceleration_Moebius(t);
    }
}






// MOEBIUS RING

const NB_STEPS: usize = 1_000;

pub struct Ellipse {
    pub a: f32,
    pub b: f32,
}


impl Ellipse {
    fn position(&self, t: f32) -> Vec2 {
        Vec2 {
            x: self.a * (TAU * t).cos(),
            y: self.b * (TAU * t).sin(), 
        }
    }

    fn speed(&self, t: f32) -> Vec2 {
        Vec2 {
            x: -TAU * self.a * (TAU * t).sin(),
            y: TAU * self.b * (TAU * t).cos(), 
        }
    }

    fn perimeter_approximation(&self) -> f32 {
        let a = self.a;
        let b = self.b;
        let h_ = (a - b)/ (a + b);
        let h = h_ * h_;
        let p = PI * (a + b) * (1. + 3. * h / (10. + (4. - 3. * h).sqrt()));

        return p;
    }

    fn perimeter(&self, nb_steps: usize) -> f32 {
        let mut p = 0f32;
        let mut u = self.position(0.);
        for i in 0..nb_steps+1 {
            let t = i as f32 / nb_steps as f32;
            let v = self.position(t);
            p += (v - u).mag();
            u = v;
        }
        p
    }

    fn scale_for(&self, half_nb_helix: usize) -> f32 {
        let p = self.perimeter(NB_STEPS);
        let desired_p = 4. * H * half_nb_helix as f32;
        desired_p / p
    }
}

pub struct MoebiusRing {
    pub curve: Ellipse,
    pub perimeter: f32,
    pub half_nb_helix: usize,
    pub scale: f32,
    pub nb_steps: usize,
    pub big_radius: f32,
    pub theta0: f32,
}

impl MoebiusRing {
    pub fn new(curve: Ellipse, half_nb_helix: usize, big_radius: f32, theta0: f32) -> Self {
        let p = curve.perimeter(NB_STEPS);
        let s = curve.scale_for(half_nb_helix);
        Self {
            curve: curve,
            perimeter: p,
            half_nb_helix: half_nb_helix,
            scale: s,
            nb_steps: NB_STEPS,
            big_radius: big_radius,
            theta0: theta0,
        }
    }

    fn t_for_curvilinear_abscissa(&self, s: f32) -> f32 {
        let mut sp = s / self.scale;
        while sp < 0. { 
            sp += self.perimeter;
        }
        while sp > self.perimeter {
            sp -= self.perimeter;
        }
        let nb_steps = self.nb_steps;
        let ds = 1. / nb_steps as f32;
        let mut u = self.curve.position(0.);
        let mut t = 0f32;
        for i in 0..nb_steps+1 { // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
            if sp <= 0. {
                break;
            }
            t = i as f32 / nb_steps as f32;
            let v = self.curve.position(t);
            sp -= (v - u).mag();
            u = v;
        }
        t
    }

    fn theta(&self, t: f32) -> f32 {
        TAU * (2. * self.half_nb_helix as f32) * t / 2. + self.theta0
    }

    fn theta_dt(&self) -> f32 {
        TAU * (2. * self.half_nb_helix as f32) / 2.
    }

    fn position_Moebius(&self, t:f32) -> Vec3 {
        let theta = self.theta(t);
        let theta_dt = self.theta_dt();
        let perimeter = self.perimeter * self.scale;
        let s_dtheta = (self.perimeter / 2. + 4. * H) / TAU; 
        let s = s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let a = self.curve.a * self.scale;
        let b = self.curve.b * self.scale;
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (pc, ps) = (phi.cos(), phi.sin());
        let (x,y) = (a * phi.cos(), b * phi.sin());
        Vec3 {
            x: (x * t2c - y * t2s + self.big_radius) * theta.cos(),
            y: x * t2s + y * t2c,
            z: (x * t2c - y * t2s + self.big_radius) * theta.sin(),
        }
    }

    fn speed_Moebius(&self, t:f32) -> Vec3 {
        let theta = self.theta(t);
        let theta_dt = self.theta_dt();
        let s_dtheta = (self.perimeter / 2. + 4. * H) / TAU; 
        let s = s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let a = self.curve.a * self.scale;
        let b = self.curve.b * self.scale;
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (pc, ps) = (phi.cos(), phi.sin());
        let (x,y) = (a * phi.cos(), b * phi.sin());
        let n_dt = (a*a*ps*ps + b*b*pc*pc).sqrt() / theta_dt / s_dtheta;
        let (x_dt, y_dt) = (- a * ps / n_dt, b * pc / n_dt);
        Vec3 {
            x: theta_dt * ( 
                - (x * t2c - y * t2s + self.big_radius) * theta.sin()
                + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.cos()
            ), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2)+R)*cos(t),t) = 1/2 (-2 R sin(t) + cos(t) (2 cos(t/2) X'(t) + X(t) (-sin(t/2)) - 2 sin(t/2) Y'(t) - Y(t) cos(t/2)) - 2 sin(t) (X(t) cos(t/2) - Y(t) sin(t/2)))
            y: theta_dt * (
                x_dt * t2s + x * t2c / 2. + y_dt * t2c - y * t2s / 2.
            ), // diff((X(t)*sin(t/2)+Y(t)*cos(t/2)),t) = d/dt(X(t) sin(t/2) + Y(t) cos(t/2)) = sin(t/2) X'(t) + 1/2 X(t) cos(t/2) + cos(t/2) Y'(t) - 1/2 Y(t) sin(t/2)
            z: theta_dt * ( 
                (x * t2c - y * t2s) * theta.cos()
                + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.sin()
            ), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2))*sin(t),t) = sin(t) cos(t/2) X'(t) - 1/2 X(t) sin(t/2) sin(t) + X(t) cos(t/2) cos(t) - sin(t/2) sin(t) Y'(t) - Y(t) sin(t/2) cos(t) - 1/2 Y(t) sin(t) cos(t/2)
        }
    }

    fn acceleration_Moebius(&self, t: f32) -> Vec3 {
        Vec3 {
            x: 0.,
            y: 0.,
            z: 1.,
        }
    }

}

