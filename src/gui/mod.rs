/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
//! The [Gui Manager](gui::Gui) handles redraw request on textures that corresponds to regions
//! attributed to GUI components and events happening on these regions.
//!
//! When a message is emmitted by a Gui component that have consequences that must be forwarded to
//! other components of the program it is forwarded to the [main](main) function via the
//! [Request](Requests) data structure.

/// Draw the top bar of the GUI
pub mod top_bar;
pub use top_bar::TopBar;
/// Draw the left pannel of the GUI
pub mod left_panel;
pub use left_panel::{
    ColorOverlay, HyperboloidRequest, LeftPanel, RigidBodyParametersRequest, SimulationRequest,
};
pub mod status_bar;
mod ui_size;
use crate::ApplicationState;
pub use ui_size::*;

use status_bar::StatusBar;

use crate::design::{DnaAttribute, DnaElementKey, GridTypeDescr};
use crate::mediator::{ActionMode, Background3D, Operation, RenderingMode, SelectionMode};
use crate::scene::FogParameters;
use crate::SplitMode;
use crate::{DrawArea, ElementType, IcedMessages, Multiplexer};
use ensnano_organizer::OrganizerTree;
use iced_native::Event;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, program, winit, Debug, Size};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ultraviolet::Vec3;
use wgpu::Device;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

pub trait Requests: 'static + Send {
    /// Show a pop up asking if the user want to use the default scaffold.
    fn ask_use_default_scaffold(&mut self);
    fn close_overlay(&mut self, overlay_type: OverlayType);
    fn open_overlay(&mut self, overlay_type: OverlayType);
    /// Change the color of the selected strands
    fn change_strand_color(&mut self, color: u32);
    /// Change the background of the 3D scene
    fn change_3d_background(&mut self, bg: Background3D);
    /// Change the rendering mode
    fn change_3d_rendering_mode(&mut self, rendering_mode: RenderingMode);
    /// Set the selected strand as the scaffold
    fn set_scaffold_from_selection(&mut self);
    /// Cancel the current hyperboloid construction
    fn cancel_hyperboloid(&mut self);
    /// Change the scrolling direction
    fn invert_scroll(&mut self, invert: bool);
    /// Resize all the 2D helices, or only the selected ones
    fn resize_2d_helices(&mut self, all: bool);
    /// Make all elements of the design visible
    fn make_all_elements_visible(&mut self);
    /// Toggle the visibility of the selected elements
    fn toggle_visibility(&mut self, visible: bool);
    /// Remove empty domains in the design
    fn remove_empty_domains(&mut self);
    fn change_action_mode(&mut self, action_mode: ActionMode);
    fn change_selection_mode(&mut self, selection_mode: SelectionMode);
    /// Switch widget basis between world and object
    fn toggle_widget_basis(&mut self);
    /// Show/hide the DNA sequences
    fn set_dna_sequences_visibility(&mut self, visible: bool);
    /// Download the stapples as an xlsx file
    fn download_stapples(&mut self);
    fn set_selected_strand_sequence(&mut self, sequence: String);
    fn set_scaffold_sequence(&mut self, sequence: String);
    fn set_scaffold_shift(&mut self, shift: usize);
    /// Change the size of the UI components
    fn set_ui_size(&mut self, size: UiSize);
    /// Finalize the currently eddited hyperboloid grid
    fn finalize_hyperboloid(&mut self);
    fn stop_roll_simulation(&mut self);
    fn start_roll_simulation(&mut self, roll_request: SimulationRequest);
    /// Make a grid from the set of selected helices
    fn make_grid_from_selection(&mut self);
    /// Start of Update the rigid helices simulation
    fn update_rigid_helices_simulation(&mut self, parameters: RigidBodyParametersRequest);
    /// Start of Update the rigid grids simulation
    fn update_rigid_grids_simulation(&mut self, parameters: RigidBodyParametersRequest);
    /// Update the parameters of the current simulation (rigid grids or helices)
    fn update_rigid_body_simulation_parameters(&mut self, parameters: RigidBodyParametersRequest);
    fn create_new_hyperboloid(&mut self, parameters: HyperboloidRequest);
    /// Update the parameters of the currently eddited hyperboloid grid
    fn update_current_hyperboloid(&mut self, parameters: HyperboloidRequest);
    fn update_roll_of_selected_helices(&mut self, roll: f32);
    fn update_scroll_sensitivity(&mut self, sensitivity: f32);
    fn set_fog_parameters(&mut self, parameters: FogParameters);
    /// Show/hide the torsion indications
    fn set_torsion_visibility(&mut self, visible: bool);
    /// Set the direction and up vector of the 3D camera
    fn set_camera_dir_up_vec(&mut self, direction: Vec3, up: Vec3);
    fn perform_camera_rotation(&mut self, xz: f32, yz: f32, xy: f32);
    /// Create a new grid in front of the 3D camera
    fn create_grid(&mut self, grid_type_descriptor: GridTypeDescr);
    fn set_candidates_keys(&mut self, candidates: Vec<DnaElementKey>);
    fn set_selected_keys(&mut self, selection: Vec<DnaElementKey>);
    fn update_organizer_tree(&mut self, tree: OrganizerTree<DnaElementKey>);
    /// Update one attribute of several Dna Elements
    fn update_attribute_of_elements(
        &mut self,
        attribute: DnaAttribute,
        keys: BTreeSet<DnaElementKey>,
    );
    fn change_split_mode(&mut self, split_mode: SplitMode);
    fn export_to_oxdna(&mut self);
    /// Split/Unsplit the 2D view
    fn toggle_2d_view_split(&mut self);
    fn undo(&mut self);
    fn redo(&mut self);
    /// Display the help message in the contextual panel, regardless of the selection
    fn force_help(&mut self);
    /// Show tutorial in the contextual panel
    fn show_tutorial(&mut self);
    fn new_design(&mut self);
    fn save_as(&mut self);
    fn open_file(&mut self);
    /// Adjust the 2D and 3D cameras so that the design fit in screen
    fn fit_design_in_scenes(&mut self);
    /// Update the parameters of the current operation
    fn update_current_operation(&mut self, operation: Arc<dyn Operation>);
    /// Update the shift of the currently seleced hyperbloid grid
    fn update_hyperboloid_shift(&mut self, shift: f32);
}

