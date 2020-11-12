/// Draw the top bar of the GUI
pub mod top_bar;
pub use top_bar::TopBar;
/// Draw the left pannel of the GUI
pub mod left_panel;
pub use left_panel::{ColorOverlay, LeftPanel};

use crate::mediator::{ActionMode, SelectionMode};
use crate::SplitMode;
use crate::{DrawArea, ElementType, IcedMessages, Multiplexer};
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::Device;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

/// A structure that contains all the requests that can be made through the GUI.
pub struct Requests {
    /// A change of the rotation mode
    pub action_mode: Option<ActionMode>,
    /// A change of the selection mode
    pub selection_mode: Option<SelectionMode>,
    /// A request to move the camera so that the frustrum fits the desgin
    pub fitting: bool,
    /// A request to load a design into the scene
    pub file_add: Option<PathBuf>,
    /// A request to remove all designs
    pub file_clear: bool,
    /// A request to save the selected design
    pub file_save: Option<PathBuf>,
    /// A request to change the color of the selcted strand
    pub strand_color_change: Option<u32>,
    /// A request to change the sequence of the selected strand
    pub sequence_change: Option<String>,
    /// A request to show/hide the sequences
    pub toggle_text: Option<bool>,
    /// A request to change the view
    pub toggle_scene: Option<SplitMode>,
    /// A request to change the sensitivity of scrolling
    pub scroll_sensitivity: Option<f32>,
    pub make_grids: bool,
    pub overlay_closed: Option<OverlayType>,
    pub overlay_opened: Option<OverlayType>,
}

impl Requests {
    /// Initialise the request structures with no requests
    pub fn new() -> Self {
        Self {
            action_mode: None,
            selection_mode: None,
            fitting: false,
            file_add: None,
            file_clear: false,
            file_save: None,
            strand_color_change: None,
            sequence_change: None,
            toggle_text: None,
            toggle_scene: None,
            scroll_sensitivity: None,
            make_grids: false,
            overlay_closed: None,
            overlay_opened: None,
        }
    }
}

#[derive(PartialEq)]
pub enum OverlayType {
    Color,
}

pub struct Gui {
    top_bar_state: iced_winit::program::State<TopBar>,
    top_bar_debug: Debug,
    redraw_top_bar: bool,
    left_panel_state: iced_winit::program::State<LeftPanel>,
    left_panel_debug: Debug,
    redraw_left_panel: bool,
    renderer: iced_wgpu::Renderer,
    device: Rc<Device>,
    resized: bool,
}

impl Gui {
    pub fn new(
        device: Rc<Device>,
        window: &Window,
        multiplexer: &Multiplexer,
        requests: Arc<Mutex<Requests>>,
    ) -> Self {
        let mut renderer = Renderer::new(Backend::new(device.as_ref(), Settings::default()));
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
        let top_bar = TopBar::new(
            requests.clone(),
            top_bar_area.size.to_logical(window.scale_factor()),
        );
        let mut top_bar_debug = Debug::new();
        let top_bar_state = program::State::new(
            top_bar,
            convert_size(top_bar_area.size),
            conversion::cursor_position(cursor_position, window.scale_factor()),
            &mut renderer,
            &mut top_bar_debug,
        );

        // Left panel
        let left_panel_area = multiplexer
            .get_element_area(ElementType::LeftPanel)
            .unwrap();
        let left_panel = LeftPanel::new(
            requests.clone(),
            left_panel_area.size.to_logical(window.scale_factor()),
            left_panel_area.position.to_logical(window.scale_factor()),
        );
        let mut left_panel_debug = Debug::new();
        let left_panel_state = program::State::new(
            left_panel,
            convert_size(left_panel_area.size),
            conversion::cursor_position(cursor_position, window.scale_factor()),
            &mut renderer,
            &mut left_panel_debug,
        );

        Self {
            top_bar_state,
            top_bar_debug,
            redraw_top_bar: true,
            left_panel_state,
            left_panel_debug,
            redraw_left_panel: true,
            renderer,
            device,
            resized: true,
        }
    }

    pub fn forward_event(&mut self, area: ElementType, event: iced_native::Event) {
        match area {
            ElementType::TopBar => self.top_bar_state.queue_event(event),
            ElementType::LeftPanel => self.left_panel_state.queue_event(event),
            _ => unreachable!(),
        }
    }

    pub fn forward_messages(&mut self, messages: &mut IcedMessages) {
        for m in messages.top_bar.drain(..) {
            self.top_bar_state.queue_message(m);
        }
        for m in messages.left_panel.drain(..) {
            self.left_panel_state.queue_message(m);
        }
    }

