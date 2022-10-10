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
use super::maths_3d;
use super::{controller::Data as SurfaceInfoProvider, ClickMode, PhySize, Stereography};
use ensnano_design::{ultraviolet, SurfaceInfo, SurfacePoint};
use ensnano_utils::winit;
use std::cell::RefCell;
use std::f32::consts::{FRAC_PI_2, PI};
use std::rc::Rc;
use std::time::Duration;
use ultraviolet::{Mat3, Mat4, Rotor3, Vec3};
use winit::dpi::PhysicalPosition;
use winit::event::*;

const DEFAULT_DIST_TO_SURFACE: f32 = 20.;
const SURFACE_ABSCISSA_FACTOR: f64 = 1.;
const SURFACE_REVOLUTION_ANGLE_FACTOR: f64 = 1.;

#[derive(Debug, Clone)]
pub struct Camera {
    /// The eye of the camera
    pub position: Vec3,
    /// The orientation of the camera.
    ///
    /// `rotor` is an object that can cast as a transformation of the world basis into the camera's
    /// basis. The camera is looking in the opposite direction of its z axis with its y axis
    /// pointing up.
    pub rotor: Rotor3,
}

pub type CameraPtr = Rc<RefCell<Camera>>;

impl Camera {
    pub fn new<V: Into<Vec3>>(position: V, rotor: Rotor3) -> Self {
        Self {
            position: position.into(),
            rotor,
        }
    }

    /// The view matrix of the camera
    pub fn calc_matrix(&self) -> Mat4 {
        let at = self.position + self.direction();
        Mat4::look_at(self.position, at, self.up_vec())
    }

    /// The direction of the camera, expressed in the world coordinates
    pub fn direction(&self) -> Vec3 {
        self.rotor.reversed() * Vec3::from([0., 0., -1.])
    }

    /// The right vector of the camera, expressed in the world coordinates
    pub fn right_vec(&self) -> Vec3 {
        self.rotor.reversed() * Vec3::from([1., 0., 0.])
    }

    /// The up vector of the camera, expressed in the world coordinates.
    pub fn up_vec(&self) -> Vec3 {
        self.right_vec().cross(self.direction())
    }

    pub fn get_basis(&self) -> maths_3d::Basis3D {
        maths_3d::Basis3D::from_vecs(self.right_vec(), self.up_vec(), -self.direction())
    }
}

#[derive(Debug)]
/// This structure holds the information needed to compute the projection matrix.
pub struct Projection {
    aspect: f32,
    /// Field of view in *radiants*
    fovy: f32,
    znear: f32,
    zfar: f32,
    pub stereographic_zoom: f32,
}

pub type ProjectionPtr = Rc<RefCell<Projection>>;

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy,
            znear,
            zfar,
            stereographic_zoom: ensnano_interactor::consts::DEFAULT_STEREOGRAPHIC_ZOOM,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    /// Computes the projection matrix.
    pub fn calc_matrix(&self) -> Mat4 {
        ultraviolet::projection::rh_yup::perspective_wgpu_dx(
            self.fovy,
            self.aspect,
            self.znear,
            self.zfar,
        )
    }

    pub fn get_fovy(&self) -> f32 {
        self.fovy
    }

    pub fn get_ratio(&self) -> f32 {
        self.aspect
    }

    pub fn cube_dist(&self) -> f32 {
        2f32.sqrt() / (self.fovy / 2.).tan() * 1f32.max(1. / self.aspect)
    }
}

struct ConstrainedRotation {
    phi: f32,
    theta: f32,
    horizon_x: Vec3,
    horizon_z: Vec3,
}

