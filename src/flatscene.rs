//! This module handles the 2D view

use crate::design::Design;
use crate::mediator;
use crate::{DrawArea, Duration, PhySize, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit;
use mediator::{ActionMode, Application, Notification};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

use crate::utils::camera2d as camera;
mod controller;
mod data;
mod view;
use camera::{Camera, Globals};
use controller::Controller;
use data::Data;
use view::View;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;
type CameraPtr = Rc<RefCell<Camera>>;

/// A Flatscene handles one design at a time
pub struct FlatScene {
    /// Handle the data to send to the GPU
    view: Vec<ViewPtr>,
    /// Handle the data representing the design
    data: Vec<DataPtr>,
    /// Handle the inputs
    controller: Vec<Controller>,
    /// The area on which the flatscene is displayed
    area: DrawArea,
    /// The size of the window on which the flatscene is displayed
    window_size: PhySize,
    /// The identifer of the design being drawn
    selected_design: usize,
    device: Rc<Device>,
    queue: Rc<Queue>,
}

impl FlatScene {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhySize, area: DrawArea) -> Self {
        Self {
            view: Vec::new(),
            data: Vec::new(),
            controller: Vec::new(),
            area,
            window_size,
            selected_design: 0,
            device,
            queue,
        }
    }

    /// Add a design to the scene. This creates a new `View`, a new `Data` and a new `Controller`
    fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        let globals = Globals {
            resolution: [self.area.size.width as f32, self.area.size.height as f32],
            scroll_offset: [-1., -1.],
            zoom: 80.,
            _padding: 0.,
        };
        let camera = Rc::new(RefCell::new(Camera::new(globals)));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let view = Rc::new(RefCell::new(View::new(
            self.device.clone(),
            self.queue.clone(),
            self.area,
            camera.clone(),
            &mut encoder,
        )));
        self.queue.submit(Some(encoder.finish()));
        let data = Rc::new(RefCell::new(Data::new(view.clone(), design)));
        let controller = Controller::new(
            view.clone(),
            data.clone(),
            self.window_size,
            self.area.size,
            camera,
        );
        self.view.push(view);
        self.data.push(data);
        self.controller.push(controller);
    }

    /// Draw the view of the currently selected design
    fn draw_view(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        if let Some(view) = self.view.get(self.selected_design) {
            self.data[self.selected_design]
                .borrow_mut()
                .perform_update();
            view.borrow_mut().draw(encoder, target, self.area);
        }
    }

    /// This function must be called when the drawing area of the flatscene is modified
    fn resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.window_size = window_size;
        self.area = area;
        for view in self.view.iter() {
            view.borrow_mut().resize(area);
        }
        for controller in self.controller.iter_mut() {
            controller.resize(window_size, area.size);
        }
    }

    /// Change the action beign performed by the user
    fn change_action_mode(&mut self, action_mode: ActionMode) {
        if let Some(controller) = self.controller.get_mut(self.selected_design) {
            controller.set_action_mode(action_mode)
        }
    }

    /// Handle an input that happend while the cursor was on the flatscene drawing area
    fn input(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>) {
        if let Some(controller) = self.controller.get_mut(self.selected_design) {
            let consequence = controller.input(event, cursor_position);
            use controller::Consequence;
            match consequence {
                Consequence::Xover(nucl1, nucl2) => self.data[self.selected_design]
                    .borrow_mut()
                    .xover(nucl1, nucl2),
                Consequence::Cut(nucl) => self.data[self.selected_design]
                    .borrow_mut()
                    .split_strand(nucl),
                Consequence::FreeEnd(free_end) => self.data[self.selected_design]
                    .borrow_mut()
                    .set_free_end(free_end),
                _ => (),
            }
        }
    }

    /// Ask the view if it has been modified since the last drawing
    fn needs_redraw(&self) -> bool {
        if let Some(view) = self.view.get(self.selected_design) {
            self.data[self.selected_design]
                .borrow_mut()
                .perform_update();
            view.borrow().needs_redraw()
        } else {
            false
        }
    }
}

impl Application for FlatScene {
    fn on_notify(&mut self, notification: Notification) {
        #[allow(clippy::single_match)] // we will implement for notification in the future
        match notification {
            Notification::NewDesign(design) => self.add_design(design),
            Notification::NewActionMode(am) => self.change_action_mode(am),
            Notification::DesignNotification(_) => {
                self.data[self.selected_design].borrow_mut().notify_update()
            }
            Notification::FitRequest => self.controller[self.selected_design].fit(),
            _ => (),
        }
    }

    fn on_resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.resize(window_size, area)
    }

    fn on_event(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>) {
        self.input(event, cursor_position)
    }

    fn on_redraw_request(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _dt: Duration,
    ) {
        if self.needs_redraw() {
            self.draw_view(encoder, target)
        }
    }
}
