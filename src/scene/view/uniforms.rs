use super::camera::{CameraPtr, ProjectionPtr};
use ultraviolet::{Mat4, Vec4};

#[repr(C)] // We need this for Rust to store our data correctly for the shaders
#[derive(Debug, Copy, Clone)] // This is so we can store this in a buffer
/// Hold informations relative to camera: The camera position and the Projection,
/// and View matrices.
pub struct Uniforms {
    pub camera_position: Vec4,
    pub view: Mat4,
    pub proj: Mat4,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn from_view_proj(camera: CameraPtr, projection: ProjectionPtr) -> Self {
        Self {
            camera_position: camera.borrow().position.into_homogeneous_point(),
            view: camera.borrow().calc_matrix(),
            proj: projection.borrow().calc_matrix(),
        }
    }
}
