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
use std::collections::HashMap;
use iced_wgpu::wgpu;
use wgpu::{Device, Queue, RenderPipeline};
use super::{DrawArea, CameraPtr};
use crate::PhySize;
use crate::consts::*;
use crate::utils::bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use crate::utils::texture::Texture;
use crate::utils::{chars2d as chars, circles2d as circles};
use circles::CircleDrawer;
pub use circles::CircleInstance;
use chars::CharDrawer;
pub use chars::CharInstance;

pub struct View {
    device: Rc<Device>,
    queue: Rc<Queue>,
    background_pipeline: RenderPipeline,
    area_size: PhySize,
    depth_texture: Texture,
    circle_drawer: CircleDrawer,
    char_drawers: HashMap<char, CharDrawer>,
    char_map: HashMap<char, Vec<CharInstance>>,
    globals: UniformBindGroup,
}

impl View {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        area: DrawArea,
        camera: CameraPtr,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(device.as_ref(), &area.size, SAMPLE_COUNT);
        let globals =
            UniformBindGroup::new(device.clone(), queue.clone(), camera.borrow().get_globals());

        let depth_stencil_state = Some(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor::IGNORE,
                back: wgpu::StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        });
        let background_pipeline = background_pipeline(device.clone().as_ref(), depth_stencil_state);
        let circle_drawer =
            CircleDrawer::new(device.clone(), queue.clone(), encoder, globals.get_layout());
        let chars = [
            'A', 'T', 'G', 'C', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-',
        ];
        let mut char_drawers = HashMap::new();
        let mut char_map = HashMap::new();
        for c in chars.iter() {
            char_drawers.insert(
                *c,
                CharDrawer::new(device.clone(), queue.clone(), globals.get_layout(), *c),
            );
            char_map.insert(*c, Vec::new());
        }
        Self {
            device,
            queue,
            background_pipeline,
            area_size: area.size,
            depth_texture,
            circle_drawer,
            char_map,
            char_drawers,
            globals,
        }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        area: DrawArea,
    ) {
        let clear_color = wgpu::Color {
            r: 1.,
            g: 0.,
            b: 0.,
            a: 1.,
        };

        let msaa_texture = if SAMPLE_COUNT > 1 {
            Some(crate::utils::texture::Texture::create_msaa_texture(
                self.device.clone().as_ref(),
                &self.area_size,
                SAMPLE_COUNT,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ))
        } else {
            None
        };

        let attachment = if msaa_texture.is_some() {
            msaa_texture.as_ref().unwrap()
        } else {
            target
        };

        let resolve_target = if msaa_texture.is_some() {
            Some(target)
        } else {
            None
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: true,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: true,
                }),
            }),
        });

        // Draw the background
        render_pass.set_pipeline(&self.background_pipeline);
        render_pass.draw(0..4, 0..1);

        // Everything else requires the globals bind group to be set
        render_pass.set_bind_group(0, self.globals.get_bindgroup(), &[]);

        // Draw the circles representing the helices
        self.circle_drawer.draw(&mut render_pass);

        // Draw the helices numbers
        for drawer in self.char_drawers.values_mut() {
            drawer.draw(&mut render_pass);
        }
        
    }

    pub fn resize(&mut self, area: DrawArea) {
        self.depth_texture = 
            Texture::create_depth_texture(self.device.as_ref(), &area.size, SAMPLE_COUNT);
        self.area_size = area.size;
    }

    pub fn update_circles(&mut self, circles: Vec<CircleInstance>) {
        self.circle_drawer.new_instances(Rc::new(circles));
        for (c, v) in self.char_map.iter() {
            self.char_drawers
                .get_mut(c)
                .unwrap()
                .new_instances(Rc::new(v.clone()))
        }
    }

    pub fn get_char_map(&mut self) -> &mut HashMap<char, Vec<CharInstance>> {
        &mut self.char_map
    }
}

fn background_pipeline(device: &Device, depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>) -> RenderPipeline {
    let vs_module = &device.create_shader_module(wgpu::include_spirv!("view/background.vert.spv"));
    let fs_module = &device.create_shader_module(wgpu::include_spirv!("view/background.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[],
        push_constant_ranges: &[],
        label: None,
    });

    let desc = wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            ..Default::default()
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: SAMPLE_COUNT,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&desc)
}

