use crate::PhySize;
use iced_winit::winit;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
};

mod layout_manager;
use layout_manager::LayoutTree;

/// A structure that represents an area on which an element can be drawn
#[derive(Clone, Copy, Debug)]
pub struct DrawArea {
    /// The top left corner of the element
    pub position: PhysicalPosition<u32>,
    /// The *physical* size of the element
    pub size: PhySize,
}

/// The different elements represented on the scene. Each element is instanciated once.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ElementType {
    /// The top menu bar
    TopBar,
    /// The 3D scene
    Scene,
    /// The Left Panel
    LeftPanel,
    /// An overlay area
    GridPanel,
    Overlay(usize),
    /// An area that has not been attributed to an element
    Unattributed,
}

impl ElementType {
    pub fn need_shift(&self) -> bool {
        match self {
            ElementType::Scene | ElementType::GridPanel | ElementType::Overlay(_) => true,
            _ => false
        }
    }
}

/// A structure that handles the division of the window into different `DrawArea`
pub struct Multiplexer {
    /// The *physical* size of the window
    pub window_size: PhySize,
    /// The scale factor of the window
    pub scale_factor: f64,
    /// The object mapping pixels to drawing areas
    layout_manager: LayoutTree,
    /// The Element on which the mouse cursor is currently on.
    focus: Option<ElementType>,
    /// `true` if the left button of the mouse was pressed on the window, not released since and
    /// the cursor has not left the window since
    mouse_clicked: bool,
    /// The *physical* position of the cursor on the focus area
    cursor_position: PhysicalPosition<f64>,
    /// The area that are drawn on top of the application
    overlays: Vec<Overlay>,
}

impl Multiplexer {
    /// Create a new multiplexer for a window with size `window_size`.
    pub fn new(window_size: PhySize, scale_factor: f64) -> Self {
        let mut layout_manager = LayoutTree::new();
        let (top_bar, scene) = layout_manager.vsplit(0, 0.05);
        let (left_pannel, scene) = layout_manager.hsplit(scene, 0.2);
        let (scene, grid_panel) = layout_manager.hsplit(scene, 0.8);
        layout_manager.attribute_element(top_bar, ElementType::TopBar);
        layout_manager.attribute_element(scene, ElementType::Scene);
        layout_manager.attribute_element(left_pannel, ElementType::LeftPanel);
        layout_manager.attribute_element(grid_panel, ElementType::GridPanel);
        Self {
            window_size,
            scale_factor,
            layout_manager,
            focus: None,
            mouse_clicked: false,
            cursor_position: PhysicalPosition::new(-1., -1.),
            overlays: Vec::new(),
        }
    }

    /// Return the drawing area attributed to an element.
    pub fn get_draw_area(&self, element_type: ElementType) -> DrawArea {
        use ElementType::Overlay;
        let (position, size) = if let Overlay(n) = element_type {
            (self.overlays[n].position,
             self.overlays[n].size)
        } else {
            let (left, top, right, bottom) = self.layout_manager.get_area(element_type);
            let top = top * self.window_size.height as f64;
            let left = left * self.window_size.width as f64;
            let bottom = bottom * self.window_size.height as f64;
            let right = right * self.window_size.width as f64;

            (PhysicalPosition::new(left, top).cast::<u32>(),
            PhysicalSize::new(right - left, bottom - top).cast::<u32>(),
            )
        };
        DrawArea {
            position,
            size,
        }
    }

    /// Forwards event to the elment on which they happen.
    pub fn event(
        &mut self,
        event: WindowEvent<'static>,
    ) -> Option<(WindowEvent<'static>, ElementType)> {
        let mut focus_changed = false;
        let mut device_id_msg = None;
        match &event {
            WindowEvent::CursorMoved {
                position,
                device_id,
                ..
            } => {
                let &PhysicalPosition { x, y } = position;
                if x > 0.0 || y > 0.0 {
                    let element = self.pixel_to_element(*position);
                    let area = self.get_draw_area(element);

                    if !self.mouse_clicked {
                        self.focus = Some(element);
                    } else if self.focus != Some(element) {
                        focus_changed = true;
                        device_id_msg = Some(*device_id);
                    }
                    if self.foccused_element().filter(|elt_type| elt_type.need_shift()).is_some() {
                        self.cursor_position.x = position.x - area.position.cast::<f64>().x;
                        self.cursor_position.y = position.y - area.position.cast::<f64>().y;
                    } else {
                        self.cursor_position.x = position.x;
                        self.cursor_position.y = position.y;
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                self.window_size = *new_size;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = *scale_factor;
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
            if focus_changed {
                Some((
                        WindowEvent::CursorLeft {
                            device_id: device_id_msg.unwrap(),
                        },
                        focus,
                ))
            } else {
                Some((event, focus))
            }
        } else {
            None
        }
    }

    /// Maps *physical* pixels to an element
    fn pixel_to_element(&self, pixel: PhysicalPosition<f64>) -> ElementType {
        let pixel_u32 = pixel.cast::<u32>();
        for (n, overlay) in self.overlays.iter().enumerate() {
            if overlay.contains_pixel(pixel_u32) {
                return ElementType::Overlay(n)
            }
        }
        self.layout_manager.get_area_pixel(
            pixel.x / self.window_size.width as f64,
            pixel.y / self.window_size.height as f64,
        )
    }

    /// Get the drawing area attributed to an element.
    pub fn get_element_area(&self, element: ElementType) -> DrawArea {
        self.get_draw_area(element)
    }

    /// Return the *physical* position of the cursor, in the foccused element coordinates
    pub fn get_cursor_position(&self) -> PhysicalPosition<f64> {
        self.cursor_position
    }

    /// Return the foccused element
    pub fn foccused_element(&self) -> Option<ElementType> {
        self.focus
    }

    pub fn set_overlays(&mut self, overlays: Vec<Overlay>) {
        self.overlays = overlays
    }
}

#[derive(Clone)]
pub struct Overlay {
    pub position: PhysicalPosition<u32>,
    pub size: PhysicalSize<u32>,
}

impl Overlay {
    pub fn contains_pixel(&self, pixel: PhysicalPosition<u32>) -> bool {
        pixel.x >= self.position.x 
            && pixel.y >= self.position.y 
            && pixel.x < self.position.x + self.size.width 
            && pixel.y < self.position.y + self.size.height 
    }
}
