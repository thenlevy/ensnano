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
use std::collections::HashMap;
use std::rc::Rc;
use ultraviolet::{Mat2, Vec2, Vec4};
use wgpu::{include_spirv, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

use crate::bindgroup_manager::DynamicBindGroup;
use crate::text::{Letter, Vertex as CharVertex};
use crate::texture::Texture;
use ensnano_interactor::consts::*;
mod text_drawer;
pub use text_drawer::{Line, Sentence, TextDrawer};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CharInstance {
    /// The top left of the glyph's bounding box
    pub top_left: Vec2,
    pub rotation: Mat2,
    pub size: f32,
    pub z_index: i32,
    pub color: Vec4,
}

pub struct CharDrawer {
    device: Rc<Device>,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<CharInstance>>>,
    /// The number of instance to draw.
    number_instances: usize,
    /// The data sent the the GPU
    instances_bg: DynamicBindGroup,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
    letter: Rc<Letter>,
}

impl CharDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        globals_layout: &BindGroupLayout,
        character: char,
    ) -> Self {
        let instances_bg = DynamicBindGroup::new(device.clone(), queue.clone(), "chars instances");
        let char_texture = Rc::new(Letter::new(character, device.clone(), queue.clone()));

        let new_instances = vec![CharInstance {
            top_left: Vec2::zero(),
            rotation: Mat2::identity(),
            z_index: -1,
            size: 1.,
            color: Vec4::zero(),
        }];
        let mut ret = Self {
            device,
            new_instances: Some(Rc::new(new_instances)),
            number_instances: 0,
            pipeline: None,
            instances_bg,
            letter: char_texture.clone(),
        };
        let pipeline = ret.create_pipeline(globals_layout);
        ret.pipeline = Some(pipeline);
        ret
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        self.update_instances();
        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(1, self.instances_bg.get_bindgroup(), &[]);
        render_pass.set_bind_group(TEXTURE_BINDING_ID, &self.letter.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.letter.vertex_buffer.slice(..));
        render_pass.draw(0..4, 0..self.number_instances as u32);
    }

    pub fn new_instances(&mut self, instances: Rc<Vec<CharInstance>>) {
        self.new_instances = Some(instances)
    }

    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().cloned().collect();
            self.instances_bg.update(instances_data.as_slice());
        }
    }

    pub fn advancement_x(&self) -> f32 {
        self.letter.advance
    }

    /// Create a render pipepline. This function is meant to be called once, before drawing for the
    /// first time.
    fn create_pipeline(&self, globals_layout: &BindGroupLayout) -> RenderPipeline {
        let vertex_module = self
            .device
            .create_shader_module(&include_spirv!("chars.vert.spv"));
        let fragment_module = self
            .device
            .create_shader_module(&include_spirv!("chars.frag.spv"));
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        globals_layout,
                        &self.instances_bg.get_layout(),
                        &self.letter.bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                    label: Some("render_pipeline_layout"),
                });

        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let targets = &[wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        }];

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: Some(wgpu::IndexFormat::Uint16),
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_module,
                    entry_point: "main",
                    buffers: &[CharVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_module,
                    entry_point: "main",
                    targets,
                }),
                primitive,
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLE_COUNT,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                label: Some("Char drawer render pipeline"),
            })
    }
}

pub fn char_positions_x(string: &str, drawers: &HashMap<char, CharDrawer>) -> Vec<f32> {
    let mut ret = vec![0f32];
    let mut x = 0f32;
    for c in string.chars() {
        x += drawers.get(&c).unwrap().advancement_x();
        ret.push(x);
    }
    ret
}

pub fn char_positions_y(string: &str, drawers: &HashMap<char, CharDrawer>) -> Vec<f32> {
    let max_height = height(string, drawers);
    let mut ret = vec![];

    for c in string.chars() {
        ret.push(
            max_height
                - drawers.get(&c).unwrap().letter.height
                - drawers.get(&c).unwrap().letter.advance_height,
        )
    }
    ret
}

pub fn height(string: &str, drawers: &HashMap<char, CharDrawer>) -> f32 {
    let mut ret = 0f32;
    for c in string.chars() {
        ret = ret.max(drawers.get(&c).unwrap().letter.height)
    }
    ret
}
