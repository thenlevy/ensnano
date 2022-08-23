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
/// This modules contains structure that manipulate bind groups and their associated buffers.
use std::rc::Rc;

use crate::create_buffer_with_data;
use crate::wgpu;
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferDescriptor, Device, Queue};

/// A bind group with an associated buffer whose size may varry
pub struct DynamicBindGroup {
    layout: BindGroupLayout,
    buffer: Buffer,
    capacity: usize,
    length: u64,
    bind_group: BindGroup,
    device: Rc<Device>,
    queue: Rc<Queue>,
}

const INITIAL_CAPACITY: u64 = 1024;

impl DynamicBindGroup {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, label: &str) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: INITIAL_CAPACITY,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let capacity = INITIAL_CAPACITY as usize;
        let length = 0;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    // We don't plan on changing the size of this buffer
                    has_dynamic_offset: false,
                    // The shader is not allowed to modify it's contents
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    size: None,
                    offset: 0,
                }),
            }],
            label: Some("instance_bind_group"),
        });

        Self {
            device,
            queue,
            bind_group,
            layout,
            buffer,
            capacity,
            length,
        }
    }

    /// Replace the data of the associated buffer.
    pub fn update<I: bytemuck::Pod>(&mut self, data: &[I]) {
        let bytes = bytemuck::cast_slice(data);
        if self.capacity < bytes.len() {
            self.length = bytes.len() as u64;
            self.buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("capacity = {}", 2 * bytes.len())),
                size: 2 * bytes.len() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = 2 * bytes.len();
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.buffer,
                        size: wgpu::BufferSize::new(self.length),
                        offset: 0,
                    }),
                }],
                label: None,
            });
        } else if self.length != bytes.len() as u64 {
            self.length = bytes.len() as u64;
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.buffer,
                        size: wgpu::BufferSize::new(self.length),
                        offset: 0,
                    }),
                }],
                label: None,
            });
        }
        self.queue.write_buffer(&self.buffer, 0, bytes);
    }

    #[allow(dead_code)]
    /// Write in the self.buffer with an offset
    pub fn update_offset(&mut self, offset: usize, bytes: &[u8]) {
        debug_assert!(self.length as usize >= offset + bytes.len());
        self.queue.write_buffer(&self.buffer, offset as u64, bytes);
    }

    pub fn get_bindgroup(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn get_layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}

/// A structure that manages a bind group associated to a uniform buffer
pub struct UniformBindGroup {
    layout: BindGroupLayout,
    buffer: Buffer,
    bind_group: BindGroup,
    queue: Rc<Queue>,
}

static UNIFORM_BG_ENTRY: &'static [wgpu::BindGroupLayoutEntry] = &[wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStages::from_bits_truncate(
        wgpu::ShaderStages::VERTEX.bits() | wgpu::ShaderStages::FRAGMENT.bits(),
    ),
    ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
}];

impl UniformBindGroup {
    pub fn new<I: bytemuck::Pod>(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_data: &I,
        label: &str,
    ) -> Self {
        let buffer = create_buffer_with_data(
            &device,
            bytemuck::cast_slice(&[*viewer_data]),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            label,
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: UNIFORM_BG_ENTRY,
            label: Some("uniform_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                // perspective and view
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        size: None,
                        offset: 0,
                    }),
                },
            ],
            label: Some("uniform_bind_group"),
        });

        Self {
            queue,
            bind_group,
            layout,
            buffer,
        }
    }

    pub fn update<I: bytemuck::Pod>(&mut self, new_data: &I) {
        self.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[*new_data]));
    }

    pub fn get_bindgroup(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn get_layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    pub fn get_layout_desc(&self) -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            entries: UNIFORM_BG_ENTRY,
            label: Some("uniform_bind_group"),
        }
    }
}
