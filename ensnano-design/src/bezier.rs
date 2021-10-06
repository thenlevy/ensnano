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

use ultraviolet::Vec3;
const EPSILON: f32 = 1e-6;

pub struct CubicBezier {
    start: Vec3,
    control1: Vec3,
    control2: Vec3,
    end: Vec3,
    polynomial: CubicBezierPolynom,
}

pub struct CubicBezierConstructor {
    pub start: Vec3,
    pub control1: Vec3,
    pub control2: Vec3,
    pub end: Vec3,
}

impl CubicBezier {
    pub fn new(constructor: CubicBezierConstructor) -> Self {
        let polynomial = CubicBezierPolynom::new(
            constructor.start,
            constructor.control1,
            constructor.control2,
            constructor.end,
        );
        Self {
            start: constructor.start,
            end: constructor.end,
            control1: constructor.control1,
            control2: constructor.control2,
            polynomial,
        }
    }
}

struct CubicBezierPolynom {
    q0: Vec3,
    q1: Vec3,
    q2: Vec3,
    q3: Vec3,
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

        if q23.mag() < 1e-6 && q13.mag() < 1e-6 {
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
                vec![]
            } else {
                let x_roots = x_poly.roots();
                let y_roots = y_poly.roots();
                let z_roots = z_poly.roots();

                let roots = if !x_poly.never_zeroes() {
                    &x_roots
                } else {
                    if !y_poly.never_zeroes() {
                        &y_roots
                    } else {
                        &z_roots
                    }
                };
                let mut ret = Vec::new();
                for t in roots.iter().cloned() {
                    if x_poly.is_zero(t) && y_poly.is_zero(t) && z_poly.is_zero(t) {
                        ret.push(t);
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
        if self.a.abs() < EPSILON {
            let ret = if self.b.abs() < EPSILON {
                vec![]
            } else {
                vec![-self.c / self.b]
            };
            return ret;
        }

        let delta = self.b * self.b - 4. * self.a * self.c;
        if delta > EPSILON {
            vec![
                (-self.b + delta.sqrt()) / (2. * self.a),
                (-self.b - delta.sqrt()) / (2. * self.a),
            ]
        } else if delta < -EPSILON {
            vec![]
        } else {
            vec![-self.b / (2. * self.a)]
        }
    }

    fn is_zero(&self, t: f32) -> bool {
        (self.a * t.powi(2) + self.b * t + self.c).abs() < EPSILON
    }
}

#[cfg(test)]
mod tests {
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
                + control2 * 3. * ((2. - 3. * t) - 3. * t)
                + 6. * end * t
        };
        assert!((poly.acceleration(x) - classical_evaluation(x)).mag() < EPSILON);
        assert!((poly.acceleration(0.0) - classical_evaluation(0.0)).mag() < EPSILON);
        assert!((poly.acceleration(1.0) - classical_evaluation(1.0)).mag() < EPSILON);
    }

    #[test]
    fn find_inflexion_for_s_shape() {
        let start = Vec3::new(188., 229., 0.);
        let control1 = Vec3::new(89., 186., 0.);
        let control2 = Vec3::new(221., 117., 0.);
        let end = Vec3::new(74., 96., 0.);
        let curve = CubicBezier::new(CubicBezierConstructor {
            start,
            control1,
            control2,
            end,
        });
        assert_eq!(curve.polynomial.inflextion_points().len(), 1)
    }
}
