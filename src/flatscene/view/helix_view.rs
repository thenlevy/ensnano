use super::Selection;
use super::{FreeEnd, Helix, Strand};
use iced_wgpu::wgpu;
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{Buffer, Device, Queue, RenderPass};

pub struct HelixView {
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    num_instance: u32,
    background: bool,
}

impl HelixView {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, background: bool) -> Self {
        Self {
            vertex_buffer: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsage::VERTEX,
            ),
            index_buffer: DynamicBuffer::new(device, queue, wgpu::BufferUsage::INDEX),
            num_instance: 0,
            background,
        }
    }

    pub fn update(&mut self, helix: &Helix) {
        let vertices = if self.background {
            helix.background_vertices()
        } else {
            helix.to_vertices()
        };
        self.vertex_buffer.update(vertices.vertices.as_slice());
        self.index_buffer.update(vertices.indices.as_slice());
        self.num_instance = vertices.indices.len() as u32;
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_index_buffer(self.index_buffer.get_slice());
        render_pass.set_vertex_buffer(0, self.vertex_buffer.get_slice());
        render_pass.draw_indexed(0..self.num_instance, 0, 0..1);
    }
}

pub struct StrandView {
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    num_instance: u32,
}

impl StrandView {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        Self {
            vertex_buffer: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsage::VERTEX,
            ),
            index_buffer: DynamicBuffer::new(device, queue, wgpu::BufferUsage::INDEX),
            num_instance: 0,
        }
    }

    pub fn update(
        &mut self,
        strand: &Strand,
        helices: &[Helix],
        free_end: &Option<FreeEnd>,
        id_map: &HashMap<usize, usize>,
        selection: &Selection,
    ) {
        let vertices = strand.to_vertices(helices, free_end, id_map, selection);
        self.vertex_buffer.update(vertices.vertices.as_slice());
        self.index_buffer.update(vertices.indices.as_slice());
        self.num_instance = vertices.indices.len() as u32;
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
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
    usage: wgpu::BufferUsage,
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
            usage,
        }
    }

    /// Replace the data of the associated buffer.
    pub fn update<I: bytemuck::Pod>(&mut self, data: &[I]) {
        let mut bytes: Vec<u8> = bytemuck::cast_slice(data).into();
        let length = bytes.len();
        while bytes.len() % 4 != 0 {
            bytes.push(0)
        }
        if self.capacity < bytes.len() {
            self.length = length as u64;
            self.buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("capacity = {}", 2 * bytes.len())),
                size: 2 * bytes.len() as u64,
                usage: self.usage | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = 2 * bytes.len();
        } else if self.length != length as u64 {
            self.length = length as u64;
        }
        self.queue.write_buffer(&self.buffer, 0, bytes.as_slice());
    }

    pub fn get_slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.length)
    }
}
