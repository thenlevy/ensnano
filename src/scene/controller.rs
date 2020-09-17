use super::{ Duration, ViewPtr, camera };
use crate::{ WindowEvent, PhysicalPosition, PhySize};
use iced_winit::winit::event::*;
use ultraviolet::{Rotor3, Vec3};

use camera::CameraController;

pub struct Controller {
    view: ViewPtr,
    camera_controller: CameraController,
    last_clicked_position: Option<PhysicalPosition<f64>>,
    mouse_position: PhysicalPosition<f64>,
    window_size: PhySize,
}

const NO_POS: PhysicalPosition<f64> = PhysicalPosition::new(f64::NAN, f64::NAN);

pub enum Consequence {
    CameraMoved,
    PixelSelected(PhysicalPosition<f64>),
    Nothing,
}

impl Controller {
    pub fn new(view: ViewPtr, window_size: PhySize) -> Self {
        let camera_controller = {
            let view = view.borrow();
            CameraController::new(4.0, 0.04, view.get_camera(), view.get_projection())
        };
        Self {
            view,
            camera_controller,
            last_clicked_position: None,
            mouse_position: PhysicalPosition::new(0., 0.),
            window_size,
        }
    }

    pub fn teleport_camera(&mut self, position: Vec3, rotation: Rotor3) {
        self.camera_controller.teleport_camera(position, rotation)
    }

    pub fn input(
        &mut self,
        event: &WindowEvent,
    ) -> Consequence {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => {
                if self.camera_controller.process_keyboard(*key, *state) {
                    Consequence::CameraMoved
                } else {
                    Consequence::Nothing
                }
            },
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                Consequence::CameraMoved
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.camera_controller.process_click(state);
                if *state == ElementState::Pressed {
                    self.last_clicked_position = Some(self.mouse_position);
                } else if position_difference(self.last_clicked_position.unwrap_or(NO_POS), self.mouse_position) < 5.
                {
                    return Consequence::PixelSelected(self.last_clicked_position.take().unwrap())
                } else {
                    self.last_clicked_position = None;
                }
                if self.last_clicked_position.is_some() {
                    Consequence::CameraMoved
                } else {
                    Consequence::Nothing
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = *position;
                if let Some(clicked_position) = self.last_clicked_position {
                    let mouse_dx =
                        (position.x - clicked_position.x) / self.window_size.width as f64;
                    let mouse_dy =
                        (position.y - clicked_position.y) / self.window_size.height as f64;
                    self.camera_controller.process_mouse(mouse_dx, mouse_dy);
                    Consequence::CameraMoved
                } else {
                    Consequence::Nothing
                }
            }
            _ => Consequence::Nothing,
        }
    }

    pub fn camera_is_moving(&self) -> bool {
        self.camera_controller.is_moving()
    }

    pub fn set_middle_point(&mut self, point: Vec3) {
        self.camera_controller.set_middle_point(point)
    }

    pub fn update_camera(&mut self, dt: Duration) {
        self.camera_controller.update_camera(dt);
    }

    pub fn resize(&mut self, window_size: PhySize) {
        self.window_size = window_size;
        self.camera_controller.resize(window_size);
        self.view.borrow_mut().update(super::view::ViewUpdate::Size(window_size));
    }

    pub fn get_window_size(&self) -> PhySize {
        self.window_size
    }
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

