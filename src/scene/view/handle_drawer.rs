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
use super::{CameraPtr, Drawable, Drawer, ProjectionPtr, Vertex};
use crate::consts::*;
use ensnano_design::group_attributes::GroupPivot;
use iced_wgpu::wgpu;
use std::rc::Rc;
use ultraviolet::{Rotor3, Vec3};
use wgpu::Device;

#[derive(Clone, Debug)]
pub struct HandlesDescriptor {
    pub origin: Vec3,
    pub orientation: HandleOrientation,
    pub size: f32,
    pub colors: HandleColors,
}

#[derive(Debug, Clone, Copy)]
pub enum HandleOrientation {
    Rotor(Rotor3),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandleColors {
    Rgb,
    Cym,
}

impl From<HandleOrientation> for Rotor3 {
    fn from(orientation: HandleOrientation) -> Self {
        let HandleOrientation::Rotor(r) = orientation;
        r
    }
}

impl HandlesDescriptor {
    pub fn make_handles(&self, camera: CameraPtr, projection: ProjectionPtr) -> [Handle; 3] {
        let dist = (camera.borrow().position - self.origin).mag();
        let (right, up, dir) = self.make_axis();
        let length = self.size * dist * (projection.borrow().get_fovy() / 2.).tan();
        let colors = match self.colors {
            HandleColors::Cym => crate::consts::CYM_HANDLE_COLORS,
            HandleColors::Rgb => crate::consts::RGB_HANDLE_COLORS,
        };
        [
            Handle::new(self.origin, right, up, colors[0], RIGHT_HANDLE_ID, length),
            Handle::new(self.origin, up, right, colors[1], UP_HANDLE_ID, length),
            Handle::new(self.origin, dir, up, colors[2], DIR_HANDLE_ID, length),
        ]
    }

