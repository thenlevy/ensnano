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

use std::sync::Arc;

use super::{CurveInstantiator, Edge};
use crate::grid::GridPosition;
use crate::utils::vec_to_dvec;
use ultraviolet::{DMat3, DVec3, Vec3};

use num_enum::{IntoPrimitive, TryFromPrimitive};

mod instantiator;
pub(crate) use instantiator::PieceWiseBezierInstantiator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            vec_to_dvec(constructor.end),
        );

        Self { polynomial }
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

    pub fn max_x(&self) -> f64 {
        let a = 3. * self.q3.x;
        let b = 2. * self.q2.x;
        let c = self.q1.x;

        let delta = b * b - 4. * a * c;
        let mut ret = self.evaluate(0.).x.max(self.evaluate(1.).x);

        if delta > 0. {
            let root_1 = (delta.sqrt() - b) / 2. / a;
            let root_2 = (-delta.sqrt() - b) / 2. / a;
            for root in [root_1, root_2] {
                if root < 1. && root > 0. {
                    ret = ret.max(self.evaluate(root).x);
                }
            }
        }

        ret
    }

    pub fn min_x(&self) -> f64 {
        let a = 3. * self.q3.x;
        let b = 2. * self.q2.x;
        let c = self.q1.x;

        let delta = b * b - 4. * a * c;
        let mut ret = self.evaluate(0.).x.min(self.evaluate(1.).x);

        if delta > 0. {
            let root_1 = (delta.sqrt() - b) / 2. / a;
            let root_2 = (-delta.sqrt() - b) / 2. / a;
            for root in [root_1, root_2] {
                if root < 1. && root > 0. {
                    ret = ret.min(self.evaluate(root).x);
                }
            }
        }
        ret
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanciatedPiecewiseBezier {
    pub ends: Vec<BezierEndCoordinates>,
    pub t_min: Option<f64>,
    pub t_max: Option<f64>,
    pub cyclic: bool,
    /// An identifier of the PiecewiseBezier generated at random.
    pub id: u64,
    #[serde(default, skip_serializing_if = "is_false")]
    /// Indicate that this curve must be discretized quickly, even at the cost of precision.
    pub discretize_quickly: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

impl PartialEq for InstanciatedPiecewiseBezier {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for InstanciatedPiecewiseBezier {}

impl std::hash::Hash for InstanciatedPiecewiseBezier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BezierEndCoordinates {
    pub position: Vec3,
    pub vector_in: Vec3,
    pub vector_out: Vec3,
}

impl InstanciatedPiecewiseBezier {
    /// Return the index of the bezier curve that determines the position associated to time `t`.
    fn t_to_segment_time(&self, t: f64) -> SegmentTime {
        if t < 0.0 {
            if self.cyclic {
                let t = t.rem_euclid(self.ends.len() as f64);
                self.t_to_segment_time(t)
            } else {
                SegmentTime {
                    segment: 0,
                    time: t,
                }
            }
        } else {
            let (segment, time);
            if self.cyclic && !self.ends.is_empty() {
                segment = t.floor() as usize;
                time = t.fract();
            } else {
                segment = (t.floor() as usize).min(self.ends.len() - 2);
                time = t - segment as f64;
            }
            SegmentTime { segment, time }
        }
    }

    /// Return the CubicBezier with index i
    fn ith_cubic_bezier(&self, i: usize) -> CubicBezier {
        CubicBezier::new(CubicBezierConstructor {
            start: self.ends.iter().cycle().nth(i).unwrap().position,
            end: self.ends.iter().cycle().nth(i + 1).unwrap().position,
            control1: self.ends.iter().cycle().nth(i).unwrap().position
                + self.ends.iter().cycle().nth(i).unwrap().vector_out,
            control2: self.ends.iter().cycle().nth(i + 1).unwrap().position
                - self.ends.iter().cycle().nth(i + 1).unwrap().vector_in,
        })
    }

    pub fn max_x(&self) -> f64 {
        let i_max = if self.cyclic {
            self.ends.len()
        } else {
            self.ends.len() - 1
        };

        (0..=i_max).fold(f64::NEG_INFINITY, |x, i| {
            x.max(self.ith_cubic_bezier(i).polynomial.max_x())
        })
    }

    pub fn min_x(&self) -> f64 {
        let i_max = if self.cyclic {
            self.ends.len()
        } else {
            self.ends.len() - 1
        };

        (0..=i_max).fold(f64::INFINITY, |x, i| {
            x.min(self.ith_cubic_bezier(i).polynomial.min_x())
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
        grid_reader: &dyn CurveInstantiator,
    ) -> Option<Self> {
        grid_reader
            .translate_by_edge(self.position, edge)
            .map(|position| Self { position, ..self })
    }
}

struct SegmentTime {
    segment: usize,
    time: f64,
}

impl super::Curved for InstanciatedPiecewiseBezier {
    fn t_max(&self) -> f64 {
        let n = if self.cyclic {
            self.ends.len() as f64
        } else {
            self.ends.len() as f64 - 1.0
        };
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
        let s = self.t_to_segment_time(t);
        let b_i = self.ith_cubic_bezier(s.segment);
        b_i.position(s.time)
    }

    fn speed(&self, t: f64) -> DVec3 {
        let s = self.t_to_segment_time(t);
        let b_i = self.ith_cubic_bezier(s.segment);
        b_i.speed(s.time)
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        let s = self.t_to_segment_time(t);
        let b_i = self.ith_cubic_bezier(s.segment);
        b_i.acceleration(s.time)
    }

    fn bounds(&self) -> super::CurveBounds {
        super::CurveBounds::BiInfinite
    }

    fn discretize_quickly(&self) -> bool {
        self.discretize_quickly
    }
}

pub(super) struct TranslatedPiecewiseBezier {
    pub original_curve: Arc<InstanciatedPiecewiseBezier>,
    pub translation: DVec3,
    pub initial_frame: DMat3,
}

impl super::Curved for TranslatedPiecewiseBezier {
    fn position(&self, t: f64) -> DVec3 {
        self.original_curve.position(t)
    }

    fn speed(&self, t: f64) -> DVec3 {
        self.original_curve.speed(t)
    }

    fn acceleration(&self, t: f64) -> DVec3 {
        self.original_curve.acceleration(t)
    }

    fn bounds(&self) -> super::CurveBounds {
        self.original_curve.bounds()
    }

    fn t_max(&self) -> f64 {
        if self.original_curve.cyclic {
            self.original_curve.t_max() + 2.
        } else {
            self.original_curve.t_max() + 1.
        }
    }

    fn t_min(&self) -> f64 {
        self.original_curve.t_min()
    }

    fn translation(&self) -> Option<DVec3> {
        Some(self.translation)
    }

    fn initial_frame(&self) -> Option<ultraviolet::DMat3> {
        Some(self.initial_frame)
    }

    fn full_turn_at_t(&self) -> Option<f64> {
        if self.original_curve.cyclic {
            Some(self.original_curve.ends.len() as f64)
        } else {
            Some(self.original_curve.ends.len() as f64 - 1.)
        }
    }

    fn pre_compute_polynomials(&self) -> bool {
        true
    }
}
