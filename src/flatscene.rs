//! This module handles the 2D view

use crate::{DrawArea, PhySize, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue};
use winit::dpi::PhysicalPosition;

mod camera;
mod controller;
mod data;
mod view;
use camera::{Camera, Globals};
use controller::Controller;
use data::{Data, Helix};
use view::View;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;
type CameraPtr = Rc<RefCell<Camera>>;

pub struct FlatScene {
    view: ViewPtr,
    data: DataPtr,
    controller: Controller,
    area: DrawArea,
}

impl FlatScene {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhySize, area: DrawArea) -> Self {
        let globals = Globals {
            resolution: [area.size.width as f32, area.size.height as f32],
            scroll_offset: [0., 0.],
            zoom: 100.,
            _padding: 0.,
        };
        let camera = Rc::new(RefCell::new(Camera::new(globals)));
        let view = Rc::new(RefCell::new(View::new(
            device,
            queue,
            window_size,
            camera.clone(),
        )));
        let data = Rc::new(RefCell::new(Data::new(view.clone())));
        let controller =
            Controller::new(view.clone(), data.clone(), window_size, area.size, camera);
        view.borrow_mut()
            .add_helix(Helix::new(3, 10, ultraviolet::Vec2::new(0., 0.)));
        Self {
            view,
            data,
            controller,
            area,
        }
    }

    pub fn draw_view(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.view.borrow_mut().draw(encoder, target, self.area);
    }

    pub fn input(&mut self, event: &WindowEvent, cursor_position: PhysicalPosition<f64>) {
        self.controller.input(event, cursor_position);
    }

    pub fn needs_redraw(&self) -> bool {
        self.view.borrow().needs_redraw()
    }
}
