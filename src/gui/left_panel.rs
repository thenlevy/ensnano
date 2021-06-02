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
use ensnano_organizer::{Organizer, OrganizerMessage, OrganizerTree};
use std::sync::{Arc, Mutex};

use iced::{
    button, pick_list, slider, text_input, Button, Checkbox, Color, Command, Element, Length,
    PickList, Scrollable, Slider, Text, TextInput,
};
use iced::{container, Background, Column, Container, Image, Row};
use iced_aw::{TabLabel, Tabs};
use iced_native::{clipboard::Null as NullClipboard, Program};
use iced_wgpu::{Backend, Renderer};
use iced_winit::winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::ModifiersState,
};
use ultraviolet::Vec3;

use color_space::{Hsv, Rgb};

use crate::design::{DnaElement, DnaElementKey, ScaffoldInfo};
use crate::mediator::{ActionMode, Selection, SelectionMode};

use super::{
    icon_btn, slider_style::DesactivatedSlider, text_btn, ApplicationState, FogParameters as Fog,
    GridTypeDescr, OverlayType, Requests, UiSize,
};
mod color_picker;
use color_picker::ColorPicker;
mod sequence_input;
use sequence_input::SequenceInput;
use text_input_style::BadValue;
mod discrete_value;
use discrete_value::{FactoryId, RequestFactory, Requestable, ValueId};
mod tabs;
use crate::consts::*;
mod contextual_panel;
use contextual_panel::ContextualPanel;

use crate::mediator::HyperboloidRequest;
use material_icons::{icon_to_char, Icon as MaterialIcon, FONT as MATERIALFONT};
use std::collections::BTreeMap;
use std::thread;
use tabs::{
    CameraShortcut, CameraTab, EditionTab, GridTab, ParametersTab, SequenceTab, SimulationTab,
};

const ICONFONT: iced::Font = iced::Font::External {
    name: "IconFont",
    bytes: MATERIALFONT,
};

pub(super) const ENSNANO_FONT: iced::Font = iced::Font::External {
    name: "EnsNanoFont",
    bytes: include_bytes!("../../font/ensnano.ttf"),
};

fn icon(icon: MaterialIcon, ui_size: &UiSize) -> iced::Text {
    iced::Text::new(format!("{}", icon_to_char(icon)))
        .font(ICONFONT)
        .size(ui_size.icon())
}

const CHECKBOXSPACING: u16 = 5;

pub struct LeftPanel<R: Requests> {
    dialoging: Arc<Mutex<bool>>,
    logical_size: LogicalSize<f64>,
    #[allow(dead_code)]
    logical_position: LogicalPosition<f64>,
    #[allow(dead_code)]
    open_color: button::State,
    sequence_input: SequenceInput,
    requests: Arc<Mutex<R>>,
    #[allow(dead_code)]
    show_torsion: bool,
    selected_tab: usize,
    organizer: Organizer<DnaElement>,
    ui_size: UiSize,
    grid_tab: GridTab,
    edition_tab: EditionTab,
    camera_tab: CameraTab,
    simulation_tab: SimulationTab,
    sequence_tab: SequenceTab,
    parameters_tab: ParametersTab,
    contextual_panel: ContextualPanel,
    camera_shortcut: CameraShortcut,
    application_state: ApplicationState,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectionModeChanged(SelectionMode),
    Resized(LogicalSize<f64>, LogicalPosition<f64>),
    #[allow(dead_code)]
    OpenColor,
    MakeGrids,
    ActionModeChanged(ActionMode),
    SequenceChanged(String),
    SequenceFileRequested,
    StrandColorChanged(Color),
    HueChanged(f32),
    NewGrid(GridTypeDescr),
    FixPoint(Vec3, Vec3),
    RotateCam(f32, f32, f32),
    PositionHelicesChanged(String),
    LengthHelicesChanged(String),
    ScaffoldPositionInput(String),
    #[allow(dead_code)]
    ShowTorsion(bool),
    FogRadius(f32),
    FogLength(f32),
    SimRequest,
    NewDesign,
    DescreteValue {
        factory_id: FactoryId,
        value_id: ValueId,
        value: f32,
    },
    HelixRoll(f32),
    NewHyperboloid,
    FinalizeHyperboloid,
    RollTargeted(bool),
    RigidGridSimulation(bool),
    RigidHelicesSimulation(bool),
    VolumeExclusion(bool),
    TabSelected(usize),
    NewDnaElement(Vec<DnaElement>),
    NewSelection(Vec<DnaElementKey>),
    OrganizerMessage(OrganizerMessage<DnaElement>),
    Selection(Selection, Vec<String>),
    ModifiersChanged(ModifiersState),
    NewTreeApp(OrganizerTree<DnaElementKey>),
    UiSizeChanged(UiSize),
    UiSizePicked(UiSize),
    StapplesRequested,
    ToggleText(bool),
    #[allow(dead_code)]
    CleanRequested,
    AddDoubleStrandHelix(bool),
    ToggleVisibility(bool),
    AllVisible,
    Redim2dHelices(bool),
    InvertScroll(bool),
    BrownianMotion(bool),
    Nothing,
    CancelHyperboloid,
    CanMakeGrid(bool),
    SelectionValueChanged(usize, String),
    SetSmallSpheres(bool),
    ScaffoldIdSet(usize, bool),
    NewScaffoldInfo(Option<ScaffoldInfo>),
    SelectScaffold,
    ForceHelp,
    ShowTutorial,
    RenderingMode(crate::mediator::RenderingMode),
    Background3D(crate::mediator::Background3D),
    OpenLink(&'static str),
    NewApplicationState(ApplicationState),
    FogChoice(tabs::FogChoice),
    SetScaffoldSeqButtonPressed,
}

impl<R: Requests> LeftPanel<R> {
    pub fn new(
        requests: Arc<Mutex<R>>,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
        first_time: bool,
        dialoging: Arc<Mutex<bool>>,
    ) -> Self {
        let selected_tab = if first_time { 0 } else { 5 };
        let mut organizer = Organizer::new();
        organizer.set_width(logical_size.width as u16);
        Self {
            logical_size,
            logical_position,
            open_color: Default::default(),
            sequence_input: SequenceInput::new(),
            requests,
            show_torsion: false,
            selected_tab,
            organizer,
            ui_size: Default::default(),
            grid_tab: GridTab::new(),
            edition_tab: EditionTab::new(),
            camera_tab: CameraTab::new(),
            simulation_tab: SimulationTab::new(),
            sequence_tab: SequenceTab::new(),
            parameters_tab: ParametersTab::new(),
            dialoging,
            contextual_panel: ContextualPanel::new(logical_size.width as u32),
            camera_shortcut: CameraShortcut::new(),
            application_state: Default::default(),
        }
    }

