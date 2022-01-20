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

use ultraviolet::{DMat3, DVec3};
const EPSILON: f64 = 1e-6;
const DISCRETISATION_STEP: usize = 100;
use super::{Helix, Parameters};
use std::sync::Arc;
mod bezier;
mod sphere_like_spiral;
mod torus;
mod twist;
use super::GridDescriptor;
pub use bezier::{CubicBezierConstructor, PiecewiseBezier};
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
        ret.discretize(parameters.z_step as f64, DISCRETISATION_STEP);
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
        let mut first_non_negative = true;

        while t < self.geometry.t_max() {
            if first_non_negative && t >= 0.0 {
                first_non_negative = false;
                self.nucl_t0 = points.len();
            }
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

    fn idx_convertsion(&self, n: isize) -> Option<usize> {
        if n > 0 {
            Some(n as usize + self.nucl_t0)
        } else {
            let nb_neg = self.nucl_t0;
            if (-n as usize) <= nb_neg {
                Some(nb_neg - (-n as usize))
            } else {
                None
            }
        }
    }

    pub fn nucl_pos(&self, n: isize, theta: f64, parameters: &Parameters) -> Option<DVec3> {
        let idx = self.idx_convertsion(n)?;
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
        points: Vec<(usize, isize, isize)>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        t_min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        t_max: Option<f64>,
        #[serde(skip, default)]
        instanciated_descriptor: Option<InstanciatedPiecewiseBezierDescriptor>,
    },
}

#[derive(Clone, Debug)]
pub struct InstanciatedPiecewiseBezierDescriptor {
    desc: PiecewiseBezier,
    grids: Arc<GridDescriptor>,
}

impl CurveDescriptor {
    fn into_curve(self, parameters: &Parameters, cache: &mut CurveCache) -> Curve {
        match self {
            Self::Bezier(constructor) => Curve::new(constructor.into_bezier(), parameters),
            Self::SphereLikeSpiral(spiral) => Curve::new(spiral, parameters),
            Self::Twist(twist) => Curve::new(twist, parameters),
            Self::Torus(torus) => Curve::new(torus, parameters),
            Self::TwistedTorus(ref desc) => {
                if let Some(curve) = cache.0.get(desc) {
                    Curve::clone(curve)
                } else {
                    let ret = Curve::new(TwistedTorus::new(desc.clone()), parameters);
                    println!("Number of nucleotides {}", ret.nb_points());
                    cache.0.insert(desc.clone(), ret.clone());
                    ret
                }
            }
            Self::PiecewiseBezier { .. } => todo!(),
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
pub struct CurveCache(HashMap<TwistedTorusDescriptor, Curve>);

#[derive(Clone)]
pub(super) struct InstanciatedCurve {
    source: Arc<CurveDescriptor>,
    pub(super) curve: Arc<Curve>,
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
    pub(super) fn need_curve_update(&self) -> bool {
        let up_to_date = self.curve.as_ref().map(|source| Arc::as_ptr(source))
            == self
                .instanciated_curve
                .as_ref()
                .map(|target| Arc::as_ptr(&target.source));
        !up_to_date
    }

    pub fn update_curve(&mut self, parameters: &Parameters, cached_curve: &mut CurveCache) {
        if self.need_curve_update() {
            if let Some(construtor) = self.curve.as_ref() {
                let curve = Arc::new(
                    CurveDescriptor::clone(construtor).into_curve(parameters, cached_curve),
                );
                self.instanciated_curve = Some(InstanciatedCurve {
                    source: construtor.clone(),
                    curve,
                });
            } else {
                self.instanciated_curve = None;
            }
        }
    }
}
