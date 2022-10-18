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

use ultraviolet::{DMat3, DVec3, Isometry2, Rotor3, Vec2, Vec3};
const EPSILON: f64 = 1e-6;
const DISCRETISATION_STEP: usize = 100;

/// To compute curvilinear abcissa over long distances
const DELTA_MAX: f64 = 256.0;
use crate::{
    grid::{Edge, GridPosition},
    utils::vec_to_dvec,
    BezierPathData, BezierPathId, NAMED_DNA_PARAMETERS,
};

use super::{Helix, Parameters};
use std::sync::Arc;
mod bezier;
mod discretization;
mod revolution;
mod sphere_like_spiral;
mod supertwist;
mod time_nucl_map;
mod torus;
mod tube_spiral;
mod twist;
use super::GridId;
use crate::grid::*;
use bezier::TranslatedPiecewiseBezier;
pub use bezier::{
    BezierControlPoint, BezierEnd, BezierEndCoordinates, CubicBezierConstructor,
    CubicBezierControlPoint,
};
pub(crate) use bezier::{InstanciatedPiecewiseBezier, PieceWiseBezierInstantiator};
pub use revolution::{InterpolatedCurveDescriptor, InterpolationDescriptor};
pub use sphere_like_spiral::{SphereLikeSpiralDescriptor, SphereOrientation};
use std::collections::HashMap;
pub use supertwist::SuperTwist;
pub use time_nucl_map::AbscissaConverter;
pub(crate) use time_nucl_map::{PathTimeMaps, RevolutionCurveTimeMaps};
pub use torus::Torus;
use torus::TwistedTorus;
pub use torus::{CurveDescriptor2D, TwistedTorusDescriptor};
pub use tube_spiral::TubeSpiralDescritor;
pub use twist::{nb_turn_per_100_nt_to_omega, twist_to_omega, Twist};

const EPSILON_DERIVATIVE: f64 = 1e-6;
/// Types that implements this trait represents curves.
pub trait Curved {
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

    /// This method can be overriden to express the fact that a translation should be applied to
    /// every point of the curve. For each point of the curve, the translation is expressed in the
    /// coordinate of the frame associated to the point.
    fn translation(&self) -> Option<DVec3> {
        None
    }

    fn initial_frame(&self) -> Option<DMat3> {
        None
    }

    fn full_turn_at_t(&self) -> Option<f64> {
        None
    }

    /// This method can be overriden to express the fact that a curve needs to be represented by
    /// several helices segments in 2D.
    /// If that is the case, return the index of the corresponding segment for t. This methods must
    /// be increasing.
    fn subdivision_for_t(&self, _t: f64) -> Option<usize> {
        None
    }

    /// This method can be overriden to express the fact that a curve will be the only member of
    /// its synchornization group.
    /// In that case, the abscissa converter can be storred dirrectly in the curve.
    fn is_time_maps_singleton(&self) -> bool {
        false
    }

    fn first_theta(&self) -> Option<f64> {
        None
    }

    fn last_theta(&self) -> Option<f64> {
        None
    }

    /// This method can be overriden to express the fact the a curve is a portion of a surface.
    /// In that case return the information about the surface at the point corresponding to time t
    fn surface_info_time(&self, _t: f64, _helix_id: usize) -> Option<SurfaceInfo> {
        None
    }

    /// This method can be overriden to express the fact the a curve is a portion of a surface.
    /// In that case return the information about the surface at the specified point
    fn surface_info(&self, _point: SurfacePoint) -> Option<SurfaceInfo> {
        None
    }

    /// This method can be overriden to specify the additional isometry associated to each segment
    /// of the helix.
    fn additional_isometry(&self, segment_idx: usize) -> Option<Isometry2> {
        None
    }
}

/// The bounds of the curve. This describe the interval in which t can be taken
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveBounds {
    /// t ∈ [t_min, t_max]
    Finite,
    #[allow(dead_code)]
    /// t ∈ [t_min, +∞[
    PositiveInfinite,
    /// t ∈ ]-∞, +∞[
    BiInfinite,
}

