use std::f32::consts::PI;
use std::rc::Rc;
use iced_wgpu::wgpu;
use wgpu::Device;
use ultraviolet::{Rotor3, Vec3};
use super::{CameraPtr, Drawer, Drawable, ProjectionPtr, Vertex, maths};
use crate::consts::*;


#[derive(Debug, Clone, Copy)]
pub enum RotationMode {
    Right,
    Up,
    Front,
    Free,
}

pub struct RotationWidget {
    descriptor: Option<RotationWidgetDescriptor>,
    sphere: Option<Sphere>,
    circles: Option<[Circle; 3]>,
    sphere_drawer: Drawer<Sphere>,
    circle_drawers: [Drawer<Circle>; 3],
    rotation_origin: Option<(f32, f32)>,
    translation: Vec3,
}

impl RotationWidget {
    pub fn new(device: Rc<Device>) -> Self {
        Self {
            descriptor: None,
            sphere: None,
            circles: None,
            sphere_drawer: Drawer::new(device.clone()),
            circle_drawers: [Drawer::new(device.clone()), Drawer::new(device.clone()), Drawer::new(device.clone())],
            rotation_origin: None,
            translation: Vec3::zero(),
        }
    }

    pub fn update_decriptor(&mut self, descriptor: Option<RotationWidgetDescriptor>, camera: CameraPtr, projection: ProjectionPtr) {
        self.descriptor = descriptor;
        self.translation = Vec3::zero();
        self.update_camera(camera, projection);
    }

    pub fn update_camera(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        self.sphere = self.descriptor.as_ref().map(|desc| desc.make_sphere(camera.clone(), projection.clone()));
        self.circles = self.descriptor.as_ref().map(|desc| desc.make_circles(camera, projection));
        self.update_drawers();
    }

    fn update_drawers(&mut self) {
        if let Some(circles) = self.circles {
            for i in 0..3 {
                self.circle_drawers[i].new_object(Some(circles[i]));
            }
        } else {
            for i in 0..3 {
                self.circle_drawers[i].new_object(None);
            }
        }
        self.sphere_drawer.new_object(self.sphere);
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>, viewer_bind_group: &'a wgpu::BindGroup, viewer_bind_group_layout: &'a wgpu::BindGroupLayout, fake: bool) {
        for drawer in self.circle_drawers.iter_mut() {
            drawer.draw(render_pass, viewer_bind_group, viewer_bind_group_layout, fake);
        }
        self.sphere_drawer.draw(render_pass, viewer_bind_group, viewer_bind_group_layout, fake);
    }

    pub fn init_rotation(&mut self, x: f32, y: f32) {
        self.rotation_origin = Some((x, y))
    }

    pub fn compute_rotation(&self, x: f32, y: f32, camera: CameraPtr, projection: ProjectionPtr, mode: RotationMode) -> Option<(Rotor3, Vec3)> {
        let (x_init, y_init) = self.rotation_origin?;
        println!("origin");
        let circles = &self.circles?;
        println!("circles");
        let (origin, normal) = match mode {
            RotationMode::Right => (circles[0].origin, circles[0].normal()),
            RotationMode::Up => (circles[1].origin, circles[1].normal()),
            RotationMode::Front => (circles[2].origin, circles[2].normal()),
            _ => unreachable!(),
        };
        println!("normal {:?}", normal);
        let point_clicked = maths::unproject_point_on_plane(origin, normal, camera.clone(), projection.clone(), x_init, y_init)?;
        println!("point clicked {:?}", point_clicked);
        let point_moved = maths::unproject_point_on_plane(origin, normal, camera.clone(), projection.clone(), x, y)?;
        println!("point moved {:?}", point_moved);
        Some((Rotor3::from_rotation_between((point_clicked - origin).normalized(), (point_moved - origin).normalized()), origin))
    }

    pub fn translate(&mut self, translation: Vec3) {
        if let Some(ref mut circles) = self.circles {
            for circle in circles.iter_mut() {
                circle.translate(translation);
            }
        }
        if let Some(ref mut sphere) = self.sphere {
            sphere.translate(translation);
        }
        self.update_drawers()
    }
}

#[derive(Clone, Debug)]
pub struct RotationWidgetDescriptor {
    pub origin: Vec3,
    pub orientation: RotationWidgetOrientation,
    pub size: f32,
}

#[derive(Debug, Clone)]
pub enum RotationWidgetOrientation {
    Camera,
    Rotor(Rotor3),
}

impl RotationWidgetDescriptor {
    fn make_circles(&self, camera: CameraPtr, projection: ProjectionPtr) -> [Circle ; 3] {
        let dist = (camera.borrow().position - self.origin).mag();
        let (right, up, dir) = self.make_axis(camera);
        let length = self.size * dist * (projection.borrow().get_fovy() / 2.).tan() * 1.1;
        [
            Circle::new(self.origin, length, up, dir, 0xFF_00_00, RIGHT_CIRCLE_ID),
            Circle::new(self.origin, length, right, dir, 0xFF_00, UP_CIRCLE_ID),
            Circle::new(self.origin, length * 1.1, right, up, 0xFF_FF_00, FRONT_CIRCLE_ID),
        ]
    }

