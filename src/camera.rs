use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use iced_winit::winit;
use winit::event::*;
use winit::dpi::LogicalPosition;
use cgmath::{ Matrix4, Point3, Vector3, Rad, Quaternion };
use cgmath::prelude::*;
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
    /// The eye of the camera
    pub position: Point3<f32>,
    /// The orientation of the camera. 
    ///
    /// `quaternion` represents the camera's basis and the camera is looking forward its x axis
    /// with its y axis pointing up.
    pub quaternion: Quaternion<f32>,
}

impl Camera {
    pub fn new<
        V: Into<Point3<f32>>,
    >(
        position: V,
        quaternion: Quaternion<f32>,
    ) -> Self {
        Self {
            position: position.into(),
            quaternion,
        }
    }

    /// The view matrix of the camera
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            self.position,
            self.direction(),
            self.up_vec(),
        )
    }

    /// The camera's direction is the negative of its z-axis
    pub fn direction(&self) -> Vector3<f32> {
        self.quaternion.rotate_vector(Vector3::from([0., 0., -1.]))
    }

    /// The camera's right is its x_axis
    pub fn right_vec(&self) -> Vector3<f32> {
        self.quaternion.rotate_vector(Vector3::from([1., 0., 0.]))
    }

    /// The camera's y_axis
    pub fn up_vec(&self) -> Vector3<f32> {
        self.right_vec().cross(self.direction())
    }

}

/// This structure holds the information needed to compute the projection matrix.
pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    /// Computes the projection matrix.
    ///
    /// The matrix is multiplied by `OPENGL_TO_WGPU_MATRIX` so that the product Projection * View *
    /// Model is understood by wgpu
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }

    pub fn get_fovy(&self) -> Rad<f32> {
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
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
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
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool{
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
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

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(LogicalPosition {
                y: scroll,
                ..
            }) => *scroll as f32,
        };
    }

    pub fn update_quaternion(&mut self, camera: &mut Camera, old_quaternion: Quaternion<f32>) {
        let x_angle = self.rotate_horizontal * FRAC_PI_2;
        let y_angle = -self.rotate_vertical * FRAC_PI_2;
        let rotation = Quaternion::from_axis_angle(Vector3::from([0., 1., 0.]), Rad(x_angle))
            * Quaternion::from_axis_angle(Vector3::from([0., 0., 1.]), Rad(y_angle));

        camera.quaternion = old_quaternion * rotation;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
    }
    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
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

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.

        camera.position += camera.up_vec() * (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate    
        self.scroll = 0.;
    }
}
