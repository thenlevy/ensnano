use crate::{camera, instance, mesh, pipeline_handler, texture, utils, design};
use crate::{PhySize, WindowEvent};
use camera::{Camera, CameraController, Projection};
use iced_wgpu::wgpu;
use iced_winit::winit;
use instance::Instance;
use pipeline_handler::PipelineHandler;
use std::time::Duration;
use texture::Texture;
use wgpu::{Device, PrimitiveTopology};
use winit::dpi::PhysicalPosition;
use winit::event::*;
use futures::executor;
use utils::{BufferDimensions};
use ultraviolet::{Vec3, Rotor3};
use design::Design;

pub struct Scene {
    state: State,
    designs: Vec<Design>,
    pipeline_handlers: PipelineHandlers,
    /// the number of tube to display
    depth_texture: Texture,
    update: SceneUpdate,
    selected_id: Option<u32>,
}

impl Scene {
    /// Create a new scene that will be displayed on `device`
    pub fn new(device: &Device, size: PhySize) -> Self {
        let state = State::new(size);

        let number_instances = 3;

        let depth_texture = texture::Texture::create_depth_texture(device, &size);
        let pipeline_handlers = PipelineHandlers::init(device, &state.camera, &state.projection);

        let update = SceneUpdate::new();

        Self {
            designs: Vec::new(),
            state,
            depth_texture,
            update,
            pipeline_handlers,
            selected_id: None,
        }
    }

    /// Input an event to the scene. Return true, if the selected object of the scene has changed
    pub fn input(&mut self, event: &WindowEvent, device: &Device, queue: &mut wgpu::Queue) -> bool {
        let mut clicked_pixel: Option<PhysicalPosition<f64>> = None;
        if self.state.input(event, &mut clicked_pixel) {
            self.notify(SceneNotification::CameraMoved);
        }
        if clicked_pixel.is_some() {
            let clicked_pixel = clicked_pixel.unwrap();
            let selected_id = self.set_selected_id(clicked_pixel, device, queue);
            if selected_id != 0xFFFFFF {
                self.selected_id = Some(selected_id);
            } else {
                self.selected_id = None;
            }
            true
        } else {
            false
        }
    }

    fn set_selected_id(
        &mut self,
        clicked_pixel: PhysicalPosition<f64>,
        device: &Device,
        queue: &mut wgpu::Queue,
    ) -> u32 {
        let size = wgpu::Extent3d {
            width: self.state.size.width,
            height: self.state.size.height,
            depth: 1,
        };
        let desc = wgpu::TextureDescriptor {
            size,
            //array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_SRC,
            label: Some("desc"),
        };
        let texture_view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("texture_view_descriptor"),
            format: Some(wgpu::TextureFormat::Bgra8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let texture = device.create_texture(&desc);
        let texture_view = texture.create_view(&texture_view_descriptor);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.draw(
            &mut encoder,
            &texture_view,
            device,
            std::time::Duration::from_millis(0),
            true,
        );

        let buffer_dimensions = BufferDimensions::new(size.width as usize, size.height as usize);
        let buf_size = buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height;
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: buf_size as u64,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
            label: Some("staging_buffer"),
        });

