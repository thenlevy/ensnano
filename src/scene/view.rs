//! The view module handles the drawing of the scene on texture. The scene can be drawn on the next
//! frame to be displayed, or on a "fake texture" that is used to map pixels to objects.

use super::{camera, ActionMode};
use crate::consts::*;
use crate::design::Axis;
use crate::utils::{bindgroup_manager, instance, mesh, texture};
use crate::{DrawArea, PhySize};
use camera::{Camera, CameraPtr, Projection, ProjectionPtr};
use iced_wgpu::wgpu;
use instance::Instance;
use std::cell::RefCell;
use std::rc::Rc;
use texture::Texture;
use ultraviolet::{Mat4, Rotor3, Vec3};
use wgpu::{Device, PrimitiveTopology, Queue};

/// A `PipelineHandler` is a structure that is responsible for drawing a mesh
mod pipeline_handler;
use pipeline_handler::PipelineHandler;
/// A `Uniform` is a structure that manages view and projection matrices.
mod uniforms;
use uniforms::Uniforms;
/// This modules defines a trait for drawing widget made of several meshes.
mod drawable;
mod grid;
mod grid_disc;
/// A HandleDrawer draws the widget for translating objects
mod handle_drawer;
mod instances_drawer;
mod letter;
mod maths;
/// A RotationWidget draws the widget for rotating objects
mod rotation_widget;

use bindgroup_manager::{DynamicBindGroup, UniformBindGroup};
use drawable::{Drawable, Drawer, Vertex};
use grid::GridDrawer;
pub use grid::{GridInstance, GridIntersection, GridTypeDescr};
pub use grid_disc::GridDisc;
use handle_drawer::HandlesDrawer;
pub use handle_drawer::{HandleDir, HandleOrientation, HandlesDescriptor};
use instances_drawer::InstanceDrawer;
use letter::LetterDrawer;
pub use letter::LetterInstance;
use maths::unproject_point_on_line;
use rotation_widget::RotationWidget;
pub use rotation_widget::{RotationMode, RotationWidgetDescriptor, RotationWidgetOrientation};
//use plane_drawer::PlaneDrawer;
//pub use plane_drawer::Plane;

static MODEL_BG_ENTRY: &'static [wgpu::BindGroupLayoutEntry] = &[wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStage::from_bits_truncate(wgpu::ShaderStage::VERTEX.bits()),
    ty: wgpu::BindingType::StorageBuffer {
        dynamic: false,
        min_binding_size: None,
        readonly: true,
    },
    count: None,
}];

/// An object that handles the communication with the GPU to draw the scene.
pub struct View {
    /// The camera, that is in charge of producing the view and projection matrices.
    camera: CameraPtr,
    projection: ProjectionPtr,
    /// The pipeline handler contains the pipepline that draw meshes
    pipeline_handlers: PipelineHandlers,
    /// The depth texture is updated every time the size of the drawing area is modified
    depth_texture: Texture,
    /// The fake depth texture is updated every time the size of the drawing area is modified and
    /// has a sample count of 1
    fake_depth_texture: Texture,
    /// The handle drawers draw handles to translate the elements
    handle_drawers: HandlesDrawer,
    /// The rotation widget draw the widget to rotate the elements
    rotation_widget: RotationWidget,
    /// A possible update of the size of the drawing area, must be taken into account before
    /// drawing the next frame
    new_size: Option<PhySize>,
    /// The pipilines that draw the basis symbols
    letter_drawer: Vec<LetterDrawer>,
    device: Rc<Device>,
    /// A bind group associated to the uniform buffer containing the view and projection matrices.
    //TODO this is currently only passed to the widgets, it could be passed to the mesh pipeline as
    //well.
    viewer: UniformBindGroup,
    models: DynamicBindGroup,
    redraw_twice: bool,
    need_redraw: bool,
    need_redraw_fake: bool,
    draw_letter: bool,
    msaa_texture: Option<wgpu::TextureView>,
    grid_drawer: GridDrawer,
    disc_drawer: InstanceDrawer<GridDisc>,
}