#[derive(PartialEq)]
pub enum OverlayType {
    Color,
}

enum GuiState<R: Requests> {
    TopBar(iced_winit::program::State<TopBar<R>>),
    LeftPanel(iced_winit::program::State<LeftPanel<R>>),
    StatusBar(iced_winit::program::State<StatusBar<R>>),
}

impl<R: Requests> GuiState<R> {
    fn queue_event(&mut self, event: Event) {
        match self {
            GuiState::TopBar(state) => state.queue_event(event),
            GuiState::LeftPanel(state) => state.queue_event(event),
            GuiState::StatusBar(state) => state.queue_event(event),
        }
    }

    fn queue_top_bar_message(&mut self, message: top_bar::Message) {
        if let GuiState::TopBar(ref mut state) = self {
            state.queue_message(message)
        } else {
            panic!("wrong message type")
        }
    }

    fn queue_left_panel_message(&mut self, message: left_panel::Message) {
        if let GuiState::LeftPanel(ref mut state) = self {
            state.queue_message(message)
        } else {
            panic!("wrong message type")
        }
    }

    fn queue_status_bar_message(&mut self, message: status_bar::Message) {
        if let GuiState::StatusBar(ref mut state) = self {
            state.queue_message(message)
        } else {
            panic!("wrong message type")
        }
    }

    fn resize(&mut self, area: DrawArea, window: &Window) {
        match self {
            GuiState::TopBar(ref mut state) => state.queue_message(top_bar::Message::Resize(
                area.size.to_logical(window.scale_factor()),
            )),
            GuiState::LeftPanel(ref mut state) => {
                state.queue_message(left_panel::Message::Resized(
                    area.size.to_logical(window.scale_factor()),
                    area.position.to_logical(window.scale_factor()),
                ))
            }
            GuiState::StatusBar(_) => {}
        }
    }

    fn is_queue_empty(&self) -> bool {
        match self {
            GuiState::TopBar(state) => state.is_queue_empty(),
            GuiState::LeftPanel(state) => state.is_queue_empty(),
            GuiState::StatusBar(state) => state.is_queue_empty(),
        }
    }

    fn update(
        &mut self,
        size: iced::Size,
        cursor_position: iced::Point,
        renderer: &mut Renderer,
        debug: &mut Debug,
    ) {
        let mut clipboard = iced_native::clipboard::Null;
        match self {
            GuiState::TopBar(state) => {
                state.update(size, cursor_position, renderer, &mut clipboard, debug);
            }
            GuiState::LeftPanel(state) => {
                state.update(size, cursor_position, renderer, &mut clipboard, debug);
            }
            GuiState::StatusBar(state) => {
                state.update(size, cursor_position, renderer, &mut clipboard, debug);
            }
        }
    }