        let buffer_copy_view = wgpu::BufferCopyView {
            buffer: &staging_buffer,
            layout: wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: buffer_dimensions.padded_bytes_per_row as u32,
                rows_per_image: 0,
            }
        };
        let texture_copy_view = wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        };
        encoder.copy_texture_to_buffer(texture_copy_view, buffer_copy_view, size);
        queue.submit(Some(encoder.finish()));

        let pixel = (clicked_pixel.y as u32 * size.width+ clicked_pixel.x as u32)
            * std::mem::size_of::<u32>() as u32;
        let pixel = pixel as usize;

        let buffer_slice = staging_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);
        let color = async {
            if let Ok(()) = buffer_future.await {
                let data = buffer_slice.get_mapped_range();
                let pixels:Vec<u8> = data.chunks_exact(buffer_dimensions.padded_bytes_per_row)
                    .flat_map(|chunk| chunk[..buffer_dimensions.unpadded_bytes_per_row].to_vec())
                    .collect();

                let a = (pixels[pixel + 3] as u32) << 24;
                let r = (pixels[pixel + 2] as u32) << 16;
                let g = (pixels[pixel + 1] as u32) << 8;
                let b = pixels[pixel] as u32;
                let color = a + r + g + b;
                drop(data);
                staging_buffer.unmap();
                //color
                color
            } else {
                panic!("could not read fake texture");
            }
        };
        let color = executor::block_on(color);
        color & 0x00FFFFFF
    }

    pub fn get_selected_id(&self) -> Option<u32> {
        self.selected_id
    }

    pub fn update_selected_tube(&mut self, source: [f32; 3], dest: [f32; 3]) {
        let bound = create_bound(source.into(), dest.into(), 0);
        self.update.selected_tube = Some(bound);
        self.update.need_update = true;
    }

    pub fn update_selected_sphere(&mut self, position: [f32; 3]) {
        let instance = Instance {
            position: position.into(),
            rotor: Rotor3::identity(),
            color: Instance::color_from_u32(0),
        };
        self.update.selected_sphere = Some(instance);
        self.update.need_update = true;
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        device: &Device,
        dt: Duration,
        fake_color: bool,
    ) {
        if self.state.camera_is_moving() {
            self.notify(SceneNotification::CameraMoved);
        }
        if self.update.need_update {
            self.perform_update(device, dt);
        }
        let clear_color = if fake_color {
            wgpu::Color {
                r: 1.,
                g: 1.,
                b: 1.,
                a: 1.,
            }
        } else {
            wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.,
            }
        };
        let mut handlers = if fake_color {
            self.pipeline_handlers.fake()
        } else {
            self.pipeline_handlers.real()
        };
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: target,
                resolve_target: None,
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
        for pipeline_handler in handlers.iter_mut() {
            pipeline_handler.draw(device, &mut render_pass);
        }
    }

    fn perform_update(&mut self, device: &Device, dt: Duration) {
        self.pipeline_handlers.update(&mut self.update, device);
        if self.update.camera_update {
            self.state.update_camera(dt);
            for handler in self.pipeline_handlers.all().iter_mut() {
                handler.update_viewer(device, &self.state.camera, &self.state.projection);
            }
            self.update.camera_update = false;
        }
        self.update.need_update = false;
    }

    pub fn get_fovy(&self) -> f32 {
        self.state.projection.get_fovy()
    }

    pub fn get_ratio(&self) -> f32 {
        self.state.projection.get_ratio()
    }

}

/// Create an instance of a cylinder going from source to dest
fn create_bound(
    source: Vec3,
    dest: Vec3,
    color: u32,
) -> Vec<Instance> {
    let mut ret = Vec::new();
    let color = Instance::color_from_u32(color);
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());
    //let rotation = cgmath::Quaternion::between_vectors(
        //cgmath::Vector3::new(1., 0., 0.),
        //(dest - source).normalize(),
    //);

    let obj = (dest - source).mag();
    let mut current_source = source.clone();
    let mut current_length = 0.;
    let one_step_len = crate::consts::BOUND_LENGTH;
    let step_dir = (dest - source).normalized();
    let one_step = step_dir * one_step_len;
    while current_length < obj {
        let position = if current_length + one_step_len > obj {
            current_source + step_dir * (obj - current_length) / 2.
        } else {
            current_source + one_step / 2.
        };
        current_source = position + one_step / 2.;
        current_length = (source - current_source).mag();
        ret.push(Instance {
            position,
            rotor,
            color,
        });
    }
    ret
}

/// A structure that stores the element that needs to be updated in a scene
pub struct SceneUpdate {
    pub tube_instances: Option<Vec<Instance>>,
    pub sphere_instances: Option<Vec<Instance>>,
    pub fake_tube_instances: Option<Vec<Instance>>,
    pub fake_sphere_instances: Option<Vec<Instance>>,
    pub selected_tube: Option<Vec<Instance>>,
    pub selected_sphere: Option<Instance>,
    pub need_update: bool,
    pub camera_update: bool,
}

