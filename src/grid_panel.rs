/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
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
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhySize, area: DrawArea, encoder: &mut wgpu::CommandEncoder) -> Self {
        let globals = Globals {
            resolution: [area.size.width as f32, area.size.height as f32],
            scroll_offset: [0., 0.],
            zoom: 10.,
            _padding: 0.,
        };
        let camera = Rc::new(RefCell::new(Camera::new(globals)));
        let view = Rc::new(RefCell::new(View::new(device.clone(), queue.clone(), area, camera.clone(), encoder)));
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

    pub fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        self.data.borrow_mut().add_design(design)
    }

    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.data.borrow().update_view();
        self.view.borrow_mut().draw(
            encoder,
            target,
            self.area,
        );
    }

    pub fn resize(&mut self, window_size: PhySize, area: DrawArea) {
        self.area = area;
        self.window_size = window_size;
        self.view.borrow_mut().resize(area);
    }

}

impl Application for GridPanel {
    fn on_notify(&mut self, notification: Notification) {
        #[allow(clippy::single_match)] // we will implement for notification in the future
        match notification {
            Notification::NewDesign(design) => self.add_design(design),
            _ => (),
        }
    }
}