    fn render(
        &mut self,
        renderer: &mut Renderer,
        device: &Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: &iced_graphics::Viewport,
        debug: &Debug,
        mouse_interaction: &mut iced::mouse::Interaction,
    ) {
        match self {
            GuiState::TopBar(ref state) => {
                *mouse_interaction = renderer.backend_mut().draw(
                    device,
                    staging_belt,
                    encoder,
                    target,
                    viewport,
                    state.primitive(),
                    &debug.overlay(),
                );
            }
            GuiState::LeftPanel(ref state) => {
                renderer.backend_mut().draw(
                    device,
                    staging_belt,
                    encoder,
                    target,
                    viewport,
                    state.primitive(),
                    &debug.overlay(),
                );
            }
            GuiState::StatusBar(ref state) => {
                renderer.backend_mut().draw(
                    device,
                    staging_belt,
                    encoder,
                    target,
                    viewport,
                    state.primitive(),
                    &debug.overlay(),
                );
            }
        }
    }

    fn has_keyboard_priority(&self) -> bool {
        match self {
            Self::TopBar(_) => false,
            Self::LeftPanel(left_panel) => left_panel.program().has_keyboard_priority(),
            Self::StatusBar(status_bar) => status_bar.program().has_keyboard_priority(),
        }
    }
}

/// A Gui component.
struct GuiElement<R: Requests> {
    state: GuiState<R>,
    debug: Debug,
    redraw: bool,
    element_type: ElementType,
}

impl<R: Requests> GuiElement<R> {
    /// Initialize the top bar gui component
    fn top_bar(
        renderer: &mut Renderer,
        window: &Window,
        multiplexer: &Multiplexer,
        requests: Arc<Mutex<R>>,
        dialoging: Arc<Mutex<bool>>,
    ) -> Self {
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
        let top_bar = TopBar::new(
            requests.clone(),
            top_bar_area.size.to_logical(window.scale_factor()),
            dialoging,
        );
        let mut top_bar_debug = Debug::new();
        let top_bar_state = program::State::new(
            top_bar,
            convert_size(top_bar_area.size),
            conversion::cursor_position(cursor_position, window.scale_factor()),
            renderer,
            &mut top_bar_debug,
        );
        Self {
            state: GuiState::TopBar(top_bar_state),
            debug: top_bar_debug,
            redraw: true,
            element_type: ElementType::TopBar,
        }
    }

    /// Initialize the left panel gui component
    fn left_panel(
        renderer: &mut Renderer,
        window: &Window,
        multiplexer: &Multiplexer,
        requests: Arc<Mutex<R>>,
        first_time: bool,
        dialoging: Arc<Mutex<bool>>,
    ) -> Self {
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let left_panel_area = multiplexer
            .get_element_area(ElementType::LeftPanel)
            .unwrap();
        let left_panel = LeftPanel::new(
            requests.clone(),
            left_panel_area.size.to_logical(window.scale_factor()),
            left_panel_area.position.to_logical(window.scale_factor()),
            first_time,
            dialoging,
        );
        let mut left_panel_debug = Debug::new();
        let left_panel_state = program::State::new(
            left_panel,
            convert_size(left_panel_area.size),
            conversion::cursor_position(cursor_position, window.scale_factor()),
            renderer,
            &mut left_panel_debug,
        );
        Self {
            state: GuiState::LeftPanel(left_panel_state),
            debug: left_panel_debug,
            redraw: true,
            element_type: ElementType::LeftPanel,
        }
    }

    fn status_bar(
        renderer: &mut Renderer,
        window: &Window,
        multiplexer: &Multiplexer,
        requests: Arc<Mutex<R>>,
    ) -> Self {
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let status_bar_area = multiplexer
            .get_element_area(ElementType::StatusBar)
            .unwrap();
        let status_bar = StatusBar::new(requests);
        let mut status_bar_debug = Debug::new();
        let status_bar_state = program::State::new(
            status_bar,
            convert_size(status_bar_area.size),
            conversion::cursor_position(cursor_position, window.scale_factor()),
            renderer,
            &mut status_bar_debug,
        );
        Self {
            state: GuiState::StatusBar(status_bar_state),
            debug: status_bar_debug,
            redraw: true,
            element_type: ElementType::StatusBar,
        }
    }

    fn forward_event(&mut self, event: Event) {
        self.state.queue_event(event)
    }

    fn get_state(&mut self) -> &mut GuiState<R> {
        &mut self.state
    }

    fn has_keyboard_priority(&self) -> bool {
        self.state.has_keyboard_priority()
    }

    fn resize(&mut self, window: &Window, multiplexer: &Multiplexer) {
        let area = multiplexer.get_draw_area(self.element_type).unwrap();
        self.state.resize(area, window);
        self.redraw = true;
    }

