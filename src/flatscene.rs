//! This module handles the 2D view

use crate::{DrawArea, PhySize};
use iced_wgpu::wgpu;
use iced_winit::winit;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue};

mod controller;
mod data;
mod view;
use controller::Controller;
use data::{Helix, Data};
use view::View;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;

pub struct FlatScene {
    view: ViewPtr,
    data: DataPtr,
    controller: Controller,
    area: DrawArea,
    globals: Globals,
}

impl FlatScene {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhySize, area: DrawArea) -> Self {
        let globals = Globals {
            resolution: [area.size.width as f32, area.size.height as f32],
            scroll_offset: [0. ,0.],
            zoom: 1.
        };
        let view = Rc::new(RefCell::new(View::new(device, queue, window_size, &globals)));
        let data = Rc::new(RefCell::new(Data::new(view.clone())));
        let controller = Controller::new(view.clone(), data.clone());
        view.borrow_mut().add_helix(Helix::new(3, 10, ultraviolet::Vec2::new(1., 1.)));
        Self {
            globals,
            view,
            data,
            controller,
            area,
        }
    }

    pub fn draw_view(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.view.borrow_mut().draw(encoder, target, self.area);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Globals {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
}

unsafe impl bytemuck::Zeroable for Globals {}
unsafe impl bytemuck::Pod for Globals {}
