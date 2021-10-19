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

use super::{Arc, Helix, Parameters};
use ultraviolet::{Mat3, Vec3};
const EPSILON: f32 = 1e-6;
const DISCRETISATION_STEP: usize = 100;

pub struct CubicBezier {
    start: Vec3,
    control1: Vec3,
    control2: Vec3,
    end: Vec3,
    polynomial: CubicBezierPolynom,
    inflexion_points: Vec<f32>,
    discrete_points: Vec<Vec3>,
    discrete_axis: Vec<Mat3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubicBezierConstructor {
    pub start: Vec3,
    pub control1: Vec3,
    pub control2: Vec3,
    pub end: Vec3,
}

impl CubicBezier {
    pub fn new(constructor: CubicBezierConstructor, parameters: &Parameters) -> Self {
        let polynomial = CubicBezierPolynom::new(
            constructor.start,
            constructor.control1,
            constructor.control2,
            constructor.end,
        );
        let inflexion_points = polynomial.inflextion_points();
        let mut ret = Self {
            start: constructor.start,
            end: constructor.end,
            control1: constructor.control1,
            control2: constructor.control2,
            polynomial,
            inflexion_points,
            discrete_axis: vec![],
            discrete_points: vec![],
        };
        ret.discretize(parameters.z_step, DISCRETISATION_STEP);
        ret
    }

    pub fn length_by_descretisation(&self, t0: f32, t1: f32, nb_step: usize) -> f32 {
        if t0 < 0. || t1 > 1. || t0 > t1 {
            log::error!(
                "Bad parameters ofr length by descritisation: \n t0 {} \n t1 {} \n nb_step {}",
                t0,
                t1,
                nb_step
            );
        }
        let mut p = self.polynomial.evaluate(t0);
        let mut len = 0f32;
        for i in 1..=nb_step {
            let t = t0 + (i as f32) / (nb_step as f32) * (t1 - t0);
            let q = self.polynomial.evaluate(t);
            len += (q - p).mag();
            p = q;
        }
        len
    }

    fn discretize(&mut self, len_segment: f32, nb_step: usize) {
        let len = self.length_by_descretisation(0., 1., nb_step);
        let nb_points = (len / len_segment) as usize;
        let small_step = 1. / (nb_step as f32 * nb_points as f32);

        let mut points = Vec::with_capacity(nb_points + 1);
        let mut axis = Vec::with_capacity(nb_points + 1);
        let mut t = 0f32;
        points.push(self.polynomial.evaluate(t));
        axis.push(self.axis(t));

        for _ in 0..nb_points {
            let mut s = 0f32;
            let mut p = self.polynomial.evaluate(t);

            while s < len_segment {
                t += small_step;
                let q = self.polynomial.evaluate(t);
                s += (q - p).mag();
                p = q;
            }
            points.push(p);
            axis.push(self.axis(t));
        }

        self.discrete_axis = axis;
        self.discrete_points = points;
    }

    fn axis(&self, t: f32) -> Mat3 {
        let speed = self.polynomial.derivative(t);
        let acceleration = self.polynomial.acceleration(t);

        if speed.mag_sq() < EPSILON {
            let mat = perpendicular_basis(acceleration);
            return Mat3::new(mat.cols[2], mat.cols[1], mat.cols[0]);
        }
        if acceleration.mag_sq() < EPSILON {
            return perpendicular_basis(speed);
        }

        let forward = speed.normalized();
        let _normal = acceleration - (acceleration.dot(forward)) * forward;
        let mut normal = _normal.normalized();

        if self.inflexion_points.len() > 0 {
            if t > self.inflexion_points[0] {
                if self.inflexion_points.len() > 1 {
                    if t < self.inflexion_points[1] {
                        normal *= -1.;
                    }
                    // else, nothing to do since we are after 2 inflexion points
                } else {
                    normal *= -1.;
                }
            }
        }

        Mat3::new(normal, forward.cross(normal), forward)
    }

    pub fn nb_points(&self) -> usize {
        self.discrete_axis.len()
    }

    pub fn axis_pos(&self, n: usize) -> Option<Vec3> {
        self.discrete_points.get(n).cloned()
    }

