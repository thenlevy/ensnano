use super::camera;
use crate::{instance, mesh, texture};
use crate::{DrawArea, PhySize};
use camera::{Camera, CameraPtr, Projection, ProjectionPtr};
use iced_wgpu::wgpu;
use instance::Instance;
use std::cell::RefCell;
use std::rc::Rc;
use texture::Texture;
use ultraviolet::{Mat4, Rotor3};
use wgpu::{Device, PrimitiveTopology, Queue};

mod pipeline_handler;
use pipeline_handler::PipelineHandler;
mod uniforms;
use uniforms::Uniforms;

pub struct View {
    camera: CameraPtr,
    projection: ProjectionPtr,
    pipeline_handlers: PipelineHandlers,
    depth_texture: Texture,
    new_size: Option<PhySize>,
}

impl View {
    pub fn new(window_size: PhySize, area_size: PhySize, device: &Device) -> Self {
        let camera = Rc::new(RefCell::new(Camera::new(
            (0.0, 5.0, 10.0),
            Rotor3::identity(),
        )));
        let projection = Rc::new(RefCell::new(Projection::new(
            area_size.width,
            area_size.height,
            70f32.to_radians(),
            0.1,
            1000.0,
        )));
        let pipeline_handlers = PipelineHandlers::init(device, &camera, &projection);
        let depth_texture = texture::Texture::create_depth_texture(device, &window_size);
        Self {
            camera,
            projection,
            pipeline_handlers,
            depth_texture,
            new_size: None,
        }
    }

    pub fn update(&mut self, view_update: ViewUpdate) {
        match view_update {
            ViewUpdate::Size(size) => self.new_size = Some(size),
            ViewUpdate::Camera => self
                .pipeline_handlers
                .new_viewer(self.camera.clone(), self.projection.clone()),
            _ => self.pipeline_handlers.update(view_update),
        }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        device: &Device,
        fake_color: bool,
        queue: &Queue,
        area: DrawArea,
    ) {
        if let Some(size) = self.new_size.take() {
            self.depth_texture = Texture::create_depth_texture(device, &size);
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
                r: 0.4,
                g: 0.4,
                b: 0.4,
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
        render_pass.set_viewport(
            area.position.x as f32,
            area.position.y as f32,
            area.size.width as f32,
            area.size.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            area.position.x,
            area.position.y,
            area.size.width,
            area.size.height,
        );

        for pipeline_handler in handlers.iter_mut() {
            pipeline_handler.draw(device, &mut render_pass, queue);
        }
    }

    pub fn get_camera(&self) -> CameraPtr {
        self.camera.clone()
    }

    pub fn get_projection(&self) -> ProjectionPtr {
        self.projection.clone()
    }
}

#[derive(Debug)]
pub enum ViewUpdate {
    Camera,
    Spheres(Vec<Instance>),
    Tubes(Vec<Instance>),
    SelectedSpheres(Vec<Instance>),
    SelectedTubes(Vec<Instance>),
    Size(PhySize),
    ModelMatricies(Vec<Mat4>),
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
    fn init(device: &Device, camera: &CameraPtr, projection: &ProjectionPtr) -> Self {
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

    fn update(&mut self, update: ViewUpdate) {
        match update {
            ViewUpdate::Spheres(instances) => {
                let instances = Rc::new(instances);
                self.sphere.new_instances(instances.clone());
                self.fake_sphere.new_instances(instances);
            }
            ViewUpdate::Tubes(instances) => {
                let instances = Rc::new(instances);
                self.tube.new_instances(instances.clone());
                self.fake_tube.new_instances(instances);
            }
            ViewUpdate::SelectedTubes(instances) => {
                self.selected_sphere.new_instances(Rc::new(Vec::new()));
                self.selected_tube.new_instances(Rc::new(instances));
            }
            ViewUpdate::SelectedSpheres(instances) => {
                self.selected_tube.new_instances(Rc::new(Vec::new()));
                self.selected_sphere.new_instances(Rc::new(instances));
            }
            ViewUpdate::ModelMatricies(matrices) => {
                let matrices = Rc::new(matrices);
                for pipeline in self.all().iter_mut() {
                    pipeline.new_model_matrices(matrices.clone());
                }
            }
            ViewUpdate::Camera | ViewUpdate::Size(_) => {
                unreachable!();
            }
        }
    }

    fn new_viewer(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        for pipeline in self.all() {
            pipeline.new_viewer(camera.clone(), projection.clone())
        }
    }
}
