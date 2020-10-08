use super::camera;
use crate::utils::{instance, mesh, texture};
use crate::{DrawArea, PhySize};
use camera::{Camera, CameraPtr, Projection, ProjectionPtr};
use iced_wgpu::wgpu;
use instance::Instance;
use std::cell::RefCell;
use std::rc::Rc;
use texture::Texture;
use ultraviolet::{Mat4, Rotor3, Vec3};
use wgpu::{Device, PrimitiveTopology, Queue};

mod pipeline_handler;
use pipeline_handler::PipelineHandler;
mod uniforms;
use uniforms::Uniforms;
mod bindgroup_manager;
//mod plane_drawer;
mod drawable;
mod handle_drawer;
mod maths;
mod rotation_widget;

use bindgroup_manager::UniformBindGroup;
use drawable::{Drawable, Drawer, Vertex};
use handle_drawer::HandlesDrawer;
pub use handle_drawer::{HandleDir, HandleOrientation, HandlesDescriptor};
use maths::unproject_point_on_line;
use rotation_widget::RotationWidget;
pub use rotation_widget::{RotationMode, RotationWidgetDescriptor, RotationWidgetOrientation};
//use plane_drawer::PlaneDrawer;
//pub use plane_drawer::Plane;

/// An object that handles the communication with the GPU to draw the scene.
pub struct View {
    /// The camera, that is in charge of producing the view and projection matrices.
    camera: CameraPtr,
    projection: ProjectionPtr,
    /// The pipeline handler contains the pipepline that draw meshes
    pipeline_handlers: PipelineHandlers,
    /// The depth texture is updated every time the size of the drawing area is modified
    depth_texture: Texture,
    /// The handle drawers draw handles to translate the elements
    handle_drawers: HandlesDrawer,
    /// The rotation widget draw the widget to rotate the elements
    rotation_widget: RotationWidget,
    /// A possible update of the size of the drawing area, must be taken into account before
    /// drawing the next frame
    new_size: Option<PhySize>,
    device: Rc<Device>,
    viewer: Rc<RefCell<UniformBindGroup>>,
    need_redraw: bool,
    need_redraw_fake: bool,
}

impl View {
    pub fn new(
        window_size: PhySize,
        area_size: PhySize,
        device: Rc<Device>,
        queue: Rc<Queue>,
    ) -> Self {
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
        let pipeline_handlers =
            PipelineHandlers::init(device.clone(), queue.clone(), &camera, &projection);
        let depth_texture =
            texture::Texture::create_depth_texture(device.clone().as_ref(), &window_size);
        let viewer = Rc::new(RefCell::new(UniformBindGroup::new(
            device.clone(),
            queue.clone(),
            &Uniforms::from_view_proj(camera.clone(), projection.clone()),
        )));
        Self {
            camera,
            projection,
            pipeline_handlers,
            depth_texture,
            new_size: None,
            device: device.clone(),
            viewer,
            handle_drawers: HandlesDrawer::new(device.clone()),
            rotation_widget: RotationWidget::new(device.clone()),
            need_redraw: true,
            need_redraw_fake: true,
        }
    }

    /// Notify the view of an update
    pub fn update(&mut self, view_update: ViewUpdate) {
        self.need_redraw = true;
        self.need_redraw_fake = true;
        match view_update {
            ViewUpdate::Size(size) => self.new_size = Some(size),
            ViewUpdate::Camera => {
                self.pipeline_handlers
                    .new_viewer(self.camera.clone(), self.projection.clone());
                self.viewer.borrow_mut().update(&Uniforms::from_view_proj(
                    self.camera.clone(),
                    self.projection.clone(),
                ));
                self.handle_drawers
                    .update_camera(self.camera.clone(), self.projection.clone());
            }
            ViewUpdate::Handles(descr) => self.handle_drawers.update_decriptor(
                descr,
                self.camera.clone(),
                self.projection.clone(),
            ),
            ViewUpdate::RotationWidget(descr) => self.rotation_widget.update_decriptor(
                descr,
                self.camera.clone(),
                self.projection.clone(),
            ),
            _ => self.pipeline_handlers.update(view_update),
        }
    }

    pub fn need_redraw_fake(&self) -> bool {
        self.need_redraw_fake
    }

    pub fn need_redraw(&self) -> bool {
        self.need_redraw
    }

