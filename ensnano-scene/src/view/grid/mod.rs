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
use ensnano_design::ultraviolet;
use ensnano_utils::wgpu;
use ultraviolet::{Mat4, Vec2, Vec3, Vec4};
use wgpu::{include_spirv, Device, RenderPass};

use super::{grid_disc::GridDisc, instances_drawer::*, LetterInstance};
use ensnano_design::grid::{Grid, GridDivision, GridId, GridPosition, GridType};
use std::collections::BTreeMap;

mod texture;

#[derive(Debug, Clone)]
pub struct GridInstance {
    pub grid: Grid,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub color: u32,
    pub design: usize,
    pub id: GridId,
    pub fake: bool,
    pub visible: bool,
}

impl GridInstance {
    pub fn disc(&self, x: isize, y: isize, color: u32, model_id: u32) -> (GridDisc, GridDisc) {
        let position = self.grid.position_helix(x, y);
        let orientation = self.grid.orientation;
        let gd = GridDisc {
            position,
            orientation,
            model_id,
            radius: 1.1 * self.grid.parameters.helix_radius,
            color,
        };
        (
            GridDisc {
                position: gd.position + 0.001 * self.grid.axis_helix(),
                ..gd
            },
            GridDisc {
                position: gd.position - 0.001 * self.grid.axis_helix(),
                ..gd
            },
        )
    }

    pub fn letter_instance(
        &self,
        x: isize,
        y: isize,
        h_id: usize,
        instances: &mut Vec<Vec<LetterInstance>>,
        right: Vec3,
        up: Vec3,
    ) {
        let position = self.grid.position_helix(x, y);
        for (c_idx, c) in h_id.to_string().chars().enumerate() {
            let shift = 0.5 * up - 0.35 * h_id.to_string().len() as f32 * right;
            let instance = LetterInstance {
                position: position + 0.7 * c_idx as f32 * right + shift,
                color: ultraviolet::Vec4::new(0., 0., 0., 1.),
                design_id: self.design as u32,
                scale: 3.,
                shift: Vec3::zero(),
            };
            let idx = c.to_digit(10).unwrap();
            instances[idx as usize].push(instance);
        }
    }

    fn to_fake(&self) -> Self {
        let color = match self.id {
            GridId::FreeGrid(id) => id as u32,
            GridId::BezierPathGrid(vertex) => {
                crate::element_selector::bezier_vertex_id(vertex.path_id, vertex.vertex_id)
            }
        };
        Self {
            color,
            fake: true,
            ..self.clone()
        }
    }

    fn to_raw(&self) -> GridInstanceRaw {
        use ensnano_utils::instance::Instance;
        let (min_x, min_y, max_x, max_y);
        if let GridType::Hyperboloid(ref h) = self.grid.grid_type {
            min_x = -h.grid_radius(&self.grid.parameters);
            max_x = h.grid_radius(&self.grid.parameters);
            min_y = -h.grid_radius(&self.grid.parameters);
            max_y = h.grid_radius(&self.grid.parameters);
        } else {
            min_x = self.min_x as f32;
            max_x = self.max_x as f32;
            min_y = self.min_y as f32;
            max_y = self.max_y as f32;
        }
        let grid_type = if self.fake {
            self.grid.grid_type.descr().to_u32() + 1000
        } else {
            self.grid.grid_type.descr().to_u32()
        };
        GridInstanceRaw {
            model: Mat4::from_translation(self.grid.position)
                * self.grid.orientation.into_matrix().into_homogeneous(),
            min_x,
            max_x,
            min_y,
            max_y,
            grid_type,
            color: Instance::color_from_au32(self.color),
            inter_helix_gap: self.grid.parameters.inter_helix_gap,
            helix_radius: self.grid.parameters.helix_radius,
            design_id: self.design as u32,
        }
    }