    pub fn nucl_pos(&self, n: usize, theta: f32, parameters: &Parameters) -> Option<Vec3> {
        if let Some(matrix) = self.discrete_axis.get(n).cloned() {
            let mut ret = matrix
                * Vec3::new(
                    -theta.cos() * parameters.helix_radius,
                    theta.sin() * parameters.helix_radius,
                    0.,
                );
            ret += self.discrete_points[n];
            Some(ret)
        } else {
            None
        }
    }
}

struct CubicBezierPolynom {
    q0: Vec3,
    q1: Vec3,
    q2: Vec3,
    q3: Vec3,
}

macro_rules! print_test {
    ($($arg:tt)*) => {
        if cfg!(test) {
            println!($($arg)*)
        }
    }
}

impl CubicBezierPolynom {
    fn new(start: Vec3, control1: Vec3, control2: Vec3, end: Vec3) -> Self {
        let q0 = start;
        let q1 = 3. * (control1 - start);
        let q2 = 3. * (control2 - 2. * control1 + start);
        let q3 = (end - start) + 3. * (control1 - control2);
        Self { q0, q1, q2, q3 }
    }

    fn evaluate(&self, t: f32) -> Vec3 {
        let mut ret = self.q2 + t * self.q3;
        ret = self.q1 + t * ret;
        ret = self.q0 + t * ret;
        ret
    }

    fn derivative(&self, t: f32) -> Vec3 {
        let mut ret = (3. * t) * self.q3 + 2. * self.q2;
        ret = self.q1 + t * ret;
        ret
    }

    // a.k.a second order derivative
    fn acceleration(&self, t: f32) -> Vec3 {
        (6. * t) * self.q3 + 2. * self.q2
    }

    fn inflextion_points(&self) -> Vec<f32> {
        let q23 = 6. * self.q2.cross(self.q3);
        let q13 = 6. * self.q1.cross(self.q3);
        let q12 = 2. * self.q1.cross(self.q2);

        if q23.mag_sq() < 1e-6 && q13.mag_sq() < 1e-6 {
            vec![]
        } else {
            let x_poly = QuadPoly::new(q23.x, q13.x, q12.x);
            let y_poly = QuadPoly::new(q23.y, q13.y, q12.y);
            let z_poly = QuadPoly::new(q23.z, q13.z, q12.z);

            if cfg!(test) {
                println!("x poly {:?}", x_poly);
                println!("y poly {:?}", y_poly);
                println!("z poly {:?}", z_poly);
            }

            if x_poly.never_zeroes() || y_poly.never_zeroes() || z_poly.never_zeroes() {
                print_test!("never zeroes");
                vec![]
            } else {
                let x_roots = x_poly.roots();
                let y_roots = y_poly.roots();
                let z_roots = z_poly.roots();
                print_test!("x_roots, {:?}", x_roots);
                print_test!("y_roots, {:?}", y_roots);
                print_test!("z_roots, {:?}", z_roots);

                let roots = if !x_poly.is_always_zero() {
                    print_test!("x_roots");
                    &x_roots
                } else {
                    if !y_poly.is_always_zero() {
                        print_test!("y_roots");
                        &y_roots
                    } else {
                        print_test!("z_roots");
                        &z_roots
                    }
                };
                let mut ret = Vec::new();
                for t in roots.iter().cloned().filter(|t| 0. <= *t && *t <= 1.) {
                    print_test!("t = {}", t);
                    if x_poly.is_zero_at(t) && y_poly.is_zero_at(t) && z_poly.is_zero_at(t) {
                        ret.push(t);
                    } else if cfg!(test) {
                        if !x_poly.is_zero_at(t) {
                            println!("x = {}", x_poly.evaluate(t));
                        }
                        if !y_poly.is_zero_at(t) {
                            println!("y = {}", y_poly.evaluate(t));
                        }
                        if !z_poly.is_zero_at(t) {
                            println!("z = {}", z_poly.evaluate(t));
                        }
                    }
                }
                ret
            }
        }
    }
}

/// a x*x + b*x + c
#[derive(Debug)]
struct QuadPoly {
    a: f32,
    b: f32,
    c: f32,
}

impl QuadPoly {
    fn new(a: f32, b: f32, c: f32) -> Self {
        Self { a, b, c }
    }

