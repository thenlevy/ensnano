use std::rc::Rc;

use futures::executor;
use iced_winit::winit::dpi::{PhysicalSize, PhysicalPosition};
use iced_wgpu::wgpu;
use super::{DrawArea, DrawType, Device, ViewPtr, Queue, DataPtr};
use crate::utils;
use utils::BufferDimensions;

pub struct ElementSelector {
    device: Rc<Device>,
    queue: Rc<Queue>,
    readers: Vec<SceneReader>,
    window_size: PhysicalSize<u32>,
    view: ViewPtr,
    data: DataPtr,
    area: DrawArea,

}

impl ElementSelector {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhysicalSize<u32>, view: ViewPtr, data: DataPtr, area: DrawArea) -> Self {
        let readers = vec![
            SceneReader::new(DrawType::Widget),
            SceneReader::new(DrawType::Phantom),
            SceneReader::new(DrawType::Design),
        ];
        Self {
            device,
            queue,
            window_size,
            readers,
            view,
            data,
            area,
        }
    }

    pub fn resize(&mut self, window_size: PhysicalSize<u32>, area: DrawArea) {
        self.area = area;
        self.window_size = window_size;
    }

    pub fn set_selected_id(&mut self, clicked_pixel: PhysicalPosition<f64>) -> Option<SceneElement> {
        let pixel = (
            clicked_pixel.cast::<u32>().x.min(self.area.size.width - 1) + self.area.position.x,
            clicked_pixel.cast::<u32>().y.min(self.area.size.height - 1) + self.area.position.y,
        );

        if self.readers[0].pixels.is_none() || self.view.borrow().need_redraw_fake() {
            for i in 0..self.readers.len() {
                let pixels = self.update_fake_pixels(self.readers[i].draw_type);
                self.readers[i].pixels = Some(pixels)
            }
        }

        let byte0 = (pixel.1 * self.window_size.width + pixel.0) as usize
            * std::mem::size_of::<u32>();

        self.get_highest_priority_element(byte0)
    }

    fn get_highest_priority_element(&self, byte0: usize) -> Option<SceneElement> {
        for reader in self.readers.iter() {
            if let Some(element) = reader.read_pixel(byte0) {
                return Some(element)
            }
        } 
        return None
    }

    fn get_pixel_from_bytes(byte0: usize, pixels: &[u8]) -> Option<(u32, u32)> {
        let a = pixels[byte0 + 3] as u32;
        let r = (pixels[byte0 + 2] as u32) << 16;
        let g = (pixels[byte0 + 1] as u32) << 8;
        let b = pixels[byte0] as u32;
        let color = r + g + b;
        if (color, a) != (0xFF_FF_FF, 0xFF) {
            Some((color, a))
        } else {
            None
        }
    }

    fn update_fake_pixels(&self, draw_type: DrawType) -> Vec<u8> {
        let size = wgpu::Extent3d {
            width: self.window_size.width,
            height: self.window_size.height,
            depth: 1,
        };

        let (texture, texture_view) = self.create_fake_scene_texture(self.device.as_ref(), size);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.view
            .borrow_mut()
            .draw(&mut encoder, &texture_view, draw_type, self.area, self.data.borrow().get_action_mode());

        // create a buffer and fill it with the texture
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth: 1,
        };
        let buffer_dimensions =
            BufferDimensions::new(extent.width as usize, extent.height as usize);
        let buf_size = buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height;
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            size: buf_size as u64,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
            label: Some("staging_buffer"),
        });
        let buffer_copy_view = wgpu::BufferCopyView {
            buffer: &staging_buffer,
            layout: wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: buffer_dimensions.padded_bytes_per_row as u32,
                rows_per_image: 0,
            },
        };
        let origin = wgpu::Origin3d { x: 0, y: 0, z: 0 };
        let texture_copy_view = wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            origin,
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
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_SRC,
            label: Some("desc"),
        };
        let texture_view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("texture_view_descriptor"),
            format: Some(wgpu::TextureFormat::Bgra8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
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
    PhantomElement(utils::PhantomElement)
}

struct SceneReader {
    pixels: Option<Vec<u8>>,
    draw_type: DrawType
}

impl SceneReader {
    pub fn new(draw_type: DrawType) -> Self {
        Self {
            pixels: None,
            draw_type
        }
    }

    fn read_pixel(&self, byte0: usize) -> Option<SceneElement> {
        let pixels = self.pixels.as_ref().unwrap();
        let a = pixels[byte0 + 3] as u32;
        let r = (pixels[byte0 + 2] as u32) << 16;
        let g = (pixels[byte0 + 1] as u32) << 8;
        let b = pixels[byte0] as u32;
        let color = r + g + b;
        if a == 0xFF {
            None
        } else {
            match self.draw_type {
                DrawType::Design => Some(SceneElement::DesignElement(a, color)),
                DrawType::Phantom => Some(SceneElement::PhantomElement(utils::phantom_helix_decoder(color))),
                DrawType::Widget => Some(SceneElement::WidgetElement(color)),
                DrawType::Scene => unreachable!(),
            }
        }
    }
}

