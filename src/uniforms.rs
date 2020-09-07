use crate::camera::Camera;

#[repr(C)] // We need this for Rust to store our data correctly for the shaders
#[derive(Debug, Copy, Clone)] // This is so we can store this in a buffer
/// Hold informations relative to camera: The camera position and the Projection,
/// and View matrices.
pub struct Uniforms {
    pub camera_position: cgmath::Vector4<f32>,
    /// View * Projection matrix
    pub view_proj: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            camera_position: cgmath::Zero::zero(),
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.camera_position = camera.eye.to_homogeneous();
        self.view_proj = camera.build_view_projection_matrix();
    }
}