    pub fn resize(&mut self, multiplexer: &Multiplexer, window: &Window) {
        let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
        self.top_bar_state.queue_message(top_bar::Message::Resize(
            top_bar_area.size.to_logical(window.scale_factor()),
        ));

        let left_panel_area = multiplexer
            .get_element_area(ElementType::LeftPanel)
            .unwrap();
        self.left_panel_state
            .queue_message(left_panel::Message::Resized(
                left_panel_area.size.to_logical(window.scale_factor()),
                left_panel_area.position.to_logical(window.scale_factor()),
            ));
        self.resized = true;
        self.redraw_top_bar = true;
        self.redraw_left_panel = true;
    }

    pub fn fetch_change(&mut self, window: &Window, multiplexer: &Multiplexer) {
        let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
        let top_bar_cursor = if multiplexer.foccused_element() == Some(ElementType::TopBar) {
            multiplexer.get_cursor_position()
        } else {
            PhysicalPosition::new(-1., -1.)
        };
        if !self.top_bar_state.is_queue_empty() {
            // We update iced
            self.redraw_top_bar = true;
            let _ = self.top_bar_state.update(
                convert_size(top_bar_area.size),
                conversion::cursor_position(top_bar_cursor, window.scale_factor()),
                None,
                &mut self.renderer,
                &mut self.top_bar_debug,
            );
        }

        let left_panel_cursor = if multiplexer.foccused_element() == Some(ElementType::LeftPanel) {
            multiplexer.get_cursor_position()
        } else {
            PhysicalPosition::new(-1., -1.)
        };
        if !self.left_panel_state.is_queue_empty() {
            self.redraw_left_panel = true;
            let _ = self.left_panel_state.update(
                convert_size(window.inner_size()),
                conversion::cursor_position(left_panel_cursor, window.scale_factor()),
                None,
                &mut self.renderer,
                &mut self.top_bar_debug,
            );
        }
    }

    pub fn update(&mut self, multiplexer: &Multiplexer, window: &Window) {
        let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
        let top_bar_cursor = if multiplexer.foccused_element() == Some(ElementType::TopBar) {
            multiplexer.get_cursor_position()
        } else {
            PhysicalPosition::new(-1., -1.)
        };
        let left_panel_cursor = if multiplexer.foccused_element() == Some(ElementType::LeftPanel) {
            multiplexer.get_cursor_position()
        } else {
            PhysicalPosition::new(-1., -1.)
        };
        if !self.top_bar_state.is_queue_empty() || self.resized {
            // We update iced
            let _ = self.top_bar_state.update(
                convert_size(top_bar_area.size),
                conversion::cursor_position(top_bar_cursor, window.scale_factor()),
                None,
                &mut self.renderer,
                &mut self.top_bar_debug,
            );
        }

        if !self.left_panel_state.is_queue_empty() || self.resized {
            let _ = self.left_panel_state.update(
                convert_size(window.inner_size()),
                conversion::cursor_position(left_panel_cursor, window.scale_factor()),
                None,
                &mut self.renderer,
                &mut self.left_panel_debug,
            );
        }

        self.resized = false;
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        multiplexer: &Multiplexer,
        staging_belt: &mut wgpu::util::StagingBelt,
        mouse_interaction: &mut iced::mouse::Interaction,
    ) {
        if self.redraw_left_panel {
            let viewport_left_panel = Viewport::with_physical_size(
                convert_size_u32(
                    multiplexer
                        .get_element_area(ElementType::LeftPanel)
                        .unwrap()
                        .size,
                ),
                window.scale_factor(),
            );
            let _left_panel_interaction = self.renderer.backend_mut().draw(
                self.device.as_ref(),
                staging_belt,
                encoder,
                multiplexer
                    .get_texture_view(ElementType::LeftPanel)
                    .unwrap(),
                &viewport_left_panel,
                self.left_panel_state.primitive(),
                &self.left_panel_debug.overlay(),
            );
            self.redraw_left_panel = false;
        }

        if self.redraw_top_bar {
            let viewport_top_bar = Viewport::with_physical_size(
                convert_size_u32(
                    multiplexer
                        .get_element_area(ElementType::TopBar)
                        .unwrap()
                        .size,
                ),
                window.scale_factor(),
            );
            *mouse_interaction = self.renderer.backend_mut().draw(
                self.device.as_ref(),
                staging_belt,
                encoder,
                multiplexer.get_texture_view(ElementType::TopBar).unwrap(),
                &viewport_top_bar,
                self.top_bar_state.primitive(),
                &self.top_bar_debug.overlay(),
            );
            self.redraw_top_bar = false;
        }
    }
}

fn convert_size(size: PhysicalSize<u32>) -> Size<f32> {
    Size::new(size.width as f32, size.height as f32)
}

fn convert_size_u32(size: PhysicalSize<u32>) -> Size<u32> {
    Size::new(size.width, size.height)
}
