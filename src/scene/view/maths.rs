use super::{Vec3, CameraPtr, ProjectionPtr};

/// Use to compute the shortes line between two lines in 3D.
/// Let P1, P2, P3, P4 be 4 points.
/// We want to find the shortest line between the segment (P1, P2) and (P3, P4).
/// This line is a line (Pa, Pb) where Pa = P1 + mua (P2 - P1).
/// This function returns mua
fn mu_unprojection(
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    p4: Vec3,
) -> Option<f32> {
    if (p2 - p1).cross(p4 - p3).mag() > 1e-3 {
        // http://paulbourke.net/geometry/pointlineplane/

        let d = |x: Vec3, y: Vec3, z: Vec3, w: Vec3| (x - y).dot(z - w);
        // mua = ( d1343 d4321 - d1321 d4343 ) / ( d2121 d4343 - d4321 d4321 )

        let mu_num =
            d(p1, p3, p4, p3) * d(p4, p3, p2, p1) - d(p1, p3, p2, p1) * d(p4, p3, p4, p3);
        let mu_den =
            d(p2, p1, p2, p1) * d(p4, p3, p4, p3) - d(p4, p3, p2, p1) * d(p4, p3, p2, p1);
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
    x_coord: f32,
    y_coord: f32,
) -> Option<Vec3> {

    let p1 = camera.borrow().position;
    let p2 = {
        let right = camera.borrow().right_vec();
        let up = camera.borrow().up_vec();
        let direction = camera.borrow().direction();
        p1 + right * (x_coord - 0.5) * projection.borrow().get_ratio() + up * (0.5 - y_coord) + direction
    };

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

