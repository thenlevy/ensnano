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

use ultraviolet::{DMat3, DVec3, Rotor3, Vec3};
const EPSILON: f64 = 1e-6;
const DISCRETISATION_STEP: usize = 100;
const DELTA_MAX: f64 = 256.0;
use crate::grid::{Edge, GridPosition};

use super::{Helix, Parameters};
use std::sync::Arc;
mod bezier;
mod sphere_like_spiral;
mod torus;
mod twist;
use super::GridDescriptor;
pub use bezier::{BezierControlPoint, BezierEnd, CubicBezierConstructor, CubicBezierControlPoint};
pub(crate) use bezier::{InstanciatedBeizerEnd, InstanciatedPiecewiseBeizer};
pub use sphere_like_spiral::SphereLikeSpiral;
use std::collections::HashMap;
pub use torus::Torus;
use torus::TwistedTorus;
pub use torus::{CurveDescriptor2D, TwistedTorusDescriptor};
pub use twist::{nb_turn_per_100_nt_to_omega, twist_to_omega, Twist};

const EPSILON_DERIVATIVE: f64 = 1e-6;
/// Types that implements this trait represents curves.
pub(super) trait Curved {
    /// A function that maps a `0.0 <= t <= Self::t_max` to a point in Space.
    fn position(&self, t: f64) -> DVec3;

    /// The upper bound of the definition domain of `Self::position`.
    ///
    /// By default this is 1.0, but for curves that are infinite
    /// this value may be overriden to allow the helix to have more nucleotides
    fn t_max(&self) -> f64 {
        1.0
    }

    /// The lower bound of the definition domain of `Self::position`.
    ///
    /// By default this is 0.0, but for curves that are infinite
    /// this value may be overriden to allow the helix to have more nucleotides
    fn t_min(&self) -> f64 {
        0.0
    }

    /// The derivative of `Self::position` with respect to time.
    ///
    /// If no implementation is provided, a default implementation is available using numeric
    /// derivation.
    fn speed(&self, t: f64) -> DVec3 {
        (self.position(t + EPSILON_DERIVATIVE / 2.) - self.position(t - EPSILON_DERIVATIVE / 2.))
            / EPSILON_DERIVATIVE
    }

    /// The second derivative of `Self::position` with respect to time.
    ///
    /// If no implementation is provided, a default implementation is provided using numeric
    /// derivation.
    fn acceleration(&self, t: f64) -> DVec3 {
        ((self.position(t + EPSILON_DERIVATIVE) + self.position(t - EPSILON_DERIVATIVE))
            - 2. * self.position(t))
            / (EPSILON_DERIVATIVE * EPSILON_DERIVATIVE)
    }

    /// The curvature of the curve at point `t`.
    ///
    /// This is the radius of the osculating circle of the curve at the point `t`.
    /// See `https://en.wikipedia.org/wiki/Curvature`
    fn curvature(&self, t: f64) -> f64 {
        let speed = self.speed(t);
        let numerator = speed.cross(self.acceleration(t)).mag();
        let denominator = speed.mag().powi(3);
        numerator / denominator
    }

    /// The bounds of the curve
    fn bounds(&self) -> CurveBounds;

    /// Curved for which there exists a closed formula for the curvilinear abscissa can override
    /// this method.
    fn curvilinear_abscissa(&self, _t: f64) -> Option<f64> {
        None
    }

    fn inverse_curvilinear_abscissa(&self, _x: f64) -> Option<f64> {
        None
    }

    /// If the z_step along the curve is not the same than for straight helices, this method should
    /// be overriden
    fn z_step_ratio(&self) -> Option<f64> {
        None
    }

    fn theta_shift(&self, parameters: &Parameters) -> Option<f64> {
        if let Some(real_z_ratio) = self.z_step_ratio() {
            let r = parameters.helix_radius as f64;
            let z = parameters.z_step as f64;
            let real_z = z * real_z_ratio;
            let d1 = parameters.dist_ac() as f64;
            let cos_ret = 1.0 - (d1 * d1 - real_z * real_z) / (r * r * 2.0);
            if cos_ret.abs() > 1.0 {
                None
            } else {
                Some(cos_ret.acos())
            }
        } else {
            None
        }
    }
}

