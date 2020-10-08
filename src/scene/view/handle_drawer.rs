use std::rc::Rc;
use iced_wgpu::wgpu;
use wgpu::Device;
use ultraviolet::{Rotor3, Vec3};
use super::{Drawer, CameraPtr, ProjectionPtr, Drawable, Vertex};
use crate::consts::*;

#[derive(Clone, Debug)]
pub struct HandlesDescriptor {
    pub origin: Vec3,
    pub orientation: HandleOrientation,
    pub size: f32,
}

#[derive(Debug, Clone)]
pub enum HandleOrientation {
    Camera,
    Rotor(Rotor3),
}

impl HandlesDescriptor {
    pub fn make_handles(&self, camera: CameraPtr, projection: ProjectionPtr) -> [Handle ; 3] {
        let dist = (camera.borrow().position - self.origin).mag();
        let (right, up, dir) = self.make_axis(camera);
        let length = self.size * dist * (projection.borrow().get_fovy() / 2.).tan();
        [
            Handle::new(self.origin, right, up, 0xFF0000, RIGHT_HANDLE_ID , length),
            Handle::new(self.origin, up, right, 0xFF00, UP_HANDLE_ID, length),
            Handle::new(self.origin, dir, up, 0xFF, DIR_HANDLE_ID, length)
        ]
    }

    fn make_axis(&self, camera: CameraPtr) -> (Vec3, Vec3, Vec3) {
        match self.orientation {
            HandleOrientation::Camera => {
                let right = camera.borrow().right_vec();
                let up = camera.borrow().up_vec();
                let dir = camera.borrow().direction();
                let rotor = Rotor3::from_angle_plane(-std::f32::consts::FRAC_PI_4, right.wedge(dir).normalized());
                (rotor * camera.borrow().right_vec(),
                 up,
                 rotor * -camera.borrow().direction())
            }
            HandleOrientation::Rotor(rotor) => (rotor * Vec3::unit_x(), rotor * Vec3::unit_y(), rotor * -Vec3::unit_z())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HandleDir {
    Right,
    Up,
    Dir,
}

pub struct HandlesDrawer {
    descriptor: Option<HandlesDescriptor>,
    handles: Option<[Handle ; 3]>,
    drawers: [Drawer<Handle> ; 3],
    origin_translation: Option<(f32, f32)>,
}

impl HandlesDrawer {
    pub fn new(device: Rc<Device>) -> Self {
        Self {
            descriptor: None,
            handles: None,
            drawers: [Drawer::new(device.clone()), Drawer::new(device.clone()), Drawer::new(device.clone())],
            origin_translation: None,
        }
    }

    pub fn update_decriptor(&mut self, descriptor: Option<HandlesDescriptor>, camera: CameraPtr, projection: ProjectionPtr) {
        self.descriptor = descriptor;
        self.update_camera(camera, projection);
    }

    pub fn update_camera(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        self.handles = self.descriptor.as_ref().map(|desc| desc.make_handles(camera, projection));
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
            for i in 0..3 {
                self.drawers[i].new_object(Some(handles[i]));
            }
        } else {
            for i in 0..3 {
                self.drawers[i].new_object(None);
            }
        }
    }

    pub fn drawers(&mut self) -> &mut [Drawer<Handle> ;3] {
        &mut self.drawers
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

    pub fn translate(&mut self, translation: Vec3) {
        self.handles.as_mut().map(|handles| {
            for h in handles.iter_mut() {
                h.translation = translation;
            }
        }).unwrap_or(());
        self.update_drawers();
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
    pub fn new(origin: Vec3, direction: Vec3, normal: Vec3, color: u32, id: u32, length: f32) -> Self {
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
}

impl Drawable for Handle {

    fn vertices(&self, fake: bool) -> Vec<Vertex> {
        let mut ret = Vec::new();
        let width = self.length / 30.;
        let color = if fake {
            self.id
        } else {
            self.color
        };
        for x in [-1f32, 1.].iter() {
            for y in [-1., 1.].iter() {
                for z in [0., 1.].iter() {
                    ret.push(Vertex::new(self.origin + self.normal * *x * width + *y * self.direction.cross(self.normal) * width + *z * self.direction * self.length + self.translation,color));
                }
            }
        }
        ret
    }

    fn indices() -> Vec<u16> {
        vec![
            0, 1, 2,
            1, 2, 3,
            0, 1, 5,
            0, 4, 5,
            0, 4, 6,
            0, 6, 2,
            5, 4, 6,
            5, 6, 7,
            2, 6, 7,
            3, 6, 7,
            1, 5, 7,
            1, 3, 7]
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }
}