    pub fn resize(
        &mut self,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
    ) {
        self.logical_size = logical_size;
        self.logical_position = logical_position;
        self.contextual_panel.new_width(logical_size.width as u32);
        self.organizer.set_width(logical_size.width as u16);
    }

    fn organizer_message(&mut self, m: OrganizerMessage<DnaElement>) -> Option<Message> {
        match m {
            OrganizerMessage::InternalMessage(m) => {
                return self
                    .organizer
                    .message(&m)
                    .map(|m_| Message::OrganizerMessage(m_))
            }
            OrganizerMessage::Selection(s) => self.requests.lock().unwrap().set_selected_keys(s),
            OrganizerMessage::NewAttribute(a, keys) => {
                self.requests
                    .lock()
                    .unwrap()
                    .update_attribute_of_elements(a, keys.into_iter().collect());
            }
            OrganizerMessage::NewTree(tree) => {
                self.requests.lock().unwrap().update_organizer_tree(tree)
            }
            OrganizerMessage::Candidates(candidates) => self
                .requests
                .lock()
                .unwrap()
                .set_candidates_keys(candidates),
            _ => (),
        }
        None
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.sequence_input.has_keyboard_priority()
            || self.grid_tab.has_keyboard_priority()
            || self.organizer.has_keyboard_priority()
            || self.sequence_tab.has_keyboard_priority()
    }
}

impl<R: Requests> Program for LeftPanel<R> {
    type Renderer = Renderer;
    type Message = Message;
    type Clipboard = NullClipboard;