    fn fetch_change(
        &mut self,
        window: &Window,
        multiplexer: &Multiplexer,
        renderer: &mut Renderer,
        resized: bool,
    ) -> bool {
        let area = multiplexer.get_element_area(self.element_type).unwrap();
        let cursor = if multiplexer.foccused_element() == Some(self.element_type) {
            multiplexer.get_cursor_position()
        } else {
            PhysicalPosition::new(-1., -1.)
        };
        if !self.state.is_queue_empty() || resized {
            // We update iced
            self.redraw = true;
            let _ = self.state.update(
                convert_size(area.size),
                conversion::cursor_position(cursor, window.scale_factor()),
                renderer,
                &mut self.debug,
            );
            true
        } else {
            false
        }
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        encoder: &mut wgpu::CommandEncoder,
        device: &Device,
        window: &Window,
        multiplexer: &Multiplexer,
        staging_belt: &mut wgpu::util::StagingBelt,
        mouse_interaction: &mut iced::mouse::Interaction,
    ) {
        if self.redraw {
            let viewport = Viewport::with_physical_size(
                convert_size_u32(
                    multiplexer
                        .get_element_area(self.element_type)
                        .unwrap()
                        .size,
                ),
                window.scale_factor(),
            );
            let target = multiplexer.get_texture_view(self.element_type).unwrap();
            self.state.render(
                renderer,
                device,
                staging_belt,
                encoder,
                target,
                &viewport,
                &self.debug,
                mouse_interaction,
            );
            self.redraw = false;
        }
    }
}

/// The Gui manager.
pub struct Gui<R: Requests> {
    /// HashMap mapping [ElementType](ElementType) to a GuiElement
    elements: HashMap<ElementType, GuiElement<R>>,
    renderer: iced_wgpu::Renderer,
    settings: Settings,
    device: Rc<Device>,
    resized: bool,
    requests: Arc<Mutex<R>>,
    dialoging: Arc<Mutex<bool>>,
    ui_size: UiSize,
}

impl<R: Requests> Gui<R> {
    pub fn new(
        device: Rc<Device>,
        window: &Window,
        multiplexer: &Multiplexer,
        requests: Arc<Mutex<R>>,
        settings: Settings,
    ) -> Self {
        let mut renderer = Renderer::new(Backend::new(device.as_ref(), settings.clone()));
        let mut elements = HashMap::new();
        let dialoging: Arc<Mutex<bool>> = Default::default();
        elements.insert(
            ElementType::TopBar,
            GuiElement::top_bar(
                &mut renderer,
                window,
                multiplexer,
                requests.clone(),
                dialoging.clone(),
            ),
        );
        elements.insert(
            ElementType::LeftPanel,
            GuiElement::left_panel(
                &mut renderer,
                window,
                multiplexer,
                requests.clone(),
                true,
                dialoging.clone(),
            ),
        );
        elements.insert(
            ElementType::StatusBar,
            GuiElement::status_bar(&mut renderer, window, multiplexer, requests.clone()),
        );

        Self {
            settings,
            requests,
            elements,
            renderer,
            device,
            resized: true,
            dialoging,
            ui_size: Default::default(),
        }
    }

    /// Forward an event to the appropriate gui component
    pub fn forward_event(&mut self, area: ElementType, event: iced_native::Event) {
        self.elements.get_mut(&area).unwrap().forward_event(event);
    }

    pub fn forward_event_all(&mut self, event: iced_native::Event) {
        for e in self.elements.values_mut() {
            e.forward_event(event.clone())
        }
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.elements.values().any(|e| e.has_keyboard_priority())
    }

    /// Forward a message to the appropriate gui component
    pub fn forward_messages(&mut self, messages: &mut IcedMessages) {
        for m in messages.top_bar.drain(..) {
            self.elements
                .get_mut(&ElementType::TopBar)
                .unwrap()
                .get_state()
                .queue_top_bar_message(m);
        }
        for m in messages.left_panel.drain(..) {
            self.elements
                .get_mut(&ElementType::LeftPanel)
                .unwrap()
                .get_state()
                .queue_left_panel_message(m);
        }
        for m in messages.status_bar.drain(..) {
            self.elements
                .get_mut(&ElementType::StatusBar)
                .unwrap()
                .get_state()
                .queue_status_bar_message(m);
        }
    }

    /// Get the new size of each gui component from the multiplexer and forwards them.
    pub fn resize(&mut self, multiplexer: &Multiplexer, window: &Window) {
        for element in self.elements.values_mut() {
            element.resize(window, multiplexer)
        }
        self.resized = true;
    }

