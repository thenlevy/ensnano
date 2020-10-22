use super::{CameraPtr, DataPtr, PhySize, PhysicalPosition, ViewPtr, WindowEvent};
use iced_winit::winit::event::*;
use ultraviolet::Vec2;

pub struct Controller {
    #[allow(dead_code)]
    view: ViewPtr,
    #[allow(dead_code)]
    data: DataPtr,
    last_left_clicked_position: Option<PhysicalPosition<f64>>,
    mouse_position: PhysicalPosition<f64>,
    window_size: PhySize,
    area_size: PhySize,
    camera: CameraPtr,
    state: State,
}

pub enum Consequence {
    #[allow(dead_code)]
    GlobalsChanged,
    Nothing,
    MovementEnded,
    Clicked(f32, f32),
    Translated(f32, f32),
    Rotated(Vec2, f32),
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
            state: State::Normal,
        }
    }

    pub fn resize(&mut self, window_size: PhySize, area_size: PhySize) {
        self.area_size = area_size;
        self.window_size = window_size;
        self.camera
            .borrow_mut()
            .resize(area_size.width as f32, area_size.height as f32);
    }

    pub fn input(&mut self, event: &WindowEvent, position: PhysicalPosition<f64>) -> Consequence {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                if *state == ElementState::Released {
                    self.last_left_clicked_position = None;
                    self.camera.borrow_mut().end_movement();
                    Consequence::MovementEnded
                } else {
                    self.last_left_clicked_position = Some(self.mouse_position);
                    let (x, y) = self.camera.borrow().screen_to_world(
                        self.mouse_position.x as f32,
                        self.mouse_position.y as f32,
                    );
                    Consequence::Clicked(x, y)
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if let Some(clicked_position) = self.last_left_clicked_position {
                    match self.state {
                        State::Normal => {
                            let mouse_dx =
                                (position.x - clicked_position.x) / self.area_size.width as f64;
                            let mouse_dy =
                                (position.y - clicked_position.y) / self.area_size.height as f64;
                            self.camera
                                .borrow_mut()
                                .process_mouse(mouse_dx as f32, mouse_dy as f32);
                            Consequence::Nothing
                        }
                        State::Translating => {
                            let (mouse_dx, mouse_dy) = {
                                let (x, y) = self
                                    .camera
                                    .borrow()
                                    .screen_to_world(position.x as f32, position.y as f32);
                                let (old_x, old_y) = self.camera.borrow().screen_to_world(
                                    clicked_position.x as f32,
                                    clicked_position.y as f32,
                                );
                                (x - old_x, y - old_y)
                            };

                            Consequence::Translated(mouse_dx as f32, mouse_dy as f32)
                        }
                        State::Rotating(pivot) => {
                            let angle = {
                                let (x, y) = self
                                    .camera
                                    .borrow()
                                    .screen_to_world(position.x as f32, position.y as f32);
                                let (old_x, old_y) = self.camera.borrow().screen_to_world(
                                    clicked_position.x as f32,
                                    clicked_position.y as f32,
                                );
                                (y - pivot.y).atan2(x - pivot.x)
                                    - (old_y - pivot.y).atan2(old_x - pivot.x)
                            };
                            Consequence::Rotated(pivot, angle)
                        }
                    }
                } else {
                    Consequence::Nothing
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
                    Consequence::Nothing
                }
                VirtualKeyCode::Down => {
                    self.camera.borrow_mut().zoom_out();
                    Consequence::Nothing
                }
                _ => Consequence::Nothing,
            },
            _ => Consequence::Nothing,
        }
    }

    pub fn notify_select(&mut self) {
        self.state = State::Translating
    }

    pub fn set_pivot(&mut self, pivot: Vec2) {
        self.state = State::Rotating(pivot)
    }

    pub fn notify_unselect(&mut self) {
        self.state = State::Normal
    }
}

enum State {
    Normal,
    Translating,
    Rotating(Vec2),
}