    fn make_sphere(&self, camera: CameraPtr, projection: ProjectionPtr) -> Sphere {
        let dist = (camera.borrow().position - self.origin).mag();
        let length = self.size * dist * (projection.borrow().get_fovy() / 2.).tan();
        Sphere::new(self.origin, length, 0xA0_54_54_44, SPHERE_WIDGET_ID)
    }

    fn make_axis(&self, camera: CameraPtr) -> (Vec3, Vec3, Vec3) {
        match self.orientation {
            RotationWidgetOrientation::Camera => {
                let right = camera.borrow().right_vec();
                let up = camera.borrow().up_vec();
                let dir = camera.borrow().direction();
                (right,
                 up,
                 dir)
            }
            RotationWidgetOrientation::Rotor(rotor) => (rotor * Vec3::unit_x(), rotor * Vec3::unit_y(), rotor * -Vec3::unit_z())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub origin: Vec3,
    pub radius: f32,
    right: Vec3,
    up: Vec3,
    color: u32,
    id: u32,
    translation: Vec3,
}

impl Circle {
    pub fn new(origin: Vec3, radius: f32, right: Vec3, up: Vec3, color: u32, id: u32) -> Self {
        Self {
            origin,
            radius,
            right,
            up,
            color,
            id,
            translation: Vec3::zero(),
        }
    }

    pub fn normal(&self) -> Vec3 {
        self.right.cross(self.up)
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.translation = translation;
    }
}

impl Drawable for Circle {
    fn vertices(&self, fake: bool) -> Vec<Vertex> {
        let mut vertices = Vec::with_capacity(2 * NB_SECTOR_CIRCLE as usize + 2);
        let color = if fake {
            self.id
        } else {
            self.color
        };
        for i in 0..=NB_SECTOR_CIRCLE {
            let theta = 2. * PI * i as f32 / NB_SECTOR_CIRCLE as f32;
            vertices.push(Vertex::new(self.translation + self.origin + self.radius * ( self.right * theta.cos() + self.up * theta.sin()), color));
            vertices.push(Vertex::new(self.translation + self.origin + self.radius * 0.9 * ( self.right * theta.cos() + self.up * theta.sin()), color));
        }
        vertices
    }

    fn indices() -> Vec<u16> {
        (0..=2*NB_SECTOR_CIRCLE + 1).collect()
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleStrip
    }

}

#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub position: Vec3,
    pub radius: f32,
    color: u32,
    id: u32,
    translation: Vec3,
}

impl Sphere {
    pub fn new(position: Vec3, radius: f32, color: u32, id: u32) -> Self {
        Self {
            position,
            radius,
            color,
            id,
            translation: Vec3::zero(),
        }
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.translation = translation;
    }
}

impl Drawable for Sphere {
    fn vertices(&self, fake: bool) -> Vec<Vertex> {
        let mut vertices = Vec::new();
        let stack_step = PI / NB_STACK_SPHERE as f32;
        let sector_step = 2. * PI / NB_SECTOR_SPHERE as f32;
        let color = if fake {
            self.id
        } else {
            self.color
        };
        for i in 0..=NB_STACK_SPHERE {
            // 0..=x means that x is included
            let stack_angle = PI / 2. - (i as f32) * stack_step;
            let xy = self.radius * stack_angle.cos();
            let z = self.radius * stack_angle.sin();

            for j in 0..=NB_SECTOR_SPHERE {
                let sector_angle = j as f32 * sector_step;

                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();

                vertices.push(Vertex::new(self.translation + self.position + Vec3::new(x, y, z), color))
            }
        }
        vertices
    }

    fn indices() -> Vec<u16> {
        let mut indices = Vec::new();

        for i in 0..NB_STACK_SPHERE {
            let mut k1: u16 = i * (NB_SECTOR_SPHERE + 1); // begining of ith stack
            let mut k2: u16 = k1 + NB_SECTOR_SPHERE + 1; // begining of (i + 1)th stack

            for _ in 0..NB_SECTOR_SPHERE {
                if i > 0 {
                    indices.push(k1);
                    indices.push(k2);
                    indices.push(k1 + 1);
                }

                if i < NB_STACK_SPHERE - 1 {
                    indices.push(k1 + 1);
                    indices.push(k2);
                    indices.push(k2 + 1);
                }
                k1 += 1;
                k2 += 1;
            }
        }
        indices
    }

    fn primitive_topology() -> wgpu::PrimitiveTopology {
        wgpu::PrimitiveTopology::TriangleList
    }

    fn use_alpha() -> bool {
        true
    }
}