    fn update(&mut self, message: Message, _cb: &mut NullClipboard) -> Command<Message> {
        match message {
            Message::SelectionModeChanged(selection_mode) => {
                if selection_mode != self.application_state.selection_mode {
                    self.requests
                        .lock()
                        .unwrap()
                        .change_selection_mode(selection_mode);
                }
            }
            Message::ActionModeChanged(action_mode) => {
                if self.application_state.action_mode != action_mode {
                    self.requests
                        .lock()
                        .unwrap()
                        .change_action_mode(action_mode)
                } else {
                    match action_mode {
                        ActionMode::Rotate | ActionMode::Translate => {
                            self.requests.lock().unwrap().toggle_widget_basis();
                        }
                        _ => (),
                    }
                }
            }
            Message::SequenceChanged(s) => {
                self.requests
                    .lock()
                    .unwrap()
                    .set_selected_strand_sequence(s.clone());
                self.sequence_input.update_sequence(s);
            }
            Message::SequenceFileRequested => {
                let dialog = rfd::AsyncFileDialog::new().pick_file();
                let requests = self.requests.clone();
                std::thread::spawn(move || {
                    let save_op = async move {
                        let file = dialog.await;
                        if let Some(handle) = file {
                            let content = std::fs::read_to_string(handle.path());
                            if let Ok(content) = content {
                                requests
                                    .lock()
                                    .unwrap()
                                    .set_selected_strand_sequence(content);
                            }
                        }
                    };
                    futures::executor::block_on(save_op);
                });
            }
            Message::OpenColor => self
                .requests
                .lock()
                .unwrap()
                .open_overlay(OverlayType::Color),
            Message::StrandColorChanged(color) => {
                let requested_color = self.edition_tab.strand_color_change(color);
                self.requests
                    .lock()
                    .unwrap()
                    .change_strand_color(requested_color);
            }
            Message::HueChanged(x) => self.edition_tab.change_hue(x),
            Message::Resized(size, position) => self.resize(size, position),
            Message::NewGrid(grid_type) => {
                self.requests.lock().unwrap().create_grid(grid_type);
                let action_mode = self.grid_tab.get_build_helix_mode();
                self.requests
                    .lock()
                    .unwrap()
                    .change_action_mode(action_mode);
            }
            Message::RotateCam(xz, yz, xy) => {
                self.camera_shortcut
                    .set_angles(xz as isize, yz as isize, xy as isize);
                self.requests
                    .lock()
                    .unwrap()
                    .perform_camera_rotation(xz, yz, xy);
            }
            Message::FixPoint(point, up) => {
                self.requests
                    .lock()
                    .unwrap()
                    .set_camera_dir_up_vec(point, up);
                self.camera_shortcut.reset_angles();
            }
            Message::LengthHelicesChanged(length_str) => {
                let action_mode = self.grid_tab.update_length_str(length_str.clone());
                if self.application_state.action_mode != action_mode {
                    self.requests
                        .lock()
                        .unwrap()
                        .change_action_mode(action_mode)
                }
            }
            Message::PositionHelicesChanged(position_str) => {
                let action_mode = self.grid_tab.update_pos_str(position_str.clone());
                if self.application_state.action_mode != action_mode {
                    self.requests
                        .lock()
                        .unwrap()
                        .change_action_mode(action_mode)
                }
            }
            Message::ScaffoldPositionInput(position_str) => {
                if let Some(n) = self.sequence_tab.update_pos_str(position_str) {
                    self.requests.lock().unwrap().set_scaffold_shift(n);
                }
            }
            Message::ShowTorsion(b) => {
                self.requests.lock().unwrap().set_torsion_visibility(b);
                self.show_torsion = b;
            }
            Message::FogLength(length) => {
                self.camera_tab.fog_length(length);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().set_fog_parameters(request);
            }
            Message::FogRadius(radius) => {
                self.camera_tab.fog_radius(radius);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().set_fog_parameters(request);
            }
            Message::NewDesign => {
                self.show_torsion = false;
                self.camera_tab.notify_new_design();
                self.edition_tab.notify_new_design();
                self.grid_tab.notify_new_design();
                self.organizer.reset();
            }
            Message::SimRequest => {
                let request = self.simulation_tab.get_physical_simulation_request();
                self.requests.lock().unwrap().start_roll_simulation(request);
            }
            Message::FogChoice(choice) => {
                let (visble, from_camera) = choice.to_param();
                self.camera_tab.fog_camera(from_camera);
                self.camera_tab.fog_visible(visble);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().set_fog_parameters(request);
            }
            Message::DescreteValue {
                factory_id,
                value_id,
                value,
            } => match factory_id {
                FactoryId::Scroll => {
                    let mut request = None;
                    self.parameters_tab
                        .update_scroll_request(value_id, value, &mut request);
                    if let Some(request) = request {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_scroll_sensitivity(request);
                    }
                }
                FactoryId::HelixRoll => {
                    let mut request = None;
                    self.edition_tab
                        .update_roll_request(value_id, value, &mut request);
                    if let Some(request) = request {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_roll_of_selected_helices(request);
                    }
                }
                FactoryId::Hyperboloid => {
                    let mut request = None;
                    self.grid_tab
                        .update_hyperboloid_request(value_id, value, &mut request);
                    if let Some(request) = request {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_current_hyperboloid(request);
                    }
                }
                FactoryId::RigidBody => {
                    let mut request = None;
                    self.simulation_tab
                        .update_request(value_id, value, &mut request);
                    if let Some(request) = request {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_rigid_body_simulation_parameters(request);
                    }
                }
                FactoryId::Brownian => {
                    let mut request = None;
                    self.simulation_tab
                        .update_brownian(value_id, value, &mut request);
                    if let Some(request) = request {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_rigid_body_simulation_parameters(request);
                    }
                }
            },
            Message::VolumeExclusion(b) => {
                self.simulation_tab.set_volume_exclusion(b);
                let mut request: Option<RigidBodyParametersRequest> = None;
                self.simulation_tab.make_rigid_body_request(&mut request);
                if let Some(request) = request {
                    self.requests
                        .lock()
                        .unwrap()
                        .update_rigid_body_simulation_parameters(request);
                }
            }
            Message::BrownianMotion(b) => {
                self.simulation_tab.set_brownian_motion(b);
                let mut request: Option<RigidBodyParametersRequest> = None;
                self.simulation_tab.make_rigid_body_request(&mut request);
                if let Some(request) = request {
                    self.requests
                        .lock()
                        .unwrap()
                        .update_rigid_body_simulation_parameters(request);
                }
            }
            Message::HelixRoll(roll) => {
                self.edition_tab.update_roll(roll);
            }
            Message::NewHyperboloid => {
                let mut request: Option<HyperboloidRequest> = None;
                self.grid_tab.new_hyperboloid(&mut request);
                if let Some(request) = request {
                    self.requests
                        .lock()
                        .unwrap()
                        .create_new_hyperboloid(request);
                }
            }
            Message::FinalizeHyperboloid => {
                self.requests.lock().unwrap().finalize_hyperboloid();
                self.grid_tab.finalize_hyperboloid();
            }
            Message::RigidGridSimulation(_) => {
                let mut request: Option<RigidBodyParametersRequest> = None;
                self.simulation_tab.make_rigid_body_request(&mut request);
                if let Some(request) = request {
                    self.requests
                        .lock()
                        .unwrap()
                        .update_rigid_grids_simulation(request);
                }
            }
            Message::RigidHelicesSimulation(_) => {
                let mut request: Option<RigidBodyParametersRequest> = None;
                self.simulation_tab.make_rigid_body_request(&mut request);
                if let Some(request) = request {
                    self.requests
                        .lock()
                        .unwrap()
                        .update_rigid_helices_simulation(request);
                }
            }
            Message::MakeGrids => self.requests.lock().unwrap().make_grid_from_selection(),
            Message::RollTargeted(b) => {
                if b {
                    if let Some(simulation_request) = self.edition_tab.get_roll_request() {
                        self.requests
                            .lock()
                            .unwrap()
                            .start_roll_simulation(simulation_request);
                    }
                } else {
                    self.requests.lock().unwrap().stop_roll_simulation();
                }
            }
            Message::TabSelected(n) => {
                if let ActionMode::BuildHelix { .. } = self.application_state.action_mode {
                    if n != 0 {
                        let action_mode = ActionMode::Normal;
                        self.requests
                            .lock()
                            .unwrap()
                            .change_action_mode(action_mode);
                    }
                }
                if n != 0 {
                    if self.grid_tab.is_building_hyperboloid() {
                        self.requests.lock().unwrap().finalize_hyperboloid();
                        self.grid_tab.finalize_hyperboloid();
                    }
                }
                if n == 0 {
                    let action_mode = self.grid_tab.get_build_helix_mode();
                    self.requests
                        .lock()
                        .unwrap()
                        .change_action_mode(action_mode);
                }
                if self.selected_tab == 3 && n != 3 {
                    println!("leaving simulation tab");
                    self.simulation_tab
                        .leave_tab(self.requests.clone(), &self.application_state);
                }
                self.selected_tab = n;
            }
            Message::NewDnaElement(elements) => self.organizer.update_elements(elements),
            Message::OrganizerMessage(m) => {
                let next_message = self.organizer_message(m);
                if let Some(message) = next_message {
                    self.update(message, _cb);
                }
            }
            Message::ModifiersChanged(modifiers) => self
                .organizer
                .new_modifiers(iced_winit::conversion::modifiers(modifiers)),
            Message::NewSelection(keys) => {
                self.edition_tab.update_selection(&keys);
                self.sequence_tab.update_selection(&keys);
                self.organizer.notify_selection(keys);
            }
            Message::CanMakeGrid(b) => {
                self.grid_tab.can_make_grid = b;
            }
            Message::NewTreeApp(tree) => self.organizer.read_tree(tree),
            Message::UiSizePicked(ui_size) => self.requests.lock().unwrap().set_ui_size(ui_size),
            Message::UiSizeChanged(ui_size) => self.ui_size = ui_size,
            Message::SetScaffoldSeqButtonPressed => {
                self.requests.lock().unwrap().set_scaffold_sequence();
            }
            Message::StapplesRequested => self.requests.lock().unwrap().download_stapples(),
            Message::ToggleText(b) => {
                self.requests
                    .lock()
                    .unwrap()
                    .set_dna_sequences_visibility(b);
                self.sequence_tab.toggle_text_value(b);
            }
            Message::CleanRequested => self.requests.lock().unwrap().remove_empty_domains(),
            Message::AddDoubleStrandHelix(b) => {
                self.grid_tab.set_show_strand(b);
                if let ActionMode::BuildHelix { .. } = self.application_state.action_mode {
                    let action_mode = self.grid_tab.get_build_helix_mode();
                    self.requests
                        .lock()
                        .unwrap()
                        .change_action_mode(action_mode);
                }
            }
            Message::ToggleVisibility(b) => self.requests.lock().unwrap().toggle_visibility(b),
            Message::AllVisible => self.requests.lock().unwrap().make_all_elements_visible(),
            Message::Redim2dHelices(b) => self.requests.lock().unwrap().resize_2d_helices(b),
            Message::InvertScroll(b) => {
                self.requests.lock().unwrap().invert_scroll(b);
                self.parameters_tab.invert_y_scroll = b;
            }
            Message::CancelHyperboloid => {
                self.grid_tab.finalize_hyperboloid();
                self.requests.lock().unwrap().cancel_hyperboloid();
            }
            Message::SelectionValueChanged(n, s) => {
                self.contextual_panel
                    .selection_value_changed(n, s, self.requests.clone());
            }
            Message::SetSmallSpheres(b) => {
                self.contextual_panel
                    .set_small_sphere(b, self.requests.clone());
            }
            Message::ScaffoldIdSet(n, b) => {
                self.contextual_panel
                    .scaffold_id_set(n, b, self.requests.clone());
            }
            Message::Selection(selection, info_values) => self
                .contextual_panel
                .update_selection(selection, info_values),
            Message::NewScaffoldInfo(info) => self.sequence_tab.set_scaffold_info(info),
            Message::SelectScaffold => self.requests.lock().unwrap().set_scaffold_from_selection(),
            Message::RenderingMode(mode) => {
                self.requests
                    .lock()
                    .unwrap()
                    .change_3d_rendering_mode(mode.clone());
                self.camera_tab.rendering_mode = mode;
            }
            Message::Background3D(bg) => {
                self.requests
                    .lock()
                    .unwrap()
                    .change_3d_background(bg.clone());
                self.camera_tab.background3d = bg;
            }
            Message::ForceHelp => {
                self.contextual_panel.force_help = true;
                self.contextual_panel.show_tutorial = false;
            }
            Message::ShowTutorial => {
                self.contextual_panel.show_tutorial ^= true;
                self.contextual_panel.force_help = false;
            }
            Message::OpenLink(link) => {
                // ATM we continue even in case of error, later any error will be promted to user
                let _ = open::that(link);
            }
            Message::NewApplicationState(state) => {
                self.application_state = state;
            }
            Message::Nothing => (),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        let width = self.logical_size.cast::<u16>().width;
        let tabs: Tabs<Message, Backend> = Tabs::new(self.selected_tab, Message::TabSelected)
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::GridOn))),
                self.grid_tab
                    .view(self.ui_size.clone(), width, &self.application_state),
            )
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::Edit))),
                self.edition_tab
                    .view(self.ui_size.clone(), width, &self.application_state),
            )
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::Videocam))),
                self.camera_tab.view(self.ui_size.clone()),
            )
            .push(
                TabLabel::Icon(ICON_PHYSICAL_ENGINE),
                self.simulation_tab
                    .view(self.ui_size.clone(), &self.application_state),
            )
            .push(
                TabLabel::Icon(ICON_ATGC),
                self.sequence_tab.view(self.ui_size.clone()),
            )
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::Settings))),
                self.parameters_tab
                    .view(self.ui_size.clone(), &self.application_state),
            )
            .text_size(self.ui_size.icon())
            .text_font(ICONFONT)
            .icon_font(ENSNANO_FONT)
            .icon_size(self.ui_size.icon())
            .tab_bar_height(Length::Units(self.ui_size.button()))
            .tab_bar_style(TabStyle)
            .width(Length::Units(width))
            .height(Length::Fill);
        let camera_shortcut = self.camera_shortcut.view(self.ui_size.clone(), width);
        let contextual_menu = self.contextual_panel.view(self.ui_size.clone());
        let organizer = self.organizer.view().map(|m| Message::OrganizerMessage(m));

        Container::new(
            Column::new()
                .width(Length::Fill)
                .push(Container::new(tabs).height(Length::FillPortion(2)))
                .push(iced::Rule::horizontal(5))
                .push(Container::new(camera_shortcut).height(Length::FillPortion(1)))
                .push(iced::Rule::horizontal(5))
                .push(Container::new(contextual_menu).height(Length::FillPortion(1)))
                .push(iced::Rule::horizontal(5))
                .push(Container::new(organizer).height(Length::FillPortion(2)))
                .padding(3),
        )
        .style(TopBarStyle)
        .height(Length::Units(self.logical_size.height as u16))
        .into()
    }
}

