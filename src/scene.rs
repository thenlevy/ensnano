use iced_wgpu::wgpu;
use iced_winit::winit;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ultraviolet::{Mat4, Rotor3, Vec3};

use crate::{design, mediator, utils};
use crate::{DrawArea, PhySize, WindowEvent};
use instance::Instance;
use mediator::{
    ActionMode, AppNotification, Application, MediatorPtr, Notification, SelectionMode,
};
use utils::instance;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

/// Computation of the view and projection matrix.
mod camera;
/// Display of the scene
mod view;
use view::{
    DrawType, HandleDir, HandleOrientation, HandlesDescriptor, LetterInstance,
    RotationMode as WidgetRotationMode, RotationWidgetDescriptor, RotationWidgetOrientation, View,
    ViewUpdate,
};
/// Handling of inputs and notifications
mod controller;
use controller::{Consequence, Controller};
/// Handling of designs and internal data
mod data;
pub use controller::ClickMode;
use data::Data;
use design::{
    Design, DesignNotification, DesignNotificationContent, DesignRotation, IsometryTarget,
};
mod element_selector;
use element_selector::{ElementSelector, SceneElement};

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;

const SAMPLE_COUNT: u32 = 4;

/// A structure responsible of the 3D display of the designs
pub struct Scene {
    /// The update to be performed before next frame
    update: SceneUpdate,
    /// The Object that handles the drawing to the 3d texture
    view: ViewPtr,
    /// The Object thant handles the designs data
    data: DataPtr,
    /// The Object that handles input and notifications
    controller: Controller,
    /// The limits of the area on which the scene is displayed
    area: DrawArea,
    pixel_to_check: Option<PhysicalPosition<f64>>,
    mediator: MediatorPtr,
    element_selector: ElementSelector,
}