impl View {
    pub fn new(
        window_size: PhySize,
        area_size: PhySize,
        device: Rc<Device>,
        queue: Rc<Queue>,
        encoder: &mut wgpu::CommandEncoder,
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
        let viewer = UniformBindGroup::new(
            device.clone(),
            queue.clone(),
            &Uniforms::from_view_proj(camera.clone(), projection.clone()),
        );
        let model_bg_desc = wgpu::BindGroupLayoutDescriptor {
            entries: MODEL_BG_ENTRY,
            label: None,
        };
        let pipeline_handlers =
            PipelineHandlers::init(device.clone(), queue.clone(), &viewer.get_layout_desc(), &model_bg_desc);
        let letter_drawer = BASIS_SYMBOLS
            .iter()
            .map(|c| LetterDrawer::new(device.clone(), queue.clone(), *c, &camera, &projection))
            .collect();
        let depth_texture =
            texture::Texture::create_depth_texture(device.as_ref(), &area_size, SAMPLE_COUNT);
        let fake_depth_texture =
            texture::Texture::create_depth_texture(device.as_ref(), &window_size, 1);
        let msaa_texture = if SAMPLE_COUNT > 1 {
            Some(crate::utils::texture::Texture::create_msaa_texture(
                device.clone().as_ref(),
                &area_size,
                SAMPLE_COUNT,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ))
        } else {
            None
        };
        let models = DynamicBindGroup::new(device.clone(), queue.clone());

        let grid_drawer = GridDrawer::new(
            device.clone(),
            queue.clone(),
            &camera,
            &projection,
            encoder,
            None,
        );

        let disc_drawer = InstanceDrawer::new(
            device.clone(),
            queue.clone(),
            viewer.get_layout_desc(),
            model_bg_desc,
        );

        Self {
            camera,
            projection,
            pipeline_handlers,
            depth_texture,
            fake_depth_texture,
            new_size: None,
            device: device.clone(),
            viewer,
            models,
            handle_drawers: HandlesDrawer::new(device.clone()),
            rotation_widget: RotationWidget::new(device),
            letter_drawer,
            redraw_twice: false,
            need_redraw: true,
            need_redraw_fake: true,
            draw_letter: false,
            msaa_texture,
            grid_drawer,
            disc_drawer,
        }
    }

    /// Notify the view of an update. According to the nature of this update, the view decides if
    /// it needs to be redrawn or not.
    pub fn update(&mut self, view_update: ViewUpdate) {
        self.need_redraw = true;
        match view_update {
            ViewUpdate::Size(size) => {
                self.new_size = Some(size);
                self.need_redraw_fake = true;
            }
            ViewUpdate::Camera => {
                self.viewer.update(&Uniforms::from_view_proj(
                    self.camera.clone(),
                    self.projection.clone(),
                ));
                self.handle_drawers
                    .update_camera(self.camera.clone(), self.projection.clone());
                for i in 0..NB_BASIS_SYMBOLS {
                    self.letter_drawer[i].new_viewer(self.camera.clone(), self.projection.clone());
                }
                self.grid_drawer
                    .new_viewer(self.camera.clone(), self.projection.clone());
                self.need_redraw_fake = true;
            }
            ViewUpdate::Handles(descr) => {
                self.handle_drawers.update_decriptor(
                    descr,
                    self.camera.clone(),
                    self.projection.clone(),
                );
                self.need_redraw_fake = true;
            }

            ViewUpdate::RotationWidget(descr) => {
                self.rotation_widget.update_decriptor(
                    descr,
                    self.camera.clone(),
                    self.projection.clone(),
                );
                self.need_redraw_fake = true;
            }
            ViewUpdate::ModelMatrices(ref matrices) => {
                for i in 0..NB_BASIS_SYMBOLS {
                    self.letter_drawer[i].new_model_matrices(Rc::new(matrices.clone()));
                }
                self.models.update(matrices.clone().as_slice());
                self.pipeline_handlers.update(view_update);
            }
            ViewUpdate::Letter(letter) => {
                for (i, instance) in letter.iter().enumerate() {
                    self.letter_drawer[i].new_instances(instance.clone());
                }
            }
            ViewUpdate::Grids(grid) => self.grid_drawer.new_instances(grid),
            ViewUpdate::GridDiscs(instances) => self.disc_drawer.new_instances(instances),
            _ => {
                self.need_redraw_fake |= self.pipeline_handlers.update(view_update);
            }
        }
    }

