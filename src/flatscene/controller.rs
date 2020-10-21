use super::{CameraPtr, DataPtr, PhySize, PhysicalPosition, ViewPtr, WindowEvent};
use iced_winit::winit::event::*;

pub struct Controller {
    view: ViewPtr,
    data: DataPtr,
    last_left_clicked_position: Option<PhysicalPosition<f64>>,
    mouse_position: PhysicalPosition<f64>,
    window_size: PhySize,
    area_size: PhySize,
    camera: CameraPtr,
}

#[allow(dead_code)]
pub enum Consequence {
    GlobalsChanged,
    Nothing,
    MovementEnded,
}

impl Controller {
    pub fn new(
        view: ViewPtr,
        data: DataPtr,
        window_size: PhySize,
        area_size: PhySize,
        camera: CameraPtr,
    ) -> Self {
        Self {
            view,
            data,
            window_size,
            area_size,
            last_left_clicked_position: None,
            mouse_position: PhysicalPosition::new(-1., -1.),
            camera,
        }
    }

    pub fn resize(&mut self, window_size: PhySize, area_size: PhySize) {
        self.area_size = area_size;
        self.window_size = window_size;
        self.camera
            .borrow_mut()
            .resize(area_size.width as f32, area_size.height as f32);
    }

    pub fn input(&mut self, event: &WindowEvent, position: PhysicalPosition<f64>) {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                if *state == ElementState::Released {
                    self.last_left_clicked_position = None;
                    self.camera.borrow_mut().end_movement();
                } else {
                    self.last_left_clicked_position = Some(self.mouse_position);
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if let Some(clicked_position) = self.last_left_clicked_position {
                    let mouse_dx = (position.x - clicked_position.x) / self.area_size.width as f64;
                    let mouse_dy = (position.y - clicked_position.y) / self.area_size.height as f64;
                    self.camera
                        .borrow_mut()
                        .process_mouse(mouse_dx as f32, mouse_dy as f32);
                }
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match *key {
                VirtualKeyCode::Up => {
                    self.camera.borrow_mut().zoom_in();
                }
                VirtualKeyCode::Down => {
                    self.camera.borrow_mut().zoom_out();
                }
                _ => (),
            },
            _ => (),
        }
    }
}