    /// Ask the gui component to process the event that they have recieved
    pub fn fetch_change(&mut self, window: &Window, multiplexer: &Multiplexer) -> bool {
        let mut ret = false;
        for elements in self.elements.values_mut() {
            ret |= elements.fetch_change(window, multiplexer, &mut self.renderer, false);
        }
        ret
    }

    /// Ask the gui component to process the event and messages that they that they have recieved.
    pub fn update(&mut self, multiplexer: &Multiplexer, window: &Window) {
        for elements in self.elements.values_mut() {
            elements.fetch_change(window, multiplexer, &mut self.renderer, self.resized);
        }
        self.resized = false;
    }

    pub fn new_ui_size(&mut self, ui_size: UiSize, window: &Window, multiplexer: &Multiplexer) {
        self.set_text_size(ui_size.main_text());
        self.ui_size = ui_size.clone();

        self.rebuild_gui(window, multiplexer);
    }

    pub fn notify_scale_factor_change(&mut self, window: &Window, multiplexer: &Multiplexer) {
        self.set_text_size(self.ui_size.main_text());
        self.rebuild_gui(window, multiplexer);
    }

    fn rebuild_gui(&mut self, window: &Window, multiplexer: &Multiplexer) {
        self.elements.insert(
            ElementType::TopBar,
            GuiElement::top_bar(
                &mut self.renderer,
                window,
                multiplexer,
                self.requests.clone(),
                self.dialoging.clone(),
            ),
        );
        self.elements.insert(
            ElementType::LeftPanel,
            GuiElement::left_panel(
                &mut self.renderer,
                window,
                multiplexer,
                self.requests.clone(),
                false,
                self.dialoging.clone(),
            ),
        );
        self.elements.insert(
            ElementType::StatusBar,
            GuiElement::status_bar(
                &mut self.renderer,
                window,
                multiplexer,
                self.requests.clone(),
            ),
        );
    }

    fn set_text_size(&mut self, text_size: u16) {
        let settings = Settings {
            default_text_size: text_size,
            ..self.settings.clone()
        };
        let renderer = Renderer::new(Backend::new(self.device.as_ref(), settings.clone()));
        self.settings = settings;
        self.renderer = renderer;
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        multiplexer: &Multiplexer,
        staging_belt: &mut wgpu::util::StagingBelt,
        mouse_interaction: &mut iced::mouse::Interaction,
    ) {
        for element in self.elements.values_mut() {
            element.render(
                &mut self.renderer,
                encoder,
                self.device.as_ref(),
                window,
                multiplexer,
                staging_belt,
                mouse_interaction,
            )
        }
    }
}

fn convert_size(size: PhysicalSize<u32>) -> Size<f32> {
    Size::new(size.width as f32, size.height as f32)
}

fn convert_size_u32(size: PhysicalSize<u32>) -> Size<u32> {
    Size::new(size.width, size.height)
}

use iced::{button, Button, Length, Text};
fn text_btn<'a, M: Clone>(
    state: &'a mut button::State,
    text: &'static str,
    ui_size: UiSize,
) -> Button<'a, M> {
    let size = if text.len() > 1 {
        ui_size.main_text()
    } else {
        ui_size.icon()
    };
    Button::new(state, Text::new(text).size(size)).height(Length::Units(ui_size.button()))
}

fn icon_btn<'a, M: Clone>(
    state: &'a mut button::State,
    icon_char: char,
    ui_size: UiSize,
) -> Button<'a, M> {
    Button::new(
        state,
        Text::new(icon_char.to_string())
            .font(left_panel::ENSNANO_FONT)
            .size(ui_size.icon()),
    )
    .height(Length::Units(ui_size.button()))
}

mod slider_style {
    use iced::slider::{Handle, HandleShape, Style, StyleSheet};
    use iced::Color;

    pub struct DesactivatedSlider;

    impl StyleSheet for DesactivatedSlider {
        fn active(&self) -> Style {
            Style {
                rail_colors: ([0.6, 0.6, 0.6, 0.5].into(), Color::WHITE),
                handle: Handle {
                    shape: HandleShape::Rectangle {
                        width: 8,
                        border_radius: 4.0,
                    },
                    color: Color::from_rgb(0.65, 0.65, 0.65),
                    border_color: Color::from_rgb(0.6, 0.6, 0.6),
                    border_width: 1.0,
                },
            }
        }

        fn hovered(&self) -> Style {
            self.active()
        }

        fn dragging(&self) -> Style {
            self.active()
        }
    }
}
