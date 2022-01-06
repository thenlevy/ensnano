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

use crate::utils::vec_to_dvec;
use ultraviolet::{DVec3, Vec3};

pub struct CubicBezier {
    polynomial: CubicBezierPolynom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubicBezierConstructor {
    pub start: Vec3,
    pub control1: Vec3,
    pub control2: Vec3,
    pub end: Vec3,
}

impl CubicBezierConstructor {
    pub(super) fn into_bezier(self) -> CubicBezier {
        CubicBezier::new(self)
    }
}

impl CubicBezier {
    pub fn new(constructor: CubicBezierConstructor) -> Self {
        let polynomial = CubicBezierPolynom::new(
            vec_to_dvec(constructor.start),
            vec_to_dvec(constructor.control1),
            vec_to_dvec(constructor.control2),
            vec_to_dvec(constructor.end.into()),
        );
        let ret = Self { polynomial };
        ret
    }
}

struct CubicBezierPolynom {
    q0: DVec3,
    q1: DVec3,
    q2: DVec3,
    q3: DVec3,
}

impl CubicBezierPolynom {
    fn new(start: DVec3, control1: DVec3, control2: DVec3, end: DVec3) -> Self {
        let q0 = start;
        let q1 = 3. * (control1 - start);
        let q2 = 3. * (control2 - 2. * control1 + start);
        let q3 = (end - start) + 3. * (control1 - control2);
        Self { q0, q1, q2, q3 }
    }

    fn evaluate(&self, t: f64) -> DVec3 {
        let mut ret = self.q2 + t * self.q3;
        ret = self.q1 + t * ret;
        ret = self.q0 + t * ret;
        ret
    }

    fn derivative(&self, t: f64) -> DVec3 {
        let mut ret = (3. * t) * self.q3 + 2. * self.q2;
        ret = self.q1 + t * ret;
        ret
    }

    // a.k.a second order derivative
    fn acceleration(&self, t: f64) -> DVec3 {
        (6. * t) * self.q3 + 2. * self.q2
    }
}

#[cfg(test)]
mod tests {
    use super::super::Parameters;
    const DNA_PARAMETERS: Parameters = Parameters::DEFAULT;
    const EPSILON: f64 = 1e-6;
    use super::*;
    #[test]
    fn correct_evaluation() {
        let start = DVec3::zero();
        let control1: DVec3 = [1., 2., 3.].into();
        let control2: DVec3 = [-1., 4., 5.].into();
        let end: DVec3 = [0., 0., 10.].into();

        let poly = CubicBezierPolynom::new(start, control1, control2, end);

        let x = std::f64::consts::PI / 10.;

        let classical_evaluation = |t: f64| {
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
        let start = DVec3::zero();
        let control1: DVec3 = [1., 2., 3.].into();
        let control2: DVec3 = [-1., 4., 5.].into();
        let end: DVec3 = [0., 0., 10.].into();

        let poly = CubicBezierPolynom::new(start, control1, control2, end);

        let x = std::f64::consts::PI / 10.;

        let classical_evaluation = |t: f64| {
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
        let start = DVec3::zero();
        let control1: DVec3 = [1., 2., 3.].into();
        let control2: DVec3 = [-1., 4., 5.].into();
        let end: DVec3 = [0., 0., 10.].into();

        let poly = CubicBezierPolynom::new(start, control1, control2, end);

        let x = std::f64::consts::PI / 10.;

        let classical_evaluation = |t: f64| {
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
}

impl super::Curved for CubicBezier {
    fn position(&self, t: f64) -> DVec3 {
        self.polynomial.evaluate(t)
    }

    fn speed(&self, t: f64) -> DVec3 {
        self.polynomial.derivative(t)
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        self.polynomial.acceleration(t)
    }
}

/// A curve that is the concatenation of several cubic bezier curves.
///
/// Let (p_i, u_i)_{0 <= i < n} be the end points, the curve is defined on [0, n] by
/// C(t) = B_i({t}) where i = 1 -  ⌊t⌋ and {t} = t - ⌊t⌋ and B_i is the bezier curve with extremities
/// p_i and p_{i + 1} and control points (p_i + u_i) and p_{i + 1} - u_{i + 1}
pub struct PiecewiseBezier(pub Vec<BezierEnd>);

/// An endpoint of a piecewise bezier curve.
///
/// Let (p_i, u_i)_{0 <= i < n} be the end points, the curve is defined on [0, n] by
/// C(t) = B_i({t}) where i = 1 -  ⌊t⌋ and {t} = t - ⌊t⌋ and B_i is the bezier curve with extremities
/// p_i and p_{i + 1} and control points (p_i + u_i) and p_{i + 1} - u_{i + 1}
pub struct BezierEnd {
    /// The position of the end point, denoted p_i in the above definition
    pub position: Vec3,
    /// The control vector, denoted u_i in the above definition
    pub vector: Vec3,
}

impl super::Curved for PiecewiseBezier {
    fn t_max(&self) -> f64 {
        self.0.len() as f64 - 1.0
    }

    fn position(&self, t: f64) -> DVec3 {
        let i = t.floor() as usize;
        let b_i = CubicBezier::new(CubicBezierConstructor {
            start: self.0[i].position,
            end: self.0[i + 1].position,
            control1: self.0[i].position + self.0[i].vector,
            control2: self.0[i + 1].position - self.0[i + 1].vector,
        });
        b_i.position(t - i as f64)
    }
}