struct TopBarStyle;
impl container::StyleSheet for TopBarStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(BACKGROUND)),
            text_color: Some(Color::WHITE),
            ..container::Style::default()
        }
    }
}

pub const BACKGROUND: Color = Color::from_rgb(
    0x23 as f32 / 255.0,
    0x27 as f32 / 255.0,
    0x2A as f32 / 255.0,
);

pub struct ColorOverlay<R: Requests> {
    logical_size: LogicalSize<f64>,
    color_picker: ColorPicker,
    close_button: iced::button::State,
    requests: Arc<Mutex<R>>,
}

impl<R: Requests> ColorOverlay<R> {
    pub fn new(requests: Arc<Mutex<R>>, logical_size: LogicalSize<f64>) -> Self {
        Self {
            logical_size,
            close_button: Default::default(),
            color_picker: ColorPicker::new(),
            requests,
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize<f64>) {
        self.logical_size = logical_size;
    }
}

#[derive(Debug, Clone)]
pub enum ColorMessage {
    StrandColorChanged(Color),
    HueChanged(f32),
    #[allow(dead_code)]
    Resized(LogicalSize<f64>),
    Closed,
}

impl<R: Requests> Program for ColorOverlay<R> {
    type Renderer = Renderer;
    type Message = ColorMessage;
    type Clipboard = NullClipboard;

