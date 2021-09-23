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
use std::rc::Rc;

use super::{Device, DrawArea, DrawType, Queue, ViewPtr};
use crate::utils;
use ensnano_interactor::{phantom_helix_decoder, PhantomElement};
use futures::executor;
use iced_wgpu::wgpu;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize};
use std::convert::TryInto;
use utils::BufferDimensions;

pub struct ElementSelector {
    device: Rc<Device>,
    queue: Rc<Queue>,
    readers: Vec<SceneReader>,
    window_size: PhysicalSize<u32>,
    view: ViewPtr,
    area: DrawArea,
}

impl ElementSelector {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhysicalSize<u32>,
        view: ViewPtr,
        area: DrawArea,
    ) -> Self {
        let readers = vec![
            SceneReader::new(DrawType::Widget),
            SceneReader::new(DrawType::Grid),
            SceneReader::new(DrawType::Design),
            SceneReader::new(DrawType::Phantom),
        ];
        Self {
            device,
            queue,
            window_size,
            readers,
            view,
            area,
        }
    }

    pub fn resize(&mut self, window_size: PhysicalSize<u32>, area: DrawArea) {
        self.area = area;
        self.window_size = window_size;
    }

    pub fn set_selected_id(
        &mut self,
        clicked_pixel: PhysicalPosition<f64>,
    ) -> Option<SceneElement> {
        if self.readers[0].pixels.is_none() || self.view.borrow().need_redraw_fake() {
            for i in 0..self.readers.len() {
                let pixels = self.update_fake_pixels(self.readers[i].draw_type);
                self.readers[i].pixels = Some(pixels)
            }
        }

        self.get_highest_priority_element(clicked_pixel)
    }

    fn get_highest_priority_element(
        &self,
        clicked_pixel: PhysicalPosition<f64>,
    ) -> Option<SceneElement> {
        let pixel = (
            clicked_pixel.cast::<u32>().x.min(self.area.size.width - 1) + self.area.position.x,
            clicked_pixel.cast::<u32>().y.min(self.area.size.height - 1) + self.area.position.y,
        );
        for max_delta in 0..=5 {
            let min_x = pixel.0.max(max_delta) - max_delta;
            let max_x = (pixel.0 + max_delta).min(self.window_size.width - 1);
            let min_y = pixel.1.max(max_delta) - max_delta;
            let max_y = (pixel.1 + max_delta).min(self.window_size.height - 1);
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    let byte0 =
                        (y * self.window_size.width + x) as usize * std::mem::size_of::<u32>();
                    for reader in self.readers.iter() {
                        if let Some(element) = reader.read_pixel(byte0) {
                            return Some(element);
                        }
                    }
                }
            }
        }
        None
    }

    fn update_fake_pixels(&self, draw_type: DrawType) -> Vec<u8> {
        log::debug!("update fake pixels");
        let size = wgpu::Extent3d {
            width: self.window_size.width,
            height: self.window_size.height,
            depth_or_array_layers: 1,
        };

        let (texture, texture_view) = self.create_fake_scene_texture(self.device.as_ref(), size);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.view
            .borrow_mut()
            .draw(&mut encoder, &texture_view, draw_type, self.area);

        // create a buffer and fill it with the texture
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let buffer_dimensions =
            BufferDimensions::new(extent.width as usize, extent.height as usize);
        let buf_size = buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height;
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            size: buf_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("staging_buffer"),
        });
        let buffer_copy_view = wgpu::ImageCopyBuffer {
            buffer: &staging_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: (buffer_dimensions.padded_bytes_per_row as u32)
                    .try_into()
                    .ok(),
                rows_per_image: None,
            },
        };
        let origin = wgpu::Origin3d { x: 0, y: 0, z: 0 };
        let texture_copy_view = wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin,
            aspect: Default::default(),
        };

        encoder.copy_texture_to_buffer(texture_copy_view, buffer_copy_view, extent);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
        self.device.poll(wgpu::Maintain::Wait);

        let pixels = async {
            if let Ok(()) = buffer_future.await {
                let pixels_slice = buffer_slice.get_mapped_range();
                let mut pixels = Vec::with_capacity((size.height * size.width) as usize);
                for chunck in pixels_slice.chunks(buffer_dimensions.padded_bytes_per_row) {
                    for byte in chunck[..buffer_dimensions.unpadded_bytes_per_row].iter() {
                        pixels.push(*byte);
                    }
                }
                drop(pixels_slice);
                staging_buffer.unmap();
                pixels
            } else {
                panic!("could not read fake texture");
            }
        };
        executor::block_on(pixels)
    }

    fn create_fake_scene_texture(
        &self,
        device: &Device,
        size: wgpu::Extent3d,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let desc = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            label: Some("desc"),
        };
        let texture_view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("texture_view_descriptor"),
            format: Some(wgpu::TextureFormat::Bgra8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&texture_view_descriptor);
        (texture, view)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneElement {
    DesignElement(u32, u32),
    WidgetElement(u32),
    PhantomElement(PhantomElement),
    Grid(u32, usize),
    GridCircle(u32, usize, isize, isize),
}

impl SceneElement {
    pub fn get_design(&self) -> Option<u32> {
        match self {
            SceneElement::DesignElement(d, _) => Some(*d),
            SceneElement::WidgetElement(_) => None,
            SceneElement::PhantomElement(p) => Some(p.design_id),
            SceneElement::Grid(d, _) => Some(*d),
            SceneElement::GridCircle(d, _, _, _) => Some(*d),
        }
    }

    #[allow(dead_code)]
    pub fn is_widget(&self) -> bool {
        match self {
            SceneElement::WidgetElement(_) => true,
            _ => false,
        }
    }
}

struct SceneReader {
    pixels: Option<Vec<u8>>,
    draw_type: DrawType,
}

impl SceneReader {
    pub fn new(draw_type: DrawType) -> Self {
        Self {
            pixels: None,
            draw_type,
        }
    }

    fn read_pixel(&self, byte0: usize) -> Option<SceneElement> {
        let pixels = self.pixels.as_ref().unwrap();
        let a = *pixels.get(byte0 + 3)? as u32;
        let r = (*pixels.get(byte0 + 2)? as u32) << 16;
        let g = (*pixels.get(byte0 + 1)? as u32) << 8;
        let b = (*pixels.get(byte0)?) as u32;
        log::trace!(
            "pixel color: r {} \n  g  \n {} \n b {}  \n a {}",
            r,
            g,
            b,
            a
        );
        let color = r + g + b;
        if a == 0xFF {
            None
        } else {
            match self.draw_type {
                DrawType::Grid => Some(SceneElement::Grid(a, color as usize)),
                DrawType::Design => Some(SceneElement::DesignElement(a, color)),
                DrawType::Phantom => {
                    Some(SceneElement::PhantomElement(phantom_helix_decoder(color)))
                }
                DrawType::Widget => Some(SceneElement::WidgetElement(color)),
                DrawType::Scene => unreachable!(),
            }
        }
    }
}
