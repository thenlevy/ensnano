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
use ensnano_design::grid::{GridId, GridPosition};
use ensnano_design::{BezierPathId, BezierPlaneId, BezierVertexId};
use ensnano_interactor::{phantom_helix_decoder, BezierControlPoint, PhantomElement};
use ensnano_utils as utils;
use futures::executor;
use num_enum::IntoPrimitive;
use std::convert::TryInto;
use utils::wgpu;
use utils::winit::dpi::{PhysicalPosition, PhysicalSize};
use utils::BufferDimensions;

pub struct ElementSelector {
    pub device: Rc<Device>,
    pub queue: Rc<Queue>,
    readers: Vec<SceneReader>,
    window_size: PhysicalSize<u32>,
    view: ViewPtr,
    area: DrawArea,
    stereographic: bool,
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
            stereographic: false,
        }
    }

    pub fn set_stereographic(&mut self, stereographic: bool) {
        if self.stereographic != stereographic {
            self.readers[0].pixels = None;
        }
        self.stereographic = stereographic;
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
                let pixels = self.update_fake_pixels(self.readers[i].draw_type, self.stereographic);
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

    fn update_fake_pixels(&self, draw_type: DrawType, stereographic: bool) -> Vec<u8> {
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

        self.view.borrow_mut().draw(
            &mut encoder,
            &texture_view,
            draw_type,
            self.area,
            stereographic,
            // The draw options are irrelevant for the fake scene
            Default::default(),
        );

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
    Grid(u32, GridId),
    GridCircle(u32, GridPosition),
    BezierControl {
        helix_id: usize,
        bezier_control: BezierControlPoint,
    },
    BezierVertex {
        path_id: BezierPathId,
        vertex_id: usize,
    },
    BezierTengent {
        path_id: BezierPathId,
        vertex_id: usize,
        tengent_in: bool,
    },
    PlaneCorner {
        plane_id: BezierPlaneId,
        corner_type: CornerType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerType {
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

impl CornerType {
    fn from_u32(x: u32) -> Self {
        match x {
            0 => Self::NorthWest,
            1 => Self::NorthEast,
            2 => Self::SouthWest,
            _ => Self::SouthEast,
        }
    }

    pub fn to_usize(self) -> usize {
        match self {
            Self::NorthWest => 0,
            Self::NorthEast => 1,
            Self::SouthWest => 2,
            Self::SouthEast => 3,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::NorthEast => Self::SouthWest,
            Self::NorthWest => Self::SouthEast,
            Self::SouthEast => Self::NorthWest,
            Self::SouthWest => Self::NorthEast,
        }
    }
}

impl SceneElement {
    pub fn get_design(&self) -> Option<u32> {
        match self {
            SceneElement::DesignElement(d, _) => Some(*d),
            SceneElement::WidgetElement(_) => None,
            SceneElement::PhantomElement(p) => Some(p.design_id),
            SceneElement::Grid(d, _) => Some(*d),
            SceneElement::GridCircle(d, _) => Some(*d),
            SceneElement::BezierControl { .. } => None,
            SceneElement::BezierVertex { .. } => Some(0),
            SceneElement::PlaneCorner { .. } => Some(0),
            SceneElement::BezierTengent { .. } => Some(0),
        }
    }

    #[allow(dead_code)]
    pub fn is_widget(&self) -> bool {
        match self {
            SceneElement::WidgetElement(_) => true,
            _ => false,
        }
    }

    pub fn transform_into_bezier(self) -> Self {
        if let Self::WidgetElement(id) = self {
            if let Some((helix_id, bezier_control)) =
                ensnano_interactor::consts::widget_id_to_bezier(id)
            {
                Self::BezierControl {
                    bezier_control,
                    helix_id,
                }
            } else {
                self
            }
        } else {
            self
        }
    }

    pub fn converted_to_grid_if_disc_on_bezier_grid(self) -> Self {
        if let Self::GridCircle(
            d_id,
            GridPosition {
                grid: g_id @ GridId::BezierPathGrid(_),
                ..
            },
        ) = self
        {
            Self::Grid(d_id, g_id)
        } else {
            self
        }
    }
}

struct SceneReader {
    pixels: Option<Vec<u8>>,
    draw_type: DrawType,
}

#[derive(IntoPrimitive)]
#[repr(u32)]
enum ObjType {
    None = 0xFF,
    BezierVertex = 0xFE,
    BezierPlaneCorner = 0xFD,
    BezierTengentIn = 0xFC,
    BezierTengentOut = 0xFB,
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
        if a == u32::from(ObjType::None) {
            None
        } else {
            match self.draw_type {
                DrawType::Grid => {
                    if a == u32::from(ObjType::BezierVertex) {
                        let vertex = BezierVertexId {
                            path_id: BezierPathId(r >> 16),
                            vertex_id: (g + b) as usize,
                        };
                        Some(SceneElement::Grid(0, GridId::BezierPathGrid(vertex)))
                    } else {
                        Some(SceneElement::Grid(0, GridId::FreeGrid(color as usize)))
                    }
                }
                DrawType::Design => {
                    if a == u32::from(ObjType::BezierVertex) {
                        Some(SceneElement::BezierVertex {
                            path_id: BezierPathId(r >> 16),
                            vertex_id: (g + b) as usize,
                        })
                    } else if a == u32::from(ObjType::BezierPlaneCorner) {
                        Some(SceneElement::PlaneCorner {
                            plane_id: BezierPlaneId(g + b),
                            corner_type: CornerType::from_u32(r >> 16),
                        })
                    } else if a == u32::from(ObjType::BezierTengentIn) {
                        Some(SceneElement::BezierTengent {
                            path_id: BezierPathId(r >> 16),
                            vertex_id: (g + b) as usize,
                            tengent_in: true,
                        })
                    } else if a == u32::from(ObjType::BezierTengentOut) {
                        Some(SceneElement::BezierTengent {
                            path_id: BezierPathId(r >> 16),
                            vertex_id: (g + b) as usize,
                            tengent_in: false,
                        })
                    } else {
                        Some(SceneElement::DesignElement(a, color))
                    }
                }
                DrawType::Phantom => {
                    Some(SceneElement::PhantomElement(phantom_helix_decoder(color)))
                }
                DrawType::Widget => {
                    Some(SceneElement::WidgetElement(color).transform_into_bezier())
                }
                DrawType::Scene => unreachable!(),
                DrawType::Png { .. } => unreachable!(),
            }
        }
    }
}

pub fn bezier_vertex_id(path_id: BezierPathId, vertex_id: usize) -> u32 {
    (u32::from(ObjType::BezierVertex) << 24) | ((path_id.0) << 16) | (vertex_id as u32)
}

pub fn bezier_tengent_id(path_id: BezierPathId, vertex_id: usize, tengent_in: bool) -> u32 {
    let front = if tengent_in {
        u32::from(ObjType::BezierTengentIn)
    } else {
        u32::from(ObjType::BezierTengentOut)
    };
    (front << 24) | ((path_id.0) << 16) | (vertex_id as u32)
}
