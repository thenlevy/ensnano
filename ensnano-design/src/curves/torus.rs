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
use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use ultraviolet::{Rotor3, Vec2, Vec3, DVec2, DVec3};

const DTAU : f64 = std::f64::consts::TAU;
const DPI : f64 = std::f64::consts::PI;

const H: f32 =
    crate::Parameters::DEFAULT.helix_radius + crate::Parameters::DEFAULT.inter_helix_gap / 2.;

const DH: f64 =
    crate::Parameters::DEFAULT.helix_radius as f64 + crate::Parameters::DEFAULT.inter_helix_gap as f64 / 2.;

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
        for i in 0..nb_steps + 1 {
            let t = TAU * i as f32 / nb_steps as f32;
            let v = Vec2 {
                x: a * t.cos(),
                y: b * t.sin(),
            };
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
        for i in 0..nb_steps + 1 {
            // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
            if sp <= 0. {
                break;
            }
            t = TAU * i as f32 / nb_steps as f32;
            let v = Vec2 {
                x: a * t.cos(),
                y: b * t.sin(),
            };
            sp -= (v - u).mag();
            u = v;
        }
        t
    }

    fn t_for_curvilinear_abscissa_poly(&self, s: f32) -> f32 {
        let p = 9.688448061179066_f32;
        let perimeter = 4. * H * self.half_nb_helix as f32;
        let scale = perimeter / p;
        let coef: [f32; 21] = [
            0.00012918397789041247,
            0.1515814901975501,
            0.10450751273807285,
            -0.6649830252676487,
            1.7278194213623754,
            -2.8339254809794006,
            3.1496843695687855,
            -2.466968925697648,
            1.4018887658135728,
            -0.5905631979287363,
            0.18721564526394163,
            -0.04507183747440176,
            0.008268531828443176,
            -0.0011525981861020874,
            0.00012080470580909873,
            -9.318521725420798e-06,
            5.085225494171747e-07,
            -1.8187171826245956e-08,
            3.5515542659526737e-10,
            -1.404395246708242e-12,
            -4.930219196343138e-14,
        ];

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
        return TAU * result;
    }

    fn position_moebius(&self, t: f32) -> Vec3 {
        let p = 9.688448061179066_f32;
        let perimeter = 4. * H * self.half_nb_helix as f32;
        // println!("p: {}\t P: {}\tφ:{}\tφφ:{}", perimeter, self.perimeter_ellipse(2.,1., NB_STEPS), self.t_for_curvilinear_abscissa_poly(perimeter/2.), self.t_for_curvilinear_abscissa(perimeter/2.));
        let scale = perimeter / p;
        let a = 2. * scale;
        let b = 1. * scale;
        let theta = self.theta(t) - self.theta0;
        let theta_dt = self.theta_dt();
        let s_dtheta = (perimeter / 2. - 4. * H) / TAU;
        let s = 4. * H * self.theta0 / TAU + s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (pc, ps) = (phi.cos(), phi.sin());
        let (x, y) = (a * phi.cos(), b * phi.sin());
        Vec3 {
            x: (x * t2c - y * t2s + self.big_radius) * theta.cos(),
            y: x * t2s + y * t2c,
            z: (x * t2c - y * t2s + self.big_radius) * theta.sin(),
        }
    }

    fn speed_moebius(&self, t: f32) -> Vec3 {
        let dt = 1. / NB_STEPS as f32;
        let x = self.position_moebius(t);
        let x_dx = self.position_moebius(t + dt);
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
        let (x, y) = (a * phi.cos(), b * phi.sin());
        let n_dt = (a * a * ps * ps + b * b * pc * pc).sqrt() / theta_dt / s_dtheta;
        let (x_dt, y_dt) = (-a * ps / n_dt, b * pc / n_dt);
        Vec3 {
            x: theta_dt
                * (-(x * t2c - y * t2s + self.big_radius) * theta.sin()
                    + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.cos()), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2)+R)*cos(t),t) = 1/2 (-2 R sin(t) + cos(t) (2 cos(t/2) X'(t) + X(t) (-sin(t/2)) - 2 sin(t/2) Y'(t) - Y(t) cos(t/2)) - 2 sin(t) (X(t) cos(t/2) - Y(t) sin(t/2)))
            y: theta_dt * (x_dt * t2s + x * t2c / 2. + y_dt * t2c - y * t2s / 2.), // diff((X(t)*sin(t/2)+Y(t)*cos(t/2)),t) = d/dt(X(t) sin(t/2) + Y(t) cos(t/2)) = sin(t/2) X'(t) + 1/2 X(t) cos(t/2) + cos(t/2) Y'(t) - 1/2 Y(t) sin(t/2)
            z: theta_dt
                * ((x * t2c - y * t2s) * theta.cos()
                    + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.sin()), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2))*sin(t),t) = sin(t) cos(t/2) X'(t) - 1/2 X(t) sin(t/2) sin(t) + X(t) cos(t/2) cos(t) - sin(t/2) sin(t) Y'(t) - Y(t) sin(t/2) cos(t) - 1/2 Y(t) sin(t) cos(t/2)
        }
    }

    fn acceleration_moebius(&self, _: f32) -> Vec3 {
        Vec3 {
            x: 0.,
            y: 0.,
            z: 1.,
        }
    }
}

