use std::rc::Rc;
use iced_wgpu::wgpu;
use wgpu::{Device, Buffer, Queue, RenderPipeline, RenderPass};
use super::super::data::Helix;

pub struct HelixView {
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    num_instance: u32,
    id: u32,
}

impl HelixView {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, id: u32) -> Self {
        Self {
            vertex_buffer: DynamicBuffer::new(device.clone(), queue.clone(), wgpu::BufferUsage::VERTEX),
            index_buffer: DynamicBuffer::new(device, queue, wgpu::BufferUsage::INDEX),
            num_instance: 0,
            id,
        }
    }

    pub fn update(&mut self, helix: Helix) {
        let vertices = helix.to_vertices(self.id);
        self.vertex_buffer.update(vertices.vertices.as_slice());
        self.index_buffer.update(vertices.indices.as_slice());
        self.num_instance = vertices.vertices.len() as u32;
    }

    pub fn draw<'a>(&'a self, pipeline: &RenderPipeline, render_pass: &mut RenderPass<'a>) {
        render_pass.set_index_buffer(self.index_buffer.get_slice());
        render_pass.set_vertex_buffer(0, self.vertex_buffer.get_slice());
        render_pass.draw_indexed(0..self.num_instance, 0, 0..1);
    }
}

struct DynamicBuffer {
    buffer: Buffer,
    capacity: usize,
    length: u64,
    device: Rc<Device>,
    queue: Rc<Queue>,
}

impl DynamicBuffer {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, usage: wgpu::BufferUsage) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 0,
            usage: usage | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        let capacity = 0;
        let length = 0;

        Self {
            device,
            queue,
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
        } else if self.length != bytes.len() as u64 {
            self.length = bytes.len() as u64;
        }
        self.queue.write_buffer(&self.buffer, 0, bytes);
    }

    pub fn get_slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.length)
    }

}
