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
use ensnano_design::ultraviolet::{Mat4, Rotor3, Vec3, Vec4};
pub use ensnano_interactor::graphics::FogParameters;

#[repr(C)] // We need this for Rust to store our data correctly for the shaders
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)] // This is so we can store this in a buffer
/// Hold informations relative to camera: The camera position and the Projection,
/// and View matrices.
pub struct Uniforms {
    //  name: type, // alignement of the next field
    pub camera_position: Vec4,    //0
    pub view: Mat4,               // 0
    pub proj: Mat4,               // 0
    pub inversed_view: Mat4,      // 0
    pub fog_radius: f32,          // 1
    pub fog_length: f32,          // 2
    pub make_fog: u32,            // 3
    pub fog_from_camera: u32,     // 0
    pub fog_alt_center: Vec3,     // 3
    pub stereography_radius: f32, // 0
    pub stereography_view: Mat4,  // 0
    pub aspect_ratio: f32,        // 1
    pub stereography_zoom: f32,
    pub _padding: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Stereography {
    pub(super) radius: f32,
    pub(super) position: Option<Vec3>,
    pub(super) orientation: Option<Rotor3>,
}

impl Stereography {
    /// The view matrix of the camera
    pub fn calc_matrix(&self) -> Option<Mat4> {
        let at = self.position? + self.direction()?;
        Some(Mat4::look_at(self.position?, at, self.up_vec()?))
    }

    /// The direction of the camera, expressed in the world coordinates
    fn direction(&self) -> Option<Vec3> {
        Some(self.orientation?.reversed() * Vec3::from([0., 0., -1.]))
    }

    /// The right vector of the camera, expressed in the world coordinates
    fn right_vec(&self) -> Option<Vec3> {
        Some(self.orientation?.reversed() * Vec3::from([1., 0., 0.]))
    }

    /// The up vector of the camera, expressed in the world coordinates.
    fn up_vec(&self) -> Option<Vec3> {
        Some(self.right_vec()?.cross(self.direction()?))
    }
}

impl Uniforms {
    pub fn from_view_proj(
        camera: CameraPtr,
        projection: ProjectionPtr,
        stereography: Option<&Stereography>,
    ) -> Self {
        let stereography_view = if let Some(s) = stereography {
            s.calc_matrix()
                .unwrap_or_else(|| camera.borrow().calc_matrix())
        } else {
            Mat4::identity()
        };
        let stereography_radius = stereography.as_ref().map(|s| s.radius).unwrap_or(0.0);
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
            stereography_radius,
            stereography_view,
            aspect_ratio: projection.borrow().get_ratio(),
            stereography_zoom: projection.borrow().stereographic_zoom,
            _padding: Default::default(),
        }
    }

    pub fn from_view_proj_fog(
        camera: CameraPtr,
        projection: ProjectionPtr,
        fog: &FogParameters,
        stereography: Option<&Stereography>,
    ) -> Self {
        let stereography_view = if let Some(s) = stereography {
            s.calc_matrix()
                .unwrap_or_else(|| camera.borrow().calc_matrix())
        } else {
            Mat4::identity()
        };
        let stereography_radius = stereography.as_ref().map(|s| s.radius).unwrap_or(0.0);
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
            stereography_view,
            stereography_radius,
            aspect_ratio: projection.borrow().get_ratio(),
            stereography_zoom: projection.borrow().stereographic_zoom,
            _padding: Default::default(),
        }
    }
}