#[derive(Debug, Clone)]
pub struct SurfacePoint {
    pub revolution_angle: f64,
    pub abscissa_along_section: f64,
    pub helix_id: usize,
    pub section_rotation_angle: f64,
    pub reversed_direction: bool,
}

#[derive(Debug)]
pub struct SurfaceInfo {
    pub point: SurfacePoint,
    pub section_tangent: Vec2,
    /// A frame where the up vector is normal to the revolution plane, and the right vector is
    /// tengent to the revolution circle
    pub local_frame: Rotor3,
    pub position: Vec3,
}

#[derive(Clone)]
/// A discretized Curve, with precomputed curve position, and an orthogonal frame moving along the
/// curve.
pub struct Curve {
    /// The object describing the curve.
    pub geometry: Arc<dyn Curved + Sync + Send>,
    /// The precomputed points along the curve for the forward strand
    pub(crate) positions_forward: Vec<DVec3>,
    /// The procomputed points along the curve for the backward strand
    pub(crate) positions_backward: Vec<DVec3>,
    /// The precomputed orthgonal frames moving along the curve for the forward strand
    axis_forward: Vec<DMat3>,
    /// The precomputed orthgonal frames moving along the curve for the backward strand
    axis_backward: Vec<DMat3>,
    /// The precomputed values of the curve's curvature
    curvature: Vec<f64>,
    /// The index in positions that was reached when t became non-negative
    nucl_t0: usize,
    /// The time point at which nucleotides where positioned
    t_nucl: Arc<Vec<f64>>,
    nucl_pos_full_turn: Option<f64>,
    /// The first nucleotide of each additional helix segment needed to represent the curve.
    additional_segment_left: Vec<usize>,
    pub abscissa_converter: Option<AbscissaConverter>,
}

