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
use ultraviolet::{Mat4, Vec3, Rotor3};
mod camera;
mod view;
use view::{View, ViewUpdate};
mod controller;
use controller::{ Controller, Consequence };
use design::Design;
use std::path::PathBuf;

type ViewPtr = Rc<RefCell<View>>;
pub struct Scene {
    designs: Vec<Design>,
    update: SceneUpdate,
    selected_id: Option<u32>,
    selected_design: Option<u32>,
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
            selected_design: None,
            controller,
        }
    }

    pub fn add_design(&mut self, path: &PathBuf) {
        self.designs.push(Design::new_with_path(path, self.designs.len() as u32))
    }

    pub fn clear_design(&mut self, path: &PathBuf) {
        self.designs = vec![Design::new_with_path(path, 0)]
    }

    /// Input an event to the scene. Return true, if the selected object of the scene has changed
    pub fn input(&mut self, event: &WindowEvent, device: &Device, queue: &mut wgpu::Queue) {
        let mut clicked_pixel = None;
        let consequence = self.controller.input(event);
        match consequence {
            Consequence::Nothing => (),
            Consequence::CameraMoved => self.notify(SceneNotification::CameraMoved),
            Consequence::PixelSelected(clicked) => clicked_pixel = Some(clicked),
        };
        if clicked_pixel.is_some() {
            let clicked_pixel = clicked_pixel.unwrap();
            let (selected_id, design_id) = self.set_selected_id(clicked_pixel, device, queue);
            println!("selected {}, design{}", selected_id, design_id);
            if selected_id != 0xFFFFFF {
                self.selected_id = Some(selected_id);
                self.selected_design = Some(design_id);
                for i in 0..self.designs.len() {
                    let arg = if i == design_id as usize { Some(selected_id) } else { None };
                    self.designs[i].update_selection(arg);
                }
            } else {
                self.selected_id = None;
                self.selected_design = None;
            }
        }
    }

    fn set_selected_id(
        &mut self,
        clicked_pixel: PhysicalPosition<f64>,
        device: &Device,
        queue: &mut wgpu::Queue,
    ) -> (u32, u32) {
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

        let pixel = clicked_pixel.y as usize * buffer_dimensions.padded_bytes_per_row
            + clicked_pixel.x as usize * std::mem::size_of::<u32>();

        let buffer_slice = staging_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);

        let future_color = async {
            if let Ok(()) = buffer_future.await {
                let pixels = buffer_slice.get_mapped_range();
                let a = pixels[pixel + 3] as u32;
                let r = (pixels[pixel + 2] as u32) << 16;
                let g = (pixels[pixel + 1] as u32) << 8;
                let b = pixels[pixel] as u32;
                let color = r + g + b;
                drop(pixels);
                staging_buffer.unmap();
                println!("a {} r {} g {} b{}", a, r, g, b);
                (color, a)
            } else {
                panic!("could not read fake texture");
            }
        };
        executor::block_on(future_color)
    }

    pub fn fit_design(&mut self) {
        if self.designs.len() > 0 {
            let (position, rotor) = self.designs[0].fit(self.get_fovy(), self.get_ratio());
            self.controller.set_middle_point(self.designs[0].middle_point());
            self.notify(SceneNotification::NewCamera(position, rotor));
        }
    }

    pub fn get_selected_id(&self) -> Option<u32> {
        self.selected_id
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
        self.fetch_data_updates();
        self.fetch_view_updates();
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
        if let Some(matrices) = self.update.model_matrices.take() {
            self.view.borrow_mut().update(ViewUpdate::ModelMatricies(matrices))
        }

        if self.update.camera_update {
            self.controller.update_camera(dt);
            self.view.borrow_mut().update(ViewUpdate::Camera);
            self.update.camera_update = false;
        }
        self.update.need_update = false;
    }

    fn fetch_data_updates(&mut self) {
        let need_update = self.designs.iter_mut().fold(false, |acc, design| acc | design.data_was_updated());

        if need_update {
            let mut sphere_instances = vec![];
            let mut tube_instances = vec![];
            let mut selected_sphere_instances = vec![];
            let mut selected_tube_instances = vec![];
            for d in self.designs.iter() {
                for s in d.spheres().iter() {
                    sphere_instances.push(*s);
                }
                for t in d.tubes().iter() {
                    tube_instances.push(*t);
                }
                for s in d.selected_spheres().iter() {
                    selected_sphere_instances.push(*s);
                }
                for t in d.selected_tubes().iter() {
                    selected_tube_instances.push(*t);
                }
            }
            self.update.sphere_instances = Some(sphere_instances);
            self.update.tube_instances = Some(tube_instances);
            self.update.selected_tube = if selected_tube_instances.len() > 0 {
                Some(selected_tube_instances)
            } else {
                None
            };
            self.update.selected_sphere = if selected_sphere_instances.len() > 0 {
                Some(selected_sphere_instances[0])
            } else {
                None
            };
        }
        self.update.need_update |= need_update;
    }

    fn fetch_view_updates(&mut self) {
        let need_update = self.designs.iter_mut().fold(false, |acc, design| acc | design.view_was_updated());

        if need_update {
            let matrices: Vec<_> = self.designs.iter().map(|d| d.model_matrix()).collect();
            println!("{:?}", matrices);
            self.update.model_matrices = Some(matrices);
        }
        self.update.need_update |= need_update;

    }

    pub fn get_fovy(&self) -> f32 {
        self.view.borrow().get_projection().borrow().get_fovy()
    }

    pub fn get_ratio(&self) -> f32 {
        self.view.borrow().get_projection().borrow().get_ratio()
    }

}

/// A structure that stores the element that needs to be updated in a scene
pub struct SceneUpdate {
    pub tube_instances: Option<Vec<Instance>>,
    pub sphere_instances: Option<Vec<Instance>>,
    pub fake_tube_instances: Option<Vec<Instance>>,
    pub fake_sphere_instances: Option<Vec<Instance>>,
    pub selected_tube: Option<Vec<Instance>>,
    pub selected_sphere: Option<Instance>,
    pub model_matrices: Option<Vec<Mat4>>,
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
            model_matrices: None,
        }
    }
}

pub enum SceneNotification {
    CameraMoved,
    NewCamera(Vec3, Rotor3),
    Resize(PhySize),
}

impl Scene {
    pub fn notify(&mut self, notification: SceneNotification) {
        match notification {
            SceneNotification::NewCamera(position, projection) => {
                self.controller.teleport_camera(position, projection);
                self.update.camera_update = true;
            }
            SceneNotification::CameraMoved => self.update.camera_update = true,
            SceneNotification::Resize(size) => self.resize(size),
        };
        self.update.need_update = true;

    }

    fn resize(&mut self, size: PhySize) {
        self.view.borrow_mut().update(ViewUpdate::Size(size));
        self.controller.resize(size);
        self.update.camera_update = true;
    }
}
