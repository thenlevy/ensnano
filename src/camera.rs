use ultraviolet::{Vec3, Rotor3, Mat4};
use iced_winit::winit;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::LogicalPosition;
use winit::event::*;

#[derive(Debug)]
pub struct Camera {
    /// The eye of the camera
    pub position: Vec3,
    /// The orientation of the camera.
    ///
    /// `rotor` is an object that can cat as a transformation of the world basis into the camera's
    /// basis. The camera is looking in the opposite direction of its z axis with its y axis 
    /// pointing up.
    pub rotor: Rotor3,
}

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

    /// The camera's direction is the negative of its z-axis
    pub fn direction(&self) -> Vec3 {
        self.rotor * Vec3::from([0., 0., -1.])
    }

    /// The camera's right is its x_axis
    pub fn right_vec(&self) -> Vec3 {
        self.rotor * Vec3::from([1., 0., 0.])
    }

    /// The camera's y_axis
    pub fn up_vec(&self) -> Vec3 {
        self.right_vec().cross(self.direction())
    }
}

/// This structure holds the information needed to compute the projection matrix.
pub struct Projection {
    aspect: f32,
    /// Field of view in *radiants*
    fovy: f32,
    znear: f32,
    zfar: f32,
}

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
        ultraviolet::projection::rh_yup::perspective_wgpu_dx(self.fovy, self.aspect, self.znear, self.zfar)
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
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, camera: &Camera) -> Self {
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
            last_rotor: camera.rotor,
            processed_move: false,
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
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
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

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
        self.processed_move = true;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(LogicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    fn rotate_camera(&mut self, camera: &mut Camera) {
        let x_angle = self.rotate_horizontal * FRAC_PI_2;
        let y_angle = self.rotate_vertical * FRAC_PI_2;
        let rotation = Rotor3::from_rotation_xz(x_angle) * Rotor3::from_rotation_yz(-y_angle);

        camera.rotor = self.last_rotor * rotation;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
    }

    fn move_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let forward = camera.direction();
        let right = camera.right_vec();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let scrollward = camera.direction();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.;

        // Move up/down
        camera.position += camera.up_vec() * (self.amount_up - self.amount_down) * self.speed * dt;
    }
    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        if self.processed_move {
            self.rotate_camera(camera);
        }
        self.move_camera(camera, dt);
    }

    pub fn process_click(&mut self, camera: &Camera, state: &ElementState) {
        match *state {
            ElementState::Released => {
                self.last_rotor = camera.rotor;
                self.rotate_vertical = 0.;
                self.rotate_horizontal = 0.;
            }
            ElementState::Pressed => self.processed_move = false,
        }
    }
}
