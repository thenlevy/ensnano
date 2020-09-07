use std::time::Duration;
use crate::consts::*;
use crate::{camera, instance, mesh, pipeline_handler, texture};
use crate::{PhySize, WindowEvent};
use camera::{Camera, CameraController, Projection};
use cgmath::prelude::*;
use iced_wgpu::wgpu;
use winit::event::*;
use instance::Instance;
use pipeline_handler::PipelineHandler;
use texture::Texture;
use wgpu::{Device, PrimitiveTopology};
use iced_winit::winit;
use winit::dpi::{PhysicalPosition};

pub struct Scene {
    state: State,
    sphere_pipeline_handler: PipelineHandler,
    tube_pipeline_handler: PipelineHandler,
    /// the number of tube to display
    pub number_instances: u32,
    depth_texture: Texture,
    update: SceneUpdate,
}

impl Scene {
    /// Create a new scene that will be displayed on `device`
    pub fn new(device: &Device, size: PhySize) -> Self {
        let state = State::new(size);

        let number_instances = 3;
        let (sphere_instances, tube_instances) = create_instances(3);

        let sphere_mesh = mesh::Mesh::sphere(device);
        let tube_mesh = mesh::Mesh::tube(device);

        let depth_texture = texture::Texture::create_depth_texture(device, &size);

        let sphere_pipeline_handler = PipelineHandler::new(
            device,
            sphere_mesh,
            sphere_instances,
            &state.camera,
            &state.projection,
            PrimitiveTopology::TriangleList,
        );
        let tube_pipeline_handler = PipelineHandler::new(
            device,
            tube_mesh,
            tube_instances,
            &state.camera,
            &state.projection,
            PrimitiveTopology::TriangleStrip,
        );

        let update = SceneUpdate::new();

        Self {
            number_instances,
            state,
            depth_texture,
            update,
            sphere_pipeline_handler,
            tube_pipeline_handler,
        }
    }

    pub fn update(&mut self) {
        let (sphere_instances, tube_instances) = create_instances(self.number_instances);
        self.update.sphere_instances = Some(sphere_instances);
        self.update.tube_instances = Some(tube_instances);
        self.update.need_update = true;
    }

    pub fn resize(&mut self, size: PhySize, device: &Device) {
        self.depth_texture = texture::Texture::create_depth_texture(device, &size);
        self.state.resize(size);
    }

    pub fn update_spheres(&mut self, positions: &Vec<[f32; 3]>) {
        let instances = positions
            .iter()
            .map(|v| Instance {
                position: cgmath::Vector3::<f32> { x: v[0], y: v[1], z: v[2] },
                rotation: cgmath::Quaternion::new(1., 0., 0., 0.),
            })
            .collect();
        self.update.sphere_instances = Some(instances);
        self.update.need_update = true;
    }

    pub fn update_tubes(&mut self, pairs: &Vec<([f32; 3], [f32; 3])>) {
        let instances = pairs
            .iter()
            .map(|(a, b)| {
                let position_a = cgmath::Vector3::<f32> { x: a[0], y: a[1], z: a[2] };
                let position_b = cgmath::Vector3::<f32> { x: b[0], y: b[1], z: b[2] };
                create_bound(position_a, position_b)
            })
            .flatten()
            .collect();
        self.update.tube_instances = Some(instances);
        self.update.need_update = true;
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self.state.input(event) {
            self.update.camera_update = true;
            self.update.need_update = true;
            true
        } else {
            false
        }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        device: &Device,
        dt: Duration,
    ) {
        if self.update.need_update {
            self.perform_update(device, dt);
        }
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: target,
                resolve_target: None,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &self.depth_texture.view,
                depth_load_op: wgpu::LoadOp::Clear,
                depth_store_op: wgpu::StoreOp::Store,
                clear_depth: 1.0,
                stencil_load_op: wgpu::LoadOp::Clear,
                stencil_store_op: wgpu::StoreOp::Store,
                clear_stencil: 0,
            }),
        });

        self.sphere_pipeline_handler.draw(device, &mut render_pass);
        self.tube_pipeline_handler.draw(device, &mut render_pass);
    }

    fn perform_update(&mut self, device: &Device, dt: Duration) {
        if let Some(tube_instances) = self.update.tube_instances.take() {
            self.tube_pipeline_handler
                .update_instances(device, tube_instances);
        }
        if let Some(sphere_instances) = self.update.sphere_instances.take() {
            self.sphere_pipeline_handler
                .update_instances(device, sphere_instances);
        }
        if self.update.camera_update {
            self.state.update_camera(dt);
            self.sphere_pipeline_handler
                .update_viewer(device, &self.state.camera, &self.state.projection);
            self.tube_pipeline_handler
                .update_viewer(device, &self.state.camera, &self.state.projection);
            self.update.camera_update = false;
        }
        self.update.need_update = false;
    }
}