impl Curved for Torus {
    fn position(&self, t: f32) -> Vec3 {
        return self.position_moebius(t);
    }

    fn speed(&self, t: f32) -> Vec3 {
        return self.speed_moebius(t);
    }

    fn acceleration(&self, t: f32) -> Vec3 {
        return self.acceleration_moebius(t);
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
        let h_ = (a - b) / (a + b);
        let h = h_ * h_;
        let p = PI * (a + b) * (1. + 3. * h / (10. + (4. - 3. * h).sqrt()));

        return p;
    }

    fn perimeter(&self, nb_steps: usize) -> f32 {
        let mut p = 0f32;
        let mut u = self.position(0.);
        for i in 0..nb_steps + 1 {
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
        for i in 0..nb_steps + 1 {
            // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
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

    fn position_Moebius(&self, t: f32) -> Vec3 {
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
        let (x, y) = (a * phi.cos(), b * phi.sin());
        Vec3 {
            x: (x * t2c - y * t2s + self.big_radius) * theta.cos(),
            y: x * t2s + y * t2c,
            z: (x * t2c - y * t2s + self.big_radius) * theta.sin(),
        }
    }

    fn speed_Moebius(&self, t: f32) -> Vec3 {
        let theta = self.theta(t);
        let theta_dt = self.theta_dt();
        let s_dtheta = (self.perimeter / 2. + 4. * H) / TAU;
        let s = s_dtheta * theta;
        let phi = self.t_for_curvilinear_abscissa(s);
        let a = self.curve.a * self.scale;
        let b = self.curve.b * self.scale;
        let (t2c, t2s) = ((theta / 2.).cos(), (theta / 2.).sin());
        let (pc, ps) = (phi.cos(), phi.sin());
        let (x, y) = (a * phi.cos(), b * phi.sin());
        let n_dt = (a * a * ps * ps + b * b * pc * pc).sqrt() / theta_dt / s_dtheta;
        let (x_dt, y_dt) = (-a * ps / n_dt, b * pc / n_dt);
        Vec3 {
            x: theta_dt
                * (-(x * t2c - y * t2s + self.big_radius) * theta.sin()
                    + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.cos()), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2)+R)*cos(t),t) = 1/2 (-2 R sin(t) + cos(t) (2 cos(t/2) X'(t) + X(t) (-sin(t/2)) - 2 sin(t/2) Y'(t) - Y(t) cos(t/2)) - 2 sin(t) (X(t) cos(t/2) - Y(t) sin(t/2)))
            y: theta_dt * (x_dt * t2s + x * t2c / 2. + y_dt * t2c - y * t2s / 2.), // diff((X(t)*sin(t/2)+Y(t)*cos(t/2)),t) = d/dt(X(t) sin(t/2) + Y(t) cos(t/2)) = sin(t/2) X'(t) + 1/2 X(t) cos(t/2) + cos(t/2) Y'(t) - 1/2 Y(t) sin(t/2)
            z: theta_dt
                * ((x * t2c - y * t2s) * theta.cos()
                    + (x_dt * t2c - x * t2s / 2. - y_dt * t2s - y * t2c / 2.) * theta.sin()), // diff((X(t)*cos(t/2)-Y(t)*sin(t/2))*sin(t),t) = sin(t) cos(t/2) X'(t) - 1/2 X(t) sin(t/2) sin(t) + X(t) cos(t/2) cos(t) - sin(t/2) sin(t) Y'(t) - Y(t) sin(t/2) cos(t) - 1/2 Y(t) sin(t) cos(t/2)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveDescriptor2D {
    Ellipse {
        semi_minor_axis: f64,
        semi_major_axis: f64,
    },
}

struct InstanciatedEllipse {
    semi_major_axis: f64,
    semi_minor_axis: f64,
    cached_curvlinear_abscissa: Vec<f64>,
}

impl InstanciatedEllipse {
    fn new(semi_minor_axis: f32, semi_major_axis: f32) -> Self {
        let mut ret = Self {
            semi_minor_axis: semi_minor_axis as f64,
            semi_major_axis: semi_major_axis as f64,
            cached_curvlinear_abscissa: Vec::new(),
        };
        ret.initialise_cache();
        ret
    }
}

impl Curve2D for InstanciatedEllipse {
    fn Dposition(&self, t: f64) -> DVec2 {
        let t = DTAU * t;
        DVec2 {
            x: self.semi_major_axis * t.cos(),
            y: self.semi_minor_axis * t.sin(),
        }
    }

    fn position(&self, t: f32) -> Vec2 {
        let p = self.Dposition(t as f64);
        Vec2 {
            x: p.x as f32, 
            y: p.y as f32, 
        }
    }

    fn symetry_order(&self) -> usize {
        2
    }

    fn get_cached_curvlinear_abscissa(&self) -> Option<&[f64]> {
        Some(&self.cached_curvlinear_abscissa)
    }

    fn get_cached_curvlinear_abscissa_mut(&mut self) -> Option<&mut Vec<f64>> {
        Some(&mut self.cached_curvlinear_abscissa)
    }
}

impl CurveDescriptor2D {
    fn instanciate(self) -> Arc<dyn Curve2D + Sync + Send> {
        match self {
            Self::Ellipse {
                semi_minor_axis,
                semi_major_axis,
            } => Arc::new(InstanciatedEllipse::new(semi_minor_axis, semi_major_axis)),
        }
    }
}

trait Curve2D {
    fn position(&self, t: f32) -> Vec2;
    fn Dposition(&self, t: f64) -> DVec2;

    fn symetry_order(&self) -> usize;

    fn get_cached_curvlinear_abscissa_mut(&mut self) -> Option<&mut Vec<f64>>;

    fn get_cached_curvlinear_abscissa(&self) -> Option<&[f64]>;

    fn t_for_curvilinear_abscissa(&self, s_objective: f64) -> f64 {
        self.get_cached_curvlinear_abscissa()
            .map(|cache| {
                let idx = search_dicho(s_objective, cache).expect("search dicho");
                idx as f64 / (cache.len() - 1) as f64
            })
            .unwrap_or_else(|| {
                let mut sp = s_objective;
                let mut u = self.position(0.);
                let mut t = 0.;
                for i in 0..=NB_STEPS {
                    // SHOULD COMPUTE A CHEBYCHEB POLY APPROX
                    if sp <= 0. {
                        return t;
                    }
                    t = TAU * i as f64 / NB_STEPS as f64;
                    let v = self.position(t);
                    sp -= (v - u).mag();
                    u = v;
                }
                t
            })
    }

    fn curvilinear_abscissa(&self, t: f64) -> f64 {
        if let Some(cache) = self.get_cached_curvlinear_abscissa() {
            let idx = (t * (cache.len() - 1) as f64) as usize;
            let s = cache[idx];
            let p = self.position(idx as f64 / (cache.len() - 1) as f64);
            let p_ = self.position(t);
            s + (p - p_).mag()
        } else {
            let mut s = 0.;
            let mut p = self.position(0.);
            let t_obj = t;
            for i in 0..NB_STEPS {
                let t = t_obj * (i as f64 / (NB_STEPS - 1) as f64);
                let p_ = self.position(t);
                s += (p - p_).mag();
                p = p_;
            }
            s
        }
    }

    fn initialise_cache(&mut self) {
        let len = if let Some(_) = self.get_cached_curvlinear_abscissa_mut() {
            NB_STEPS
        } else {
            0
        };

        if len > 1 {
            let mut cache = Vec::with_capacity(len);
            let mut s = 0.;
            let mut p = self.position(0.);
            cache.push(s);
            for i in 1..len {
                let t = i as f64 / (len - 1) as f64;
                let p_ = self.position(t);
                s += (p - p_).mag();
                p = p_;
                cache.push(s);
            }
            if let Some(saved_cache) = self.get_cached_curvlinear_abscissa_mut() {
                *saved_cache = cache;
            }
        }
    }

    fn perimeter(&self) -> f64 {
        self.curvilinear_abscissa(1.)
    }
}

fn search_dicho(goal: f64, slice: &[f64]) -> Option<usize> {
    if slice.len() > 0 {
        let mut a = 0usize;
        let mut b = slice.len() - 1;
        while b - a > 2 {
            let c = (b + a) / 2;
            if slice[c] < goal {
                a = c;
            } else {
                b = c;
            }
        }
        if slice[a] < goal {
            Some(a)
        } else {
            Some(b)
        }
    } else {
        None
    }
}

pub(super) struct TwistedTorus {
    instanciated_curve: Arc<dyn Curve2D + Sync + Send>,
    descriptor: TwistedTorusDescriptor,
    scale: f64,
    perimeter: f64,
    nb_turn_per_helix: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwistedTorusDescriptor {
    pub curve: CurveDescriptor2D,
    /// Number of half rotation of the Ellipse per turn
    pub half_twist_count_per_turn: isize,
    /// Radius of the structure,
    pub big_radius: f32,
    pub number_of_helix_per_section: usize,
    pub helix_index_shift_per_turn: isize,
    // Common to all helices of the shape
    pub initial_curvilinear_abscissa: f32,
    pub initial_index_shift: isize,
}

impl TwistedTorus {
    pub fn new(descriptor: TwistedTorusDescriptor) -> Self {
        let instanciated_curve = descriptor.curve.clone().instanciate();
        let scale =
            2. * DH * descriptor.number_of_helix_per_section as f64 / instanciated_curve.perimeter();
        let k = descriptor.helix_index_shift_per_turn;
        let n = descriptor.number_of_helix_per_section;
        let q = descriptor.half_twist_count_per_turn;
        let ρ = instanciated_curve.symetry_order();
        let nb_turn_per_helix = n as usize / gcd(n as isize, k + (n as isize * q)/ρ as isize);
        Self {
            descriptor,
            scale,
            perimeter: instanciated_curve.perimeter(),
            instanciated_curve,
            nb_turn_per_helix,
        }
    }
}

fn gcd(a: isize, b: isize) -> usize {
    let mut a = a.abs() as usize;
    let mut b = b.abs() as usize;

    if a < b {
        std::mem::swap(&mut a, &mut b);
    }

    while b > 0 {
        let b_ = b;
        b = a % b;
        a = b_;
    }
    return a;
}

impl TwistedTorus {
    fn theta(&self, t: f64) -> f64 {
        self.nb_turn_per_helix as f64 * t * DTAU
    }

    fn objective_s(&self, theta: f64) -> f64 {
        self.descriptor.initial_curvilinear_abscissa
            + 2. * DH
                * (self.descriptor.helix_index_shift_per_turn as f64 * theta / TAU
                    + self.descriptor.initial_index_shift as f64)
    }
}

impl Curved for TwistedTorus {
    fn position(&self, t: f32) -> Vec3 {
        let p = self.Dposition(t as f64);
        Vec3 {
            x: p.x as f32,
            y: p.y as f32,
            z: p.z as f32,
        }
    }

    fn Dposition(&self, t: f64) -> DVec3 {
        let theta = self.theta(t);
        let s_theta = self.objective_s(theta) / self.scale;

        let t_curve = self
            .instanciated_curve
            .t_for_curvilinear_abscissa(s_theta.rem_euclid(self.perimeter));
        let point_curve = self.instanciated_curve.position(t_curve) * self.scale;
        let phi = self.descriptor.half_twist_count_per_turn as f64 * theta
            / (self.instanciated_curve.symetry_order() as f64);

        DVec3 {
            x: (point_curve.x * phi.cos() - point_curve.y * phi.sin() + self.descriptor.big_radius)
                * theta.cos(),
            y: point_curve.x * phi.sin() + point_curve.y * phi.cos(),
            z: (point_curve.x * phi.cos() - point_curve.y * phi.sin() + self.descriptor.big_radius)
                * theta.sin(),
        }
    }
}
