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
use iced_wgpu::wgpu;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize, Pixel};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub mod bindgroup_manager;
pub mod camera2d;
pub mod chars2d;
pub mod circles2d;
pub mod id_generator;
pub mod instance;
pub mod light;
pub mod mesh;
pub mod texture;

pub fn create_buffer_with_data(
    device: &wgpu::Device,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let descriptor = BufferInitDescriptor {
        label: Some("descriptor"),
        contents: data,
        usage,
    };
    device.create_buffer_init(&descriptor)
}

pub struct BufferDimensions {
    pub width: usize,
    pub height: usize,
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
}

impl BufferDimensions {
    pub fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

pub fn new_color(color_idx: &mut usize) -> u32 {
    let color = {
        let hue = (*color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
        let saturation = (*color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.25 + 0.75;
        let value = (*color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.5 + 0.5;
        let hsv = color_space::Hsv::new(hue, saturation, value);
        let rgb = color_space::Rgb::from(hsv);
        (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
    };
    *color_idx += 1;
    color
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ndc {
    pub x: f32,
    pub y: f32,
}

impl Ndc {
    pub fn from_physical<S: Pixel, T: Pixel>(
        position: PhysicalPosition<S>,
        window_size: PhysicalSize<T>,
    ) -> Self {
        let position = position.cast::<f32>();
        let size = window_size.cast::<f32>();
        Self {
            x: position.x / size.width * 2. - 1.,
            y: position.y / size.height * -2. + 1.,
        }
    }
}