    pub fn need_redraw_fake(&self) -> bool {
        self.need_redraw_fake
    }

    pub fn need_redraw(&self) -> bool {
        self.need_redraw | self.redraw_twice
    }

    /// Draw the scene
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        draw_type: DrawType,
        area: DrawArea,
        action_mode: ActionMode,
    ) {
        let fake_color = draw_type.is_fake();
        if let Some(size) = self.new_size.take() {
            self.depth_texture =
                Texture::create_depth_texture(self.device.as_ref(), &area.size, SAMPLE_COUNT);
            self.fake_depth_texture = Texture::create_depth_texture(self.device.as_ref(), &size, 1);
            self.msaa_texture = if SAMPLE_COUNT > 1 {
                Some(crate::utils::texture::Texture::create_msaa_texture(
                    self.device.clone().as_ref(),
                    &area.size,
                    SAMPLE_COUNT,
                    wgpu::TextureFormat::Bgra8UnormSrgb,
                ))
            } else {
                None
            };
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
                r: 0.8,
                g: 0.8,
                b: 0.8,
                a: 1.,
            }
        };
        let mut handlers = match draw_type {
            DrawType::Design => self.pipeline_handlers.fake(),
            DrawType::Scene => self.pipeline_handlers.real(),
            DrawType::Phantom => self.pipeline_handlers.fake_phantoms(),
            _ => Vec::new(),
        };
        let viewer = &self.viewer;
        let viewer_bind_group = viewer.get_bindgroup();
        let viewer_bind_group_layout = viewer.get_layout();

        let attachment = if !fake_color {
            if let Some(ref msaa) = self.msaa_texture {
                msaa
            } else {
                target
            }
        } else {
            target
        };

        let resolve_target = if !fake_color && self.msaa_texture.is_some() {
            Some(target)
        } else {
            None
        };

        let depth_attachement = if !fake_color {
            &self.depth_texture
        } else {
            &self.fake_depth_texture
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &depth_attachement.view,
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
        if fake_color {
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
        }

        for pipeline_handler in handlers.iter_mut() {
            pipeline_handler.draw(&mut render_pass, self.viewer.get_bindgroup(), self.models.get_bindgroup());
        }

        if draw_type.wants_widget() {
            if action_mode.wants_handle() {
                self.handle_drawers.draw(
                    &mut render_pass,
                    viewer_bind_group,
                    viewer_bind_group_layout,
                    fake_color,
                );
            }

            if action_mode.wants_rotation() {
                self.rotation_widget.draw(
                    &mut render_pass,
                    viewer_bind_group,
                    viewer_bind_group_layout,
                    fake_color,
                );
            }
        }

        if !fake_color && self.draw_letter {
            for drawer in self.letter_drawer.iter_mut() {
                drawer.draw(&mut render_pass)
            }
        }

        if !fake_color {
            self.grid_drawer.draw(&mut render_pass);
            self.disc_drawer.draw(
                &mut render_pass,
                viewer_bind_group,
                &self.models.get_bindgroup(),
            );
        }

        if fake_color {
            self.need_redraw_fake = false;
        } else if self.redraw_twice {
            self.redraw_twice = false;
            self.need_redraw = true;
        } else {
            self.need_redraw = false;
        }
    }

    /// Get a pointer to the camera
    pub fn get_camera(&self) -> CameraPtr {
        self.camera.clone()
    }

    /// A pointer to the projection camera
    pub fn get_projection(&self) -> ProjectionPtr {
        self.projection.clone()
    }

    pub fn set_draw_letter(&mut self, value: bool) {
        self.draw_letter = value;
    }

    /// Compute the translation that needs to be applied to the objects affected by the handle
    /// widget.
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

    /// Translate the widgets when the associated objects are translated.
    pub fn translate_widgets(&mut self, translation: Vec3) {
        self.need_redraw = true;
        self.handle_drawers.translate(translation);
        self.rotation_widget.translate(translation);
    }

    /// Initialise the rotation that will be applied on objects affected by the rotation widget.
    pub fn init_rotation(&mut self, x_coord: f32, y_coord: f32) {
        self.need_redraw = true;
        self.rotation_widget.init_rotation(x_coord, y_coord)
    }

    /// Initialise the translation that will be applied on objects affected by the handle widget.
    pub fn init_translation(&mut self, x: f32, y: f32) {
        self.need_redraw = true;
        self.handle_drawers.init_translation(x, y)
    }

    /// Compute the rotation that needs to be applied to the objects affected by the rotation
    /// widget.
    pub fn compute_rotation(
        &self,
        x: f32,
        y: f32,
        mode: RotationMode,
    ) -> Option<(Rotor3, Vec3, bool)> {
        self.rotation_widget.compute_rotation(
            x,
            y,
            self.camera.clone(),
            self.projection.clone(),
            mode,
        )
    }

    pub fn set_widget_candidate(&mut self, selected_id: Option<u32>) {
        self.redraw_twice |= self.rotation_widget.set_selected(selected_id);
        self.redraw_twice |= self.handle_drawers.set_selected(selected_id);
    }

    pub fn compute_projection_axis(
        &self,
        axis: &Axis,
        mouse_x: f64,
        mouse_y: f64,
    ) -> Option<isize> {
        let p1 = unproject_point_on_line(
            axis.origin,
            axis.direction,
            self.camera.clone(),
            self.projection.clone(),
            mouse_x as f32,
            mouse_y as f32,
        )?;

        let sign = (p1 - axis.origin).dot(axis.direction).signum();
        Some(((p1 - axis.origin).mag() * sign / axis.direction.mag()).round() as isize)
    }

    pub fn grid_intersection(&self, x_ndc: f32, y_ndc: f32) -> Option<GridIntersection> {
        let ray = maths::cast_ray(x_ndc, y_ndc, self.camera.clone(), self.projection.clone());
        self.grid_drawer.intersect(ray.0, ray.1)
    }

    pub fn set_candidate_grid(&mut self, grid: Option<(u32, u32)>) {
        self.grid_drawer.set_candidate_grid(grid)
    }

    pub fn set_selected_grid(&mut self, grid: Option<(u32, u32)>) {
        self.grid_drawer.set_selected_grid(grid)
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
    Letter(Vec<Rc<Vec<LetterInstance>>>),
    Grids(Rc<Vec<GridInstance>>),
    GridDiscs(Vec<GridDisc>),
}

