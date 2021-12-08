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
use super::camera::{CameraPtr, ProjectionPtr};
pub use ensnano_interactor::graphics::FogParameters;
use ultraviolet::{Mat4, Vec3, Vec4};

#[repr(C)] // We need this for Rust to store our data correctly for the shaders
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)] // This is so we can store this in a buffer
/// Hold informations relative to camera: The camera position and the Projection,
/// and View matrices.
pub struct Uniforms {
    pub camera_position: Vec4,
    pub view: Mat4,
    pub proj: Mat4,
    pub inversed_view: Mat4,
    pub fog_radius: f32,
    pub fog_length: f32,
    pub make_fog: u32,
    pub fog_from_camera: u32,
    pub fog_alt_center: Vec3,
}

impl Uniforms {
    pub fn from_view_proj(camera: CameraPtr, projection: ProjectionPtr) -> Self {
        Self {
            camera_position: camera.borrow().position.into_homogeneous_point(),
            view: camera.borrow().calc_matrix(),
            proj: projection.borrow().calc_matrix(),
            inversed_view: camera.borrow().calc_matrix().inversed(),
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
        let mut make_fog = fog.fog_kind;
        if !fog.from_camera && fog.alt_fog_center.is_none() {
            make_fog = ensnano_interactor::graphics::fog_kind::NO_FOG;
        }
        Self {
            camera_position: camera.borrow().position.into_homogeneous_point(),
            view: camera.borrow().calc_matrix(),
            proj: projection.borrow().calc_matrix(),
            inversed_view: camera.borrow().calc_matrix().inversed(),
            fog_length: fog.length,
            fog_radius: fog.radius,
            make_fog,
            fog_from_camera: fog.from_camera as u32,
            fog_alt_center: fog.alt_fog_center.unwrap_or(Vec3::zero()),
        }
    }
}