impl ConstrainedRotation {
    fn init(current_rotor: Rotor3, force_horizon: bool) -> Self {
        let current_pos_on_sphere = (current_rotor.reversed() * Vec3::unit_z()).normalized();

        if force_horizon {
            let horizon_x = Vec3::unit_x();
            let horizon_z = Vec3::unit_z();

            let theta = if current_pos_on_sphere.cross(Vec3::unit_y()).mag() > 1e-3 {
                // if the current position is not on a pole, use it to compute theta

                // We project on the zx plane (z to the right, x up) so the z coordinate is the `x` argument of atan2 and the
                // x coordinate is the `y` arugment of atan2
                current_pos_on_sphere
                    .dot(horizon_x)
                    .atan2(current_pos_on_sphere.dot(horizon_z))
            } else {
                // The current right vector is in the xz plane so we can use it to dertermine theta

                // We project the right vector in the x(-z) plane
                let current_right = current_rotor.reversed() * Vec3::from([1., 0., 0.]);
                current_right
                    .dot(Vec3::unit_x())
                    .atan2(current_right.dot(-Vec3::unit_z()))
            };

            let phi = current_pos_on_sphere.dot(Vec3::unit_y()).asin();

            let current_up = current_rotor.reversed() * Vec3::from([0., 1., 0.]);
            let upside_down = if current_up.dot(Vec3::unit_y()) >= 0. {
                1.
            } else {
                // We are looking upside down
                -1.
            };

            Self {
                phi: phi * upside_down,
                theta: theta * upside_down,
                horizon_x: upside_down * horizon_x,
                horizon_z,
            }
        } else {
            let horizon_x = current_rotor.reversed() * Vec3::unit_x();

            /*
            let horizon_z = if horizon_x.cross(Vec3::unit_y()).mag() > 1e-3 {
                let current_up = current_rotor.reversed() * Vec3::from([0., 1., 0.]);
                let upside_down = if current_up.dot(Vec3::unit_y()) >= 0. {
                    1.
                } else {
                    // We are looking upside down
                    -1.
                };
               upside_down * horizon_x.cross(Vec3::unit_y()).normalized()
            } else {
                current_rotor.reversed() * Vec3::unit_z()
            };*/
            let horizon_z = current_rotor.reversed() * Vec3::unit_z();
            let theta = current_pos_on_sphere
                .dot(horizon_x)
                .atan2(current_pos_on_sphere.dot(horizon_z));
            let up = horizon_z.cross(horizon_x);
            let phi = current_pos_on_sphere.dot(up).asin();
            Self {
                phi,
                theta,
                horizon_x,
                horizon_z,
            }
        }
    }

    fn add_angle_xz(&mut self, delta_xz: f32) {
        self.theta += delta_xz;
    }

    fn add_angle_yz(&mut self, delta_yz: f32) {
        self.phi += delta_yz;
    }

    fn compute_rotor(&self) -> Rotor3 {
        let horizon_y = self.horizon_z.cross(self.horizon_x);

        let position_on_sphere = (horizon_y * self.phi.sin()
            + self.phi.cos()
                * (self.horizon_z * self.theta.cos() + self.horizon_x * self.theta.sin()))
        .normalized();

        let right =
            (self.theta.cos() * self.horizon_x - self.theta.sin() * self.horizon_z).normalized();

        let up = position_on_sphere.cross(right).normalized();

        Mat3::new(right, up, position_on_sphere)
            .into_rotor3()
            .reversed()
    }
}