    /// Draw the scene
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        fake_color: bool,
        area: DrawArea,
    ) {
        if fake_color {
            self.need_redraw_fake = false;
        } else {
            self.need_redraw = false;
        }
        if let Some(size) = self.new_size.take() {
            self.depth_texture = Texture::create_depth_texture(self.device.as_ref(), &size);
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
        let viewer = self.viewer.borrow();
        let viewer_bind_group = viewer.get_bindgroup();
        let viewer_bind_group_layout = viewer.get_layout();
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
            pipeline_handler.draw(&mut render_pass);
        }

        /*
         * TODO: plane drawer needs better implementation
         * if !fake_color {
            self.plane_drawer.draw(&mut render_pass, &viewer_bind_group)
        }*/

        for drawer in self.handle_drawers.drawers() {
            drawer.draw(
                &mut render_pass,
                viewer_bind_group,
                viewer_bind_group_layout,
                fake_color,
            );
        }

        self.rotation_widget.draw(
            &mut render_pass,
            viewer_bind_group,
            viewer_bind_group_layout,
            fake_color,
        );
    }

    /// Update the model matrix associated to a given desgin.
    pub fn update_model_matrix(&mut self, design_id: usize, matrix: Mat4) {
        self.pipeline_handlers
            .update_model_matrix(design_id, matrix)
    }

    /// Get a pointer to the camera
    pub fn get_camera(&self) -> CameraPtr {
        self.camera.clone()
    }

    /// The position of the camera. A.k.a the point that is mapped to (0,0,0) by the view matrix
    pub fn get_camera_position(&self) -> Vec3 {
        self.camera.borrow().position
    }

    /// The direction vector of the camera. A.k.a. the vector that is mapped to (0,0,-1) by the
    /// view matrix
    pub fn get_camera_direction(&self) -> Vec3 {
        self.camera.borrow().direction()
    }

    /// A pointer to the projection camera
    pub fn get_projection(&self) -> ProjectionPtr {
        self.projection.clone()
    }

    /// The right vector of the camera. A.k.a. the vector that is mapped to (1,0,0) by the view
    /// matrix
    pub fn right_vec(&self) -> Vec3 {
        self.camera.borrow().right_vec()
    }

    /// The up vector of the camera. A.k.a. the vector that is mapped to (0,1,0) by the view matrix
    pub fn up_vec(&self) -> Vec3 {
        self.camera.borrow().up_vec()
    }

    pub fn compute_translation_handle(
        &self,
        x_coord: f32,
        y_coord: f32,
        direction: HandleDir,
    ) -> Option<Vec3> {
        let (origin, dir) = self.handle_drawers.get_handle(direction)?;
        let (x0, y0) = self.handle_drawers.get_origin_translation()?;
        let p1 = unproject_point_on_line(
            origin,
            dir,
            self.camera.clone(),
            self.projection.clone(),
            x0,
            y0,
        )?;
        let p2 = unproject_point_on_line(
            origin,
            dir,
            self.camera.clone(),
            self.projection.clone(),
            x_coord,
            y_coord,
        )?;
        Some(p2 - p1)
    }

    pub fn translate_widgets(&mut self, translation: Vec3) {
        self.handle_drawers.translate(translation);
        self.rotation_widget.translate(translation);
    }

    pub fn init_rotation(&mut self, x_coord: f32, y_coord: f32) {
        self.rotation_widget.init_rotation(x_coord, y_coord)
    }

    pub fn init_translation(&mut self, x: f32, y: f32) {
        self.handle_drawers.init_translation(x, y)
    }

    pub fn compute_rotation(&self, x: f32, y: f32, mode: RotationMode) -> Option<(Rotor3, Vec3)> {
        self.rotation_widget.compute_rotation(
            x,
            y,
            self.camera.clone(),
            self.projection.clone(),
            mode,
        )
    }
}

/// An notification to be given to the view
#[derive(Debug)]
pub enum ViewUpdate {
    /// The camera has moved and the view and projection matrix must be updated.
    Camera,
    /// The set of spheres have been modified
    Spheres(Rc<Vec<Instance>>),
    /// The set of tubes have been modified
    Tubes(Rc<Vec<Instance>>),
    /// The set of selected spheres has been modified
    SelectedSpheres(Rc<Vec<Instance>>),
    /// The set of selected tubes has been modified
    SelectedTubes(Rc<Vec<Instance>>),
    /// The set of candidate spheres has been modified
    CandidateSpheres(Rc<Vec<Instance>>),
    /// The set of candidate tubes has been modified
    CandidateTubes(Rc<Vec<Instance>>),
    /// The size of the drawing area has been modified
    Size(PhySize),
    /// The set of model matrices has been modified
    ModelMatrices(Vec<Mat4>),
    /// The set of phantom instances has been modified
    PhantomInstances(Rc<Vec<Instance>>, Rc<Vec<Instance>>),
    Handles(Option<HandlesDescriptor>),
    RotationWidget(Option<RotationWidgetDescriptor>),
}

struct PipelineHandlers {
    sphere: PipelineHandler,
    tube: PipelineHandler,
    fake_tube: PipelineHandler,
    fake_sphere: PipelineHandler,
    selected_sphere: PipelineHandler,
    selected_tube: PipelineHandler,
    candidate_sphere: PipelineHandler,
    candidate_tube: PipelineHandler,
    phantom_sphere: PipelineHandler,
    phantom_tube: PipelineHandler,
}