    fn update(&mut self, message: ColorMessage, _cb: &mut NullClipboard) -> Command<ColorMessage> {
        match message {
            ColorMessage::StrandColorChanged(color) => {
                let red = ((color.r * 255.) as u32) << 16;
                let green = ((color.g * 255.) as u32) << 8;
                let blue = (color.b * 255.) as u32;
                self.color_picker.update_color(color);
                let hue = Hsv::from(Rgb::new(
                    color.r as f64 * 255.,
                    color.g as f64 * 255.,
                    color.b as f64 * 255.,
                ))
                .h;
                self.color_picker.change_hue(hue as f32);
                let color = red + green + blue;
                self.requests.lock().unwrap().change_strand_color(color);
            }
            ColorMessage::HueChanged(x) => self.color_picker.change_hue(x),
            ColorMessage::Closed => {
                self.requests
                    .lock()
                    .unwrap()
                    .close_overlay(OverlayType::Color);
            }
            ColorMessage::Resized(size) => self.resize(size),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<ColorMessage> {
        let width = self.logical_size.cast::<u16>().width;

        let widget = Column::new()
            .width(Length::Units(width))
            .height(Length::Fill)
            .spacing(5)
            .push(self.color_picker.new_view())
            .spacing(5)
            .push(
                Button::new(&mut self.close_button, Text::new("Close"))
                    .on_press(ColorMessage::Closed),
            );

        Container::new(widget)
            .style(FloatingStyle)
            .height(Length::Fill)
            .into()
    }
}

struct FloatingStyle;
impl container::StyleSheet for FloatingStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(BACKGROUND)),
            text_color: Some(Color::WHITE),
            border_width: 3_f32,
            border_radius: 3_f32,
            border_color: Color::BLACK,
            ..container::Style::default()
        }
    }
}

