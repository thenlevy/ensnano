use super::{CameraPtr, DataPtr, PhySize, PhysicalPosition, ViewPtr, WindowEvent};
use iced_winit::winit::event::*;

const NO_POS: PhysicalPosition<f64> = PhysicalPosition::new(f64::NAN, f64::NAN);

pub struct Controller {
    view: ViewPtr,
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
    #[allow(dead_code)]
    MovementEnded,
    Clicked(f32, f32),
    Translated(f32, f32),
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
                    let attempt_select = position_difference(self.last_left_clicked_position.unwrap_or(NO_POS), self.mouse_position) < 5.;
                    self.last_left_clicked_position = None;
                    self.camera.borrow_mut().end_movement();
                    if attempt_select {
                        let (x, y) = self.camera.borrow().screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                        Consequence::Clicked(x, y)
                    } else {
                        Consequence::Nothing
                    }
                } else {
                    self.last_left_clicked_position = Some(self.mouse_position);
                    Consequence::Nothing
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if let Some(clicked_position) = self.last_left_clicked_position {
                    match self.state {
                        State::Normal => {
                            let mouse_dx = (position.x - clicked_position.x) / self.area_size.width as f64;
                            let mouse_dy = (position.y - clicked_position.y) / self.area_size.height as f64;
                            self.camera
                            .borrow_mut()
                            .process_mouse(mouse_dx as f32, mouse_dy as f32);
                            Consequence::Nothing
                        }
                        State::Translating => {
                            let (mouse_dx, mouse_dy) = {
                                let (x, y) = self.camera.borrow().screen_to_world(position.x as f32, position.y as f32);
                                let (old_x, old_y) = self.camera.borrow().screen_to_world(clicked_position.x as f32, clicked_position.y as f32);
                                (x - old_x, y - old_y)
                            };

                            Consequence::Translated(mouse_dx as f32, mouse_dy as f32)
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
                _ => Consequence::Nothing ,
            },
            _ => Consequence::Nothing,
        }
    }

    pub fn notify_select(&mut self) {
        self.state = State::Translating
    } 

    pub fn notify_unselect(&mut self) {
        self.state = State::Normal
    }
    
}

enum State {
    Normal,
    Translating,
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}
