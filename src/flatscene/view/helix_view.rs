use super::{FlatNucl, FreeEnd, Helix, Strand};
use iced_wgpu::wgpu;
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
        render_pass.set_index_buffer(self.index_buffer.get_slice(), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.get_slice());
        render_pass.draw_indexed(0..self.num_instance, 0, 0..1);
    }
}

pub struct StrandView {
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    num_instance: u32,
    previous_points: Option<Vec<FlatNucl>>,
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
            previous_points: None,
        }
    }

    pub fn update(&mut self, strand: &Strand, helices: &[Helix], free_end: &Option<FreeEnd>) {
        let need_update = if self.previous_points.as_ref() != Some(&strand.points) {
            true
        } else if let Some(free_end) = free_end {
            free_end.strand_id == strand.id
        } else {
            false
        };

        if need_update {
            let vertices = strand.to_vertices(helices, free_end);
            self.vertex_buffer.update(vertices.vertices.as_slice());
            self.index_buffer.update(vertices.indices.as_slice());
            self.num_instance = vertices.indices.len() as u32;
        }
    }

    pub fn set_indication(&mut self, nucl1: FlatNucl, nucl2: FlatNucl, helices: &[Helix]) {
        let vertices = Strand::indication(nucl1, nucl2, helices);
        self.vertex_buffer.update(vertices.vertices.as_slice());
        self.index_buffer.update(vertices.indices.as_slice());
        self.num_instance = vertices.indices.len() as u32;
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_index_buffer(self.index_buffer.get_slice(), wgpu::IndexFormat::Uint16);
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
