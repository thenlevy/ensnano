use std::cell::RefCell;
use std::rc::Rc;

use crate::design::Design;
use crate::mediator;
use crate::{DrawArea, PhySize, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit;
use mediator::{ActionMode, Application, Notification};
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

mod view;
mod data;
mod controller;
use crate::utils::camera2d as camera;

use camera::{Camera, Globals};
type CameraPtr = Rc<RefCell<Camera>>;
use view::View;
type ViewPtr = Rc<RefCell<View>>;
use data::Data;
type DataPtr = Rc<RefCell<Data>>;
use controller::Controller;


/// The application that draws a grid
pub struct GridPanel {
    view: ViewPtr,
    data: DataPtr,
    controller: Controller,
    /// The area on which the flatscene is displayed
    area: DrawArea,
    /// The size of the window on which the flatscene is displayed
    window_size: PhySize,
    device: Rc<Device>,
    queue: Rc<Queue>,
}

impl GridPanel {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhySize, area: DrawArea) -> Self {
        let globals = Globals {
            resolution: [area.size.width as f32, area.size.height as f32],
            scroll_offset: [0., 0.],
            zoom: 100.,
            _padding: 0.,
        };
        let camera = Rc::new(RefCell::new(Camera::new(globals)));
        let view = Rc::new(RefCell::new(View::new(device.clone(), queue.clone(), window_size, camera.clone())));
        let data = Rc::new(RefCell::new(Data::new(view.clone())));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller,
            area,
            window_size,
            device,
            queue,
        }
    }

    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.view.borrow_mut().draw(
            encoder,
            target,
            self.area,
        );
    }

}