struct ButtonStyle(bool);

impl iced_wgpu::button::StyleSheet for ButtonStyle {
    fn active(&self) -> iced_wgpu::button::Style {
        iced_wgpu::button::Style {
            border_width: if self.0 { 3_f32 } else { 1_f32 },
            border_radius: if self.0 { 3_f32 } else { 2_f32 },
            border_color: if self.0 {
                Color::BLACK
            } else {
                [0.7, 0.7, 0.7].into()
            },
            background: Some(Background::Color([0.87, 0.87, 0.87].into())),
            //background: Some(Background::Color(BACKGROUND)),
            ..Default::default()
        }
    }
}

struct ButtonColor(iced::Color);

impl ButtonColor {
    fn red_green(active: bool) -> Self {
        if active {
            Self(iced::Color::from_rgb(1., 0., 0.))
        } else {
            Self(iced::Color::from_rgb(0., 1., 0.))
        }
    }
}

impl iced_wgpu::button::StyleSheet for ButtonColor {
    fn active(&self) -> iced_wgpu::button::Style {
        iced_wgpu::button::Style {
            background: Some(Background::Color(self.0)),
            //background: Some(Background::Color(BACKGROUND)),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: [0.7, 0.7, 0.7].into(),
            text_color: Color::BLACK,
            ..Default::default()
        }
    }

    fn hovered(&self) -> iced_wgpu::button::Style {
        let active = self.active();
        iced_wgpu::button::Style {
            background: active.background.map(|background| match background {
                Background::Color(color) => Background::Color(Color {
                    a: color.a * 0.75,
                    ..color
                }),
            }),
            ..active
        }
    }
}

#[derive(Default, Debug, Clone)]
struct SelectionModeState {
    pub nucleotide: button::State,
    pub strand: button::State,
    pub helix: button::State,
    pub grid: button::State,
}

impl SelectionModeState {
    fn get_states<'a>(&'a mut self) -> BTreeMap<SelectionMode, &'a mut button::State> {
        let mut ret = BTreeMap::new();
        ret.insert(SelectionMode::Nucleotide, &mut self.nucleotide);
        ret.insert(SelectionMode::Strand, &mut self.strand);
        ret.insert(SelectionMode::Helix, &mut self.helix);
        ret.insert(SelectionMode::Grid, &mut self.grid);
        ret
    }
}

#[derive(Default, Debug, Clone)]
struct ActionModeState {
    pub select: button::State,
    pub translate: button::State,
    pub rotate: button::State,
    pub build: button::State,
    pub cut: button::State,
    pub add_grid: button::State,
    pub add_hyperboloid: button::State,
}

impl ActionModeState {
    fn get_states<'a>(
        &'a mut self,
        len_helix: usize,
        position_helix: isize,
        make_strands: bool,
    ) -> BTreeMap<ActionMode, &'a mut button::State> {
        let mut ret = BTreeMap::new();
        ret.insert(ActionMode::Normal, &mut self.select);
        ret.insert(ActionMode::Translate, &mut self.translate);
        ret.insert(ActionMode::Rotate, &mut self.rotate);
        let (position, length) = if make_strands {
            (position_helix, len_helix)
        } else {
            (0, 0)
        };
        ret.insert(ActionMode::BuildHelix { position, length }, &mut self.build);
        ret
    }
}

fn target_message(i: usize) -> Message {
    match i {
        0 => Message::FixPoint(Vec3::unit_x(), Vec3::unit_y()),
        1 => Message::FixPoint(-Vec3::unit_x(), Vec3::unit_y()),
        2 => Message::FixPoint(Vec3::unit_y(), Vec3::unit_z()),
        3 => Message::FixPoint(-Vec3::unit_y(), -Vec3::unit_z()),
        4 => Message::FixPoint(Vec3::unit_z(), Vec3::unit_y()),
        _ => Message::FixPoint(-Vec3::unit_z(), Vec3::unit_y()),
    }
}

fn rotation_message(i: usize, _xz: isize, _yz: isize, _xy: isize) -> Message {
    let angle_xz = match i {
        0 => 15f32.to_radians(),
        1 => -15f32.to_radians(),
        _ => 0f32,
    };
    let angle_yz = match i {
        2 => -15f32.to_radians(),
        3 => 15f32.to_radians(),
        _ => 0f32,
    };
    let angle_xy = match i {
        4 => 15f32.to_radians(),
        5 => -15f32.to_radians(),
        _ => 0f32,
    };
    Message::RotateCam(angle_xz, angle_yz, angle_xy)
}

