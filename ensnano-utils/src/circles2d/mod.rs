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
use std::rc::Rc;
use ultraviolet::Vec2;
use wgpu::{include_spirv, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

use crate::bindgroup_manager::DynamicBindGroup;
use crate::texture::Texture;
use ensnano_interactor::consts::*;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CircleInstance {
    pub center: Vec2,
    pub radius: f32,
    pub angle: f32,
    pub z_index: i32,
    color: u32,
}

impl CircleInstance {
    pub fn new(center: Vec2, radius: f32, z_index: i32, color: u32) -> Self {
        Self {
            center,
            radius,
            angle: 0.,
            z_index,
            color,
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color
    }

    #[allow(dead_code)]
    pub fn angle(self, angle: f32) -> Self {
        Self { angle, ..self }
    }

    pub fn in_rectangle(&self, c1: &Vec2, c2: &Vec2) -> bool {
        let min_x = c1.x.min(c2.x);
        let max_x = c1.x.max(c2.x);
        let min_y = c1.y.min(c2.y);
        let max_y = c1.y.max(c2.y);

        (self.center.x >= min_x
            && self.center.x <= max_x
            && self.center.y >= min_y
            && self.center.y <= max_y)
            || (self.center - Vec2::new(min_x, min_y)).mag() <= self.radius
            || (self.center - Vec2::new(min_x, max_y)).mag() <= self.radius
            || (self.center - Vec2::new(max_x, min_y)).mag() <= self.radius
            || (self.center - Vec2::new(max_x, max_y)).mag() <= self.radius
    }
}

pub struct CircleDrawer {
    device: Rc<Device>,
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<Rc<Vec<CircleInstance>>>,
    /// The number of instance to draw.
    number_instances: usize,
    /// The data sent the the GPU
    instances_bg: DynamicBindGroup,
    /// The pipeline created by `self`
    pipeline: Option<RenderPipeline>,
}

pub enum CircleKind {
    FullCircle,
    RotationWidget,
}

impl CircleDrawer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        globals_layout: &BindGroupLayout,
        circle_kind: CircleKind,
    ) -> Self {
        let instances_bg =
            DynamicBindGroup::new(device.clone(), queue.clone(), "circles instances");

        let mut ret = Self {
            device,
            new_instances: None,
            number_instances: 0,
            pipeline: None,
            instances_bg,
        };
        let pipeline = ret.create_pipeline(globals_layout, circle_kind);
        ret.pipeline = Some(pipeline);
        ret
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        self.update_instances();
        if self.number_instances > 0 {
            render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
            render_pass.set_bind_group(1, self.instances_bg.get_bindgroup(), &[]);
            render_pass.draw(0..4, 0..self.number_instances as u32);
        }
    }

    pub fn new_instances(&mut self, instances: Rc<Vec<CircleInstance>>) {
        self.new_instances = Some(instances)
    }

    fn update_instances(&mut self) {
        if let Some(ref instances) = self.new_instances {
            self.number_instances = instances.len();
            let instances_data: Vec<_> = instances.iter().cloned().collect();
            self.instances_bg.update(instances_data.as_slice());
        }
    }

    /// Create a render pipepline. This function is meant to be called once, before drawing for the
    /// first time.
    fn create_pipeline(
        &self,
        globals_layout: &BindGroupLayout,
        circle_kind: CircleKind,
    ) -> RenderPipeline {
        let vertex_module = self
            .device
            .create_shader_module(&include_spirv!("circle.vert.spv"));

        let fragment_module = match circle_kind {
            CircleKind::FullCircle => self
                .device
                .create_shader_module(&include_spirv!("circle.frag.spv")),
            CircleKind::RotationWidget => self
                .device
                .create_shader_module(&include_spirv!("rotation_widget.frag.spv")),
        };

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[globals_layout, &self.instances_bg.get_layout()],
                    push_constant_ranges: &[],
                    label: Some("Circle drawer pipeline layout"),
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
                    buffers: &[],
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
                label: Some("CircleDrawer render pipeline"),
            })
    }
}