    /// Return x >= 0 so that orgin + x axis is on the grid, or None if such an x does not exist.
    fn ray_intersection(&self, origin: Vec3, axis: Vec3) -> Option<GridIntersection> {
        let ret = self.grid.ray_intersection(origin, axis)?;
        if ret < 0. {
            return None;
        }
        let (x, y) = {
            let intersection = origin + ret * axis;
            let vec = intersection - self.grid.position;
            let x_dir = Vec3::unit_z().rotated_by(self.grid.orientation);
            let y_dir = Vec3::unit_y().rotated_by(self.grid.orientation);
            (vec.dot(x_dir), vec.dot(y_dir))
        };
        if self.contains_point(x, y) {
            let (x, y) = self.grid.grid_type.interpolate(&self.grid.parameters, x, y);

            Some(GridIntersection {
                depth: ret,
                grid_id: self.id,
                design_id: self.design,
                x,
                y,
            })
        } else {
            None
        }
    }

    fn convert_coord(&self, x: f32, y: f32) -> (f32, f32) {
        match self.grid.grid_type {
            GridType::Square(_) => {
                let r =
                    self.grid.parameters.helix_radius * 2. + self.grid.parameters.inter_helix_gap;
                (x / r, y / r)
            }
            GridType::Honeycomb(_) => {
                let r =
                    self.grid.parameters.helix_radius * 2. + self.grid.parameters.inter_helix_gap;
                (x * 2. / (3f32.sqrt() * r), (y - r / 2.) * 2. / (3. * r))
            }
            GridType::Hyperboloid(_) => unreachable!(),
        }
    }

