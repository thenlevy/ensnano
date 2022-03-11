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

use super::{Edge, GridPositionProvider};
use crate::grid::GridPosition;
use crate::utils::vec_to_dvec;
use ultraviolet::{DVec3, Vec3};

use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(usize)]
/// A control point of a cubic bezier curve.
///
/// This enum implements Into<usize>.
pub enum CubicBezierControlPoint {
    Start,
    End,
    Control1,
    Control2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A control point of a bezier curve
pub enum BezierControlPoint {
    /// One of the control points of a cubic bezier curve
    CubicBezier(CubicBezierControlPoint),
    /// One of the control points of a piecewise bezier curve
    PiecewiseBezier(usize),
}

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

    /// Returns an iterator over the control points of self
    pub fn iter(&self) -> impl Iterator<Item = (CubicBezierControlPoint, &Vec3)> {
        vec![
            (CubicBezierControlPoint::Start, &self.start),
            (CubicBezierControlPoint::Control1, &self.control1),
            (CubicBezierControlPoint::Control2, &self.control2),
            (CubicBezierControlPoint::End, &self.end),
        ]
        .into_iter()
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

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::BiInfinite
    }
}

/// A curve that is the concatenation of several cubic bezier curves.
///
/// The process to derive a curve from `ends` is decribed in [BezierEnd](The documentation on `BezierEnd`).
#[derive(Clone, Debug)]
pub(crate) struct InstanciatedPiecewiseBeizer {
    pub ends: Vec<InstanciatedBeizerEnd>,
    pub t_min: Option<f64>,
    pub t_max: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct InstanciatedBeizerEnd {
    pub position: Vec3,
    pub vector_in: Vec3,
    pub vector_out: Vec3,
}

impl InstanciatedPiecewiseBeizer {
    fn t_to_i(&self, t: f64) -> usize {
        if t < 0.0 {
            0
        } else {
            // for t = self.t_max() - 1 we take i = self.t_max() - 2
            (t.floor() as usize).min(self.ends.len() - 2)
        }
    }

    fn ith_cubic_bezier(&self, i: usize) -> CubicBezier {
        CubicBezier::new(CubicBezierConstructor {
            start: self.ends[i].position,
            end: self.ends[i + 1].position,
            control1: self.ends[i].position + self.ends[i].vector_out,
            control2: self.ends[i + 1].position - self.ends[i + 1].vector_in,
        })
    }
}

/// An endpoint of a piecewise bezier curve.
///
/// Let (p_i, c-_i, c+_i)_{0 <= i < n} be the end points, the curve is defined on [0, n] by
/// C(t) = B_i({t}) where i = 1 -  ⌊t⌋ and {t} = t - ⌊t⌋ and B_i is the bezier curve with extremities
/// p_i and p_{i + 1} and whose tengents at positions p_i and p_{i +1} is proportional to c+_i and
/// c-_{i+1}
///
/// Note that c-_0 and c+_{n - 1} are never used
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BezierEnd {
    /// The position of the end point, denoted p_i in the above definition
    pub position: GridPosition,
    /// The inward derivative coeffcient, denoted c-_i in the above definition
    pub inward_coeff: f32,
    /// The outward derivative coefficient, denoted c+_i in the above definition
    pub outward_coeff: f32,
}

impl BezierEnd {
    pub(super) fn translated_by(
        self,
        edge: Edge,
        grid_reader: &dyn GridPositionProvider,
    ) -> Option<Self> {
        grid_reader
            .translate_by_edge(self.position, edge)
            .map(|position| Self { position, ..self })
    }
}

impl super::Curved for InstanciatedPiecewiseBeizer {
    fn t_max(&self) -> f64 {
        let n = self.ends.len() as f64 - 1.0;
        if let Some(tmax) = self.t_max {
            tmax.max(n)
        } else {
            n
        }
    }

    fn t_min(&self) -> f64 {
        if let Some(tmin) = self.t_min {
            tmin.min(0.0)
        } else {
            0.0
        }
    }

    fn position(&self, t: f64) -> DVec3 {
        let i = self.t_to_i(t);
        let b_i = self.ith_cubic_bezier(i);
        b_i.position(t - i as f64)
    }

    fn speed(&self, t: f64) -> DVec3 {
        let i = self.t_to_i(t);
        let b_i = self.ith_cubic_bezier(i);
        b_i.speed(t - i as f64)
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let i = self.t_to_i(t);
        let b_i = self.ith_cubic_bezier(i);
        b_i.acceleration(t - i as f64)
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::BiInfinite
    }
}