    fn make_axis(&self) -> (Vec3, Vec3, Vec3) {
        match self.orientation {
            HandleOrientation::Rotor(rotor) => (
                rotor * Vec3::unit_x(),
                rotor * Vec3::unit_y(),
                rotor * -Vec3::unit_z(),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HandleDir {
    Right,
    Up,
    Dir,
}

impl HandleDir {
    pub fn from_widget_id(widget_id: u32) -> Self {
        match widget_id {
            RIGHT_HANDLE_ID => Self::Right,
            UP_HANDLE_ID => Self::Up,
            DIR_HANDLE_ID => Self::Dir,
            _ => unreachable!("from widget id"),
        }
    }
}

pub struct HandlesDrawer {
    descriptor: Option<HandlesDescriptor>,
    handles: Option<[Handle; 3]>,
    drawers: [Drawer<Handle>; 3],
    big_handle: Option<Handle>,
    big_handle_drawer: Drawer<Handle>,
    selected: Option<usize>,
    origin_translation: Option<(f32, f32)>,
}

impl HandlesDrawer {
    pub fn new(device: Rc<Device>) -> Self {
        Self {
            descriptor: None,
            handles: None,
            drawers: [
                Drawer::new(device.clone()),
                Drawer::new(device.clone()),
                Drawer::new(device.clone()),
            ],
            big_handle: None,
            big_handle_drawer: Drawer::new(device),
            selected: None,
            origin_translation: None,
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        viewer_bind_group: &'a wgpu::BindGroup,
        viewer_bind_group_layout: &'a wgpu::BindGroupLayout,
        fake: bool,
    ) {
        for drawer in self.drawers.iter_mut() {
            drawer.draw(
                render_pass,
                viewer_bind_group,
                viewer_bind_group_layout,
                fake,
            );
        }
        self.big_handle_drawer.draw(
            render_pass,
            viewer_bind_group,
            viewer_bind_group_layout,
            fake,
        );
    }

    pub fn update_decriptor(
        &mut self,
        descriptor: Option<HandlesDescriptor>,
        camera: CameraPtr,
        projection: ProjectionPtr,
    ) {
        if self.origin_translation.is_none() {
            self.descriptor = descriptor;
            self.update_camera(camera, projection);
        }
    }

    pub fn end_movement(&mut self) {
        self.origin_translation = None;
    }

    pub fn update_camera(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        self.handles = self
            .descriptor
            .as_ref()
            .map(|desc| desc.make_handles(camera, projection));
        self.update_drawers();
    }

    pub fn init_translation(&mut self, x: f32, y: f32) {
        self.origin_translation = Some((x, y))
    }

    pub fn get_origin_translation(&self) -> Option<(f32, f32)> {
        self.origin_translation
    }

    fn update_drawers(&mut self) {
        if let Some(handles) = self.handles {
            for (i, handle) in handles.iter().enumerate() {
                self.drawers[i].new_object(Some(*handle));
            }
        } else {
            for i in 0..3 {
                self.drawers[i].new_object(None);
            }
        }
        self.select_handle(self.selected);
        self.big_handle_drawer.new_object(self.big_handle)
    }

    pub fn get_handle(&self, direction: HandleDir) -> Option<(Vec3, Vec3)> {
        self.handles.as_ref().map(|handles| {
            let i = match direction {
                HandleDir::Right => 0,
                HandleDir::Up => 1,
                HandleDir::Dir => 2,
            };
            let handle = handles[i];
            (handle.origin, handle.direction)
        })
    }

    pub fn set_selected(&mut self, selected_id: Option<u32>) -> bool {
        let selected_id = selected_id;
        let new_selection = match selected_id {
            Some(RIGHT_HANDLE_ID) => Some(0),
            Some(UP_HANDLE_ID) => Some(1),
            Some(DIR_HANDLE_ID) => Some(2),
            _ => None,
        };
        let ret = new_selection != self.selected;
        self.select_handle(new_selection);
        ret
    }

    fn select_handle(&mut self, selected: Option<usize>) {
        self.big_handle = if let Some(selected) = selected {
            self.handles.map(|t| t[selected].bigger_version())
        } else {
            None
        };
        self.selected = selected;
        self.big_handle_drawer.new_object(self.big_handle);
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.handles
            .as_mut()
            .map(|handles| {
                for h in handles.iter_mut() {
                    h.translation = translation;
                }
            })
            .unwrap_or(());
        if let Some(h) = self.big_handle.as_mut() {
            h.translation = translation
        }
        self.update_drawers();
    }

    pub fn get_pivot_position(&self) -> Option<GroupPivot> {
        self.descriptor.as_ref().map(|d| GroupPivot {
            position: d.origin,
            orientation: d.orientation.into(),
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Handle {
    pub origin: Vec3,
    pub direction: Vec3,
    pub translation: Vec3,
    normal: Vec3,
    color: u32,
    id: u32,
    length: f32,
}

impl Handle {
    pub fn new(
        origin: Vec3,
        direction: Vec3,
        normal: Vec3,
        color: u32,
        id: u32,
        length: f32,
    ) -> Self {
        Self {
            origin,
            direction,
            translation: Vec3::zero(),
            normal,
            color,
            id,
            length,
        }
    }

    pub fn bigger_version(&self) -> Self {
        Self {
            length: 1.1 * self.length,
            ..*self
        }
    }
}

impl Drawable for Handle {
    fn vertices(&self, fake: bool) -> Vec<Vertex> {
        let mut ret = Vec::new();
        let length = if fake {
            self.length * 1.03
        } else {
            self.length
        };
        let width = if fake {
            length / 30. * SELECT_SCALE_FACTOR
        } else {
            length / 30.
        };
        let color = if fake { self.id } else { self.color };
        for x in [-1f32, 1.].iter() {
            for y in [-1., 1.].iter() {
                for z in [0., 1.].iter() {
                    ret.push(Vertex::new(
                        self.origin
                            + self.normal * *x * width
                            + *y * self.direction.cross(self.normal) * width
                            + *z * self.direction * length
                            + self.translation,
                        color,
                        fake,
                    ));
                }
            }
        }
        ret
    }

    fn indices() -> Vec<u16> {
        vec![
            0, 1, 2, 1, 2, 3, 0, 1, 5, 0, 4, 5, 0, 4, 6, 0, 6, 2, 5, 4, 6, 5, 6, 7, 2, 6, 7, 3, 6,
            7, 1, 5, 7, 1, 3, 7,
        ]
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }
}
