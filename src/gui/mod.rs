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
use ensnano_organizer::GroupId;
pub use top_bar::TopBar;
/// Draw the left pannel of the GUI
pub mod left_panel;
pub use left_panel::{ColorOverlay, LeftPanel, RigidBodyParametersRequest};
pub mod status_bar;
mod ui_size;
pub use ui_size::*;
mod material_icons_light;
pub use ensnano_design::{Camera, CameraId};
pub use status_bar::{CurentOpState, StrandBuildingStatus};

mod icon;

use status_bar::StatusBar;

use crate::scene::FogParameters;
use ensnano_design::{
    elements::{DnaAttribute, DnaElement, DnaElementKey},
    grid::GridTypeDescr,
    Nucl, Parameters,
};
use ensnano_interactor::{
    graphics::{Background3D, DrawArea, ElementType, RenderingMode, SplitMode},
    Selection, SimulationState, SuggestionParameters, WidgetBasis,
};
use ensnano_interactor::{operation::Operation, ScaffoldInfo};
use ensnano_interactor::{ActionMode, HyperboloidRequest, RollRequest, SelectionMode};
pub use ensnano_organizer::OrganizerTree;
use iced_native::Event;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, program, winit, Debug, Size};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ultraviolet::{Rotor3, Vec3};
use wgpu::Device;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::ModifiersState,
    window::Window,
};

pub trait Requests: 'static + Send {
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
    fn set_scaffold_sequence(&mut self, shift: usize);
    fn set_scaffold_shift(&mut self, shift: usize);
    /// Change the size of the UI components
    fn set_ui_size(&mut self, size: UiSize);
    /// Finalize the currently eddited hyperboloid grid
    fn finalize_hyperboloid(&mut self);
    fn stop_roll_simulation(&mut self);
    fn start_roll_simulation(&mut self, roll_request: RollRequest);
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
    fn set_selected_keys(
        &mut self,
        selection: Vec<DnaElementKey>,
        group_id: Option<ensnano_organizer::GroupId>,
        new_group: bool,
    );
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
    fn save(&mut self);
    fn open_file(&mut self);
    /// Adjust the 2D and 3D cameras so that the design fit in screen
    fn fit_design_in_scenes(&mut self);
    /// Update the parameters of the current operation
    fn update_current_operation(&mut self, operation: Arc<dyn Operation>);
    /// Update the shift of the currently seleced hyperbloid grid
    fn update_hyperboloid_shift(&mut self, shift: f32);
    fn display_error_msg(&mut self, msg: String);
    /// Set the scaffold to be the some strand with id `s_id`, or none
    fn set_scaffold_id(&mut self, s_id: Option<usize>);
    /// make the spheres of the currently selected grid large/small
    fn toggle_helices_persistance_of_grid(&mut self, persistant: bool);
    /// make the spheres of the currently selected grid large/small
    fn set_small_sphere(&mut self, small: bool);
    fn finish_changing_color(&mut self);
    fn stop_simulations(&mut self);
    fn reset_simulations(&mut self);
    fn reload_file(&mut self);
    fn add_double_strand_on_new_helix(&mut self, parameters: Option<(isize, usize)>);
    fn set_strand_name(&mut self, s_id: usize, name: String);
    fn create_new_camera(&mut self);
    fn delete_camera(&mut self, cam_id: CameraId);
    fn select_camera(&mut self, cam_id: CameraId);
    fn set_favourite_camera(&mut self, cam_id: CameraId);
    fn update_camera(&mut self, cam_id: CameraId);
    fn set_camera_name(&mut self, cam_id: CameraId, name: String);
    fn set_suggestion_parameters(&mut self, param: SuggestionParameters);
    fn set_grid_position(&mut self, grid_id: usize, position: Vec3);
    fn set_grid_orientation(&mut self, grid_id: usize, orientation: Rotor3);
    fn flip_split_views(&mut self);
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayType {
    Color,
}

enum GuiState<R: Requests, S: AppState> {
    TopBar(iced_winit::program::State<TopBar<R, S>>),
    LeftPanel(iced_winit::program::State<LeftPanel<R, S>>),
    StatusBar(iced_winit::program::State<StatusBar<R, S>>),
}