    fn never_zeroes(&self) -> bool {
        self.a.abs() > EPSILON
            && self.b.abs() > EPSILON
            && self.c.abs() > EPSILON
            && self.b * self.b - 4. * self.a * self.c < -EPSILON
    }

    /// Roots the polynomial *without multiplicity*
    fn roots(&self) -> Vec<f32> {
        let a: f64 = self.a as f64;
        let b: f64 = self.b as f64;
        let c: f64 = self.c as f64;
        #[allow(non_snake_case)]
        let EPSILON_64 = EPSILON as f64;
        if a.abs() < EPSILON_64 {
            let ret = if b.abs() < EPSILON_64 {
                vec![]
            } else {
                vec![(-c / b) as f32]
            };
            return ret;
        }

        let delta = b * b - 4. * a * c;
        print_test!("delta {}, sqrt {}", delta, delta.sqrt());
        if delta > EPSILON_64 {
            vec![
                ((-b + delta.sqrt()) / (2. * a)) as f32,
                ((-b - delta.sqrt()) / (2. * a)) as f32,
            ]
        } else if delta < -EPSILON_64 {
            vec![]
        } else {
            vec![-self.b / (2. * self.a)]
        }
    }

    fn is_always_zero(&self) -> bool {
        self.a.abs() < EPSILON && self.b.abs() < EPSILON && self.c.abs() < EPSILON
    }

    fn is_zero_at(&self, t: f32) -> bool {
        (self.a * t.powi(2) + self.b * t + self.c).powi(2) < EPSILON
    }

    #[allow(dead_code)] // used in tests
    fn evaluate(&self, t: f32) -> f32 {
        self.a * t.powi(2) + self.b * t + self.c
    }
}

#[cfg(test)]
mod tests {
    const DNA_PARAMETERS: Parameters = Parameters::DEFAULT;
    use super::*;
    #[test]
    fn correct_evaluation() {
        let start = Vec3::zero();
        let control1: Vec3 = [1., 2., 3.].into();
        let control2: Vec3 = [-1., 4., 5.].into();
        let end: Vec3 = [0., 0., 10.].into();

        let poly = CubicBezierPolynom::new(start, control1, control2, end);

        let x = std::f32::consts::PI / 10.;

        let classical_evaluation = |t: f32| {
            start * (1. - t).powi(3)
                + control1 * 3. * (1. - t).powi(2) * t
                + control2 * 3. * (1. - t) * t.powi(2)
                + end * t.powi(3)
        };
        assert!((poly.evaluate(x) - classical_evaluation(x)).mag() < EPSILON);
        assert!((poly.evaluate(0.0) - classical_evaluation(0.0)).mag() < EPSILON);
        assert!((poly.evaluate(1.0) - classical_evaluation(1.0)).mag() < EPSILON);
    }

    #[test]
    fn correct_derivative() {
        let start = Vec3::zero();
        let control1: Vec3 = [1., 2., 3.].into();
        let control2: Vec3 = [-1., 4., 5.].into();
        let end: Vec3 = [0., 0., 10.].into();

        let poly = CubicBezierPolynom::new(start, control1, control2, end);

        let x = std::f32::consts::PI / 10.;

        let classical_evaluation = |t: f32| {
            -3. * start * (1. - t).powi(2)
                + control1 * 3. * (3. * t.powi(2) - 4. * t + 1.)
                + control2 * 3. * t * (2. - 3. * t)
                + 3. * end * t.powi(2)
        };
        assert!((poly.derivative(x) - classical_evaluation(x)).mag() < EPSILON);
        assert!((poly.derivative(0.0) - classical_evaluation(0.0)).mag() < EPSILON);
        assert!((poly.derivative(1.0) - classical_evaluation(1.0)).mag() < EPSILON);
    }