impl Scene {
    /// Create a new scene.
    /// # Argument
    ///
    /// * `device` a reference to a `Device` object. This can be seen as a socket to the GPU
    ///
    /// * `queue` the command queue of `device`.
    ///
    /// * `window_size` the *Physical* size of the window in which the application is displayed
    ///
    /// * `area` the limits, in *physical* size of the area on which the scene is displayed
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        window_size: PhySize,
        area: DrawArea,
        mediator: MediatorPtr,
    ) -> Self {
        let update = SceneUpdate::new();
        let view: ViewPtr = Rc::new(RefCell::new(View::new(
            window_size,
            area.size,
            device.clone(),
            queue.clone(),
        )));
        let data: DataPtr = Rc::new(RefCell::new(Data::new(view.clone())));
        let controller: Controller =
            Controller::new(view.clone(), data.clone(), window_size, area.size);
        let element_selector = ElementSelector::new(
            device,
            queue,
            controller.get_window_size(),
            view.clone(),
            data.clone(),
            area,
        );
        Self {
            view,
            data,
            update,
            controller,
            area,
            pixel_to_check: None,
            mediator,
            element_selector,
        }
    }

    /// Add a design to be rendered.
    pub fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        self.data.borrow_mut().add_design(design)
    }

    /// Remove all designs
    pub fn clear_design(&mut self) {
        self.data.borrow_mut().clear_designs()
    }

    /// Return the list of designs selected
    fn get_selected_designs(&self) -> HashSet<u32> {
        self.data.borrow().get_selected_designs()
    }

    /// Input an event to the scene. The controller parse the event and return the consequence that
    /// the event must have.
    pub fn input(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>) {
        let consequence = self.controller.input(event, cursor_position);
        match consequence {
            Consequence::Nothing => (),
            Consequence::CameraMoved => self.notify(SceneNotification::CameraMoved),
            Consequence::PixelSelected(clicked) => self.click_on(clicked),
            Consequence::Translation(dir, x_coord, y_coord) => {
                let translation = self.view.borrow().compute_translation_handle(
                    x_coord as f32,
                    y_coord as f32,
                    dir,
                );
                if let Some(t) = translation {
                    self.translate_selected_design(t);
                }
            }
            Consequence::MovementEnded => {
                self.mediator
                    .lock()
                    .unwrap()
                    .notify_all_designs(AppNotification::MovementEnded);
                self.data.borrow_mut().end_movement();
                self.update_handle();
            }
            Consequence::InitRotation(x, y) => {
                self.view.borrow_mut().init_rotation(x as f32, y as f32)
            }
            Consequence::InitTranslation(x, y) => {
                self.view.borrow_mut().init_translation(x as f32, y as f32)
            }
            Consequence::Rotation(mode, x, y) => {
                let rotation = self
                    .view
                    .borrow()
                    .compute_rotation(x as f32, y as f32, mode);
                if let Some((rotation, origin)) = rotation {
                    self.rotate_selected_desgin(rotation, origin)
                } else {
                    println!("Warning rotiation was None")
                }
            }
            Consequence::Swing(x, y) => {
                let pivot = self.data.borrow().get_selected_position();
                if let Some(pivot) = pivot {
                    self.controller.set_pivot_point(pivot);
                    self.controller.swing(-x, -y);
                    self.notify(SceneNotification::CameraMoved);
                }
            }
            Consequence::CursorMoved(clicked) => self.pixel_to_check = Some(clicked),
            Consequence::ToggleWidget => self.data.borrow_mut().toggle_widget_basis(),
            Consequence::BuildEnded(d_id, id) => {
                self.select(Some(SceneElement::DesignElement(d_id, id)))
            }
        };
    }

    fn click_on(&mut self, clicked_pixel: PhysicalPosition<f64>) {
        let element = self.element_selector.set_selected_id(clicked_pixel);
        self.select(element);
    }

    fn select(&mut self, element: Option<SceneElement>) {
        let selection = self.data.borrow_mut().set_selection(element);
        if let Some(selection) = selection {
            self.mediator.lock().unwrap().notify_selection(selection);
        }
        let pivot = self.data.borrow().get_selected_position();
        if let Some(pivot) = pivot {
            self.controller.set_pivot_point(pivot);
        }
        self.update_handle();
    }

    fn check_on(&mut self, clicked_pixel: PhysicalPosition<f64>) {
        let element = self.element_selector.set_selected_id(clicked_pixel);
        self.controller.notify(element);
        self.data.borrow_mut().set_candidate(element);
        let widget = if let Some(SceneElement::WidgetElement(widget_id)) = element {
            Some(widget_id)
        } else {
            None
        };
        /*
        if let Some(element) = element {
            match element {
                SceneElement::DesignElement(design_id, element_id) =>
                    self.data.borrow_mut().set_candidate(design_id, element_id),
                SceneElement::WidgetElement(widget_id) =>
                    widget = Some(widget_id),
                _ => ()
            }
        } else {
            self.data.borrow_mut().reset_candidate();
        }*/
        self.view.borrow_mut().set_widget_candidate(widget);
    }

    fn translate_selected_design(&mut self, translation: Vec3) {
        self.view.borrow_mut().translate_widgets(translation);
        self.mediator.lock().unwrap().notify_designs(
            &self.get_selected_designs(),
            AppNotification::Translation(&translation),
        );
    }

    fn rotate_selected_desgin(&mut self, rotation: Rotor3, origin: Vec3) {
        let target = match self.data.borrow().selection_mode {
            SelectionMode::Helix => IsometryTarget::Helix(self.data.borrow().get_selected_group()),
            _ => IsometryTarget::Design,
        };
        let rotation = DesignRotation {
            rotation,
            origin,
            target,
        };
        self.mediator.lock().unwrap().notify_designs(
            &self.data.borrow().get_selected_designs(),
            AppNotification::Rotation(&rotation),
        );
    }

    /// Adapt the camera, position, orientation and pivot point to a design so that the design fits
    /// the scene, and the pivot point of the camera is the center of the design.
    pub fn fit_design(&mut self) {
        let camera = self
            .data
            .borrow()
            .get_fitting_camera(self.get_ratio(), self.get_fovy());
        if let Some((position, rotor)) = camera {
            let pivot_point = self.data.borrow().get_middle_point(0);
            self.controller.set_pivot_point(pivot_point);
            self.notify(SceneNotification::NewCamera(position, rotor));
        }
    }

    pub fn need_redraw(&mut self, dt: Duration) -> bool {
        if let Some(pixel) = self.pixel_to_check.take() {
            self.check_on(pixel)
        }
        if self.controller.camera_is_moving() {
            self.notify(SceneNotification::CameraMoved);
        }
        if self.update.need_update {
            self.perform_update(dt);
        }
        self.data.borrow_mut().update_view();
        self.view.borrow().need_redraw()
    }

    /// Draw the scene
    pub fn draw_view(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.view.borrow_mut().draw(
            encoder,
            target,
            DrawType::Scene,
            self.area,
            self.data.borrow().get_action_mode(),
        );
    }

    fn update_handle(&mut self) {
        let origin = self.data.borrow().get_selected_position();
        let descr = origin.clone().map(|origin| HandlesDescriptor {
            origin,
            orientation: HandleOrientation::Rotor(self.data.borrow().get_widget_basis()),
            size: 0.25,
        });
        self.view.borrow_mut().update(ViewUpdate::Handles(descr));
        let descr = origin.map(|origin| RotationWidgetDescriptor {
            origin,
            orientation: RotationWidgetOrientation::Rotor(self.data.borrow().get_widget_basis()),
            size: 0.2,
        });
        self.view
            .borrow_mut()
            .update(ViewUpdate::RotationWidget(descr));
    }

    fn perform_update(&mut self, dt: Duration) {
        if self.update.camera_update {
            self.controller.update_camera(dt);
            self.view.borrow_mut().update(ViewUpdate::Camera);
            self.update.camera_update = false;
            self.update_handle()
        }
        self.update.need_update = false;
    }

    /// Return the vertical field of view of the camera in radians
    pub fn get_fovy(&self) -> f32 {
        self.view.borrow().get_projection().borrow().get_fovy()
    }

    /// Return the width/height ratio of the camera
    pub fn get_ratio(&self) -> f32 {
        self.view.borrow().get_projection().borrow().get_ratio()
    }

    pub fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.data.borrow_mut().change_selection_mode(selection_mode)
    }

    pub fn change_action_mode(&mut self, action_mode: ActionMode) {
        self.data.borrow_mut().change_action_mode(action_mode)
    }

    pub fn change_sensitivity(&mut self, sensitivity: f32) {
        self.controller.change_sensitivity(sensitivity)
    }
}

