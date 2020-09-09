use crate::{camera, instance, mesh, pipeline_handler, texture};
use crate::{PhySize, WindowEvent};
use camera::{Camera, CameraController, Projection};
use cgmath::prelude::*;
use cgmath::{Quaternion, Vector3};
use iced_wgpu::wgpu;
use iced_winit::winit;
use instance::Instance;
use pipeline_handler::PipelineHandler;
use std::time::Duration;
use texture::Texture;
use wgpu::{Device, PrimitiveTopology};
use winit::dpi::PhysicalPosition;
use winit::event::*;

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
        let sphere_instances = Vec::new();
        let tube_instances = Vec::new();

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

    pub fn resize(&mut self, size: PhySize, device: &Device) {
        self.depth_texture = texture::Texture::create_depth_texture(device, &size);
        self.state.resize(size);
        self.update_camera();
    }

    pub fn fit(&mut self, position: Vector3<f32>, quaternion: Quaternion<f32>) {
        self.state.update_with_parameters(position, quaternion);
    }

    pub fn update_spheres(&mut self, positions: &Vec<([f32; 3], u32)>) {
        let instances = positions
            .iter()
            .map(|(v, color)| Instance {
                position: cgmath::Vector3::<f32> {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                },
                rotation: cgmath::Quaternion::new(1., 0., 0., 0.),
                color: Instance::color_from_u32(*color),
            })
            .collect();
        self.update.sphere_instances = Some(instances);
        self.update.need_update = true;
    }

    pub fn update_camera(&mut self) {
        self.update.need_update = true;
        self.update.camera_update = true;
    }

    pub fn update_tubes(&mut self, pairs: &Vec<([f32; 3], [f32; 3], u32)>) {
        let instances = pairs
            .iter()
            .map(|(a, b, color)| {
                let position_a = cgmath::Vector3::<f32> {
                    x: a[0],
                    y: a[1],
                    z: a[2],
                };
                let position_b = cgmath::Vector3::<f32> {
                    x: b[0],
                    y: b[1],
                    z: b[2],
                };
                create_bound(position_a, position_b, *color)
            })
            .flatten()
            .collect();
        self.update.tube_instances = Some(instances);
        self.update.need_update = true;
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self.state.input(event) {
            self.update_camera();
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
        if self.state.camera_is_moving() {
            self.update_camera();
        }
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
            self.sphere_pipeline_handler.update_viewer(
                device,
                &self.state.camera,
                &self.state.projection,
            );
            self.tube_pipeline_handler.update_viewer(
                device,
                &self.state.camera,
                &self.state.projection,
            );
            self.update.camera_update = false;
        }
        self.update.need_update = false;
    }

    pub fn get_fovy(&self) -> f32 {
        self.state.projection.get_fovy().0
    }

    pub fn get_ratio(&self) -> f32 {
        self.state.projection.get_ratio()
    }

    pub fn camera_is_moving(&self) -> bool {
        self.state.camera_is_moving()
    }
}

/// Create an instance of a cylinder going from source to dest
fn create_bound(
    source: cgmath::Vector3<f32>,
    dest: cgmath::Vector3<f32>,
    color: u32,
) -> Vec<Instance> {
    let mut ret = Vec::new();
    let color = Instance::color_from_u32(color);
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
        ret.push(Instance {
            position,
            rotation,
            color,
        });
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

/// Process the inputs on a scene and gives instruction to the camera_controller
struct State {
    camera: Camera,
    projection: Projection,
    size: PhySize,
    camera_controller: CameraController,
    last_clicked_position: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
    mouse_pressed: bool,
}

impl State {
    pub fn new(size: PhySize) -> Self {
        let camera = Camera::new((0.0, 5.0, 10.0), Quaternion::from([1., 0., 0., 0.]));
        let projection = Projection::new(size.width, size.height, cgmath::Deg(70.0), 0.1, 1000.0);
        let camera_controller = camera::CameraController::new(4.0, 0.04, &camera);
        Self {
            camera,
            projection,
            size,
            camera_controller,
            last_clicked_position: (0., 0.).into(),
            mouse_position: (0., 0.).into(),
            mouse_pressed: false,
        }
    }

    pub fn update_with_parameters(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>) {
        let position: [f32; 3] = position.into();
        self.camera = Camera::new(position, rotation);
        self.projection = Projection::new(
            self.size.width,
            self.size.height,
            cgmath::Deg(70.0),
            0.1,
            1000.0,
        );
        self.camera_controller = camera::CameraController::new(4.0, 0.04, &self.camera);
    }

    pub fn resize(&mut self, new_size: PhySize) {
        self.projection.resize(new_size.width, new_size.height);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.camera_controller.process_click(&self.camera, state);
                self.mouse_pressed = *state == ElementState::Pressed;
                if *state == ElementState::Pressed {
                    self.last_clicked_position = self.mouse_position;
                }
                self.mouse_pressed
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = *position;
                if self.mouse_pressed {
                    let mouse_dx =
                        (position.x - self.last_clicked_position.x) / self.size.width as f64;
                    let mouse_dy =
                        (position.y - self.last_clicked_position.y) / self.size.height as f64;
                    self.camera_controller.process_mouse(mouse_dx, mouse_dy);
                }
                self.mouse_pressed
            }
            _ => false,
        }
    }

    pub fn camera_is_moving(&self) -> bool {
        self.camera_controller.is_moving()
    }

    fn update_camera(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
    }
}