    #[test]
    fn correct_acceleration() {
        let start = Vec3::zero();
        let control1: Vec3 = [1., 2., 3.].into();
        let control2: Vec3 = [-1., 4., 5.].into();
        let end: Vec3 = [0., 0., 10.].into();

        let poly = CubicBezierPolynom::new(start, control1, control2, end);

        let x = std::f32::consts::PI / 10.;

        let classical_evaluation = |t: f32| {
            6. * start * (1. - t)
                + control1 * 3. * (6. * t - 4.)
                + control2 * 6. * (1. - 3. * t)
                + 6. * end * t
        };
        println!("acc {:?}", poly.acceleration(x));
        println!("classical {:?}", classical_evaluation(x));
        assert!((poly.acceleration(x) - classical_evaluation(x)).mag_sq() < EPSILON);
        assert!((poly.acceleration(0.0) - classical_evaluation(0.0)).mag_sq() < EPSILON);
        assert!((poly.acceleration(1.0) - classical_evaluation(1.0)).mag_sq() < EPSILON);
    }

    #[test]
    fn find_inflexion_for_s_shape() {
        let start = Vec3::new(188., 229., 0.);
        let control1 = Vec3::new(89., 186., 0.);
        let control2 = Vec3::new(221., 117., 0.);
        let end = Vec3::new(74., 96., 0.);
        let curve = CubicBezier::new(
            CubicBezierConstructor {
                start,
                control1,
                control2,
                end,
            },
            &DNA_PARAMETERS,
        );
        assert_eq!(curve.polynomial.inflextion_points().len(), 1)
    }

    #[test]
    fn length_of_flat_line() {
        let start = Vec3::zero();
        let control1 = Vec3::zero();
        let control2 = Vec3::zero();
        let end = Vec3::new(74., 96., 29.);
        let curve = CubicBezier::new(
            CubicBezierConstructor {
                start,
                control1,
                control2,
                end,
            },
            &DNA_PARAMETERS,
        );
        let len = curve.length_by_descretisation(0., 1., 100);
        assert!((len - end.mag()) < EPSILON)
    }

    #[test]
    fn other_s_shape() {
        let start = Vec3 {
            x: 2.65,
            y: 2.35,
            z: 0.0,
        };
        let control1 = Vec3 {
            x: 2.6499999,
            y: 2.3500001,
            z: 19.799992,
        };
        let control2 = Vec3 {
            x: 2.6499999,
            y: 30.374388,
            z: 19.799995,
        };
        let end = Vec3 {
            x: 2.6499999,
            y: 30.374388,
            z: 39.59999,
        };
        let curve = CubicBezier::new(
            CubicBezierConstructor {
                start,
                control1,
                control2,
                end,
            },
            &Parameters::DEFAULT,
        );
        assert_eq!(curve.inflexion_points.len(), 1);
    }
}

#[derive(Clone)]
pub(super) struct InstanciatedBezier {
    source: Arc<CubicBezierConstructor>,
    pub(super) curve: Arc<CubicBezier>,
}

impl std::fmt::Debug for InstanciatedBezier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanciatedBezier")
            .field("source", &Arc::as_ptr(&self.source))
            .finish()
    }
}

impl Helix {
    pub(super) fn need_bezier_update(&self) -> bool {
        let up_to_date = self.bezier.as_ref().map(|source| Arc::as_ptr(source))
            == self
                .instanciated_bezier
                .as_ref()
                .map(|target| Arc::as_ptr(&target.source));
        !up_to_date
    }

    pub fn update_bezier(&mut self, parameters: &Parameters) {
        if self.need_bezier_update() {
            if let Some(construtor) = self.bezier.as_ref() {
                let curve = Arc::new(CubicBezier::new(
                    CubicBezierConstructor::clone(construtor.as_ref()),
                    parameters,
                ));
                self.instanciated_bezier = Some(InstanciatedBezier {
                    source: construtor.clone(),
                    curve,
                });
            } else {
                self.instanciated_bezier = None;
            }
        }
    }
}

fn perpendicular_basis(point: Vec3) -> Mat3 {
    let norm = point.mag();

    if norm < EPSILON {
        return Mat3::identity();
    }

    let axis_z = point.normalized();

    let mut axis_x = Vec3::unit_x();
    if axis_z.x >= 1. - EPSILON {
        axis_x = Vec3::unit_y();
    }
    axis_x = (axis_x.cross(axis_z)).normalized();

    Mat3::new(axis_x, axis_x.cross(-axis_z), axis_z)
}
