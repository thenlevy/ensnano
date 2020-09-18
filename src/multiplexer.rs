use crate::PhySize;
use iced_winit::winit;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, ModifiersState, MouseButton, WindowEvent},
};

mod layout_manager;
use layout_manager::LayoutTree;

#[derive(Clone, Copy, Debug)]
pub struct DrawArea {
    pub position: PhysicalPosition<u32>,
    pub size: PhySize,
}

pub struct Multiplexer {
    pub window_size: PhySize,
    layout_manager: LayoutTree,
    pub top_bar: usize,
    pub scene: usize,
    focus: Option<usize>,
    mouse_clicked: bool,
    pub scene_cursor_position: PhysicalPosition<f64>,
    pub top_bar_cursor_position: PhysicalPosition<f64>,
}

impl Multiplexer {
    pub fn new(window_size: PhySize) -> Self {
        let mut layout_manager = LayoutTree::new();
        let (top_bar, scene) = layout_manager.vsplit(0, 0.05);
        Self {
            window_size,
            layout_manager,
            top_bar,
            scene,
            focus: None,
            mouse_clicked: false,
            scene_cursor_position: PhysicalPosition::new(-1., -1.),
            top_bar_cursor_position: PhysicalPosition::new(-1., -1.),
        }
    }

    pub fn get_draw_area(&self, area: usize) -> DrawArea {
        let (left, top, right, bottom) = self.layout_manager.get_area(area);
        let top = top * self.window_size.height as f64;
        let left = left * self.window_size.width as f64;
        let bottom = bottom * self.window_size.height as f64;
        let right = right * self.window_size.width as f64;

        DrawArea {
            position: PhysicalPosition::new(left as u32, top as u32),
            size: PhysicalSize::new((right - left) as u32, (bottom - top) as u32),
        }
    }
    pub fn event(&mut self, event: WindowEvent<'static>) -> Option<(WindowEvent<'static>, usize)> {
        match &event {
            WindowEvent::CursorMoved { position, .. } => {
                let &PhysicalPosition { x, y } = position;
                if x > 0.0 || y > 0.0 {
                    let area = self.pixel_to_area(x, y);
                    if !self.mouse_clicked {
                        self.focus = Some(area);
                    }
                    if self.focus == Some(self.top_bar) {
                        self.top_bar_cursor_position = *position;
                    } else if self.focus == Some(self.scene) {
                        let scene_area = self.get_scene_area();
                        self.scene_cursor_position = *position;
                        self.scene_cursor_position.x -= scene_area.position.x as f64;
                        self.scene_cursor_position.y -= scene_area.position.y as f64;
                    }
                }
            }
            /*WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = *new_modifiers;
            }*/
            WindowEvent::Resized(new_size) => {
                self.window_size = *new_size;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => match state {
                ElementState::Pressed => self.mouse_clicked = true,
                ElementState::Released => self.mouse_clicked = false,
            },
            _ => {}
        }

        if let Some(focus) = self.focus {
            Some((event, focus))
        } else {
            None
        }
    }

    fn pixel_to_area(&self, x: f64, y: f64) -> usize {
        self.layout_manager.get_area_pixel(
            x / self.window_size.width as f64,
            y / self.window_size.height as f64,
        )
    }

    pub fn get_scene_area(&self) -> DrawArea {
        self.get_draw_area(self.scene)
    }

    pub fn get_top_bar_area(&self) -> DrawArea {
        self.get_draw_area(self.top_bar)
    }
}
