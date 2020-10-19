//! This module handles the 2D view


use iced_wgpu::wgpu;
use wgpu::{Device, Queue};
use iced_winit::winit;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crate::{DrawArea, PhySize};


mod view;
mod controller;
mod data;
use data::Data;
use view::View;
use controller::Controller;

type ViewPtr = Rc<RefCell<View>>;
type DataPtr = Rc<RefCell<Data>>;

pub struct FlatScene {
    view: ViewPtr,
    data: DataPtr,
    controller: Controller,
    area: DrawArea,
}

impl FlatScene {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, window_size: PhySize, area: DrawArea) -> Self {
        let view = Rc::new(RefCell::new(View::new(device, queue, window_size)));
        let data = Rc::new(RefCell::new(Data::new(view.clone())));
        let controller = Controller::new(view.clone(), data.clone());
        Self {
            view,
            data,
            controller,
            area
        }
    }

    pub fn draw_view(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.view.borrow_mut().draw(
            encoder,
            target,
            self.area,
        );
    }
}

