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
use super::{CameraPtr, FlatNucl, FreeEnd, Helix, Strand};
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
                wgpu::BufferUsages::VERTEX,
            ),
            index_buffer: DynamicBuffer::new(device, queue, wgpu::BufferUsages::INDEX),
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
    vertex_buffer_top: DynamicBuffer,
    index_buffer_top: DynamicBuffer,
    num_instance_top: u32,
    vertex_buffer_bottom: DynamicBuffer,
    index_buffer_bottom: DynamicBuffer,
    num_instance_bottom: u32,

    split_vbo_top: DynamicBuffer,
    split_ibo_top: DynamicBuffer,
    num_instance_split_top: u32,
    split_vbo_bottom: DynamicBuffer,
    split_ibo_bottom: DynamicBuffer,
    num_instance_split_bottom: u32,
    #[allow(dead_code)]
    previous_points: Option<Vec<FlatNucl>>,
}

impl StrandView {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        Self {
            vertex_buffer_top: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::VERTEX,
            ),
            index_buffer_top: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::INDEX,
            ),
            split_vbo_top: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::VERTEX,
            ),
            split_ibo_top: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::INDEX,
            ),
            split_vbo_bottom: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::VERTEX,
            ),
            split_ibo_bottom: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::INDEX,
            ),
            vertex_buffer_bottom: DynamicBuffer::new(
                device.clone(),
                queue.clone(),
                wgpu::BufferUsages::VERTEX,
            ),
            index_buffer_bottom: DynamicBuffer::new(device, queue, wgpu::BufferUsages::INDEX),
            num_instance_top: 0,
            num_instance_bottom: 0,
            num_instance_split_top: 0,
            num_instance_split_bottom: 0,
            previous_points: None,
        }
    }

    pub fn update(
        &mut self,
        strand: &Strand,
        helices: &[Helix],
        free_end: &Option<FreeEnd>,
        top_cam: &CameraPtr,
        bottom_cam: &CameraPtr,
    ) {
        /*
        let need_update = if self.previous_points.as_ref() != Some(&strand.points) {
            true
        } else if let Some(free_end) = free_end {
            free_end.strand_id == strand.id
        } else {
            false
        };*/
        let need_update = true; //TODO improve this

        if need_update {
            let (vertices_top, split_vertices_top) =
                strand.to_vertices(helices, free_end, top_cam, bottom_cam);
            self.vertex_buffer_top
                .update(vertices_top.vertices.as_slice());
            self.index_buffer_top
                .update(vertices_top.indices.as_slice());
            self.num_instance_top = vertices_top.indices.len() as u32;
            self.split_vbo_top
                .update(split_vertices_top.vertices.as_slice());
            self.split_ibo_top
                .update(split_vertices_top.indices.as_slice());
            self.num_instance_split_top = split_vertices_top.indices.len() as u32;
            let (vertices_bottom, split_vertices_bottom) =
                strand.to_vertices(helices, free_end, bottom_cam, top_cam);
            self.vertex_buffer_bottom
                .update(vertices_bottom.vertices.as_slice());
            self.index_buffer_bottom
                .update(vertices_bottom.indices.as_slice());
            self.num_instance_bottom = vertices_bottom.indices.len() as u32;
            self.split_vbo_bottom
                .update(split_vertices_bottom.vertices.as_slice());
            self.split_ibo_bottom
                .update(split_vertices_bottom.indices.as_slice());
            self.num_instance_split_bottom = split_vertices_bottom.indices.len() as u32;
        }
    }

    pub fn set_indication(&mut self, nucl1: FlatNucl, nucl2: FlatNucl, helices: &[Helix]) {
        let vertices = Strand::indication(nucl1, nucl2, helices);
        self.vertex_buffer_top.update(vertices.vertices.as_slice());
        self.index_buffer_top.update(vertices.indices.as_slice());
        self.num_instance_top = vertices.indices.len() as u32;
        self.vertex_buffer_bottom
            .update(vertices.vertices.as_slice());
        self.index_buffer_bottom.update(vertices.indices.as_slice());
        self.num_instance_bottom = vertices.indices.len() as u32;
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>, bottom: bool) {
        if bottom {
            render_pass.set_index_buffer(
                self.index_buffer_bottom.get_slice(),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.set_vertex_buffer(0, self.vertex_buffer_bottom.get_slice());
            render_pass.draw_indexed(0..self.num_instance_bottom, 0, 0..1);
        } else {
            render_pass
                .set_index_buffer(self.index_buffer_top.get_slice(), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, self.vertex_buffer_top.get_slice());
            render_pass.draw_indexed(0..self.num_instance_top, 0, 0..1);
        }
    }

    pub fn draw_split<'a>(&'a self, render_pass: &mut RenderPass<'a>, bottom: bool) {
        if bottom {
            if self.num_instance_split_bottom > 0 {
                render_pass
                    .set_index_buffer(self.split_ibo_bottom.get_slice(), wgpu::IndexFormat::Uint16);
                render_pass.set_vertex_buffer(0, self.split_vbo_bottom.get_slice());
                render_pass.draw_indexed(0..self.num_instance_split_bottom, 0, 0..1);
            }
        } else {
            if self.num_instance_split_top > 0 {
                render_pass
                    .set_index_buffer(self.split_ibo_top.get_slice(), wgpu::IndexFormat::Uint16);
                render_pass.set_vertex_buffer(0, self.split_vbo_top.get_slice());
                render_pass.draw_indexed(0..self.num_instance_split_top, 0, 0..1);
            }
        }
    }
}

struct DynamicBuffer {
    buffer: Buffer,
    capacity: usize,
    length: u64,
    device: Rc<Device>,
    queue: Rc<Queue>,
    usage: wgpu::BufferUsages,
}

impl DynamicBuffer {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, usage: wgpu::BufferUsages) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 0,
            usage: usage | wgpu::BufferUsages::COPY_DST,
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
                usage: self.usage | wgpu::BufferUsages::COPY_DST,
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
