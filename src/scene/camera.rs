use super::PhySize;
use iced_winit::winit;
use std::cell::RefCell;
use std::f32::consts::{FRAC_PI_2, PI};
use std::rc::Rc;
use std::time::Duration;
use ultraviolet::{Mat4, Rotor3, Vec3};
use winit::dpi::LogicalPosition;
use winit::event::*;

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
}

#[derive(Debug)]
/// This structure holds the information needed to compute the projection matrix.
pub struct Projection {
    aspect: f32,
    /// Field of view in *radiants*
    fovy: f32,
    znear: f32,
    zfar: f32,
}

pub type ProjectionPtr = Rc<RefCell<Projection>>;

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy,
            znear,
            zfar,
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
}

pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    amount_up: f32,
    amount_down: f32,
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    last_rotor: Rotor3,
    processed_move: bool,
    camera: CameraPtr,
    cam0: Camera,
    projection: ProjectionPtr,
    pivot_point: Option<Vec3>,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, camera: CameraPtr, projection: ProjectionPtr) -> Self {
        Self {
            speed,
            sensitivity,
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            last_rotor: camera.clone().borrow().rotor,
            processed_move: false,
            camera: camera.clone(),
            cam0: camera.borrow().clone(),
            projection,
            pivot_point: None,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::E => {
                self.amount_down = amount;
                true
            }
            VirtualKeyCode::H if amount > 0. => {
                self.rotate_camera_around(
                    FRAC_PI_2 / 20.,
                    0.,
                    self.pivot_point.unwrap_or(Vec3::zero()),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            VirtualKeyCode::L if amount > 0. => {
                self.rotate_camera_around(
                    -FRAC_PI_2 / 20.,
                    0.,
                    self.pivot_point.unwrap_or(Vec3::zero()),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            VirtualKeyCode::J if amount > 0. => {
                self.rotate_camera_around(
                    0.,
                    FRAC_PI_2 / 20.,
                    self.pivot_point.unwrap_or(Vec3::zero()),
                );
                self.cam0 = self.camera.borrow().clone();
                true
            }
            VirtualKeyCode::K if amount > 0. => {
                self.rotate_camera_around(
                    0.,
                    -FRAC_PI_2 / 20.,
                    self.pivot_point.unwrap_or(Vec3::zero()),
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
            || self.amount_forward > 0.
            || self.amount_backward > 0.
    }

    pub fn set_pivot_point(&mut self, point: Vec3) {
        self.pivot_point = Some(point)
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = -mouse_dy as f32;
        self.processed_move = true;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(LogicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    /// Rotate the head of the camera on its yz plane and xz plane according to the values of
    /// self.rotate_horizontal and self.rotate_vertical.
    fn rotate_camera(&mut self) {
        let xz_angle = self.rotate_horizontal * FRAC_PI_2;
        let yz_angle = self.rotate_vertical * FRAC_PI_2;

        // We want to build a rotation that will
        // first maps (1, 0, 0) to (cos(yz_angle), -sin(yz_angle), 0)
        // and then (0, 1, 0) to (0, cos(xz_angle), -sin(yz_angle))

        let rotation = Rotor3::from_rotation_xz(xz_angle) * Rotor3::from_rotation_yz(yz_angle);

        self.camera.borrow_mut().rotor = rotation * self.cam0.rotor;

        // Since we have rotated the camera we can reset those values
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
    }

    fn move_camera(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let forward = self.camera.borrow().direction();
        let right = self.camera.borrow().right_vec();

        {
            let mut camera = self.camera.borrow_mut();
            camera.position +=
                forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
            camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;
        }

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let scrollward = self.camera.borrow().direction();
        {
            let mut camera = self.camera.borrow_mut();
            camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        }
        self.scroll = 0.;

        // Move up/down
        let up_vec = self.camera.borrow().up_vec();
        {
            let mut camera = self.camera.borrow_mut();
            camera.position += up_vec * (self.amount_up - self.amount_down) * self.speed * dt;
        }
    }
    pub fn update_camera(&mut self, dt: Duration) {
        if self.processed_move {
            self.rotate_camera();
        }
        self.move_camera(dt);
    }

    pub fn process_click(&mut self, state: &ElementState) {
        let camera = self.camera.borrow();
        match *state {
            ElementState::Released => {
                self.last_rotor = camera.rotor;
                self.cam0 = self.camera.borrow().clone();
                self.rotate_vertical = 0.;
                self.rotate_horizontal = 0.;
            }
            ElementState::Pressed => self.processed_move = false,
        }
    }

    pub fn teleport_camera(&mut self, position: Vec3, rotation: Rotor3) {
        let mut camera = self.camera.borrow_mut();
        camera.position = position;
        camera.rotor = rotation;
        self.last_rotor = rotation;
        self.cam0 = camera.clone();
    }

    pub fn resize(&mut self, size: PhySize) {
        self.projection.borrow_mut().resize(size.width, size.height)
    }

    pub fn foccus(&mut self) {
        if let Some(point) = self.pivot_point {
            self.look_at_point(point)
        }
    }

    /// Swing the camera arrond `self.pivot_point`. Assumes that the pivot_point is where the
    /// camera points at.
    pub fn swing(&mut self, x: f64, y: f64) {
        let angle_yz = -(y.min(1.).max(-1.)) as f32 * PI;
        let angle_xz = x.min(1.).max(-1.) as f32 * PI;
        self.rotate_camera_around(angle_xz, angle_yz, self.pivot_point.unwrap());
    }

    /// Rotate the camera arround a point.
    /// `point` is given in the world's coordiantes.
    pub fn rotate_camera_around(&mut self, xz_angle: f32, yz_angle: f32, point: Vec3) {
        // We first modify the camera orientation and then position it at the correct position
        let distance = (self.camera.borrow().position - point).mag();
        let rotation = Rotor3::from_rotation_xz(xz_angle) * Rotor3::from_rotation_yz(yz_angle);
        self.camera.borrow_mut().rotor = rotation * self.cam0.rotor;
        let new_direction = self.camera.borrow().direction();
        self.camera.borrow_mut().position = point - distance * new_direction;
    }

    /// Modify the camera's rotor so that the camera looks at `point`.
    /// `point` is given in the world's coordinates
    pub fn look_at_point(&mut self, point: Vec3) {
        // We express the rotation of the camera in the camera's coordinates
        // The current camera's direction is the opposite of it's z axis
        //
        // The future camera's direction is the vector from it to the point, to express it in the
        // camera's coordinates, the camera's rotor is applied to it.
        let new_direction = (point - self.camera.borrow().position)
            .normalized()
            .rotated_by(self.camera.borrow().rotor);
        let angle_xz = -new_direction.dot(Vec3::unit_x()).asin();
        let angle_yz = -new_direction.dot(Vec3::unit_y()).asin();
        let rotation = Rotor3::from_rotation_yz(angle_yz) * Rotor3::from_rotation_xz(angle_xz);

        // and we apply this rotation to the camera
        let new_rotor = rotation * self.camera.borrow().rotor;
        self.camera.borrow_mut().rotor = new_rotor;
    }
}