/// A structure that stores the element that needs to be updated in a scene
pub struct SceneUpdate {
    pub tube_instances: Option<Vec<Instance>>,
    pub sphere_instances: Option<Vec<Instance>>,
    pub fake_tube_instances: Option<Vec<Instance>>,
    pub fake_sphere_instances: Option<Vec<Instance>>,
    pub selected_tube: Option<Vec<Instance>>,
    pub selected_sphere: Option<Vec<Instance>>,
    pub candidate_spheres: Option<Vec<Instance>>,
    pub candidate_tubes: Option<Vec<Instance>>,
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
            candidate_spheres: None,
            candidate_tubes: None,
            need_update: false,
            camera_update: false,
            model_matrices: None,
        }
    }
}

/// A notification to be given to the scene
pub enum SceneNotification {
    /// The camera has moved. As a consequence, the projection and view matrix must be
    /// updated.
    CameraMoved,
    /// The camera is replaced by a new one.
    NewCamera(Vec3, Rotor3),
    /// The drawing area has been modified
    NewSize(PhySize, DrawArea),
}

impl Scene {
    /// Send a notificatoin to the scene
    pub fn notify(&mut self, notification: SceneNotification) {
        match notification {
            SceneNotification::NewCamera(position, projection) => {
                self.controller.teleport_camera(position, projection);
                self.update.camera_update = true;
            }
            SceneNotification::CameraMoved => self.update.camera_update = true,
            SceneNotification::NewSize(window_size, area) => {
                self.area = area;
                self.resize(window_size);
            }
        };
        self.update.need_update = true;
    }

    fn resize(&mut self, window_size: PhySize) {
        self.view.borrow_mut().update(ViewUpdate::Size(window_size));
        self.controller.resize(window_size, self.area.size);
        self.update.camera_update = true;
        self.element_selector
            .resize(self.controller.get_window_size(), self.area);
    }
}

impl Application for Scene {
    fn on_notify(&mut self, notification: Notification) {
        match notification {
            Notification::DesignNotification(notification) => {
                self.handle_design_notification(notification)
            }
            Notification::AppNotification(_) => (),
            Notification::NewDesign(design) => self.add_design(design),
            Notification::ClearDesigns => self.clear_design(),
            Notification::ToggleText(value) => self.view.borrow_mut().set_draw_letter(value),
        }
    }
}

impl Scene {
    fn handle_design_notification(&mut self, notification: DesignNotification) {
        let design_id = notification.design_id;
        match notification.content {
            DesignNotificationContent::ModelChanged(matrix) => {
                self.update.need_update = true;
                self.data.borrow_mut().notify_matrices_update();
                self.view
                    .borrow_mut()
                    .update_model_matrix(design_id, matrix)
            }
            DesignNotificationContent::InstanceChanged => {
                self.data.borrow_mut().notify_instance_update()
            }
        }
    }
}