impl PipelineHandlers {
    fn init(
        device: Rc<Device>,
        queue: Rc<Queue>,
        camera: &CameraPtr,
        projection: &ProjectionPtr,
    ) -> Self {
        let sphere_mesh = mesh::Mesh::sphere(device.as_ref(), false);
        let tube_mesh = mesh::Mesh::tube(device.as_ref(), false);
        let fake_sphere_mesh = mesh::Mesh::sphere(device.as_ref(), false);
        let fake_tube_mesh = mesh::Mesh::tube(device.as_ref(), false);
        let selected_sphere_mesh = mesh::Mesh::sphere(device.as_ref(), true);
        let selected_tube_mesh = mesh::Mesh::tube(device.as_ref(), true);
        let candidate_sphere_mesh = mesh::Mesh::sphere(device.as_ref(), true);
        let candidate_tube_mesh = mesh::Mesh::tube(device.as_ref(), true);
        let phantom_tube_mesh = mesh::Mesh::tube(device.as_ref(), false);
        let phantom_sphere_mesh = mesh::Mesh::sphere(device.as_ref(), false);

        let sphere_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            sphere_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleList,
            pipeline_handler::Flavour::Real,
        );
        let tube_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            tube_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Real,
        );
        let fake_tube_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            fake_tube_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let fake_sphere_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            fake_sphere_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let selected_sphere_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            selected_sphere_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Selected,
        );
        let selected_tube_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            selected_tube_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Selected,
        );
        let candidate_sphere = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            candidate_sphere_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Candidate,
        );
        let candidate_tube = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            candidate_tube_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Candidate,
        );
        let phantom_sphere = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            phantom_sphere_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Phantom,
        );
        let phantom_tube = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            phantom_tube_mesh,
            camera,
            projection,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Phantom,
        );

        Self {
            sphere: sphere_pipeline_handler,
            tube: tube_pipeline_handler,
            fake_sphere: fake_sphere_pipeline_handler,
            fake_tube: fake_tube_pipeline_handler,
            selected_sphere: selected_sphere_pipeline_handler,
            selected_tube: selected_tube_pipeline_handler,
            candidate_sphere,
            candidate_tube,
            phantom_sphere,
            phantom_tube,
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
            &mut self.candidate_tube,
            &mut self.candidate_sphere,
            &mut self.phantom_tube,
            &mut self.phantom_sphere,
        ]
    }

    fn real(&mut self) -> Vec<&mut PipelineHandler> {
        vec![
            &mut self.sphere,
            &mut self.tube,
            &mut self.selected_sphere,
            &mut self.selected_tube,
            &mut self.candidate_tube,
            &mut self.candidate_sphere,
            &mut self.phantom_tube,
            &mut self.phantom_sphere,
        ]
    }

    fn fake(&mut self) -> Vec<&mut PipelineHandler> {
        vec![&mut self.fake_sphere, &mut self.fake_tube]
    }

    fn update(&mut self, update: ViewUpdate) {
        match update {
            ViewUpdate::Spheres(instances) => {
                self.sphere.new_instances(instances.clone());
                self.fake_sphere.new_instances(instances);
            }
            ViewUpdate::Tubes(instances) => {
                self.tube.new_instances(instances.clone());
                self.fake_tube.new_instances(instances);
            }
            ViewUpdate::SelectedTubes(instances) => {
                self.selected_tube.new_instances(instances);
            }
            ViewUpdate::SelectedSpheres(instances) => {
                self.selected_sphere.new_instances(instances);
            }
            ViewUpdate::ModelMatrices(matrices) => {
                let matrices = Rc::new(matrices);
                for pipeline in self.all().iter_mut() {
                    pipeline.new_model_matrices(matrices.clone());
                }
            }
            ViewUpdate::CandidateSpheres(instances) => {
                self.candidate_sphere.new_instances(instances);
            }
            ViewUpdate::CandidateTubes(instances) => {
                self.candidate_tube.new_instances(instances);
            }
            ViewUpdate::PhantomInstances(sphere, tube) => {
                self.phantom_sphere.new_instances(sphere);
                self.phantom_tube.new_instances(tube);
            }
            ViewUpdate::Camera
            | ViewUpdate::Size(_)
            | ViewUpdate::Handles(_)
            | ViewUpdate::RotationWidget(_) => {
                unreachable!();
            }
        }
    }

    fn new_viewer(&mut self, camera: CameraPtr, projection: ProjectionPtr) {
        for pipeline in self.all() {
            pipeline.new_viewer(camera.clone(), projection.clone())
        }
    }

    pub fn update_model_matrix(&mut self, design_id: usize, matrix: Mat4) {
        for pipeline in self.all() {
            pipeline.update_model_matrix(design_id, matrix)
        }
    }
}
