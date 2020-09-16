use crate::{instance, utils, design};
use crate::{PhySize, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit;
use instance::Instance;
use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;
use futures::executor;
use utils::BufferDimensions;
use ultraviolet::{Vec3, Rotor3};
mod camera;
mod view;
use view::{View, ViewUpdate};
mod controller;
use controller::{ Controller, Consequence };
use design::Design;

type ViewPtr = Rc<RefCell<View>>;
pub struct Scene {
    designs: Vec<Design>,
    update: SceneUpdate,
    selected_id: Option<u32>,
    view: ViewPtr,
    controller: Controller,
}


impl Scene {
    /// Create a new scene that will be displayed on `device`
    pub fn new(device: &Device, size: PhySize) -> Self {
        let update = SceneUpdate::new();

        let view = Rc::new(RefCell::new(View::new(size, device)));
        let controller = Controller::new(view.clone(), size);
        Self {
            view,
            designs: Vec::new(),
            update,
            selected_id: None,
            controller,
        }
    }

    /// Input an event to the scene. Return true, if the selected object of the scene has changed
    pub fn input(&mut self, event: &WindowEvent, device: &Device, queue: &mut wgpu::Queue) -> bool {
        let mut clicked_pixel = None;
        let consequence = self.controller.input(event);
        match consequence {
            Consequence::Nothing => (),
            Consequence::CameraMoved => self.notify(SceneNotification::CameraMoved),
            Consequence::PixelSelected(clicked) => clicked_pixel = Some(clicked),
        };
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
            width: self.controller.get_window_size().width,
            height: self.controller.get_window_size().height,
            depth: 1,
        };
        let desc = wgpu::TextureDescriptor {
            size,
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

        self.view.borrow_mut().draw(
            &mut encoder,
            &texture_view,
            device,
            true,
            queue,
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
                println!("{}, {}, {}, {}", a, r, g, b);
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
        let bound = create_bound(source.into(), dest.into(), 0, 0);
        self.update.selected_tube = Some(bound);
        self.update.need_update = true;
    }

    pub fn update_selected_sphere(&mut self, position: [f32; 3]) {
        let instance = Instance {
            position: position.into(),
            rotor: Rotor3::identity(),
            color: Instance::color_from_u32(0),
            id: 0,
        };
        self.update.selected_sphere = Some(instance);
        self.update.need_update = true;
    }

    pub fn draw_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        device: &Device,
        dt: Duration,
        fake_color: bool,
        queue: &Queue,
    ) {
        if self.controller.camera_is_moving() {
            self.notify(SceneNotification::CameraMoved);
        }
        if self.update.need_update {
            self.perform_update(dt);
        }
        self.view.borrow_mut().draw(encoder, target, device, fake_color, queue);
    }

    fn perform_update(&mut self, dt: Duration) {
        if let Some(instance) = self.update.sphere_instances.take() {
            self.view.borrow_mut().update(ViewUpdate::Spheres(instance))
        }
        if let Some(instance) = self.update.tube_instances.take() {
            self.view.borrow_mut().update(ViewUpdate::Tubes(instance))
        }
        if let Some(sphere) = self.update.selected_sphere.take() {
            self.view.borrow_mut().update(ViewUpdate::SelectedSpheres(vec![sphere]))
        }
        if let Some(tubes) = self.update.selected_tube.take() {
            self.view.borrow_mut().update(ViewUpdate::SelectedTubes(tubes))
        }

        if self.update.camera_update {
            self.controller.update_camera(dt);
            self.view.borrow_mut().update(ViewUpdate::Camera);
            self.update.camera_update = false;
        }
        self.update.need_update = false;
    }

    pub fn get_fovy(&self) -> f32 {
        self.view.borrow().get_projection().borrow().get_fovy()
    }

    pub fn get_ratio(&self) -> f32 {
        self.view.borrow().get_projection().borrow().get_ratio()
    }

}

/// Create an instance of a cylinder going from source to dest
fn create_bound(
    source: Vec3,
    dest: Vec3,
    color: u32,
    id: u32,
) -> Vec<Instance> {
    let mut ret = Vec::new();
    let color = Instance::color_from_u32(color);
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());

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
            id,
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

pub enum SceneNotification<'a> {
    CameraMoved,
    NewCamera(Vec3, Rotor3),
    NewSpheres(&'a Vec<([f32 ; 3], u32, u32)>),
    NewTubes(&'a Vec<([f32 ;3], [f32 ; 3], u32, u32)>),
    Resize(PhySize),
}

impl Scene {
    pub fn notify(&mut self, notification: SceneNotification) {
        match notification {
            SceneNotification::NewSpheres(instances) => self.new_spheres(instances),
            SceneNotification::NewTubes(instances) => self.new_tubes(instances),
            SceneNotification::NewCamera(position, projection) => {
                self.controller.teleport_camera(position, projection);
                self.update.camera_update = true;
            }
            SceneNotification::CameraMoved => self.update.camera_update = true,
            SceneNotification::Resize(size) => self.resize(size),
        };
        self.update.need_update = true;

    }

    fn new_spheres(&mut self, positions: &Vec<([f32; 3], u32, u32)>) {
        let instances = positions
            .iter()
            .map(|(v, color, id)| Instance {
                position: Vec3 {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                },
               rotor: Rotor3::identity(),
               color: Instance::color_from_u32(*color),
               id: *id,
            })
            .collect();
        let fake_instances = positions
            .iter()
            .map(|(v, _, fake_color)| Instance {
                position: (*v).into(),
                rotor: Rotor3::identity(),
                color: Instance::color_from_u32(*fake_color),
                id: *fake_color,
            })
            .collect();
        self.update.sphere_instances = Some(instances);
        self.update.fake_sphere_instances = Some(fake_instances);
    }

    fn new_tubes(&mut self, pairs: &Vec<([f32; 3], [f32; 3], u32, u32)>) {
        let instances = pairs
            .iter()
            .map(|(a, b, color, id)| {
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
                create_bound(position_a, position_b, *color, *id)
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
                create_bound(position_a, position_b, *fake_color, *fake_color)
            })
            .flatten()
            .collect();
        self.update.tube_instances = Some(instances);
        self.update.fake_tube_instances = Some(fake_instances);
    }

    fn resize(&mut self, size: PhySize) {
        self.view.borrow_mut().update(ViewUpdate::Size(size));
        self.controller.resize(size);
        self.update.camera_update = true;
    }
}