impl Curve {
    pub fn new<T: Curved + 'static + Sync + Send>(geometry: T, parameters: &Parameters) -> Self {
        let mut ret = Self {
            geometry: Arc::new(geometry),
            positions_forward: Vec::new(),
            positions_backward: Vec::new(),
            axis_forward: Vec::new(),
            axis_backward: Vec::new(),
            curvature: Vec::new(),
            nucl_t0: 0,
            t_nucl: Arc::new(Vec::new()),
            nucl_pos_full_turn: None,
            additional_segment_left: Vec::new(),
            abscissa_converter: None,
        };
        let len_segment = ret.geometry.z_step_ratio().unwrap_or(1.0) * parameters.z_step as f64;
        ret.discretize(
            len_segment,
            DISCRETISATION_STEP,
            parameters.inclination as f64,
        );
        ret
    }

    fn compute_length<T: Curved + 'static + Sync + Send>(
        geometry: T,
        parameters: &Parameters,
    ) -> f64 {
        if let Some((x0, x1)) = geometry
            .curvilinear_abscissa(geometry.t_min())
            .zip(geometry.curvilinear_abscissa(geometry.t_max()))
        {
            let ret = x1 - x0;
            println!("length by curvilinear_abscissa = {ret}");
            return x1 - x0;
        }
        quadrature::integrate(
            |x| geometry.speed(x).mag(),
            geometry.t_min(),
            geometry.t_max(),
            1e-5,
        )
        .integral
    }

    fn path<T: Curved + 'static + Sync + Send>(geometry: T, parameters: &Parameters) -> Vec<DVec3> {
        let nb_point = 10_000;
        (0..nb_point)
            .map(|n| {
                geometry.position(geometry.t_min() + n as f64 * geometry.t_max() / nb_point as f64)
            })
            .collect()
    }

    pub fn nb_points(&self) -> usize {
        self.positions_forward
            .len()
            .min(self.positions_backward.len())
    }

    pub fn nb_points_forwards(&self) -> usize {
        self.positions_forward.len()
    }

    pub fn nb_points_backwards(&self) -> usize {
        self.positions_backward.len()
    }

    pub fn axis_pos(&self, n: isize) -> Option<DVec3> {
        let idx = self.idx_convertsion(n)?;
        self.positions_forward.get(idx).cloned()
    }

    pub fn nucl_time(&self, n: isize) -> Option<f64> {
        let idx = self.idx_convertsion(n)?;
        self.t_nucl.get(idx).cloned()
    }

    #[allow(dead_code)]
    pub fn curvature(&self, n: usize) -> Option<f64> {
        self.curvature.get(n).cloned()
    }

    pub fn idx_convertsion(&self, n: isize) -> Option<usize> {
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

    pub fn nucl_pos(
        &self,
        n: isize,
        forward: bool,
        theta: f64,
        parameters: &Parameters,
    ) -> Option<DVec3> {
        use std::f64::consts::{PI, TAU};
        let idx = self.idx_convertsion(n)?;
        let theta = if let Some(real_theta) = self.geometry.theta_shift(parameters) {
            let base_theta = TAU / parameters.bases_per_turn as f64;
            (base_theta - real_theta) * n as f64 + theta
        } else if let Some(pos_full_turn) = self.nucl_pos_full_turn {
            let additional_angle = self
                .axis_forward
                .last()
                .zip(self.axis_forward.first())
                .map(|(f1, f2)| {
                    let y = f2[0].dot(f1[1]);
                    let x = f2[0].dot(f1[0]);
                    y.atan2(x)
                })
                .unwrap_or(0.);
            let final_angle =
                pos_full_turn as f64 * TAU / -parameters.bases_per_turn as f64 + additional_angle;
            let rem = final_angle.rem_euclid(TAU);
            /*
            let mut full_delta = if rem > PI { TAU - rem } else { -rem } + FRAC_PI_2;
            if full_delta > PI {
                full_delta -= TAU;
            }*/
            let mut full_delta = -rem;
            full_delta = full_delta.rem_euclid(TAU);
            if full_delta > PI {
                full_delta -= TAU;
            }

            theta + full_delta / pos_full_turn as f64 * n as f64
        } else {
            theta
        };
        let axis = if forward {
            &self.axis_forward
        } else {
            &self.axis_backward
        };
        let positions = if forward {
            &self.positions_forward
        } else {
            &self.positions_backward
        };
        if let Some(matrix) = axis.get(idx).cloned() {
            let mut ret = matrix
                * DVec3::new(
                    -theta.cos() * parameters.helix_radius as f64,
                    theta.sin() * parameters.helix_radius as f64,
                    0.0,
                );
            ret += positions[idx];
            Some(ret)
        } else {
            None
        }
    }

    pub fn axis_at_pos(&self, position: isize, forward: bool) -> Option<DMat3> {
        let idx = self.idx_convertsion(position)?;
        let axis = if forward {
            &self.axis_forward
        } else {
            &self.axis_backward
        };
        axis.get(idx).cloned()
    }

    pub fn points(&self) -> &[DVec3] {
        &self.positions_forward
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
        if nucl >= nucl_max - 1 {
            match self.geometry.bounds() {
                CurveBounds::BiInfinite | CurveBounds::PositiveInfinite => {
                    let objective = nucl as f64
                        * parameters.z_step as f64
                        * self.geometry.z_step_ratio().unwrap_or(1.)
                        + parameters.inclination as f64;
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

    pub fn update_additional_segments(
        &self,
        segments: &mut Vec<crate::helices::AdditionalHelix2D>,
    ) {
        segments.truncate(self.additional_segment_left.len());
        let mut iter = self
            .additional_segment_left
            .iter()
            .enumerate()
            .map(|(segment_idx, s)| crate::helices::AdditionalHelix2D {
                left: *s as isize - self.nucl_t0 as isize,
                additional_isometry: self.geometry.additional_isometry(segment_idx),
                additional_symmetry: None,
            });

        for s in segments.iter_mut() {
            if let Some(i) = iter.next() {
                s.left = i.left;
            }
        }
        segments.extend(iter);
    }

    pub fn first_theta(&self) -> Option<f64> {
        self.geometry.first_theta()
    }

    pub fn last_theta(&self) -> Option<f64> {
        self.geometry.last_theta()
    }

    pub fn has_its_own_encoded_frame(&self) -> bool {
        self.geometry.translation().is_some()
    }
}

fn perpendicular_basis(point: DVec3) -> DMat3 {
    let norm = point.mag();

    if norm < EPSILON {
        return DMat3::identity();
    }

    let axis_z = point.normalized();

    let mut axis_x = DVec3::unit_x();
    if axis_z.x.abs() >= 0.9 {
        axis_x = DVec3::unit_y();
    }
    let axis_y = axis_z.cross(axis_x).normalized();
    axis_x = axis_y.cross(axis_z).normalized();

    DMat3::new(axis_x, axis_y, axis_z)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// A descriptor of the curve that can be serialized
pub enum CurveDescriptor {
    Bezier(CubicBezierConstructor),
    SphereLikeSpiral(SphereLikeSpiralDescriptor),
    TubeSpiral(TubeSpiralDescritor),
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
    TranslatedPath {
        path_id: BezierPathId,
        translation: Vec3,
    },
    SuperTwist(SuperTwist),
    InterpolatedCurve(InterpolatedCurveDescriptor),
}

const NO_BEZIER: &[BezierEnd] = &[];

impl CurveDescriptor {
    pub fn grid_positions_involved(&self) -> impl Iterator<Item = &GridPosition> {
        let points = if let Self::PiecewiseBezier { points, .. } = self {
            points.as_slice()
        } else {
            NO_BEZIER
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
        grid_reader: &dyn CurveInstantiator,
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
                    t_max: *t_max,
                    t_min: *t_min,
                })
            }
            _ => None,
        }
    }

    pub fn compute_length(&self) -> Option<f64> {
        let desc = InstanciatedCurveDescriptor::try_instanciate(Arc::new(self.clone()))?;
        desc.instance.try_length(&Parameters::GEARY_2014_DNA)
    }

    pub fn path(&self) -> Option<Vec<DVec3>> {
        let desc = InstanciatedCurveDescriptor::try_instanciate(Arc::new(self.clone()))?;
        desc.instance.try_path(&Parameters::GEARY_2014_DNA)
    }
}

#[derive(Clone, Debug)]
/// A descriptor of the the cruve where all reference to design element have been resolved.
/// For example, GridPosition are replaced by their actual position in space.
pub struct InstanciatedCurveDescriptor {
    pub source: Arc<CurveDescriptor>,
    instance: InstanciatedCurveDescriptor_,
}

/// A type that is capable of converting Design object to concrete 3D position.
///
/// This is used to instantiate curves that reference design objects.
pub(super) trait CurveInstantiator {
    fn concrete_grid_position(&self, position: GridPosition) -> Vec3;
    fn orientation(&self, grid: GridId) -> Rotor3;
    fn source(&self) -> FreeGrids;
    fn source_paths(&self) -> Option<BezierPathData>;
    fn get_tengents_between_two_points(
        &self,
        p0: GridPosition,
        p1: GridPosition,
    ) -> Option<(Vec3, Vec3)>;
    fn translate_by_edge(&self, position: GridPosition, edge: Edge) -> Option<GridPosition>;
}

impl InstanciatedCurveDescriptor {
    /// Reads the design data to resolve the reference to elements of the design
    pub(crate) fn instanciate(
        desc: Arc<CurveDescriptor>,
        grid_reader: &dyn CurveInstantiator,
    ) -> Self {
        let instance = match desc.as_ref() {
            CurveDescriptor::Bezier(b) => InstanciatedCurveDescriptor_::Bezier(b.clone()),
            CurveDescriptor::SphereLikeSpiral(s) => {
                InstanciatedCurveDescriptor_::SphereLikeSpiral(s.clone())
            }
            CurveDescriptor::TubeSpiral(t) => InstanciatedCurveDescriptor_::TubeSpiral(t.clone()),
            CurveDescriptor::Twist(t) => InstanciatedCurveDescriptor_::Twist(t.clone()),
            CurveDescriptor::Torus(t) => InstanciatedCurveDescriptor_::Torus(t.clone()),
            CurveDescriptor::SuperTwist(t) => InstanciatedCurveDescriptor_::SuperTwist(t.clone()),
            CurveDescriptor::TwistedTorus(t) => {
                InstanciatedCurveDescriptor_::TwistedTorus(t.clone())
            }
            CurveDescriptor::PiecewiseBezier {
                points,
                t_min,
                t_max,
            } => {
                let instanciated = InstanciatedPiecewiseBezierDescriptor::instanciate(
                    points,
                    grid_reader,
                    *t_min,
                    *t_max,
                );
                InstanciatedCurveDescriptor_::PiecewiseBezier(instanciated)
            }
            CurveDescriptor::TranslatedPath {
                path_id,
                translation,
            } => grid_reader
                .source_paths()
                .and_then(|paths| Self::instanciate_translated_path(*path_id, *translation, paths))
                .unwrap_or_else(|| {
                    let instanciated = InstanciatedPiecewiseBezierDescriptor::instanciate(
                        &[],
                        grid_reader,
                        None,
                        None,
                    );
                    InstanciatedCurveDescriptor_::PiecewiseBezier(instanciated)
                }),
            CurveDescriptor::InterpolatedCurve(desc) => {
                InstanciatedCurveDescriptor_::InterpolatedCurve(desc.clone())
            }
        };
        Self {
            source: desc,
            instance,
        }
    }

    fn instanciate_translated_path(
        path_id: BezierPathId,
        translation: Vec3,
        source_path: BezierPathData,
    ) -> Option<InstanciatedCurveDescriptor_> {
        source_path
            .instanciated_paths
            .get(&path_id)
            .and_then(|path| path.curve_descriptor.as_ref().zip(path.initial_frame()))
            .map(
                |(desc, frame)| InstanciatedCurveDescriptor_::TranslatedBezierPath {
                    path_curve: desc.clone(),
                    initial_frame: frame,
                    translation: vec_to_dvec(translation),
                    paths_data: source_path.clone(),
                },
            )
    }

    pub fn try_instanciate(desc: Arc<CurveDescriptor>) -> Option<Self> {
        let instance = match desc.as_ref() {
            CurveDescriptor::Bezier(b) => Some(InstanciatedCurveDescriptor_::Bezier(b.clone())),
            CurveDescriptor::SphereLikeSpiral(s) => {
                Some(InstanciatedCurveDescriptor_::SphereLikeSpiral(s.clone()))
            }
            CurveDescriptor::TubeSpiral(s) => {
                Some(InstanciatedCurveDescriptor_::TubeSpiral(s.clone()))
            }
            CurveDescriptor::Twist(t) => Some(InstanciatedCurveDescriptor_::Twist(t.clone())),
            CurveDescriptor::Torus(t) => Some(InstanciatedCurveDescriptor_::Torus(t.clone())),
            CurveDescriptor::SuperTwist(t) => {
                Some(InstanciatedCurveDescriptor_::SuperTwist(t.clone()))
            }
            CurveDescriptor::TwistedTorus(t) => {
                Some(InstanciatedCurveDescriptor_::TwistedTorus(t.clone()))
            }
            CurveDescriptor::PiecewiseBezier { .. } => None,
            CurveDescriptor::TranslatedPath { .. } => None,
            CurveDescriptor::InterpolatedCurve(desc) => Some(
                InstanciatedCurveDescriptor_::InterpolatedCurve(desc.clone()),
            ),
        };
        instance.map(|instance| Self {
            source: desc.clone(),
            instance,
        })
    }

    /// Return true if the instanciated curve descriptor was built using these curve descriptor and
    /// grid data
    fn is_up_to_date(
        &self,
        desc: &Arc<CurveDescriptor>,
        grids: &FreeGrids,
        paths_data: &BezierPathData,
    ) -> bool {
        if Arc::ptr_eq(&self.source, desc) {
            match &self.instance {
                InstanciatedCurveDescriptor_::PiecewiseBezier(instanciated_descriptor) => {
                    FreeGrids::ptr_eq(&instanciated_descriptor.grids, grids)
                        && instanciated_descriptor
                            .paths_data
                            .as_ref()
                            .map(|data| BezierPathData::ptr_eq(paths_data, data))
                            .unwrap_or(false)
                }
                InstanciatedCurveDescriptor_::TranslatedBezierPath {
                    paths_data: source_paths,
                    ..
                } => BezierPathData::ptr_eq(paths_data, source_paths),
                _ => true,
            }
        } else {
            false
        }
    }

    pub fn make_curve(&self, parameters: &Parameters, cached_curve: &mut CurveCache) -> Arc<Curve> {
        InstanciatedCurveDescriptor_::clone(&self.instance).into_curve(parameters, cached_curve)
    }

    pub fn get_bezier_controls(&self) -> Option<CubicBezierConstructor> {
        self.instance.get_bezier_controls()
    }

    pub fn bezier_points(&self) -> Vec<Vec3> {
        match &self.instance {
            InstanciatedCurveDescriptor_::Bezier(constructor) => {
                vec![
                    constructor.start,
                    constructor.control1,
                    constructor.control2,
                    constructor.end,
                ]
            }
            InstanciatedCurveDescriptor_::PiecewiseBezier(desc) => {
                let desc = &desc.desc;
                let mut ret: Vec<_> = desc
                    .ends
                    .iter()
                    .zip(desc.ends.iter().skip(1))
                    .flat_map(|(p1, p2)| {
                        vec![
                            p1.position,
                            p1.position + p1.vector_out,
                            p2.position - p2.vector_out,
                        ]
                        .into_iter()
                    })
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
enum InstanciatedCurveDescriptor_ {
    Bezier(CubicBezierConstructor),
    SphereLikeSpiral(SphereLikeSpiralDescriptor),
    TubeSpiral(TubeSpiralDescritor),
    Twist(Twist),
    Torus(Torus),
    SuperTwist(SuperTwist),
    TwistedTorus(TwistedTorusDescriptor),
    PiecewiseBezier(InstanciatedPiecewiseBezierDescriptor),
    TranslatedBezierPath {
        path_curve: Arc<InstanciatedPiecewiseBezier>,
        translation: DVec3,
        initial_frame: DMat3,
        paths_data: BezierPathData,
    },
    InterpolatedCurve(InterpolatedCurveDescriptor),
}

/// An instanciation of a PiecewiseBezier descriptor where reference to grid positions in the
/// design have been replaced by their actual position in space using the data in `grids`.
#[derive(Clone, Debug)]
pub struct InstanciatedPiecewiseBezierDescriptor {
    /// The instanciated descriptor
    desc: InstanciatedPiecewiseBezier,
    /// The data that was used to map grid positions to space position
    grids: FreeGrids,
    /// The data that was used to map BezierVertex to grids
    paths_data: Option<BezierPathData>,
}

struct PieceWiseBezierInstantiator_<'a, 'b> {
    points: &'a [BezierEnd],
    grid_reader: &'b dyn CurveInstantiator,
}

impl<'a, 'b> PieceWiseBezierInstantiator for PieceWiseBezierInstantiator_<'a, 'b> {
    fn nb_vertices(&self) -> usize {
        self.points.len()
    }

    fn position(&self, i: usize) -> Option<Vec3> {
        let vertex = self.points.get(i)?;
        Some(self.grid_reader.concrete_grid_position(vertex.position))
    }

    fn vector_in(&self, _i: usize) -> Option<Vec3> {
        None
    }

    fn vector_out(&self, _i: usize) -> Option<Vec3> {
        None
    }

    fn cyclic(&self) -> bool {
        false
    }
}

impl InstanciatedPiecewiseBezierDescriptor {
    fn instanciate(
        points: &[BezierEnd],
        grid_reader: &dyn CurveInstantiator,
        t_min: Option<f64>,
        t_max: Option<f64>,
    ) -> Self {
        log::debug!("Instanciating {:?}", points);
        let instanciator = PieceWiseBezierInstantiator_ {
            points,
            grid_reader,
        };
        let mut desc = instanciator
            .instantiate()
            .unwrap_or(InstanciatedPiecewiseBezier {
                ends: vec![],
                t_min: None,
                t_max: None,
                cyclic: false,
            });

        desc.t_max = t_max;
        desc.t_min = t_min;
        Self {
            desc,
            grids: grid_reader.source(),
            paths_data: grid_reader.source_paths(),
        }
    }
}

impl InstanciatedCurveDescriptor_ {
    pub fn into_curve(self, parameters: &Parameters, cache: &mut CurveCache) -> Arc<Curve> {
        match self {
            Self::Bezier(constructor) => {
                Arc::new(Curve::new(constructor.into_bezier(), parameters))
            }
            Self::SphereLikeSpiral(spiral) => Arc::new(Curve::new(
                spiral.with_parameters(parameters.clone()),
                parameters,
            )),
            Self::TubeSpiral(spiral) => Arc::new(Curve::new(
                spiral.with_parameters(parameters.clone()),
                parameters,
            )),
            Self::Twist(twist) => Arc::new(Curve::new(twist, parameters)),
            Self::Torus(torus) => Arc::new(Curve::new(torus, parameters)),
            Self::SuperTwist(twist) => Arc::new(Curve::new(twist, parameters)),
            Self::TwistedTorus(ref desc) => {
                if let Some(curve) = cache.0.get(desc) {
                    curve.clone()
                } else {
                    let ret = Arc::new(Curve::new(
                        TwistedTorus::new(desc.clone(), parameters),
                        parameters,
                    ));
                    println!("Number of nucleotides {}", ret.nb_points());
                    cache.0.insert(desc.clone(), ret.clone());
                    ret
                }
            }
            Self::PiecewiseBezier(instanciated_descriptor) => {
                Arc::new(Curve::new(instanciated_descriptor.desc, parameters))
            }
            Self::TranslatedBezierPath {
                path_curve,
                translation,
                initial_frame,
                ..
            } => Arc::new(Curve::new(
                TranslatedPiecewiseBezier {
                    original_curve: path_curve.clone(),
                    translation,
                    initial_frame,
                },
                parameters,
            )),
            Self::InterpolatedCurve(desc) => {
                Arc::new(Curve::new(desc.instanciate(true), parameters))
            }
        }
    }

    pub fn try_into_curve(&self, parameters: &Parameters) -> Option<Arc<Curve>> {
        match self {
            Self::Bezier(constructor) => Some(Arc::new(Curve::new(
                constructor.clone().into_bezier(),
                parameters,
            ))),
            Self::SphereLikeSpiral(spiral) => Some(Arc::new(Curve::new(
                spiral.clone().with_parameters(parameters.clone()),
                parameters,
            ))),
            Self::TubeSpiral(spiral) => Some(Arc::new(Curve::new(
                spiral.clone().with_parameters(parameters.clone()),
                parameters,
            ))),
            Self::Twist(twist) => Some(Arc::new(Curve::new(twist.clone(), parameters))),
            Self::Torus(torus) => Some(Arc::new(Curve::new(torus.clone(), parameters))),
            Self::SuperTwist(twist) => Some(Arc::new(Curve::new(twist.clone(), parameters))),
            Self::TwistedTorus(_) => None,
            Self::PiecewiseBezier(_) => None,
            Self::TranslatedBezierPath {
                path_curve,
                translation,
                initial_frame,
                ..
            } => Some(Arc::new(Curve::new(
                TranslatedPiecewiseBezier {
                    original_curve: path_curve.clone(),
                    translation: *translation,
                    initial_frame: *initial_frame,
                },
                parameters,
            ))),
            Self::InterpolatedCurve(desc) => Some(Arc::new(Curve::new(
                desc.clone().instanciate(true),
                parameters,
            ))),
        }
    }

    fn try_length(&self, parameters: &Parameters) -> Option<f64> {
        match self {
            Self::Bezier(constructor) => Some(Curve::compute_length(
                constructor.clone().into_bezier(),
                parameters,
            )),
            Self::SphereLikeSpiral(spiral) => Some(Curve::compute_length(
                spiral.clone().with_parameters(parameters.clone()),
                parameters,
            )),
            Self::TubeSpiral(spiral) => Some(Curve::compute_length(
                spiral.clone().with_parameters(parameters.clone()),
                parameters,
            )),
            Self::Twist(twist) => Some(Curve::compute_length(twist.clone(), parameters)),
            Self::Torus(torus) => Some(Curve::compute_length(torus.clone(), parameters)),
            Self::SuperTwist(twist) => Some(Curve::compute_length(twist.clone(), parameters)),
            Self::TwistedTorus(_) => None,
            Self::PiecewiseBezier(_) => None,
            Self::TranslatedBezierPath {
                path_curve,
                translation,
                initial_frame,
                ..
            } => Some(Curve::compute_length(
                TranslatedPiecewiseBezier {
                    original_curve: path_curve.clone(),
                    translation: *translation,
                    initial_frame: *initial_frame,
                },
                parameters,
            )),
            Self::InterpolatedCurve(desc) => Some(Curve::compute_length(
                desc.clone().instanciate(false),
                parameters,
            )),
        }
    }

    fn try_path(&self, parameters: &Parameters) -> Option<Vec<DVec3>> {
        match self {
            Self::Bezier(constructor) => {
                Some(Curve::path(constructor.clone().into_bezier(), parameters))
            }
            Self::SphereLikeSpiral(spiral) => Some(Curve::path(
                spiral.clone().with_parameters(parameters.clone()),
                parameters,
            )),
            Self::TubeSpiral(spiral) => Some(Curve::path(
                spiral.clone().with_parameters(parameters.clone()),
                parameters,
            )),
            Self::Twist(twist) => Some(Curve::path(twist.clone(), parameters)),
            Self::Torus(torus) => Some(Curve::path(torus.clone(), parameters)),
            Self::SuperTwist(twist) => Some(Curve::path(twist.clone(), parameters)),
            Self::TwistedTorus(_) => None,
            Self::PiecewiseBezier(_) => None,
            Self::TranslatedBezierPath {
                path_curve,
                translation,
                initial_frame,
                ..
            } => Some(Curve::path(
                TranslatedPiecewiseBezier {
                    original_curve: path_curve.clone(),
                    translation: *translation,
                    initial_frame: *initial_frame,
                },
                parameters,
            )),
            Self::InterpolatedCurve(desc) => {
                Some(Curve::path(desc.clone().instanciate(false), parameters))
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
/// A map from curve descriptor to instanciated curves to avoid duplication of computations
pub struct CurveCache(pub(crate) HashMap<TwistedTorusDescriptor, Arc<Curve>>);

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
        grid_data: &FreeGrids,
        paths_data: &BezierPathData,
    ) -> bool {
        if let Some(current_desc) = self.curve.as_ref() {
            self.instanciated_descriptor
                .as_ref()
                .filter(|desc| desc.is_up_to_date(current_desc, grid_data, paths_data))
                .is_none()
        } else {
            // If helix should not be a curved, the descriptor is up-to-date iff there is no
            // descriptor
            self.instanciated_descriptor.is_some()
        }
    }

    pub(super) fn need_curve_update(
        &self,
        grid_data: &FreeGrids,
        paths_data: &BezierPathData,
    ) -> bool {
        self.need_curve_descriptor_update(grid_data, paths_data) || {
            self.need_curve_update_only()
        }
    }

    fn need_curve_update_only(&self) -> bool {
        let up_to_date = self
            .instanciated_curve
            .as_ref()
            .map(|c| Arc::as_ptr(&c.source))
            == self
                .instanciated_descriptor
                .as_ref()
                .map(|target| Arc::as_ptr(&target));
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