fn rotation_text(i: usize, ui_size: UiSize) -> Text {
    match i {
        0 => icon(MaterialIcon::ArrowBack, &ui_size),
        1 => icon(MaterialIcon::ArrowForward, &ui_size),
        2 => icon(MaterialIcon::ArrowUpward, &ui_size),
        3 => icon(MaterialIcon::ArrowDownward, &ui_size),
        4 => icon(MaterialIcon::Undo, &ui_size),
        _ => icon(MaterialIcon::Redo, &ui_size),
    }
}

fn target_text(i: usize) -> String {
    match i {
        0 => "Right".to_string(),
        1 => "Left".to_string(),
        2 => "Top".to_string(),
        3 => "Bottom".to_string(),
        4 => "Back".to_string(),
        _ => "Front".to_string(),
    }
}

mod text_input_style {
    use iced::{Background, Color};
    use iced_wgpu::text_input::*;
    pub struct BadValue(pub bool);
    impl iced_wgpu::text_input::StyleSheet for BadValue {
        fn active(&self) -> Style {
            Style {
                background: Background::Color(Color::WHITE),
                border_radius: 5.0,
                border_width: 1.0,
                border_color: Color::from_rgb(0.7, 0.7, 0.7),
            }
        }

        fn focused(&self) -> Style {
            Style {
                border_color: Color::from_rgb(0.5, 0.5, 0.5),
                ..self.active()
            }
        }

        fn placeholder_color(&self) -> Color {
            Color::from_rgb(0.7, 0.7, 0.7)
        }

        fn value_color(&self) -> Color {
            if self.0 {
                Color::from_rgb(0.3, 0.3, 0.3)
            } else {
                Color::from_rgb(1., 0.3, 0.3)
            }
        }

        fn selection_color(&self) -> Color {
            Color::from_rgb(0.8, 0.8, 1.0)
        }
    }
}

pub struct Hyperboloid_ {}

impl Requestable for Hyperboloid_ {
    type Request = HyperboloidRequest;
    fn request_from_values(&self, values: &[f32]) -> HyperboloidRequest {
        HyperboloidRequest {
            radius: values[0].round() as usize,
            length: values[1],
            shift: values[2],
            radius_shift: values[3],
        }
    }
    fn nb_values(&self) -> usize {
        4
    }
    fn initial_value(&self, n: usize) -> f32 {
        match n {
            0 => 10f32,
            1 => 30f32,
            2 => 0f32,
            3 => 0.2f32,
            _ => unreachable!(),
        }
    }
    fn min_val(&self, n: usize) -> f32 {
        use std::f32::consts::PI;
        match n {
            0 => 5f32,
            1 => 1f32,
            2 => -PI + 1f32.to_radians(),
            3 => 0.,
            _ => unreachable!(),
        }
    }

    fn max_val(&self, n: usize) -> f32 {
        use std::f32::consts::PI;
        match n {
            0 => 60f32,
            1 => 200f32,
            2 => PI - 1f32.to_radians(),
            3 => 1f32,
            _ => unreachable!(),
        }
    }
    fn step_val(&self, n: usize) -> f32 {
        match n {
            0 => 1f32,
            1 => 1f32,
            2 => 1f32.to_radians(),
            3 => 0.01,
            _ => unreachable!(),
        }
    }
    fn name_val(&self, n: usize) -> String {
        match n {
            0 => String::from("Nb helices"),
            1 => String::from("Strands length"),
            2 => String::from("Angle shift"),
            3 => String::from("Tube radius"),
            _ => unreachable!(),
        }
    }

    fn hidden(&self, n: usize) -> bool {
        n == 2 || n == 3
    }
}

struct ScrollSentivity {}

impl Requestable for ScrollSentivity {
    type Request = f32;
    fn request_from_values(&self, values: &[f32]) -> f32 {
        values[0]
    }
    fn nb_values(&self) -> usize {
        1
    }
    fn initial_value(&self, n: usize) -> f32 {
        if n == 0 {
            0f32
        } else {
            unreachable!()
        }
    }
    fn min_val(&self, n: usize) -> f32 {
        if n == 0 {
            -10f32
        } else {
            unreachable!()
        }
    }
    fn max_val(&self, n: usize) -> f32 {
        if n == 0 {
            10f32
        } else {
            unreachable!()
        }
    }
    fn step_val(&self, n: usize) -> f32 {
        if n == 0 {
            0.5f32
        } else {
            unreachable!()
        }
    }
    fn name_val(&self, n: usize) -> String {
        if n == 0 {
            String::from("Sentivity")
        } else {
            unreachable!()
        }
    }
}

struct HelixRoll {}

