use crate::camera::{Camera, Projection};
use ultraviolet::{Vec4, Mat4};

#[repr(C)] // We need this for Rust to store our data correctly for the shaders
#[derive(Debug, Copy, Clone)] // This is so we can store this in a buffer
/// Hold informations relative to camera: The camera position and the Projection,
/// and View matrices.
pub struct Uniforms {
    pub camera_position: Vec4,
    /// View * Projection matrix
    pub view_proj: Mat4,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            camera_position: Vec4::zero(),
            view_proj: Mat4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.camera_position = camera.position.into_homogeneous_point();
        self.view_proj = projection.calc_matrix() * camera.calc_matrix()
    }
}
