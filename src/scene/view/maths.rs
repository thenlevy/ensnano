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
use super::{CameraPtr, ProjectionPtr, Vec3};

/// Use to compute the shortes line between two lines in 3D.
/// Let P1, P2, P3, P4 be 4 points.
/// We want to find the shortest line between the segment (P1, P2) and (P3, P4).
/// This line is a line (Pa, Pb) where Pa = P1 + mua (P2 - P1).
/// This function returns mua
fn mu_unprojection(p1: Vec3, p2: Vec3, p3: Vec3, p4: Vec3) -> Option<f32> {
    if (p2 - p1).cross(p4 - p3).mag() > 1e-3 {
        // http://paulbourke.net/geometry/pointlineplane/

        let d = |x: Vec3, y: Vec3, z: Vec3, w: Vec3| (x - y).dot(z - w);
        // mua = ( d1343 d4321 - d1321 d4343 ) / ( d2121 d4343 - d4321 d4321 )

        let mu_num = d(p1, p3, p4, p3) * d(p4, p3, p2, p1) - d(p1, p3, p2, p1) * d(p4, p3, p4, p3);
        let mu_den = d(p2, p1, p2, p1) * d(p4, p3, p4, p3) - d(p4, p3, p2, p1) * d(p4, p3, p2, p1);
        Some(mu_num / mu_den)
    } else {
        None
    }
}

/// Create a line that goes from the camera to a point on the screen and project that line on a
/// line of the world
pub fn unproject_point_on_line(
    objective_origin: Vec3,
    objective_direction: Vec3,
    camera: CameraPtr,
    projection: ProjectionPtr,
    x_ndc: f32,
    y_ndc: f32,
) -> Option<Vec3> {
    let p1 = camera.borrow().position;
    let p2 = ndc_to_world(x_ndc, y_ndc, camera, projection);

    let p3 = objective_origin;
    let p4 = objective_origin + objective_direction;
    let mu = mu_unprojection(p3, p4, p1, p2);

    if let Some(mu) = mu {
        // http://paulbourke.net/geometry/pointlineplane/
        Some(p3 + (p4 - p3) * mu)
    } else {
        None
    }
}

/// Shoot a ray from the camera and compute its intersection with the plane P: (p- p0).dot(n) - 0
/// if the intersection if the point p, the return value is the coordinates of the point p.
/// If the line and the plane are parallel, None is returned
pub fn unproject_point_on_plane(
    objective_origin: Vec3,
    objective_normal: Vec3,
    camera: CameraPtr,
    projection: ProjectionPtr,
    x_ndc: f32,
    y_ndc: f32,
) -> Option<Vec3> {
    let p1 = camera.borrow().position;
    let p2 = ndc_to_world(x_ndc, y_ndc, camera, projection);

    let dir = p2 - p1;

    let denom = dir.dot(objective_normal);
    if denom.abs() > 1e-3 {
        let mu = (objective_origin - p1).dot(objective_normal) / denom;
        Some(p1 + mu * dir)
    } else {
        None
    }
}

/// Convert a point on the screen into a point in the world. Usefull for casting rays
fn ndc_to_world(x_ndc: f32, y_ndc: f32, camera: CameraPtr, projection: ProjectionPtr) -> Vec3 {
    let x_screen = 2. * x_ndc - 1.;
    let y_screen = 1. - 2. * y_ndc;

    let p1 = camera.borrow().position;
    let p2 = {
        let correction = (projection.borrow().get_fovy() / 2.).tan();
        let right = camera.borrow().right_vec() * correction;
        let up = camera.borrow().up_vec() * correction;
        let direction = camera.borrow().direction();
        p1 + right * x_screen * projection.borrow().get_ratio() + up * y_screen + direction
    };
    p2
}

pub fn cast_ray(
    x_ndc: f32,
    y_ndc: f32,
    camera: CameraPtr,
    projection: ProjectionPtr,
) -> (Vec3, Vec3) {
    let target = ndc_to_world(x_ndc, y_ndc, camera.clone(), projection);
    (camera.borrow().position, target - camera.borrow().position)
}