impl Requestable for HelixRoll {
    type Request = f32;
    fn request_from_values(&self, values: &[f32]) -> f32 {
        values[0]
    }
    fn nb_values(&self) -> usize {
        1
    }
    fn initial_value(&self, n: usize) -> f32 {
        match n {
            0 => 0f32,
            _ => unreachable!(),
        }
    }
    fn min_val(&self, n: usize) -> f32 {
        use std::f32::consts::PI;
        match n {
            0 => -PI,
            _ => unreachable!(),
        }
    }
    fn max_val(&self, n: usize) -> f32 {
        use std::f32::consts::PI;
        match n {
            0 => PI,
            _ => unreachable!(),
        }
    }
    fn step_val(&self, n: usize) -> f32 {
        match n {
            0 => 1f32.to_radians(),
            _ => unreachable!(),
        }
    }
    fn name_val(&self, n: usize) -> String {
        match n {
            0 => String::from("Roll helix"),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct RigidBodyParametersRequest {
    pub k_springs: f32,
    pub k_friction: f32,
    pub mass_factor: f32,
    pub volume_exclusion: bool,
    pub brownian_motion: bool,
    pub brownian_rate: f32,
    pub brownian_amplitude: f32,
}

struct RigidBodyFactory {
    pub volume_exclusion: bool,
    pub brownian_motion: bool,
    pub brownian_parameters: BrownianParametersFactory,
}

#[derive(Clone)]
struct BrownianParametersFactory {
    pub rate: f32,
    pub amplitude: f32,
}

impl Requestable for BrownianParametersFactory {
    type Request = Self;
    fn request_from_values(&self, values: &[f32]) -> Self {
        Self {
            rate: values[0],
            amplitude: values[1],
        }
    }

    fn nb_values(&self) -> usize {
        2
    }

    fn initial_value(&self, n: usize) -> f32 {
        match n {
            0 => 0.,
            1 => 0.08,
            _ => unreachable!(),
        }
    }

    fn min_val(&self, n: usize) -> f32 {
        match n {
            0 => -2.,
            1 => 0.,
            _ => unreachable!(),
        }
    }

    fn max_val(&self, n: usize) -> f32 {
        match n {
            0 => 2.,
            1 => 0.2,
            _ => unreachable!(),
        }
    }

    fn step_val(&self, n: usize) -> f32 {
        match n {
            0 => 0.1,
            1 => 0.02,
            _ => unreachable!(),
        }
    }

    fn name_val(&self, n: usize) -> String {
        match n {
            0 => "Rate (log scale)".to_owned(),
            1 => "Range".to_owned(),
            _ => unreachable!(),
        }
    }
}

impl Requestable for RigidBodyFactory {
    type Request = RigidBodyParametersRequest;
    fn request_from_values(&self, values: &[f32]) -> RigidBodyParametersRequest {
        RigidBodyParametersRequest {
            k_springs: values[0],
            k_friction: values[1],
            mass_factor: values[2],
            volume_exclusion: self.volume_exclusion,
            brownian_motion: self.brownian_motion,
            brownian_rate: self.brownian_parameters.rate,
            brownian_amplitude: self.brownian_parameters.amplitude,
        }
    }
    fn nb_values(&self) -> usize {
        3
    }
    fn initial_value(&self, n: usize) -> f32 {
        match n {
            0 => 0f32,
            1 => 0f32,
            2 => 0f32,
            _ => unreachable!(),
        }
    }
    fn min_val(&self, n: usize) -> f32 {
        match n {
            0 => -4.,
            1 => -4.,
            2 => -4.,
            3 => -4.,
            _ => unreachable!(),
        }
    }
    fn max_val(&self, n: usize) -> f32 {
        match n {
            0 => 4.,
            1 => 4.,
            2 => 4.,
            3 => 4.,
            _ => unreachable!(),
        }
    }
    fn step_val(&self, n: usize) -> f32 {
        match n {
            0 => 0.1f32,
            1 => 0.1f32,
            2 => 0.1f32,
            3 => 0.1f32,
            _ => unreachable!(),
        }
    }
    fn name_val(&self, n: usize) -> String {
        match n {
            0 => String::from("Stiffness (log scale)"),
            1 => String::from("Friction (log scale)"),
            2 => String::from("Mass (log scale)"),
            _ => unreachable!(),
        }
    }
}

fn use_default_scaffold<R: Requests>(requests: Arc<Mutex<R>>) {
    requests.lock().unwrap().ask_use_default_scaffold();
    /*
    crate::utils::yes_no_dialog(
        "Use default m13 sequence".into(),
        requests,
        KeepProceed::DefaultScaffold,
        Some(KeepProceed::CustomScaffold),
    )*/
}

#[derive(Clone, Copy, Debug)]
struct TabStyle;

impl iced_aw::style::tab_bar::StyleSheet for TabStyle {
    fn active(&self, is_active: bool) -> iced_aw::style::tab_bar::Style {
        iced_aw::style::tab_bar::Style {
            background: None,
            border_color: None,
            border_width: 0.0,
            tab_label_background: if !is_active {
                Background::Color([0.9, 0.9, 0.9].into())
            } else {
                Background::Color([0.6, 0.6, 0.6].into())
            },
            tab_label_border_color: [0.7, 0.7, 0.7].into(),
            tab_label_border_width: 1.0,
            icon_color: Color::BLACK,
            text_color: Color::BLACK,
        }
    }

    fn hovered(&self, is_active: bool) -> iced_aw::style::tab_bar::Style {
        iced_aw::style::tab_bar::Style {
            tab_label_background: Background::Color([0.6, 0.6, 0.6].into()),
            ..self.active(is_active)
        }
    }
}
