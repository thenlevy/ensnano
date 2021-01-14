/// This modules contains structure that manipulate bind groups and their associated buffers.
use std::rc::Rc;

use crate::utils::create_buffer_with_data;
use iced_wgpu::wgpu;
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

impl DynamicBindGroup {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 0,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        let capacity = 0;
        let length = 0;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::StorageBuffer {
                    // We don't plan on changing the size of this buffer
                    dynamic: false,
                    // The shader is not allowed to modify it's contents
                    readonly: true,
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
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
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
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = 2 * bytes.len();
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(self.buffer.slice(..self.length)),
                }],
                label: None,
            });
        } else if self.length != bytes.len() as u64 {
            self.length = bytes.len() as u64;
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(self.buffer.slice(..self.length)),
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
    visibility: wgpu::ShaderStage::from_bits_truncate(
        wgpu::ShaderStage::VERTEX.bits() | wgpu::ShaderStage::FRAGMENT.bits(),
    ),
    ty: wgpu::BindingType::UniformBuffer {
        dynamic: false,
        min_binding_size: None,
    },
    count: None,
}];

impl UniformBindGroup {
    pub fn new<I: bytemuck::Pod>(device: Rc<Device>, queue: Rc<Queue>, viewer_data: &I) -> Self {
        let buffer = create_buffer_with_data(
            &device,
            bytemuck::cast_slice(&[*viewer_data]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
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
                    resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
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