/// The bounds of the curve. This describe the interval in which t can be taken
pub(super) enum CurveBounds {
    /// t ∈ [t_min, t_max]
    Finite,
    #[allow(dead_code)]
    /// t ∈ [t_min, +∞[
    PositiveInfinite,
    /// t ∈ ]-∞, +∞[
    BiInfinite,
}

#[derive(Clone)]
/// A discretized Curve, with precomputed curve position, and an orthogonal frame moving along the
/// curve.
pub(super) struct Curve {
    /// The object describing the curve.
    geometry: Arc<dyn Curved + Sync + Send>,
    /// The precomputed points along the curve
    pub(crate) positions: Vec<DVec3>,
    /// The precomputed orthgonal frames moving along the curve
    axis: Vec<DMat3>,
    /// The precomputed values of the curve's curvature
    curvature: Vec<f64>,
    /// The index in positions that was reached when t became non-negative
    nucl_t0: usize,
}

impl Curve {
    pub fn new<T: Curved + 'static + Sync + Send>(geometry: T, parameters: &Parameters) -> Self {
        let mut ret = Self {
            geometry: Arc::new(geometry),
            positions: Vec::new(),
            axis: Vec::new(),
            curvature: Vec::new(),
            nucl_t0: 0,
        };
        let len_segment = ret.geometry.z_step_ratio().unwrap_or(1.0) * parameters.z_step as f64;
        ret.discretize(len_segment, DISCRETISATION_STEP);
        ret
    }

    pub fn length_by_descretisation(&self, t0: f64, t1: f64, nb_step: usize) -> f64 {
        if t0 > t1 {
            log::error!(
                "Bad parameters ofr length by descritisation: \n t0 {} \n t1 {} \n nb_step {}",
                t0,
                t1,
                nb_step
            );
        }
        if let Some((x0, x1)) = self
            .geometry
            .curvilinear_abscissa(t0)
            .zip(self.geometry.curvilinear_abscissa(t1))
        {
            return x1 - x0;
        }
        let mut p = self.geometry.position(t0);
        let mut len = 0f64;
        for i in 1..=nb_step {
            let t = t0 + (i as f64) / (nb_step as f64) * (t1 - t0);
            let q = self.geometry.position(t);
            len += (q - p).mag();
            p = q;
        }
        len
    }

    fn discretize(&mut self, len_segment: f64, nb_step: usize) {
        let len = self.length_by_descretisation(0., 1., nb_step);
        let nb_points = (len / len_segment) as usize;
        let small_step = 1. / (nb_step as f64 * nb_points as f64);

        let mut points = Vec::with_capacity(nb_points + 1);
        let mut axis = Vec::with_capacity(nb_points + 1);
        let mut curvature = Vec::with_capacity(nb_points + 1);
        let mut t = self.geometry.t_min();
        points.push(self.geometry.position(t));
        let mut current_axis = self.itterative_axis(t, None);
        axis.push(current_axis);
        curvature.push(self.geometry.curvature(t));
        let mut first_non_negative = t < 0.0;

        while t < self.geometry.t_max() {
            if first_non_negative && t >= 0.0 {
                first_non_negative = false;
                self.nucl_t0 = points.len();
            }
            let mut s = 0f64;
            let mut p = self.geometry.position(t);

            if let Some(t_x) = self
                .geometry
                .curvilinear_abscissa(t)
                .and_then(|s| self.geometry.inverse_curvilinear_abscissa(s + len_segment))
            {
                t = t_x;
                p = self.geometry.position(t);
                current_axis = self.itterative_axis(t, Some(&current_axis));
            } else {
                while s < len_segment {
                    t += small_step;
                    let q = self.geometry.position(t);
                    current_axis = self.itterative_axis(t, Some(&current_axis));
                    s += (q - p).mag();
                    p = q;
                }
            }
            points.push(p);
            axis.push(current_axis);
            curvature.push(self.geometry.curvature(t));
        }

        self.axis = axis;
        self.positions = points;
        self.curvature = curvature;
    }

    fn itterative_axis(&self, t: f64, previous: Option<&DMat3>) -> DMat3 {
        let speed = self.geometry.speed(t);
        if speed.mag_sq() < EPSILON {
            let acceleration = self.geometry.acceleration(t);
            let mat = perpendicular_basis(acceleration);
            return DMat3::new(mat.cols[2], mat.cols[1], mat.cols[0]);
        }

        if let Some(previous) = previous {
            let forward = speed.normalized();
            let up = forward.cross(previous.cols[0]).normalized();
            let right = up.cross(forward);

            DMat3::new(right, up, forward)
        } else {
            perpendicular_basis(speed)
        }
    }

    pub fn nb_points(&self) -> usize {
        self.positions.len()
    }

    pub fn axis_pos(&self, n: isize) -> Option<DVec3> {
        let idx = self.idx_convertsion(n)?;
        self.positions.get(idx).cloned()
    }

    #[allow(dead_code)]
    pub fn curvature(&self, n: usize) -> Option<f64> {
        self.curvature.get(n).cloned()
    }

    fn idx_convertsion(&self, n: isize) -> Option<usize> {
        if n >= 0 {
            Some(n as usize + self.nucl_t0)
        } else {
            let nb_neg = self.nucl_t0;
            if ((-n) as usize) <= nb_neg {
                Some(nb_neg - ((-n) as usize))
            } else {
                None
            }
        }
    }

    pub fn nucl_pos(&self, n: isize, theta: f64, parameters: &Parameters) -> Option<DVec3> {
        let idx = self.idx_convertsion(n)?;
        let theta = if let Some(real_theta) = self.geometry.theta_shift(parameters) {
            let base_theta = std::f64::consts::TAU / parameters.bases_per_turn as f64;
            (base_theta - real_theta) * n as f64 + theta
        } else {
            theta
        };
        if let Some(matrix) = self.axis.get(idx).cloned() {
            let mut ret = matrix
                * DVec3::new(
                    -theta.cos() * parameters.helix_radius as f64,
                    theta.sin() * parameters.helix_radius as f64,
                    0.,
                );
            ret += self.positions[idx];
            Some(ret)
        } else {
            None
        }
    }

    pub fn points(&self) -> &[DVec3] {
        &self.positions
    }

    pub fn range(&self) -> std::ops::RangeInclusive<isize> {
        let min = (-(self.nucl_t0 as isize)).max(-100);
        let max = (min + self.nb_points() as isize - 1).min(100);
        min..=max
    }

    pub fn nucl_t0(&self) -> usize {
        self.nucl_t0
    }

    /// Return a value of t_min that would allow self to have nucl
    pub fn left_extension_to_have_nucl(&self, nucl: isize, parameters: &Parameters) -> Option<f64> {
        let nucl_min = -(self.nucl_t0 as isize);
        if nucl < nucl_min {
            if let CurveBounds::BiInfinite = self.geometry.bounds() {
                let objective = (-nucl) as f64
                    * parameters.z_step as f64
                    * self.geometry.z_step_ratio().unwrap_or(1.);
                if let Some(t_min) = self.geometry.inverse_curvilinear_abscissa(objective) {
                    return Some(t_min);
                }
                let mut delta = 1.0;
                while delta < DELTA_MAX {
                    let new_tmin = self.geometry.t_min() - delta;
                    if self.length_by_descretisation(
                        new_tmin,
                        0.0,
                        nucl.abs() as usize * DISCRETISATION_STEP,
                    ) > objective
                    {
                        return Some(new_tmin);
                    }
                    delta *= 2.0;
                }
                None
            } else {
                None
            }
        } else {
            Some(self.geometry.t_min())
        }
    }

    /// Return a value of t_max that would allow self to have nucl
    pub fn right_extension_to_have_nucl(
        &self,
        nucl: isize,
        parameters: &Parameters,
    ) -> Option<f64> {
        let nucl_max = (self.nb_points() - self.nucl_t0) as isize;
        if nucl >= nucl_max {
            match self.geometry.bounds() {
                CurveBounds::BiInfinite | CurveBounds::PositiveInfinite => {
                    let objective = nucl as f64
                        * parameters.z_step as f64
                        * self.geometry.z_step_ratio().unwrap_or(1.);
                    if let Some(t_max) = self.geometry.inverse_curvilinear_abscissa(objective) {
                        return Some(t_max);
                    }
                    let mut delta = 1.0;
                    while delta < DELTA_MAX {
                        let new_tmax = self.geometry.t_max() + delta;
                        if self.length_by_descretisation(
                            0.0,
                            new_tmax,
                            nucl as usize * DISCRETISATION_STEP,
                        ) > objective
                        {
                            return Some(new_tmax);
                        }
                        delta *= 2.0;
                    }
                    None
                }
                CurveBounds::Finite => None,
            }
        } else {
            Some(self.geometry.t_max())
        }
    }
}

