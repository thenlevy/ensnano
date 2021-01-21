use super::camera::{CameraPtr, ProjectionPtr};
use ultraviolet::{Mat4, Vec3, Vec4};

#[repr(C)] // We need this for Rust to store our data correctly for the shaders
#[derive(Debug, Copy, Clone)] // This is so we can store this in a buffer
/// Hold informations relative to camera: The camera position and the Projection,
/// and View matrices.
pub struct Uniforms {
    pub camera_position: Vec4,
    pub view: Mat4,
    pub proj: Mat4,
    pub fog_radius: f32,
    pub fog_length: f32,
    pub make_fog: u32,
    pub fog_from_camera: u32,
    pub fog_alt_center: Vec3,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn from_view_proj(camera: CameraPtr, projection: ProjectionPtr) -> Self {
        Self {
            camera_position: camera.borrow().position.into_homogeneous_point(),
            view: camera.borrow().calc_matrix(),
            proj: projection.borrow().calc_matrix(),
            fog_radius: 0.,
            fog_length: 0.,
            make_fog: false as u32,
            fog_from_camera: false as u32,
            fog_alt_center: Vec3::zero(),
        }
    }

    pub fn from_view_proj_fog(
        camera: CameraPtr,
        projection: ProjectionPtr,
        fog: &FogParameters,
    ) -> Self {
        let mut make_fog = fog.active;
        if !fog.from_camera {
            make_fog &= fog.alt_fog_center.is_some();
        }
        Self {
            camera_position: camera.borrow().position.into_homogeneous_point(),
            view: camera.borrow().calc_matrix(),
            proj: projection.borrow().calc_matrix(),
            fog_length: fog.length,
            fog_radius: fog.radius,
            make_fog: make_fog as u32,
            fog_from_camera: fog.from_camera as u32,
            fog_alt_center: fog.alt_fog_center.unwrap_or(Vec3::zero()),
        }
    }
}

#[derive(Debug)]
pub struct FogParameters {
    pub radius: f32,
    pub length: f32,
    pub active: bool,
    pub from_camera: bool,
    pub alt_fog_center: Option<Vec3>,
}

impl FogParameters {
    pub fn new() -> Self {
        Self {
            radius: 10.,
            length: 10.,
            active: false,
            from_camera: true,
            alt_fog_center: None,
        }
    }
}