/// The structure gathers all the pipepline that are used to draw meshes on the scene
struct PipelineHandlers {
    /// The nucleotides
    sphere: PipelineHandler,
    /// The bounds
    tube: PipelineHandler,
    /// The pipepline used to draw nucleotides on the fake texture
    fake_sphere: PipelineHandler,
    /// The pipepline used to draw bounds on the fake texture
    fake_tube: PipelineHandler,
    /// The selected nucleotides
    selected_sphere: PipelineHandler,
    /// The selected bounds
    selected_tube: PipelineHandler,
    /// The candidate nucleotides
    candidate_sphere: PipelineHandler,
    /// The candidate tube
    candidate_tube: PipelineHandler,
    /// The nucleotides of the phantom helix
    phantom_sphere: PipelineHandler,
    /// The bounds of the phantom helix
    phantom_tube: PipelineHandler,
    fake_phantom_sphere: PipelineHandler,
    fake_phantom_tube: PipelineHandler,
}

impl PipelineHandlers {
    fn init(
        device: Rc<Device>,
        queue: Rc<Queue>,
        viewer_desc: &wgpu::BindGroupLayoutDescriptor<'static>,
        model_desc: &wgpu::BindGroupLayoutDescriptor<'static>,
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
        let fake_phantom_sphere_mesh = mesh::Mesh::sphere(device.as_ref(), false);
        let fake_phantom_tube_mesh = mesh::Mesh::tube(device.as_ref(), false);

        let sphere_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            sphere_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleList,
            pipeline_handler::Flavour::Real,
        );
        let tube_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            tube_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Real,
        );
        let fake_tube_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            fake_tube_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let fake_sphere_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            fake_sphere_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let selected_sphere_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            selected_sphere_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Selected,
        );
        let selected_tube_pipeline_handler = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            selected_tube_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Selected,
        );
        let candidate_sphere = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            candidate_sphere_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Candidate,
        );
        let candidate_tube = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            candidate_tube_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Candidate,
        );
        let phantom_sphere = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            phantom_sphere_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Phantom,
        );
        let phantom_tube = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            phantom_tube_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Phantom,
        );
        let fake_phantom_sphere = PipelineHandler::new(
            device.clone(),
            queue.clone(),
            fake_phantom_sphere_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
        );
        let fake_phantom_tube = PipelineHandler::new(
            device,
            queue,
            fake_phantom_tube_mesh,
            viewer_desc,
            model_desc,
            PrimitiveTopology::TriangleStrip,
            pipeline_handler::Flavour::Fake,
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
            fake_phantom_sphere,
            fake_phantom_tube,
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
            &mut self.fake_phantom_tube,
            &mut self.fake_phantom_sphere,
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

    fn fake_phantoms(&mut self) -> Vec<&mut PipelineHandler> {
        vec![&mut self.fake_phantom_sphere, &mut self.fake_phantom_tube]
    }

    fn fake(&mut self) -> Vec<&mut PipelineHandler> {
        vec![&mut self.fake_sphere, &mut self.fake_tube]
    }

    /// Forwards an update to the relevant piplines. Return true if the fake view must be redrawn
    fn update(&mut self, update: ViewUpdate) -> bool {
        match update {
            ViewUpdate::Spheres(instances) => {
                self.sphere.new_instances(instances.clone());
                self.fake_sphere.new_instances(instances);
                true
            }
            ViewUpdate::Tubes(instances) => {
                self.tube.new_instances(instances.clone());
                self.fake_tube.new_instances(instances);
                true
            }
            ViewUpdate::SelectedTubes(instances) => {
                self.selected_tube.new_instances(instances);
                false
            }
            ViewUpdate::SelectedSpheres(instances) => {
                self.selected_sphere.new_instances(instances);
                false
            }
            ViewUpdate::ModelMatrices(_) => {
                true
            }
            ViewUpdate::CandidateSpheres(instances) => {
                self.candidate_sphere.new_instances(instances);
                false
            }
            ViewUpdate::CandidateTubes(instances) => {
                self.candidate_tube.new_instances(instances);
                false
            }
            ViewUpdate::PhantomInstances(sphere, tube) => {
                self.phantom_sphere.new_instances(sphere.clone());
                self.phantom_tube.new_instances(tube.clone());
                self.fake_phantom_sphere.new_instances(sphere);
                self.fake_phantom_tube.new_instances(tube);
                false
            }
            ViewUpdate::Camera
            | ViewUpdate::Size(_)
            | ViewUpdate::Handles(_)
            | ViewUpdate::RotationWidget(_)
            | ViewUpdate::Letter(_)
            | ViewUpdate::Grids(_)
            | ViewUpdate::GridDiscs(_) => {
                unreachable!();
            }
        }
    }

}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DrawType {
    Scene,
    Design,
    Widget,
    Phantom,
}

impl DrawType {
    fn is_fake(&self) -> bool {
        *self != DrawType::Scene
    }

    fn wants_widget(&self) -> bool {
        match self {
            DrawType::Scene => true,
            DrawType::Design => false,
            DrawType::Widget => true,
            DrawType::Phantom => false,
        }
    }
}