    fn contains_point(&self, x: f32, y: f32) -> bool {
        if let GridType::Hyperboloid(ref h) = self.grid.grid_type {
            h.contains_point(&self.grid.parameters, x, y)
        } else {
            let (x, y) = self.convert_coord(x, y);
            x >= self.min_x as f32 - 0.025
                && x <= self.max_x as f32 + 0.025
                && y >= -self.max_y as f32 - 0.025
                && y <= -self.min_y as f32 + 0.025
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridInstanceRaw {
    pub model: Mat4,          // padding 0
    pub min_x: f32,           // padding 1
    pub max_x: f32,           // padding 2
    pub min_y: f32,           // padding 3
    pub max_y: f32,           // padding 0
    pub color: Vec4,          // padding 0
    pub grid_type: u32,       // padding 1
    pub helix_radius: f32,    // padding 2,
    pub inter_helix_gap: f32, // padding 3,
    pub design_id: u32,       // padding 0,
}

/// A structure that manages the pipepline that draw the grids
pub struct GridManager {
    /// A possible updates to the instances to be drawn. Must be taken into account before drawing
    /// next frame
    new_instances: Option<BTreeMap<GridId, GridInstance>>,
    instances: BTreeMap<GridId, GridInstance>,
    selected: Vec<(usize, GridId)>,
    candidate: Vec<(usize, GridId)>,
    drawer: InstanceDrawer<GridInstance>,
    fake_drawer: InstanceDrawer<GridInstance>,
    need_new_colors: bool,
}

impl GridManager {
    pub fn new(
        drawer: InstanceDrawer<GridInstance>,
        fake_drawer: InstanceDrawer<GridInstance>,
    ) -> Self {
        Self {
            drawer,
            fake_drawer,
            new_instances: Some(Default::default()),
            instances: Default::default(),
            selected: vec![],
            candidate: vec![],
            need_new_colors: false,
        }
    }

    /// Request an update of the set of instances to draw. This update take effects on the next frame
    pub fn new_instances(&mut self, instances: BTreeMap<GridId, GridInstance>) {
        self.new_instances = Some(instances)
    }

    /// If one or several update of the set of instances were requested before the last call of
    /// this function, perform the most recent update.
    fn update_instances(&mut self) {
        if let Some(instances) = self.new_instances.take() {
            self.instances = instances.clone();
            let fake_instances: Vec<GridInstance> =
                self.instances.values().map(GridInstance::to_fake).collect();
            if !self.need_new_colors {
                self.drawer
                    .new_instances(instances.values().cloned().collect());
            }
            self.fake_drawer.new_instances(fake_instances);
        }
    }

    /// Draw the instances of the mesh on the render pass
    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
        fake: bool,
    ) {
        self.update_instances();
        if self.need_new_colors {
            self.update_colors();
        }
        if fake {
            self.fake_drawer
                .draw(render_pass, viewer_bind_group, model_bind_group)
        } else {
            self.drawer
                .draw(render_pass, viewer_bind_group, model_bind_group)
        }
    }

    pub fn intersect(&self, origin: Vec3, direction: Vec3) -> Option<GridIntersection> {
        let mut ret = None;
        let mut depth = std::f32::INFINITY;
        for g in self.instances.values() {
            if let Some(intersection) = g.ray_intersection(origin, direction) {
                if intersection.depth < depth {
                    ret = Some(intersection.clone());
                    depth = intersection.depth;
                }
            }
        }
        ret
    }

    pub fn specific_intersect(
        &self,
        origin: Vec3,
        direction: Vec3,
        grid_id: GridId,
    ) -> Option<GridIntersection> {
        self.instances
            .get(&grid_id)
            .and_then(|g| g.ray_intersection(origin, direction))
    }

    pub fn set_candidate_grid(&mut self, grids: Vec<(usize, GridId)>) {
        self.need_new_colors = true;
        self.candidate = grids
    }

    pub fn set_selected_grid(&mut self, grids: Vec<(usize, GridId)>) {
        self.need_new_colors = true;
        self.selected = grids
    }

    fn update_colors(&mut self) {
        for instance in self.instances.values_mut() {
            if self.selected.contains(&(instance.design, instance.id)) {
                instance.color = 0xFF_00_00
            } else if self.candidate.contains(&(instance.design, instance.id)) {
                instance.color = 0x00_FF_00
            } else {
                instance.color = 0x00_00_FF
            }
        }
        self.drawer
            .new_instances(self.instances.values().cloned().collect());
        self.need_new_colors = false;
    }
}

#[derive(Clone)]
pub struct GridIntersection {
    pub depth: f32,
    pub design_id: usize,
    pub grid_id: GridId,
    pub x: isize,
    pub y: isize,
}

impl GridIntersection {
    pub fn grid_position(&self) -> GridPosition {
        GridPosition {
            grid: self.grid_id,
            x: self.x,
            y: self.y,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridVertex {
    pub position: Vec2,
}

impl Vertexable for GridVertex {
    type RawType = GridVertex;

    fn to_raw(&self) -> Self {
        *self
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GridVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        }
    }
}

pub struct GridTextures {
    square_texture: texture::SquareTexture,
    honney_texture: texture::HonneyTexture,
}

impl GridTextures {
    pub fn new(device: &Device, encoder: &mut wgpu::CommandEncoder) -> Self {
        let square_texture = texture::SquareTexture::new(device, encoder);
        let honney_texture = texture::HonneyTexture::new(device, encoder);
        Self {
            square_texture,
            honney_texture,
        }
    }
}

impl RessourceProvider for GridTextures {
    fn ressources_layout() -> &'static [wgpu::BindGroupLayoutEntry] {
        &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }

    fn ressources(&self) -> Vec<wgpu::BindGroupEntry> {
        vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.square_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&self.square_texture.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&self.honney_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&self.honney_texture.sampler),
            },
        ]
    }
}

impl Instanciable for GridInstance {
    type Vertex = GridVertex;
    type RawInstance = GridInstanceRaw;
    type Ressource = GridTextures;

    fn to_raw_instance(&self) -> GridInstanceRaw {
        self.to_raw()
    }

    fn vertices() -> Vec<GridVertex> {
        vec![
            GridVertex {
                position: Vec2::new(0f32, 0f32),
            },
            GridVertex {
                position: Vec2::new(0f32, 1f32),
            },
            GridVertex {
                position: Vec2::new(1f32, 0f32),
            },
            GridVertex {
                position: Vec2::new(1f32, 1f32),
            },
        ]
    }

    fn indices() -> Vec<u16> {
        vec![0, 1, 2, 3]
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleStrip
    }

    fn vertex_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&include_spirv!("grid.vert.spv"))
    }

    fn fragment_module(device: &Device) -> wgpu::ShaderModule {
        device.create_shader_module(&include_spirv!("grid.frag.spv"))
    }

    fn alpha_to_coverage_enabled() -> bool {
        true
    }
}
