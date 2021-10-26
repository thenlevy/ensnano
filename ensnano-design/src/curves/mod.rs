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

use ultraviolet::{Mat3, Vec3};
const EPSILON: f32 = 1e-6;
const DISCRETISATION_STEP: usize = 100;
use super::{Helix, Parameters};
use std::sync::Arc;
mod bezier;
mod sphere_like_spiral;
pub use bezier::CubicBezierConstructor;
pub use sphere_like_spiral::SphereLikeSpiral;

pub(super) trait Curved {
    fn position(&self, t: f32) -> Vec3;
    fn speed(&self, t: f32) -> Vec3;
    fn acceleration(&self, t: f32) -> Vec3;
}

pub(super) struct Curve {
    geometry: Box<dyn Curved + Sync + Send>,
    positions: Vec<Vec3>,
    axis: Vec<Mat3>,
}

impl Curve {
    pub fn new<T: Curved + 'static + Sync + Send>(geometry: T, parameters: &Parameters) -> Self {
        let mut ret = Self {
            geometry: Box::new(geometry),
            positions: Vec::new(),
            axis: Vec::new(),
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
        let mut p = self.geometry.position(t0);
        let mut len = 0f32;
        for i in 1..=nb_step {
            let t = t0 + (i as f32) / (nb_step as f32) * (t1 - t0);
            let q = self.geometry.position(t);
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
        points.push(self.geometry.position(t));
        let mut current_axis = self.itterative_axis(t, None);
        axis.push(current_axis);

        for _ in 0..nb_points {
            let mut s = 0f32;
            let mut p = self.geometry.position(t);

            while s < len_segment {
                t += small_step;
                let q = self.geometry.position(t);
                current_axis = self.itterative_axis(t, Some(&current_axis));
                s += (q - p).mag();
                p = q;
            }
            points.push(p);
            //axis.push(self.axis(t));
            axis.push(current_axis);
        }

        self.axis = axis;
        self.positions = points;
    }

    fn itterative_axis(&self, t: f32, previous: Option<&Mat3>) -> Mat3 {
        let speed = self.geometry.speed(t);
        if speed.mag_sq() < EPSILON {
            let acceleration = self.geometry.acceleration(t);
            let mat = perpendicular_basis(acceleration);
            return Mat3::new(mat.cols[2], mat.cols[1], mat.cols[0]);
        }

        if let Some(previous) = previous {
            let forward = speed.normalized();
            let up = forward.cross(previous.cols[0]).normalized();
            let right = up.cross(forward);

            Mat3::new(right, up, forward)
        } else {
            perpendicular_basis(speed)
        }
    }

    pub fn nb_points(&self) -> usize {
        self.positions.len()
    }

    pub fn axis_pos(&self, n: usize) -> Option<Vec3> {
        self.positions.get(n).cloned()
    }

    pub fn nucl_pos(&self, n: usize, theta: f32, parameters: &Parameters) -> Option<Vec3> {
        if let Some(matrix) = self.axis.get(n).cloned() {
            let mut ret = matrix
                * Vec3::new(
                    -theta.cos() * parameters.helix_radius,
                    theta.sin() * parameters.helix_radius,
                    0.,
                );
            ret += self.positions[n];
            Some(ret)
        } else {
            None
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CurveDescriptor {
    Bezier(CubicBezierConstructor),
    SphereLikeSpiral(SphereLikeSpiral),
}

impl CurveDescriptor {
    fn into_curve(self, parameters: &Parameters) -> Curve {
        match self {
            Self::Bezier(constructor) => Curve::new(constructor.into_bezier(), parameters),
            Self::SphereLikeSpiral(spiral) => Curve::new(spiral, parameters),
        }
    }
}

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

    pub fn update_bezier(&mut self, parameters: &Parameters) {
        if self.need_curve_update() {
            if let Some(construtor) = self.curve.as_ref() {
                let curve = Arc::new(CurveDescriptor::clone(construtor).into_curve(parameters));
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
