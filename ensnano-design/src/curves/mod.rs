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

use ultraviolet::{DMat3, DVec3, Vec3};
const EPSILON: f64 = 1e-6;
const DISCRETISATION_STEP: usize = 100;
use crate::grid::GridPosition;

use super::{Helix, Parameters};
use std::sync::Arc;
mod bezier;
mod sphere_like_spiral;
mod torus;
mod twist;
use super::GridData;
use super::GridDescriptor;
pub use bezier::{BezierEnd, CubicBezierConstructor, PiecewiseBezier};
pub use sphere_like_spiral::SphereLikeSpiral;
use std::collections::HashMap;
pub use torus::Torus;
use torus::TwistedTorus;
pub use torus::{CurveDescriptor2D, TwistedTorusDescriptor};
pub use twist::Twist;

const EPSILON_DERIVATIVE: f64 = 1e-6;
/// Types that implements this trait represents curves.
pub(super) trait Curved {
    /// A function that maps a `0.0 <= t <= Self::t_max` to a point in Space.
    fn position(&self, t: f64) -> DVec3;

    /// The upper bound of the definition domain of `Self::position`.
    ///
    /// By default this is 1.0, but for curves that are defined "piecewise"
    /// it may be more practical to override this value.
    fn t_max(&self) -> f64 {
        1.0
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
}

#[derive(Clone)]
/// A discretized Curve, with precomputed curve position, and an orthogonal frame moving along the
/// curve.
pub(super) struct Curve {
    /// The object describing the curve.
    geometry: Arc<dyn Curved + Sync + Send>,
    /// The precomputed points along the curve
    positions: Vec<DVec3>,
    /// The precomputed orthgonal frames moving along the curve
    axis: Vec<DMat3>,
    /// The precomputed values of the curve's curvature
    curvature: Vec<f64>,
}

impl Curve {
    pub fn new<T: Curved + 'static + Sync + Send>(geometry: T, parameters: &Parameters) -> Self {
        let mut ret = Self {
            geometry: Arc::new(geometry),
            positions: Vec::new(),
            axis: Vec::new(),
            curvature: Vec::new(),
        };
        ret.discretize(parameters.z_step as f64, DISCRETISATION_STEP);
        ret
    }

    pub fn length_by_descretisation(&self, t0: f64, t1: f64, nb_step: usize) -> f64 {
        if t0 < 0. || t1 > self.geometry.t_max() || t0 > t1 {
            log::error!(
                "Bad parameters ofr length by descritisation: \n t0 {} \n t1 {} \n nb_step {}",
                t0,
                t1,
                nb_step
            );
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
        let mut t = 0f64;
        points.push(self.geometry.position(t));
        let mut current_axis = self.itterative_axis(t, None);
        axis.push(current_axis);
        curvature.push(self.geometry.curvature(t));

        while t < self.geometry.t_max() {
            let mut s = 0f64;
            let mut p = self.geometry.position(t);

            while s < len_segment {
                t += small_step;
                let q = self.geometry.position(t);
                current_axis = self.itterative_axis(t, Some(&current_axis));
                s += (q - p).mag();
                p = q;
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

    pub fn axis_pos(&self, n: usize) -> Option<DVec3> {
        self.positions.get(n).cloned()
    }

    pub fn curvature(&self, n: usize) -> Option<f64> {
        self.curvature.get(n).cloned()
    }

    pub fn nucl_pos(&self, n: usize, theta: f64, parameters: &Parameters) -> Option<DVec3> {
        if let Some(matrix) = self.axis.get(n).cloned() {
            let mut ret = matrix
                * DVec3::new(
                    -theta.cos() * parameters.helix_radius as f64,
                    theta.sin() * parameters.helix_radius as f64,
                    0.,
                );
            ret += self.positions[n];
            Some(ret)
        } else {
            None
        }
    }

    pub fn points(&self) -> &[DVec3] {
        &self.positions
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
pub enum CurveDescriptor {
    Bezier(CubicBezierConstructor),
    SphereLikeSpiral(SphereLikeSpiral),
    Twist(Twist),
    Torus(Torus),
    TwistedTorus(TwistedTorusDescriptor),
    PiecewiseBezier {
        points: Vec<GridPosition>,
        tengents: Vec<Vec3>,
    },
}

#[derive(Clone, Debug)]
pub(super) struct InstanciatedCurveDescriptor {
    pub source: Arc<CurveDescriptor>,
    instance: InsanciatedCurveDescriptor_,
}

pub(super) trait GridPositionProvider {
    fn position(&self, position: GridPosition) -> Vec3;
    fn source(&self) -> Arc<Vec<GridDescriptor>>;
}

impl InstanciatedCurveDescriptor {
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
            CurveDescriptor::PiecewiseBezier { points, tengents } => {
                let instanciated = InstanciatedPiecewiseBezierDescriptor::instanciate(
                    &points,
                    &tengents,
                    grid_reader,
                );
                InsanciatedCurveDescriptor_::PiecewiseBezier(instanciated)
            }
        };
        Self {
            source: desc.clone(),
            instance,
        }
    }

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

#[derive(Clone, Debug)]
pub struct InstanciatedPiecewiseBezierDescriptor {
    desc: PiecewiseBezier,
    grids: Arc<Vec<GridDescriptor>>,
}

impl InstanciatedPiecewiseBezierDescriptor {
    fn instanciate(
        points: &[GridPosition],
        tengents: &[Vec3],
        grid_reader: &dyn GridPositionProvider,
    ) -> Self {
        let bezier_ends = points
            .iter()
            .zip(tengents)
            .map(|(p, t)| {
                let position = grid_reader.position(*p);
                BezierEnd {
                    position,
                    vector: *t,
                }
            })
            .collect();
        let desc = PiecewiseBezier(bezier_ends);
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

    pub fn get_bezier_controls(&self) -> Option<CubicBezierConstructor> {
        if let Self::Bezier(b) = self {
            Some(b.clone())
        } else {
            None
        }
    }
}

#[derive(Default, Clone)]
pub struct CurveCache(HashMap<TwistedTorusDescriptor, Arc<Curve>>);

#[derive(Clone)]
pub(super) struct InstanciatedCurve {
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
            self.instanciated_descriptor.is_none()
        }
    }

    pub(super) fn need_curve_update(&self, grid_data: &Arc<Vec<GridDescriptor>>) -> bool {
        self.need_curve_descriptor_update(grid_data) || {
            let up_to_date = self.curve.as_ref().map(|source| Arc::as_ptr(source))
                == self
                    .instanciated_descriptor
                    .as_ref()
                    .map(|target| Arc::as_ptr(&target.source));
            !up_to_date
        }
    }
}