impl SceneUpdate {
    pub fn new() -> Self {
        Self {
            tube_instances: None,
            sphere_instances: None,
            fake_tube_instances: None,
            fake_sphere_instances: None,
            selected_tube: None,
            selected_sphere: None,
            need_update: false,
            camera_update: false,
        }
    }
}

struct PipelineHandlers {
    sphere: PipelineHandler,
    tube: PipelineHandler,
    fake_tube: PipelineHandler,
    fake_sphere: PipelineHandler,
    selected_sphere: PipelineHandler,
    selected_tube: PipelineHandler,
}

impl PipelineHandlers {
    fn init(device: &Device, camera: &Camera, projection: &Projection) -> Self {
        let sphere_mesh = mesh::Mesh::sphere(device, false);
        let tube_mesh = mesh::Mesh::tube(device, false);
        let fake_sphere_mesh = mesh::Mesh::sphere(device, false);
        let fake_tube_mesh = mesh::Mesh::tube(device, false);
        let selected_sphere_mesh = mesh::Mesh::sphere(device, true);
        let selected_tube_mesh = mesh::Mesh::tube(device, true);

        let sphere_pipeline_handler = PipelineHandler::new(
            device,
            sphere_mesh,
            Vec::new(),
            camera,
            projection,
            PrimitiveTopology::TriangleList,
            pipeline_handler::Flavour::Real,
        );
        let tube_pipeline_handler = PipelineHandler::new(
            device,
            tube_mesh,
            Vec::new(),
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Real,
        );
        let fake_tube_pipeline_handler = PipelineHandler::new(
            device,
            fake_tube_mesh,
            Vec::new(),
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let fake_sphere_pipeline_handler = PipelineHandler::new(
            device,
            fake_sphere_mesh,
            Vec::new(),
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let selected_sphere_pipeline_handler = PipelineHandler::new(
            device,
            selected_sphere_mesh,
            Vec::new(),
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Selected,
        );
        let selected_tube_pipeline_handler = PipelineHandler::new(
            device,
            selected_tube_mesh,
            Vec::new(),
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Selected,
        );

        Self {
            sphere: sphere_pipeline_handler,
            tube: tube_pipeline_handler,
            fake_sphere: fake_sphere_pipeline_handler,
            fake_tube: fake_tube_pipeline_handler,
            selected_sphere: selected_sphere_pipeline_handler,
            selected_tube: selected_tube_pipeline_handler,
        }
    }

    fn all(&mut self) -> Vec<&mut PipelineHandler> {
        vec![
            &mut self.sphere,
            &mut self.tube,
            &mut self.fake_sphere,
            &mut self.fake_tube,
            &mut self.selected_tube,
            &mut self.selected_sphere,
        ]
    }

    fn real(&mut self) -> Vec<&mut PipelineHandler> {
        vec![
            &mut self.sphere,
            &mut self.tube,
            &mut self.selected_sphere,
            &mut self.selected_tube,
        ]
    }

    fn fake(&mut self) -> Vec<&mut PipelineHandler> {
        vec![&mut self.fake_sphere, &mut self.fake_tube]
    }

    fn update(&mut self, update: &mut SceneUpdate, device: &Device) {
        if let Some(instances) = update.tube_instances.take() {
            self.tube.update_instances(device, instances);
        }
        if let Some(instances) = update.sphere_instances.take() {
            self.sphere.update_instances(device, instances);
        }
        if let Some(instances) = update.fake_sphere_instances.take() {
            self.fake_sphere.update_instances(device, instances);
        }
        if let Some(instances) = update.fake_tube_instances.take() {
            self.fake_tube.update_instances(device, instances);
        }
        if update.selected_tube.is_some() || update.selected_sphere.is_some() {
            self.selected_sphere.update_instances(device, Vec::new());
            self.selected_tube.update_instances(device, Vec::new());
        }
        if let Some(instances) = update.selected_tube.take() {
            self.selected_tube.update_instances(device, instances);
        }
        if let Some(instances) = update.selected_sphere.take() {
            self.selected_sphere
                .update_instances(device, vec![instances]);
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
        let camera = Camera::new((0.0, 5.0, 10.0), Rotor3::identity());
        let projection = Projection::new(size.width, size.height, 70f32.to_radians(), 0.1, 1000.0);
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

    pub fn update_with_parameters(&mut self, position: Vec3, rotation: Rotor3) {
        let position: [f32; 3] = position.into();
        self.camera = Camera::new(position, rotation);
        self.projection = Projection::new(
            self.size.width,
            self.size.height,
            70f32.to_radians(),
            0.1,
            1000.0,
        );
        self.camera_controller = camera::CameraController::new(4.0, 0.04, &self.camera);
    }

    pub fn resize(&mut self, new_size: PhySize) {
        self.projection.resize(new_size.width, new_size.height);
        self.size = new_size;
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        clicked_pixel: &mut Option<PhysicalPosition<f64>>,
    ) -> bool {
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
                } else if position_difference(self.last_clicked_position, self.mouse_position) < 5.
                {
                    *clicked_pixel = Some(self.last_clicked_position);
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

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

pub enum SceneNotification<'a> {
    CameraMoved,
    NewCamera(Vec3, Rotor3),
    NewSpheres(&'a Vec<([f32 ; 3], u32, u32)>),
    NewTubes(&'a Vec<([f32 ;3], [f32 ; 3], u32, u32)>),
    Resize(PhySize, &'a Device),
}

impl Scene {
    pub fn notify(&mut self, notification: SceneNotification) {
        match notification {
            SceneNotification::NewSpheres(instances) => self.new_spheres(instances),
            SceneNotification::NewTubes(instances) => self.new_tubes(instances),
            SceneNotification::NewCamera(position, projection) => {
                self.state.update_with_parameters(position, projection);
                self.update.camera_update = true;
            }
            SceneNotification::CameraMoved => self.update.camera_update = true,
            SceneNotification::Resize(size, device) => self.resize(size, device),
        };
        self.update.need_update = true;

    }

    fn new_spheres(&mut self, positions: &Vec<([f32; 3], u32, u32)>) {
        let instances = positions
            .iter()
            .map(|(v, color, _)| Instance {
                position: Vec3 {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                },
               rotor: Rotor3::identity(),
               color: Instance::color_from_u32(*color),
            })
            .collect();
        let fake_instances = positions
            .iter()
            .map(|(v, _, fake_color)| Instance {
                position: (*v).into(),
                rotor: Rotor3::identity(),
                color: Instance::color_from_u32(*fake_color),
            })
            .collect();
        self.update.sphere_instances = Some(instances);
        self.update.fake_sphere_instances = Some(fake_instances);
    }

    fn new_tubes(&mut self, pairs: &Vec<([f32; 3], [f32; 3], u32, u32)>) {
        let instances = pairs
            .iter()
            .map(|(a, b, color, _)| {
                let position_a = Vec3 {
                    x: a[0],
                    y: a[1],
                    z: a[2],
                };
                let position_b = Vec3 {
                    x: b[0],
                    y: b[1],
                    z: b[2],
                };
                create_bound(position_a, position_b, *color)
            })
            .flatten()
            .collect();
        let fake_instances = pairs
            .iter()
            .map(|(a, b, _, fake_color)| {
                let position_a = Vec3 {
                    x: a[0],
                    y: a[1],
                    z: a[2],
                };
                let position_b = Vec3 {
                    x: b[0],
                    y: b[1],
                    z: b[2],
                };
                create_bound(position_a, position_b, *fake_color)
            })
            .flatten()
            .collect();
        self.update.tube_instances = Some(instances);
        self.update.fake_tube_instances = Some(fake_instances);
    }

    fn resize(&mut self, size: PhySize, device: &Device) {
        self.depth_texture = texture::Texture::create_depth_texture(device, &size);
        self.state.resize(size);
        self.update.camera_update = true;
    }
}