impl<R: Requests, S: AppState> GuiState<R, S> {
    fn queue_event(&mut self, event: Event) {
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key_code: iced::keyboard::KeyCode::Tab,
            ..
        }) = event
        {
            match self {
                GuiState::StatusBar(state) => {
                    self.queue_status_bar_message(status_bar::Message::TabPressed)
                }
                GuiState::TopBar(_) => (),
                GuiState::LeftPanel(_) => (),
            }
        } else {
            match self {
                GuiState::TopBar(state) => state.queue_event(event),
                GuiState::LeftPanel(state) => state.queue_event(event),
                GuiState::StatusBar(state) => state.queue_event(event),
            }
        }
    }

    fn queue_top_bar_message(&mut self, message: top_bar::Message<S>) {
        log::trace!("Queue top bar {:?}", message);
        if let GuiState::TopBar(ref mut state) = self {
            state.queue_message(message)
        } else {
            panic!("wrong message type")
        }
    }

    fn queue_left_panel_message(&mut self, message: left_panel::Message<S>) {
        log::trace!("Queue left panel {:?}", message);
        if let GuiState::LeftPanel(ref mut state) = self {
            state.queue_message(message)
        } else {
            panic!("wrong message type")
        }
    }

    fn queue_status_bar_message(&mut self, message: status_bar::Message<S>) {
        log::trace!("Queue status_bar {:?}", message);
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
                let icon = renderer.backend_mut().draw(
                    device,
                    staging_belt,
                    encoder,
                    target,
                    viewport,
                    state.primitive(),
                    &debug.overlay(),
                );
                if icon > *mouse_interaction {
                    *mouse_interaction = icon;
                }
            }
            GuiState::StatusBar(ref state) => {
                let icon = renderer.backend_mut().draw(
                    device,
                    staging_belt,
                    encoder,
                    target,
                    viewport,
                    state.primitive(),
                    &debug.overlay(),
                );
                if icon > *mouse_interaction {
                    *mouse_interaction = icon;
                }
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
struct GuiElement<R: Requests, S: AppState> {
    state: GuiState<R, S>,
    debug: Debug,
    redraw: bool,
    element_type: ElementType,
}

impl<R: Requests, S: AppState> GuiElement<R, S> {
    /// Initialize the top bar gui component
    fn top_bar(
        renderer: &mut Renderer,
        window: &Window,
        multiplexer: &dyn Multiplexer,
        requests: Arc<Mutex<R>>,
    ) -> Self {
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let top_bar_area = multiplexer.get_draw_area(ElementType::TopBar).unwrap();
        let top_bar = TopBar::new(
            requests.clone(),
            top_bar_area.size.to_logical(window.scale_factor()),
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
        multiplexer: &dyn Multiplexer,
        requests: Arc<Mutex<R>>,
        first_time: bool,
    ) -> Self {
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let left_panel_area = multiplexer.get_draw_area(ElementType::LeftPanel).unwrap();
        let left_panel = LeftPanel::new(
            requests.clone(),
            left_panel_area.size.to_logical(window.scale_factor()),
            left_panel_area.position.to_logical(window.scale_factor()),
            first_time,
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
        multiplexer: &dyn Multiplexer,
        requests: Arc<Mutex<R>>,
    ) -> Self {
        let cursor_position = PhysicalPosition::new(-1., -1.);
        let status_bar_area = multiplexer.get_draw_area(ElementType::StatusBar).unwrap();
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

    fn get_state(&mut self) -> &mut GuiState<R, S> {
        &mut self.state
    }

    fn has_keyboard_priority(&self) -> bool {
        self.state.has_keyboard_priority()
    }

    fn resize(&mut self, window: &Window, multiplexer: &dyn Multiplexer) {
        let area = multiplexer.get_draw_area(self.element_type).unwrap();
        self.state.resize(area, window);
        log::debug!("resizing {:?}", area);
        self.redraw = true;
    }

    fn fetch_change(
        &mut self,
        window: &Window,
        multiplexer: &dyn Multiplexer,
        renderer: &mut Renderer,
        resized: bool,
    ) -> bool {
        let area = multiplexer.get_draw_area(self.element_type).unwrap();
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
            log::debug!("GUI request redraw");
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
        multiplexer: &dyn Multiplexer,
        staging_belt: &mut wgpu::util::StagingBelt,
        mouse_interaction: &mut iced::mouse::Interaction,
    ) {
        if self.redraw {
            let viewport = Viewport::with_physical_size(
                convert_size_u32(multiplexer.get_draw_area(self.element_type).unwrap().size),
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
pub struct Gui<R: Requests, S: AppState> {
    /// HashMap mapping [ElementType](ElementType) to a GuiElement
    elements: HashMap<ElementType, GuiElement<R, S>>,
    renderer: iced_wgpu::Renderer,
    settings: Settings,
    device: Rc<Device>,
    resized: bool,
    requests: Arc<Mutex<R>>,
    ui_size: UiSize,
}

impl<R: Requests, S: AppState> Gui<R, S> {
    pub fn new(
        device: Rc<Device>,
        window: &Window,
        multiplexer: &dyn Multiplexer,
        requests: Arc<Mutex<R>>,
        settings: Settings,
    ) -> Self {
        let mut renderer = Renderer::new(Backend::new(
            device.as_ref(),
            settings.clone(),
            crate::TEXTURE_FORMAT,
        ));
        let mut elements = HashMap::new();
        elements.insert(
            ElementType::TopBar,
            GuiElement::top_bar(&mut renderer, window, multiplexer, requests.clone()),
        );
        elements.insert(
            ElementType::LeftPanel,
            GuiElement::left_panel(&mut renderer, window, multiplexer, requests.clone(), true),
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
            ui_size: Default::default(),
        }
    }

    /// Forward an event to the appropriate gui component
    pub fn forward_event(&mut self, area: ElementType, event: iced_native::Event) {
        self.elements.get_mut(&area).unwrap().forward_event(event);
    }

    /// Clear the foccus of all components of the GUI
    pub fn clear_foccus(&mut self) {
        for elt in self.elements.values_mut() {
            use iced_native::mouse::Event;
            elt.forward_event(iced_native::Event::Mouse(Event::CursorMoved {
                position: [-1., -1.].into(),
            }));
            elt.forward_event(iced_native::Event::Mouse(Event::ButtonPressed(
                iced_native::mouse::Button::Left,
            )))
        }
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
    pub fn forward_messages(&mut self, messages: &mut IcedMessages<S>) {
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
    pub fn resize(&mut self, multiplexer: &dyn Multiplexer, window: &Window) {
        for element in self.elements.values_mut() {
            element.resize(window, multiplexer)
        }
        self.resized = true;
    }

    /// Ask the gui component to process the event that they have recieved
    pub fn fetch_change(&mut self, window: &Window, multiplexer: &dyn Multiplexer) -> bool {
        let mut ret = false;
        for elements in self.elements.values_mut() {
            ret |= elements.fetch_change(window, multiplexer, &mut self.renderer, false);
        }
        ret
    }

    /// Ask the gui component to process the event and messages that they that they have recieved.
    pub fn update(&mut self, multiplexer: &dyn Multiplexer, window: &Window) {
        for elements in self.elements.values_mut() {
            elements.fetch_change(window, multiplexer, &mut self.renderer, self.resized);
        }
        self.resized = false;
    }

    pub fn new_ui_size(&mut self, ui_size: UiSize, window: &Window, multiplexer: &dyn Multiplexer) {
        self.set_text_size(ui_size.main_text());
        self.ui_size = ui_size.clone();

        self.rebuild_gui(window, multiplexer);
    }

    pub fn notify_scale_factor_change(&mut self, window: &Window, multiplexer: &dyn Multiplexer) {
        self.set_text_size(self.ui_size.main_text());
        self.rebuild_gui(window, multiplexer);
    }

    fn rebuild_gui(&mut self, window: &Window, multiplexer: &dyn Multiplexer) {
        self.elements.insert(
            ElementType::TopBar,
            GuiElement::top_bar(
                &mut self.renderer,
                window,
                multiplexer,
                self.requests.clone(),
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
        let renderer = Renderer::new(Backend::new(
            self.device.as_ref(),
            settings.clone(),
            crate::TEXTURE_FORMAT,
        ));
        self.settings = settings;
        self.renderer = renderer;
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        multiplexer: &dyn Multiplexer,
        staging_belt: &mut wgpu::util::StagingBelt,
        mouse_interaction: &mut iced::mouse::Interaction,
    ) {
        *mouse_interaction = Default::default();
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

use std::collections::VecDeque;
/// Message sent to the gui component
pub struct IcedMessages<S: AppState> {
    left_panel: VecDeque<left_panel::Message<S>>,
    top_bar: VecDeque<top_bar::Message<S>>,
    _color_overlay: VecDeque<left_panel::ColorMessage>,
    status_bar: VecDeque<status_bar::Message<S>>,
    application_state: S,
    last_main_state: MainState,
    redraw: bool,
}

impl<S: AppState> IcedMessages<S> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            left_panel: VecDeque::new(),
            top_bar: VecDeque::new(),
            _color_overlay: VecDeque::new(),
            status_bar: VecDeque::new(),
            application_state: Default::default(),
            last_main_state: Default::default(),
            redraw: false,
        }
    }

    pub fn push_progress(&mut self, progress_name: String, progress: f32) {
        self.status_bar
            .push_back(status_bar::Message::Progress(Some((
                progress_name,
                progress,
            ))))
    }

    pub fn finish_progess(&mut self) {
        self.status_bar
            .push_back(status_bar::Message::Progress(None))
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.left_panel
            .push_back(left_panel::Message::ModifiersChanged(modifiers))
    }

    pub fn new_ui_size(&mut self, ui_size: UiSize) {
        self.left_panel
            .push_back(left_panel::Message::UiSizeChanged(ui_size));
        self.top_bar
            .push_back(top_bar::Message::UiSizeChanged(ui_size));
        self.status_bar
            .push_back(status_bar::Message::UiSizeChanged(ui_size));
    }

    pub fn push_show_tutorial(&mut self) {
        self.left_panel.push_back(left_panel::Message::ShowTutorial);
    }

    pub fn show_help(&mut self) {
        self.left_panel.push_back(left_panel::Message::ForceHelp);
    }

    pub fn push_application_state(&mut self, state: S, main_state: MainState) {
        log::trace!("Old ptr {:p}, new ptr {:p}", state, self.application_state);
        self.application_state = state.clone();
        self.redraw |= main_state != self.last_main_state;
        self.last_main_state = main_state.clone();
        let must_update = self.application_state != state || self.redraw;
        if must_update {
            self.left_panel
                .push_back(left_panel::Message::NewApplicationState(state.clone()));
            self.top_bar
                .push_back(top_bar::Message::NewApplicationState(top_bar::MainState {
                    app_state: state.clone(),
                    can_undo: main_state.can_undo,
                    can_redo: main_state.can_redo,
                    need_save: main_state.need_save,
                    can_reload: main_state.can_reload,
                    can_split2d: main_state.can_split2d,
                    splited_2d: main_state.splited_2d,
                }));
            self.status_bar
                .push_back(status_bar::Message::NewApplicationState(state.clone()));
        }
    }
}

/// An object mapping ElementType to DrawArea
pub trait Multiplexer {
    fn get_draw_area(&self, element_type: ElementType) -> Option<DrawArea>;
    fn foccused_element(&self) -> Option<ElementType>;
    fn get_cursor_position(&self) -> PhysicalPosition<f64>;
    fn get_texture_view(&self, element_type: ElementType) -> Option<&wgpu::TextureView>;
}

pub trait AppState:
    Default + PartialEq + Clone + 'static + Send + std::fmt::Debug + std::fmt::Pointer
{
    fn get_selection_mode(&self) -> SelectionMode;
    fn get_action_mode(&self) -> ActionMode;
    fn get_build_helix_mode(&self) -> ActionMode;
    fn has_double_strand_on_new_helix(&self) -> bool;
    fn get_widget_basis(&self) -> WidgetBasis;
    fn get_simulation_state(&self) -> SimulationState;
    fn get_dna_parameters(&self) -> Parameters;
    fn is_building_hyperboloid(&self) -> bool;
    fn get_scaffold_info(&self) -> Option<ScaffoldInfo>;
    fn get_selection(&self) -> &[Selection];
    fn get_selection_as_dnaelement(&self) -> Vec<DnaElementKey>;
    fn can_make_grid(&self) -> bool;
    fn get_reader(&self) -> Box<dyn DesignReader>;
    fn design_was_modified(&self, other: &Self) -> bool;
    fn selection_was_updated(&self, other: &Self) -> bool;
    fn get_curent_operation_state(&self) -> Option<CurentOpState>;
    fn get_strand_building_state(&self) -> Option<StrandBuildingStatus>;
    fn get_selected_group(&self) -> Option<GroupId>;
    fn get_suggestion_parameters(&self) -> &SuggestionParameters;
}

pub trait DesignReader: 'static {
    fn grid_has_persistent_phantom(&self, g_id: usize) -> bool;
    fn grid_has_small_spheres(&self, g_id: usize) -> bool;
    fn get_grid_shift(&self, g_id: usize) -> Option<f32>;
    fn get_strand_length(&self, s_id: usize) -> Option<usize>;
    fn is_id_of_scaffold(&self, s_id: usize) -> bool;
    fn length_decomposition(&self, s_id: usize) -> String;
    fn nucl_is_anchor(&self, nucl: Nucl) -> bool;
    fn get_dna_elements(&self) -> &[DnaElement];
    fn get_organizer_tree(&self) -> Option<Arc<ensnano_design::EnsnTree>>;
    fn strand_name(&self, s_id: usize) -> String;
    fn get_all_cameras(&self) -> Vec<(CameraId, &str)>;
    fn get_favourite_camera(&self) -> Option<CameraId>;
    fn get_grid_position_and_orientation(&self, g_id: usize) -> Option<(Vec3, Rotor3)>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MainState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub need_save: bool,
    pub can_reload: bool,
    pub can_split2d: bool,
    pub splited_2d: bool,
}
