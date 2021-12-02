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
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};
#[derive(Debug, Copy, Clone)]
/// The instantiation of an object
pub struct Instance {
    /// The position in space
    pub position: Vec3,
    /// The rotation of the instance
    pub rotor: Rotor3,
    pub color: Vec4,
    pub id: u32,
    pub scale: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    /// The model matrix of the instance
    pub model: Mat4,
    pub color: Vec4,
    pub id: Vec4,
}

impl Instance {
    pub fn color_from_u32(color: u32) -> Vec4 {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        Vec4::new(
            red as f32 / 255.,
            green as f32 / 255.,
            blue as f32 / 255.,
            1.,
        )
    }

    pub fn color_from_au32(color: u32) -> Vec4 {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        let alpha = (color & 0xFF000000) >> 24;
        Vec4::new(
            red as f32 / 255.,
            green as f32 / 255.,
            blue as f32 / 255.,
            alpha as f32 / 255.,
        )
    }

    #[allow(dead_code)]
    pub fn id_from_u32(id: u32) -> Vec4 {
        let a = (id & 0xFF000000) >> 24;
        let r = (id & 0x00FF0000) >> 16;
        let g = (id & 0x0000FF00) >> 8;
        let b = id & 0x000000FF;
        Vec4::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., a as f32)
    }
}