fn perpendicular_basis(point: DVec3) -> DMat3 {
    let norm = point.mag();

    if norm < EPSILON {
        return DMat3::identity();
    }

    let axis_z = point.normalized();

    let mut axis_x = DVec3::unit_x();
    if axis_z.x >= 1. - EPSILON {
        axis_x = DVec3::unit_y();
    }
    axis_x = (axis_x.cross(axis_z)).normalized();

    DMat3::new(axis_x, axis_x.cross(-axis_z), axis_z)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// A descriptor of the curve that can be serialized
pub enum CurveDescriptor {
    Bezier(CubicBezierConstructor),
    SphereLikeSpiral(SphereLikeSpiral),
    Twist(Twist),
    Torus(Torus),
    TwistedTorus(TwistedTorusDescriptor),
    PiecewiseBezier {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        t_min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        t_max: Option<f64>,
        points: Vec<BezierEnd>,
    },
}

const NO_BEZIER: &'static [BezierEnd] = &[];

impl CurveDescriptor {
    pub fn grid_positions_involved(&self) -> impl Iterator<Item = &GridPosition> {
        let points = if let Self::PiecewiseBezier { points, .. } = self {
            points.as_slice()
        } else {
            &NO_BEZIER
        };
        points.iter().map(|p| &p.position)
    }
    pub fn set_t_min(&mut self, new_t_min: f64) -> bool {
        match self {
            Self::PiecewiseBezier { t_min, .. } => {
                if matches!(*t_min, Some(x) if x <= new_t_min) {
                    false
                } else {
                    *t_min = Some(new_t_min);
                    true
                }
            }
            Self::Twist(twist) => {
                if matches!(twist.t_min, Some(x) if x <= new_t_min) {
                    false
                } else {
                    twist.t_min = Some(new_t_min);
                    true
                }
            }
            _ => false,
        }
    }

    pub fn set_t_max(&mut self, new_t_max: f64) -> bool {
        match self {
            Self::PiecewiseBezier { t_max, .. } => {
                if matches!(*t_max, Some(x) if x >= new_t_max) {
                    false
                } else {
                    *t_max = Some(new_t_max);
                    true
                }
            }
            Self::Twist(twist) => {
                if matches!(twist.t_max, Some(x) if x >= new_t_max) {
                    false
                } else {
                    twist.t_max = Some(new_t_max);
                    true
                }
            }
            _ => false,
        }
    }

    pub fn t_min(&self) -> Option<f64> {
        match self {
            Self::PiecewiseBezier { t_min, .. } => *t_min,
            Self::Twist(twist) => twist.t_min,
            _ => None,
        }
    }

    pub fn t_max(&self) -> Option<f64> {
        match self {
            Self::PiecewiseBezier { t_max, .. } => *t_max,
            Self::Twist(twist) => twist.t_max,
            _ => None,
        }
    }

    pub(crate) fn translate(
        &self,
        edge: Edge,
        grid_reader: &dyn GridPositionProvider,
    ) -> Option<Self> {
        match self {
            Self::PiecewiseBezier {
                points,
                t_max,
                t_min,
            } => {
                log::debug!("translating {:?}", points);
                let translated_points: Option<Vec<_>> = points
                    .clone()
                    .into_iter()
                    .map(|p| {
                        let ret = p.clone().translated_by(edge, grid_reader);
                        log::debug!("{:?} -> {:?}", p, ret);
                        ret
                    })
                    .collect();
                Some(Self::PiecewiseBezier {
                    points: translated_points?,
                    t_max: t_max.clone(),
                    t_min: t_min.clone(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
/// A descriptor of the the cruve where all reference to design element have been resolved.
/// For example, GridPosition are replaced by their actual position in space.
pub(super) struct InstanciatedCurveDescriptor {
    pub source: Arc<CurveDescriptor>,
    instance: InsanciatedCurveDescriptor_,
}

pub(super) trait GridPositionProvider {
    fn position(&self, position: GridPosition) -> Vec3;
    fn orientation(&self, grid: usize) -> Rotor3;
    fn source(&self) -> Arc<Vec<GridDescriptor>>;
    fn get_tengents_between_two_points(
        &self,
        p0: GridPosition,
        p1: GridPosition,
    ) -> Option<(Vec3, Vec3)>;
    fn translate_by_edge(&self, position: GridPosition, edge: Edge) -> Option<GridPosition>;
}

impl InstanciatedCurveDescriptor {
    /// Reads the design data to resolve the reference to elements of the design
    pub fn instanciate(desc: Arc<CurveDescriptor>, grid_reader: &dyn GridPositionProvider) -> Self {
        let instance = match desc.as_ref() {
            CurveDescriptor::Bezier(b) => InsanciatedCurveDescriptor_::Bezier(b.clone()),
            CurveDescriptor::SphereLikeSpiral(s) => {
                InsanciatedCurveDescriptor_::SphereLikeSpiral(s.clone())
            }
            CurveDescriptor::Twist(t) => InsanciatedCurveDescriptor_::Twist(t.clone()),
            CurveDescriptor::Torus(t) => InsanciatedCurveDescriptor_::Torus(t.clone()),
            CurveDescriptor::TwistedTorus(t) => {
                InsanciatedCurveDescriptor_::TwistedTorus(t.clone())
            }
            CurveDescriptor::PiecewiseBezier {
                points,
                t_min,
                t_max,
            } => {
                let instanciated = InstanciatedPiecewiseBezierDescriptor::instanciate(
                    &points,
                    grid_reader,
                    *t_min,
                    *t_max,
                );
                InsanciatedCurveDescriptor_::PiecewiseBezier(instanciated)
            }
        };
        Self {
            source: desc.clone(),
            instance,
        }
    }

    fn try_instanciate(desc: Arc<CurveDescriptor>) -> Option<Self> {
        let instance = match desc.as_ref() {
            CurveDescriptor::Bezier(b) => Some(InsanciatedCurveDescriptor_::Bezier(b.clone())),
            CurveDescriptor::SphereLikeSpiral(s) => {
                Some(InsanciatedCurveDescriptor_::SphereLikeSpiral(s.clone()))
            }
            CurveDescriptor::Twist(t) => Some(InsanciatedCurveDescriptor_::Twist(t.clone())),
            CurveDescriptor::Torus(t) => Some(InsanciatedCurveDescriptor_::Torus(t.clone())),
            CurveDescriptor::TwistedTorus(t) => {
                Some(InsanciatedCurveDescriptor_::TwistedTorus(t.clone()))
            }
            CurveDescriptor::PiecewiseBezier { .. } => None,
        };
        instance.map(|instance| Self {
            source: desc.clone(),
            instance,
        })
    }

    /// Return true if the instanciated curve descriptor was built using these curve descriptor and
    /// grid data
    fn is_up_to_date(&self, desc: &Arc<CurveDescriptor>, grids: &Arc<Vec<GridDescriptor>>) -> bool {
        if Arc::ptr_eq(&self.source, desc) {
            if let InsanciatedCurveDescriptor_::PiecewiseBezier(instanciated_descriptor) =
                &self.instance
            {
                Arc::ptr_eq(&instanciated_descriptor.grids, grids)
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn make_curve(&self, parameters: &Parameters, cached_curve: &mut CurveCache) -> Arc<Curve> {
        InsanciatedCurveDescriptor_::clone(&self.instance).into_curve(parameters, cached_curve)
    }

    pub fn get_bezier_controls(&self) -> Option<CubicBezierConstructor> {
        self.instance.get_bezier_controls()
    }

    pub fn bezier_points(&self) -> Vec<Vec3> {
        match &self.instance {
            InsanciatedCurveDescriptor_::Bezier(constructor) => {
                vec![
                    constructor.start,
                    constructor.control1,
                    constructor.control2,
                    constructor.end,
                ]
            }
            InsanciatedCurveDescriptor_::PiecewiseBezier(desc) => {
                let desc = &desc.desc;
                let mut ret: Vec<_> = desc
                    .ends
                    .iter()
                    .zip(desc.ends.iter().skip(1))
                    .map(|(p1, p2)| {
                        vec![
                            p1.position,
                            p1.position + p1.vector_out,
                            p2.position - p2.vector_out,
                        ]
                        .into_iter()
                    })
                    .flatten()
                    .collect();
                if let Some(last_point) = desc.ends.iter().last() {
                    ret.push(last_point.position);
                }
                ret
            }
            _ => vec![],
        }
    }
}

#[derive(Clone, Debug)]
enum InsanciatedCurveDescriptor_ {
    Bezier(CubicBezierConstructor),
    SphereLikeSpiral(SphereLikeSpiral),
    Twist(Twist),
    Torus(Torus),
    TwistedTorus(TwistedTorusDescriptor),
    PiecewiseBezier(InstanciatedPiecewiseBezierDescriptor),
}

/// An instanciation of a PiecewiseBezier descriptor where reference to grid positions in the
/// design have been replaced by their actual position in space using the data in `grids`.
#[derive(Clone, Debug)]
pub struct InstanciatedPiecewiseBezierDescriptor {
    /// The instanciated descriptor
    desc: InstanciatedPiecewiseBeizer,
    /// The data that was used to map grid positions to space position
    grids: Arc<Vec<GridDescriptor>>,
}

impl InstanciatedPiecewiseBezierDescriptor {
    fn instanciate(
        points: &[BezierEnd],
        grid_reader: &dyn GridPositionProvider,
        t_min: Option<f64>,
        t_max: Option<f64>,
    ) -> Self {
        log::debug!("Instanciating {:?}", points);
        let ends = if points.len() > 2 {
            let mut bezier_points: Vec<_> = points
                .iter()
                .zip(points.iter().skip(1))
                .zip(points.iter().skip(2))
                .map(|((v_from, v), v_to)| {
                    let pos_from = grid_reader.position(v_from.position);
                    let pos = grid_reader.position(v.position);
                    let pos_to = grid_reader.position(v_to.position);
                    InstanciatedBeizerEnd {
                        position: pos,
                        vector_in: (pos_to - pos_from) / 6.,
                        vector_out: (pos_to - pos_from) / 6.,
                    }
                })
                .collect();
            let first_point = {
                let second_point = &bezier_points[0];
                let pos = grid_reader.position(points[0].position);
                let control = second_point.position - second_point.vector_in;
                InstanciatedBeizerEnd {
                    position: pos,
                    vector_out: (pos - control) / 2.,
                    vector_in: (pos - control) / 2.,
                }
            };
            bezier_points.insert(0, first_point);
            let last_point = {
                // Ok to unwrap because bezier points has length > 2
                let second_to_last_point = bezier_points.last().unwrap();

                // Ok to unwrap because vertices has length > 2
                let pos = grid_reader.position(points.last().unwrap().position);

                let control = second_to_last_point.position + second_to_last_point.vector_out;
                InstanciatedBeizerEnd {
                    position: pos,
                    vector_out: (pos - control) / 2.,
                    vector_in: (pos - control) / 2.,
                }
            };
            bezier_points.push(last_point);
            bezier_points
        } else if points.len() == 2 {
            let pos_first = grid_reader.position(points[0].position);
            let pos_last = grid_reader.position(points[1].position);
            let vec = (pos_last - pos_first) / 3.;
            vec![
                InstanciatedBeizerEnd {
                    position: pos_first,
                    vector_in: vec,
                    vector_out: vec,
                },
                InstanciatedBeizerEnd {
                    position: pos_last,
                    vector_in: -vec,
                    vector_out: -vec,
                },
            ]
        } else {
            vec![]
        };
        let desc = InstanciatedPiecewiseBeizer { ends, t_min, t_max };
        Self {
            desc,
            grids: grid_reader.source(),
        }
    }
}

impl InsanciatedCurveDescriptor_ {
    pub fn into_curve(self, parameters: &Parameters, cache: &mut CurveCache) -> Arc<Curve> {
        match self {
            Self::Bezier(constructor) => {
                Arc::new(Curve::new(constructor.into_bezier(), parameters))
            }
            Self::SphereLikeSpiral(spiral) => Arc::new(Curve::new(spiral, parameters)),
            Self::Twist(twist) => Arc::new(Curve::new(twist, parameters)),
            Self::Torus(torus) => Arc::new(Curve::new(torus, parameters)),
            Self::TwistedTorus(ref desc) => {
                if let Some(curve) = cache.0.get(desc) {
                    curve.clone()
                } else {
                    let ret = Arc::new(Curve::new(TwistedTorus::new(desc.clone()), parameters));
                    println!("Number of nucleotides {}", ret.nb_points());
                    cache.0.insert(desc.clone(), ret.clone());
                    ret
                }
            }
            Self::PiecewiseBezier(instanciated_descriptor) => {
                Arc::new(Curve::new(instanciated_descriptor.desc.clone(), parameters))
            }
        }
    }

    pub fn try_into_curve(&self, parameters: &Parameters) -> Option<Arc<Curve>> {
        match self {
            Self::Bezier(constructor) => Some(Arc::new(Curve::new(
                constructor.clone().into_bezier(),
                parameters,
            ))),
            Self::SphereLikeSpiral(spiral) => {
                Some(Arc::new(Curve::new(spiral.clone(), parameters)))
            }
            Self::Twist(twist) => Some(Arc::new(Curve::new(twist.clone(), parameters))),
            Self::Torus(torus) => Some(Arc::new(Curve::new(torus.clone(), parameters))),
            Self::TwistedTorus(_) => None,
            Self::PiecewiseBezier(_) => None,
        }
    }

    pub fn get_bezier_controls(&self) -> Option<CubicBezierConstructor> {
        if let Self::Bezier(b) = self {
            Some(b.clone())
        } else {
            None
        }
    }
}

#[derive(Default, Clone)]
/// A map from curve descriptor to instanciated curves to avoid duplication of computations
pub struct CurveCache(HashMap<TwistedTorusDescriptor, Arc<Curve>>);

#[derive(Clone)]
/// An instanciated curve with pre-computed nucleotides positions and orientations
pub(super) struct InstanciatedCurve {
    /// A descriptor of the instanciated curve
    pub source: Arc<InstanciatedCurveDescriptor>,
    pub curve: Arc<Curve>,
}

impl std::fmt::Debug for InstanciatedCurve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanciatedCurve")
            .field("source", &Arc::as_ptr(&self.source))
            .finish()
    }
}

impl AsRef<Curve> for InstanciatedCurve {
    fn as_ref(&self) -> &Curve {
        self.curve.as_ref()
    }
}

impl Helix {
    pub(super) fn need_curve_descriptor_update(
        &self,
        grid_data: &Arc<Vec<GridDescriptor>>,
    ) -> bool {
        if let Some(current_desc) = self.curve.as_ref() {
            self.instanciated_descriptor
                .as_ref()
                .filter(|desc| desc.is_up_to_date(current_desc, grid_data))
                .is_none()
        } else {
            // If helix should not be a curved, the descriptor is up-to-date iff there is no
            // descriptor
            self.instanciated_descriptor.is_some()
        }
    }

    pub(super) fn need_curve_update(&self, grid_data: &Arc<Vec<GridDescriptor>>) -> bool {
        self.need_curve_descriptor_update(grid_data) || { self.need_curve_update_only() }
    }

    fn need_curve_update_only(&self) -> bool {
        let up_to_date = self.curve.as_ref().map(|source| Arc::as_ptr(source))
            == self
                .instanciated_descriptor
                .as_ref()
                .map(|target| Arc::as_ptr(&target.source));
        !up_to_date
    }

    pub fn try_update_curve(&mut self, parameters: &Parameters) {
        if let Some(curve) = self.curve.as_ref() {
            if let Some(desc) = InstanciatedCurveDescriptor::try_instanciate(curve.clone()) {
                let desc = Arc::new(desc);
                self.instanciated_descriptor = Some(desc.clone());
                if let Some(curve) = desc.as_ref().instance.try_into_curve(parameters) {
                    self.instanciated_curve = Some(InstanciatedCurve {
                        curve,
                        source: desc,
                    })
                }
            }
        }
    }
}