fn create_instances(number_layers: u32) -> (Vec<Instance>, Vec<Instance>) {
    let mut spheres = Vec::new();
    let mut cylinders = Vec::new();
    for layer in 0isize..(number_layers as isize) {
        let y_layer = BOUND_LENGTH * layer as f32;
        for j in 0isize..layer {
            let x = -layer + 2 * j;
            for k in 0..layer {
                let z = -layer + 2 * k;
                let position =
                    cgmath::Vector3::new(x as f32 * BOUND_LENGTH, y_layer, z as f32 * BOUND_LENGTH);
                let rotation = cgmath::Quaternion::new(1., 0., 0., 0.);
                spheres.push(Instance { position, rotation });
            }
        }
        if layer >= 2 {
            let source = spheres[0].position;
            for i in 1..=4 {
                let dest = spheres[i].position;
                for cylinder in create_bound(source, dest) {
                    cylinders.push(cylinder);
                }
            }
        }
    }
    (spheres, cylinders)
}

/// Create an instance of a cylinder going from source to dest
fn create_bound(source: cgmath::Vector3<f32>, dest: cgmath::Vector3<f32>) -> Vec<Instance> {
    let mut ret = Vec::new();
    let rotation = cgmath::Quaternion::between_vectors(
        cgmath::Vector3::new(1., 0., 0.),
        (dest - source).normalize(),
    );

    let obj = (dest - source).magnitude();
    let mut current_source = source.clone();
    let mut current_length = 0.;
    let one_step_len = crate::consts::BOUND_LENGTH;
    let step_dir = (dest - source).normalize();
    let one_step = step_dir * one_step_len;
    while current_length < obj {
        let position = if current_length + one_step_len > obj {
            current_source + step_dir * (obj - current_length) / 2.
        } else {
            current_source + one_step / 2.
        };
        current_source = position + one_step / 2.;
        current_length = (source - current_source).magnitude();
        ret.push(Instance {position, rotation});
    }
    ret
}

/// A structure that stores the element that needs to be updated in a scene
pub struct SceneUpdate {
    pub tube_instances: Option<Vec<Instance>>,
    pub sphere_instances: Option<Vec<Instance>>,
    pub need_update: bool,
    pub camera_update: bool,
}

impl SceneUpdate {
    pub fn new() -> Self {
        Self {
            tube_instances: None,
            sphere_instances: None,
            need_update: false,
            camera_update: false,
        }
    }
}

struct State {
    camera: Camera,
    projection: Projection,
    camera_controller: CameraController,
    last_mouse_position: PhysicalPosition<f64>,
    mouse_pressed: bool,
}

impl State {
    pub fn new(size: PhySize) -> Self {
        let camera = Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = Projection::new(size.width, size.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 0.04);
        Self {
            camera,
            projection,
            camera_controller,
            last_mouse_position: (0., 0.).into(),
            mouse_pressed: false
        }
    }

    pub fn resize(&mut self, new_size: PhySize) {
        self.projection.resize(new_size.width, new_size.height);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    virtual_keycode: Some(key),
                    state,
                    ..
                },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel {
                delta,
                ..
            } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved {
                position,
                ..
            } => {
                let mouse_dx = position.x - self.last_mouse_position.x;
                let mouse_dy = position.y - self.last_mouse_position.y;
                self.last_mouse_position = *position;
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(mouse_dx, mouse_dy);
                }
                self.mouse_pressed
            }
            _ => false,
        }
    }

    fn update_camera(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
    }

}