pub struct CameraController {
    speed: f32,
    amount_up: f32,
    amount_down: f32,
    amount_left: f32,
    amount_right: f32,
    mouse_horizontal: f32,
    mouse_vertical: f32,
    scroll: f32,
    #[allow(dead_code)]
    last_rotor: Rotor3,
    processed_move: bool,
    camera: CameraPtr,
    cam0: Camera,
    projection: ProjectionPtr,
    pivot_point: Option<FiniteVec3>,
    zoom_plane: Option<Plane>,
    x_scroll: f32,
    y_scroll: f32,
    /// The xz angle accumulated during a free camera rotation
    free_xz_angle: f32,
    /// The yz angle accumulated during a free camera rotation
    free_yz_angle: f32,
    current_constrained_rotation: Option<ConstrainedRotation>,
    surface_point: Option<SurfacePoint>,
    surface_point0: Option<SurfacePoint>,
    dist_to_surface: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct FiniteVec3(Vec3);

use std::convert::TryFrom;
impl TryFrom<Vec3> for FiniteVec3 {
    type Error = ();
    fn try_from(value: Vec3) -> Result<Self, Self::Error> {
        if !value.x.is_finite() || !value.y.is_finite() || !value.z.is_finite() {
            Err(())
        } else {
            Ok(Self(value))
        }
    }
}

impl FiniteVec3 {
    pub fn zero() -> Self {
        Self(Vec3::zero())
    }
}

impl From<FiniteVec3> for Vec3 {
    fn from(v: FiniteVec3) -> Self {
        v.0
    }
}

impl CameraController {
    pub fn new(speed: f32, camera: CameraPtr, projection: ProjectionPtr) -> Self {
        Self {
            speed,
            amount_left: 0.0,
            amount_right: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            mouse_horizontal: 0.0,
            mouse_vertical: 0.0,
            scroll: 0.0,
            last_rotor: camera.borrow().rotor,
            processed_move: false,
            camera: camera.clone(),
            cam0: camera.borrow().clone(), // clone the camera not the pointer !
            projection,
            pivot_point: None,
            zoom_plane: None,
            x_scroll: 0.,
            y_scroll: 0.,
            free_xz_angle: 0.,
            free_yz_angle: 0.,
            current_constrained_rotation: None,
            surface_point: None,
            surface_point0: None,
            dist_to_surface: None,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::Up => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::Down => {
                self.amount_down = amount;
                true
            }
            VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::H if amount > 0. => {
                self.rotate_camera_around(
                    FRAC_PI_2 / 20.,
                    0.,
                    self.pivot_point.unwrap_or_else(FiniteVec3::zero),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            VirtualKeyCode::L if amount > 0. => {
                self.rotate_camera_around(
                    -FRAC_PI_2 / 20.,
                    0.,
                    self.pivot_point.unwrap_or_else(FiniteVec3::zero),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            VirtualKeyCode::J if amount > 0. => {
                self.rotate_camera_around(
                    0.,
                    FRAC_PI_2 / 20.,
                    self.pivot_point.unwrap_or_else(FiniteVec3::zero),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            VirtualKeyCode::K if amount > 0. => {
                self.rotate_camera_around(
                    0.,
                    -FRAC_PI_2 / 20.,
                    self.pivot_point.unwrap_or_else(FiniteVec3::zero),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            _ => false,
        }
    }

    pub fn is_moving(&self) -> bool {
        self.amount_down > 0.
            || self.amount_up > 0.
            || self.amount_right > 0.
            || self.amount_left > 0.
            || self.scroll.abs() > 0.
    }

    pub fn stop_camera_movement(&mut self) {
        self.amount_left = 0.;
        self.amount_right = 0.;
        self.amount_up = 0.;
        self.amount_down = 0.;
    }

    pub fn set_pivot_point(&mut self, point: Option<FiniteVec3>) {
        if let Some(origin) = point {
            let origin: Vec3 = origin.into();
            self.zoom_plane = Some(Plane {
                origin,
                normal: (self.camera.borrow().position - origin),
            });
        }
        self.pivot_point = point
    }

    pub fn get_projection(
        &self,
        origin: Vec3,
        x: f64,
        y: f64,
        streography: Option<&Stereography>,
    ) -> Vec3 {
        let plane = Plane {
            origin,
            normal: (self.camera.borrow().position - origin),
        };
        maths_3d::unproject_point_on_plane(
            plane.origin,
            plane.normal,
            self.camera.clone(),
            self.projection.clone(),
            x as f32,
            y as f32,
            streography,
        )
        .unwrap_or(origin)
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.mouse_horizontal = -mouse_dx as f32;
        self.mouse_vertical = -mouse_dy as f32;
        self.processed_move = true;
    }

    pub fn process_scroll(
        &mut self,
        delta: &MouseScrollDelta,
        x_cursor: f32,
        y_cursor: f32,
        sensitivity: f32,
    ) {
        self.x_scroll = x_cursor;
        self.y_scroll = y_cursor;
        self.scroll = match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll.min(1.).max(-1.),
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                scroll.signum() as f32
            }
        } * sensitivity;
    }

    pub fn update_stereographic_zoom(&mut self, delta: &MouseScrollDelta) {
        let direction = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll.signum(),
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                scroll.signum() as f32
            }
        };
        self.projection.borrow_mut().stereographic_zoom *=
            ensnano_interactor::consts::STEREOGRAPHIC_ZOOM_STEP.powf(direction);
    }

    /// Rotate the head of the camera on its yz plane and xz plane according to the values of
    /// self.mouse_horizontal and self.mouse_vertical
    fn process_angles(&mut self) {
        let xz_angle = self.mouse_horizontal * FRAC_PI_2;
        let yz_angle = self.mouse_vertical * FRAC_PI_2;

        // We want to build a rotation that will
        // first maps (1, 0, 0) to (cos(yz_angle), -sin(yz_angle), 0)
        // and then (0, 1, 0) to (0, cos(xz_angle), -sin(yz_angle))

        let rotation = Rotor3::from_rotation_xz(xz_angle) * Rotor3::from_rotation_yz(yz_angle);

        self.camera.borrow_mut().rotor = rotation * self.cam0.rotor;

        // Since we have rotated the camera we can reset those values
        self.mouse_horizontal = 0.0;
        self.mouse_vertical = 0.0;
    }

    /// Translate the camera
    fn translate_camera(&mut self, surface_info_provider: &dyn SurfaceInfoProvider) {
        let right = self.mouse_horizontal;
        let up = -self.mouse_vertical;

        if let Some(mut point) = self.surface_point0.clone() {
            log::info!("Got point");
            let sign = if point.reversed_direction { -1. } else { 1. };
            point.abscissa_along_section += up as f64 * SURFACE_ABSCISSA_FACTOR;
            point.revolution_angle += right as f64 * SURFACE_REVOLUTION_ANGLE_FACTOR * sign;

            if let Some(surface_info) = surface_info_provider.get_surface_info(point.clone()) {
                let cam_pos = surface_info.position
                    + self.dist_to_surface.unwrap_or(DEFAULT_DIST_TO_SURFACE)
                        * Vec3::unit_z().rotated_by(surface_info.local_frame);
                self.teleport_camera(cam_pos, surface_info.local_frame.reversed());
            }
            self.surface_point = Some(point);
        } else {
            let scale = if let Some(pivot) = self.pivot_point {
                (Vec3::from(pivot) - self.camera.borrow().position)
                    .dot(self.camera.borrow().direction())
            } else if let Some(origin) = self.zoom_plane.as_ref().map(|plane| plane.origin) {
                (origin - self.camera.borrow().position).dot(self.camera.borrow().direction())
            } else {
                10.
            };

            let right_vec =
                self.camera.borrow().right_vec() * scale * self.projection.borrow().get_ratio();
            let up_vec = self.camera.borrow().up_vec() * scale;

            let old_pos = self.cam0.position;
            self.camera.borrow_mut().position = old_pos + right * right_vec + up * up_vec;
        }

        self.mouse_horizontal = 0.0;
        self.mouse_vertical = 0.0;
    }

    /// Move the camera according to the keyboard input
    fn move_camera(
        &mut self,
        dt: Duration,
        modifier: &ModifiersState,
        surface_info_provider: &dyn SurfaceInfoProvider,
    ) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let right = self.camera.borrow().right_vec();
        let up_vec = self.camera.borrow().up_vec();
        let forward_vec = self.camera.borrow().direction();

        let (amount_right, amount_roll_right, amount_rotate_right) = if modifier.shift() {
            (0., 0., self.amount_right)
        } else if modifier.alt() {
            (0., self.amount_right, 0.)
        } else {
            (self.amount_right, 0., 0.)
        };

        let (amount_left, amount_roll_left, amount_rotate_left) = if modifier.shift() {
            (0., 0., self.amount_left)
        } else if modifier.alt() {
            (0., self.amount_left, 0.)
        } else {
            (self.amount_left, 0., 0.)
        };

        let (amount_up, amount_forward, amount_rotate_up) = if modifier.alt() {
            (0., 0., self.amount_up)
        } else if modifier.shift() {
            (0., self.amount_up, 0.)
        } else {
            (self.amount_up, 0., 0.)
        };

        let (amount_down, amount_backward, amount_rotate_down) = if modifier.alt() {
            (0., 0., self.amount_down)
        } else if modifier.shift() {
            (0., self.amount_down, 0.)
        } else {
            (self.amount_down, 0., 0.)
        };

        let rotation_speed = 0.1;
        {
            let mut camera = self.camera.borrow_mut();
            camera.position += right * (amount_right - amount_left) * self.speed * dt;
            camera.position += up_vec * (amount_up - amount_down) * self.speed * dt;
            camera.position += forward_vec * (amount_forward - amount_backward) * self.speed * dt;
            camera.rotor = Rotor3::from_rotation_xz(
                (amount_rotate_left - amount_rotate_right) * rotation_speed * self.speed * dt,
            ) * camera.rotor;
            camera.rotor = Rotor3::from_rotation_yz(
                (amount_rotate_down - amount_rotate_up) * rotation_speed * self.speed * dt,
            ) * camera.rotor;
            camera.rotor = Rotor3::from_rotation_xy(
                (amount_roll_left - amount_roll_right) * rotation_speed * self.speed * dt,
            ) * camera.rotor;
        }

        let pivot = self.zoom_plane.as_ref().and_then(|plane| {
            if self
                .camera
                .borrow()
                .direction()
                .normalized()
                .dot(-plane.normal.normalized())
                > 0.9
            {
                maths_3d::unproject_point_on_plane(
                    plane.origin,
                    plane.normal,
                    self.camera.clone(),
                    self.projection.clone(),
                    self.x_scroll,
                    self.y_scroll,
                    None,
                )
            } else {
                None
            }
        });

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let scrollward = if let Some(pivot) = pivot {
            let to_pivot = pivot - self.camera.borrow().position;
            let score = to_pivot
                .normalized()
                .dot(self.camera.borrow().direction().normalized());
            if score < 0. {
                self.camera.borrow().direction()
            } else if (pivot - self.camera.borrow().position).mag() < 0.1 {
                1.1 * to_pivot
            } else {
                to_pivot.normalized()
            }
        } else {
            self.camera.borrow().direction()
        };
        {
            if let Some((dist_to_surface, surface_info)) = self.dist_to_surface.as_mut().zip(
                self.surface_point
                    .as_ref()
                    .and_then(|p| surface_info_provider.get_surface_info(p.clone())),
            ) {
                if self.scroll > 0. {
                    *dist_to_surface /= 1.1
                } else {
                    *dist_to_surface *= 1.1
                };
                let cam_pos = surface_info.position
                    + self.dist_to_surface.unwrap_or(DEFAULT_DIST_TO_SURFACE)
                        * Vec3::unit_z().rotated_by(surface_info.local_frame);
                self.teleport_camera(cam_pos, surface_info.local_frame.reversed());
            } else {
                let mut camera = self.camera.borrow_mut();
                camera.position += scrollward * self.scroll * self.speed * 3.0;
            }
        }
        self.cam0 = self.camera.borrow().clone();
        self.scroll = 0.;
    }

    pub(super) fn update_camera(
        &mut self,
        dt: Duration,
        click_mode: ClickMode,
        modifier: &ModifiersState,
        surface_info_provider: &dyn SurfaceInfoProvider,
    ) {
        if self.processed_move {
            match click_mode {
                ClickMode::RotateCam => self.process_angles(),
                ClickMode::TranslateCam => self.translate_camera(surface_info_provider),
            }
        }
        if self.is_moving() {
            self.move_camera(dt, modifier, surface_info_provider);
        }
    }

    pub fn init_movement(&mut self, along_surface: bool) {
        self.processed_move = false;
        if !along_surface {
            log::info!("Setting info to None");
            self.surface_point0 = None;
            self.surface_point = None;
        }
    }

    pub fn init_constrained_rotation(&mut self, force_horizon: bool) {
        self.current_constrained_rotation = Some(ConstrainedRotation::init(
            self.camera.borrow().rotor,
            force_horizon,
        ));
    }

    pub fn end_constrained_rotation(&mut self) {
        self.current_constrained_rotation = None;
    }

    pub fn end_movement(&mut self) {
        self.last_rotor = self.camera.borrow().rotor;
        self.cam0 = self.camera.borrow().clone();
        self.surface_point0 = self.surface_point.clone();
        self.mouse_horizontal = 0.;
        self.mouse_vertical = 0.;
        if let Some(origin) = self.pivot_point {
            let origin = Vec3::from(origin);
            self.zoom_plane = Some(Plane {
                origin,
                normal: (self.camera.borrow().position - origin),
            });
        }
        self.free_yz_angle = 0.;
        self.free_xz_angle = 0.;
        self.end_constrained_rotation();
    }

    pub fn teleport_camera(&mut self, position: Vec3, rotation: Rotor3) {
        let mut camera = self.camera.borrow_mut();
        camera.position = position;
        camera.rotor = rotation;
        self.last_rotor = rotation;
        self.cam0 = camera.clone();
    }

    pub fn set_surface_point_if_unset(&mut self, info: SurfaceInfo) {
        if self.surface_point.is_none() {
            self.set_surface_point(info)
        }
    }

    pub fn set_surface_point(&mut self, info: SurfaceInfo) {
        let cam_pos =
            info.position + DEFAULT_DIST_TO_SURFACE * Vec3::unit_z().rotated_by(info.local_frame);
        self.dist_to_surface = self.dist_to_surface.or(Some(DEFAULT_DIST_TO_SURFACE));
        self.teleport_camera(cam_pos, info.local_frame.reversed());
        self.surface_point0 = Some(info.point.clone());
        self.surface_point = Some(info.point);
    }

    pub(super) fn reverse_surface_direction(
        &mut self,
        surface_info_provider: &dyn SurfaceInfoProvider,
    ) {
        if let Some(point) = self.surface_point.as_mut() {
            point.reversed_direction ^= true;
            if let Some(surface_info) = surface_info_provider.get_surface_info(point.clone()) {
                self.set_surface_point(surface_info);
            }
        }
    }

    pub fn horizon_angle(&self) -> f32 {
        let pv_matrix = self.projection.borrow().calc_matrix() * self.camera.borrow().calc_matrix();
        let far_dist = 1000.;
        let mut percieved_x_far = pv_matrix
            .transform_point3(far_dist * Vec3::unit_z() + far_dist * Vec3::unit_x())
            - pv_matrix.transform_point3(far_dist * Vec3::unit_z());
        percieved_x_far.x *= self.projection.borrow().get_ratio();
        let mut percieved_z_far = pv_matrix
            .transform_point3(far_dist * Vec3::unit_z() + far_dist * Vec3::unit_x())
            - pv_matrix.transform_point3(far_dist * Vec3::unit_x());
        percieved_z_far.x *= self.projection.borrow().get_ratio();
        let mut angle = if ultraviolet::Vec2::new(percieved_x_far.x, percieved_x_far.y).mag()
            > ultraviolet::Vec2::new(percieved_z_far.x, percieved_z_far.y).mag()
        {
            -percieved_x_far.y.atan2(percieved_x_far.x)
        } else {
            -percieved_z_far.y.atan2(percieved_z_far.x)
        };
        if angle > std::f32::consts::FRAC_PI_2 {
            angle -= std::f32::consts::PI;
        } else if angle < -std::f32::consts::FRAC_PI_2 {
            angle += std::f32::consts::PI;
        };
        angle
    }

    pub fn set_camera_position(&mut self, position: Vec3) {
        let mut camera = self.camera.borrow_mut();
        camera.position = position;
        self.cam0 = camera.clone();
    }

    pub fn resize(&mut self, size: PhySize) {
        self.projection.borrow_mut().resize(size.width, size.height)
    }

    /// Swing the camera arrond `self.pivot_point`. Assumes that the pivot_point is where the
    /// camera points at.
    pub fn swing(&mut self, x: f64, y: f64) {
        let new_angle_yz = -((y + 1.).rem_euclid(2.) - 1.) as f32 * PI;
        let new_angle_xz = ((x + 1.).rem_euclid(2.) - 1.) as f32 * PI;
        let delta_angle_yz = new_angle_yz - self.free_yz_angle;
        let delta_angle_xz = new_angle_xz - self.free_xz_angle;
        if let Some(pivot) = self.pivot_point {
            self.rotate_camera_around(delta_angle_xz, delta_angle_yz, pivot);
        } else {
            self.small_rotate_camera(new_angle_xz, new_angle_yz, None);
        }
        self.free_xz_angle = new_angle_xz;
        self.free_yz_angle = new_angle_yz;
    }

    /// Rotate the camera arround a point.
    /// `point` is given in the world's coordiantes.
    pub fn rotate_camera_around(
        &mut self,
        delta_xz_angle: f32,
        delta_yz_angle: f32,
        point: FiniteVec3,
    ) {
        let point: Vec3 = point.into();
        // We first modify the camera orientation and then position it at the correct position
        let to_point = point - self.camera.borrow().position;
        let up = to_point.dot(self.camera.borrow().up_vec());
        let right = to_point.dot(self.camera.borrow().right_vec());
        let dir = to_point.dot(self.camera.borrow().direction());

        let new_rotor =
            if let Some(constrained_rotation) = self.current_constrained_rotation.as_mut() {
                constrained_rotation.add_angle_xz(delta_xz_angle);
                constrained_rotation.add_angle_yz(delta_yz_angle);
                constrained_rotation.compute_rotor()
            } else {
                Rotor3::from_rotation_xz(delta_xz_angle)
                    * Rotor3::from_rotation_yz(delta_yz_angle)
                    * self.camera.borrow().rotor
            };

        self.camera.borrow_mut().rotor = new_rotor;
        let new_direction = self.camera.borrow().direction();
        let new_up = self.camera.borrow().up_vec();
        let new_right = self.camera.borrow().right_vec();
        self.camera.borrow_mut().position =
            point - dir * new_direction - up * new_up - right * new_right
    }

    /// Modify the camera's rotor so that the camera looks at `point`.
    /// `point` is given in the world's coordinates
    pub fn look_at_point(&mut self, point: Vec3, up: Vec3) {
        let new_direction = (point - self.camera.borrow().position).normalized();
        let right = new_direction.cross(up);
        let matrix = Mat3::new(right, up, -new_direction);
        let rotor = matrix.into_rotor3();
        self.camera.borrow_mut().rotor = rotor;
    }

    /// Modify the camera's rotor so that the camera looks at `self.position + point`.
    /// `point` is given in the world's coordinates
    pub fn look_at_orientation(&mut self, point: Vec3, up: Vec3, pivot: Option<Vec3>) {
        let dist = pivot.map(|p| (self.camera.borrow().position - p).mag());
        let point = self.camera.borrow().position + point;
        self.look_at_point(point, up);
        if let Some(dist) = dist {
            let new_pos = pivot.unwrap() - dist * self.camera.borrow().direction();
            self.camera.borrow_mut().position = new_pos;
            self.cam0.position = new_pos;
        }
        self.cam0.rotor = self.camera.borrow().rotor;
    }

    fn small_rotate_camera(&mut self, angle_xz: f32, angle_yz: f32, pivot: Option<Vec3>) {
        let dist = pivot.map(|p| (self.camera.borrow().position - p).mag());
        let rotation = Rotor3::from_rotation_yz(angle_yz) * Rotor3::from_rotation_xz(angle_xz);

        // and we apply this rotation to the camera
        let new_rotor = rotation * self.cam0.rotor;
        self.camera.borrow_mut().rotor = new_rotor;
        if let Some(dist) = dist {
            let new_pos = pivot.unwrap() - dist * self.camera.borrow().direction();
            self.camera.borrow_mut().position = new_pos;
            self.cam0.position = new_pos;
        }
    }

    pub fn rotate_camera(&mut self, angle_xz: f32, angle_yz: f32, pivot: Option<Vec3>) {
        let dist = pivot.map(|p| (self.camera.borrow().position - p).mag());
        let rotation = Rotor3::from_rotation_yz(angle_yz) * Rotor3::from_rotation_xz(angle_xz);

        // and we apply this rotation to the camera
        let new_rotor = rotation * self.cam0.rotor;
        self.camera.borrow_mut().rotor = new_rotor;
        self.cam0.rotor = new_rotor;
        if let Some(dist) = dist {
            let new_pos = pivot.unwrap() - dist * self.camera.borrow().direction();
            self.camera.borrow_mut().position = new_pos;
            self.cam0.position = new_pos;
        }
    }

    pub fn tilt_camera(&mut self, angle_xy: f32) {
        let rotation = Rotor3::from_rotation_xy(angle_xy);

        let new_rotor = rotation * self.cam0.rotor;
        self.camera.borrow_mut().rotor = new_rotor;
        self.cam0.rotor = new_rotor;
    }

    pub fn continuous_tilt(&mut self, angle_xy: f32) {
        let rotation = Rotor3::from_rotation_xy(angle_xy);
        let new_rotor = rotation * self.cam0.rotor;
        self.camera.borrow_mut().rotor = new_rotor;
    }

    pub fn shift(&mut self) {
        let vec = 0.01 * self.camera.borrow().right_vec() + 0.01 * self.camera.borrow().up_vec();
        self.camera.borrow_mut().position += vec;
        self.cam0.position = self.camera.borrow().position;
        self.cam0.rotor = self.camera.borrow().rotor;
    }

    pub fn center_camera(&mut self, center: Vec3) {
        let new_position = center - 5. * self.camera.borrow().direction();
        let orientation = self.camera.borrow().rotor;
        self.teleport_camera(new_position, orientation);
    }

    pub fn ray(&self, x_ndc: f32, y_ndc: f32) -> (Vec3, Vec3) {
        maths_3d::cast_ray(
            x_ndc,
            y_ndc,
            self.camera.clone(),
            self.projection.clone(),
            None, // we don't play we grids in stereographic view
        )
    }

    pub fn get_current_surface_pivot(&self) -> Option<Vec3> {
        if self.surface_point.is_some() {
            let dist = self.dist_to_surface.unwrap_or(DEFAULT_DIST_TO_SURFACE);
            Some(self.camera.borrow().direction() * dist + self.camera.borrow().position)
        } else {
            None
        }
    }
}

/// A plane in space defined by an origin and a normal
#[derive(Debug)]
struct Plane {
    origin: Vec3,
    normal: Vec3,
}
