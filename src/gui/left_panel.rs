use ensnano_organizer::{Organizer, OrganizerMessage, OrganizerTree};
use std::sync::{Arc, Mutex};

use iced::{
    button, pick_list, scrollable, slider, text_input, Button, Checkbox, Color, Command, Element,
    Length, PickList, Scrollable, Slider, Text, TextInput,
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

use crate::design::{DnaElement, DnaElementKey};
use crate::mediator::{ActionMode, SelectionMode};

use super::{text_btn, FogParameters as Fog, OverlayType, Requests, UiSize};
mod color_picker;
use color_picker::ColorPicker;
mod sequence_input;
use sequence_input::SequenceInput;
use text_input_style::BadValue;
mod discrete_value;
use discrete_value::{FactoryId, RequestFactory, Requestable, ValueId};
mod tabs;

use material_icons::{icon_to_char, Icon as MaterialIcon, FONT as MATERIALFONT};
use std::collections::BTreeMap;
use tabs::{CameraTab, EditionTab, GridTab};

const ICONFONT: iced::Font = iced::Font::External {
    name: "IconFont",
    bytes: MATERIALFONT,
};

fn icon(icon: MaterialIcon, ui_size: &UiSize) -> iced::Text {
    iced::Text::new(format!("{}", icon_to_char(icon)))
        .font(ICONFONT)
        .size(ui_size.icon())
}

const CHECKBOXSPACING: u16 = 5;

pub struct LeftPanel {
    selection_mode: SelectionMode,
    action_mode: ActionMode,
    global_scroll: scrollable::State,
    logical_size: LogicalSize<f64>,
    #[allow(dead_code)]
    logical_position: LogicalPosition<f64>,
    #[allow(dead_code)]
    open_color: button::State,
    sequence_input: SequenceInput,
    requests: Arc<Mutex<Requests>>,
    color_picker: ColorPicker,
    length_helices: usize,
    position_helices: isize,
    show_torsion: bool,
    physical_simulation: PhysicalSimulation,
    scroll_sensitivity_factory: RequestFactory<ScrollSentivity>,
    hyperboloid_factory: RequestFactory<Hyperboloid_>,
    helix_roll_factory: RequestFactory<HelixRoll>,
    rigid_body_factory: RequestFactory<RigidBodyFactory>,
    rigid_grid_button: GoStop,
    rigid_helices_button: GoStop,
    selected_tab: usize,
    organizer: Organizer<DnaElement>,
    ui_size: UiSize,
    size_pick_list: pick_list::State<UiSize>,
    grid_tab: GridTab,
    edition_tab: EditionTab,
    camera_tab: CameraTab,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectionModeChanged(SelectionMode),
    Resized(LogicalSize<f64>, LogicalPosition<f64>),
    #[allow(dead_code)]
    OpenColor,
    ActionModeChanged(ActionMode),
    SequenceChanged(String),
    SequenceFileRequested,
    StrandColorChanged(Color),
    HueChanged(f32),
    NewGrid,
    FixPoint(Vec3, Vec3),
    RotateCam(f32, f32),
    PositionHelicesChanged(String),
    LengthHelicesChanged(String),
    ShowTorsion(bool),
    FogVisibility(bool),
    FogRadius(f32),
    FogLength(f32),
    FogCamera(bool),
    SimRoll(bool),
    SimSprings(bool),
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
    RigidGridSimulation(bool),
    RigidHelicesSimulation(bool),
    VolumeExclusion(bool),
    TabSelected(usize),
    NewDnaElement(Vec<DnaElement>),
    NewSelection(Vec<DnaElementKey>),
    OrganizerMessage(OrganizerMessage<DnaElement>),
    ModifiersChanged(ModifiersState),
    NewTreeApp(OrganizerTree<DnaElementKey>),
    UiSizeChanged(UiSize),
    UiSizePicked(UiSize),
}

impl LeftPanel {
    pub fn new(
        requests: Arc<Mutex<Requests>>,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
    ) -> Self {
        Self {
            selection_mode: Default::default(),
            action_mode: Default::default(),
            global_scroll: Default::default(),
            logical_size,
            logical_position,
            open_color: Default::default(),
            sequence_input: SequenceInput::new(),
            requests,
            color_picker: ColorPicker::new(),
            length_helices: 0,
            position_helices: 0,
            show_torsion: false,
            physical_simulation: Default::default(),
            scroll_sensitivity_factory: RequestFactory::new(FactoryId::Scroll, ScrollSentivity {}),
            helix_roll_factory: RequestFactory::new(FactoryId::HelixRoll, HelixRoll {}),
            hyperboloid_factory: RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {}),
            rigid_body_factory: RequestFactory::new(
                FactoryId::RigidBody,
                RigidBodyFactory {
                    volume_exclusion: false,
                },
            ),
            rigid_helices_button: GoStop::new(
                String::from("Rigid Helices"),
                Message::RigidHelicesSimulation,
            ),
            rigid_grid_button: GoStop::new(
                String::from("Rigid Grids"),
                Message::RigidGridSimulation,
            ),
            selected_tab: 0,
            organizer: Organizer::new(),
            ui_size: UiSize::Small,
            size_pick_list: Default::default(),
            grid_tab: GridTab::new(),
            edition_tab: EditionTab::new(),
            camera_tab: CameraTab::new(),
        }
    }

    pub fn resize(
        &mut self,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
    ) {
        self.logical_size = logical_size;
        self.logical_position = logical_position;
    }

    fn organizer_message(&mut self, m: OrganizerMessage<DnaElement>) -> Option<Message> {
        match m {
            OrganizerMessage::InternalMessage(m) => {
                return self
                    .organizer
                    .message(&m)
                    .map(|m_| Message::OrganizerMessage(m_))
            }
            OrganizerMessage::Selection(s) => {
                self.requests.lock().unwrap().organizer_selection = Some(s)
            }
            OrganizerMessage::NewAttribute(a, keys) => {
                self.requests.lock().unwrap().new_attribute = Some((a, keys.into_iter().collect()))
            }
            OrganizerMessage::NewTree(tree) => self.requests.lock().unwrap().new_tree = Some(tree),
            OrganizerMessage::Candidates(candidates) => {
                self.requests.lock().unwrap().organizer_candidates = Some(candidates)
            }
            _ => (),
        }
        None
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.sequence_input.has_keyboard_priority() || self.grid_tab.has_keyboard_priority()
    }
}

impl Program for LeftPanel {
    type Renderer = Renderer;
    type Message = Message;
    type Clipboard = NullClipboard;

    fn update(&mut self, message: Message, _cb: &mut NullClipboard) -> Command<Message> {
        match message {
            Message::SelectionModeChanged(selection_mode) => {
                if selection_mode != self.selection_mode {
                    let action_mode = if self.action_mode.is_build() {
                        match selection_mode {
                            SelectionMode::Grid => Some(ActionMode::BuildHelix {
                                position: self.position_helices,
                                length: self.length_helices,
                            }),
                            _ => {
                                if let ActionMode::BuildHelix { .. } = self.action_mode {
                                    Some(ActionMode::Build(false))
                                } else {
                                    None
                                }
                            }
                        }
                    } else {
                        None
                    };
                    self.selection_mode = selection_mode;
                    if let Some(action_mode) = action_mode {
                        self.action_mode = action_mode.clone();
                        self.requests.lock().unwrap().action_mode = Some(action_mode);
                    }
                    self.requests.lock().unwrap().selection_mode = Some(selection_mode);
                }
            }
            Message::ActionModeChanged(action_mode) => {
                let action_mode = if action_mode.is_build() {
                    match self.selection_mode {
                        SelectionMode::Grid => ActionMode::BuildHelix {
                            position: self.position_helices,
                            length: self.length_helices,
                        },
                        _ => action_mode,
                    }
                } else {
                    action_mode
                };
                if self.action_mode != action_mode {
                    self.action_mode = action_mode;
                    self.requests.lock().unwrap().action_mode = Some(action_mode)
                }
            }
            Message::SequenceChanged(s) => {
                self.requests.lock().unwrap().sequence_change = Some(s.clone());
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
                                requests.lock().unwrap().sequence_input = Some(content);
                            }
                        }
                    };
                    futures::executor::block_on(save_op);
                });
            }
            Message::OpenColor => {
                self.requests.lock().unwrap().overlay_opened = Some(OverlayType::Color)
            }
            Message::StrandColorChanged(color) => {
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
                self.requests.lock().unwrap().strand_color_change = Some(color);
            }
            Message::HueChanged(x) => self.color_picker.change_hue(x),
            Message::Resized(size, position) => self.resize(size, position),
            Message::NewGrid => self.requests.lock().unwrap().new_grid = true,
            Message::RotateCam(xz, yz) => {
                self.camera_tab.set_angles(xz as isize, yz as isize);
                self.requests.lock().unwrap().camera_rotation = Some((xz, yz));
            }
            Message::FixPoint(point, up) => {
                self.requests.lock().unwrap().camera_target = Some((point, up));
                self.camera_tab.reset_angles();
            }
            Message::LengthHelicesChanged(length_str) => {
                let action_mode = self.grid_tab.update_length_str(length_str.clone());
                if self.action_mode != action_mode {
                    self.action_mode = action_mode;
                    self.requests.lock().unwrap().action_mode = Some(action_mode)
                }
            }
            Message::PositionHelicesChanged(position_str) => {
                let action_mode = self.grid_tab.update_pos_str(position_str.clone());
                if self.action_mode != action_mode {
                    self.action_mode = action_mode;
                    self.requests.lock().unwrap().action_mode = Some(action_mode)
                }
            }
            Message::ShowTorsion(b) => {
                self.requests.lock().unwrap().show_torsion_request = Some(b);
                self.show_torsion = b;
            }
            Message::FogVisibility(b) => {
                self.camera_tab.fog_visible(b);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().fog = Some(request);
            }
            Message::FogLength(length) => {
                self.camera_tab.fog_length(length);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().fog = Some(request);
            }
            Message::FogRadius(radius) => {
                self.camera_tab.fog_radius(radius);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().fog = Some(request);
            }
            Message::NewDesign => {
                self.show_torsion = false;
                self.physical_simulation.running = false;
                self.camera_tab.notify_new_design();
                self.rigid_grid_button.running = false;
                self.rigid_helices_button.running = false;
            }
            Message::SimRoll(b) => {
                self.physical_simulation.roll = b;
            }
            Message::SimSprings(b) => {
                self.physical_simulation.springs = b;
            }
            Message::SimRequest => {
                self.physical_simulation.running ^= true;
                self.requests.lock().unwrap().roll_request =
                    Some(self.physical_simulation.request());
            }
            Message::FogCamera(b) => {
                self.camera_tab.fog_camera(b);
                let request = self.camera_tab.get_fog_request();
                self.requests.lock().unwrap().fog = Some(request);
            }
            Message::DescreteValue {
                factory_id,
                value_id,
                value,
            } => match factory_id {
                FactoryId::Scroll => {
                    let request = &mut self.requests.lock().unwrap().scroll_sensitivity;
                    self.scroll_sensitivity_factory
                        .update_request(value_id, value, request);
                }
                FactoryId::HelixRoll => {
                    let request = &mut self.requests.lock().unwrap().helix_roll;
                    self.helix_roll_factory
                        .update_request(value_id, value, request);
                }
                FactoryId::Hyperboloid => {
                    let request = &mut self.requests.lock().unwrap().hyperboloid_update;
                    self.hyperboloid_factory
                        .update_request(value_id, value, request);
                }
                FactoryId::RigidBody => {
                    let request = &mut self.requests.lock().unwrap().rigid_body_parameters;
                    self.rigid_body_factory
                        .update_request(value_id, value, request)
                }
            },
            Message::VolumeExclusion(b) => {
                self.rigid_body_factory.requestable.volume_exclusion = b;
                let request = &mut self.requests.lock().unwrap().rigid_body_parameters;
                self.rigid_body_factory.make_request(request);
            }
            Message::HelixRoll(roll) => {
                self.helix_roll_factory.update_roll(roll);
            }
            Message::NewHyperboloid => {
                let request = &mut self.requests.lock().unwrap().new_hyperboloid;
                self.grid_tab.new_hyperboloid(request);
            }
            Message::FinalizeHyperboloid => {
                self.requests.lock().unwrap().finalize_hyperboloid = true;
                self.grid_tab.finalize_hyperboloid();
            }
            Message::RigidGridSimulation(b) => {
                let request = &mut self.requests.lock().unwrap().rigid_grid_simulation;
                self.rigid_grid_button.running = b;
                self.rigid_body_factory.make_request(request);
            }
            Message::RigidHelicesSimulation(b) => {
                let request = &mut self.requests.lock().unwrap().rigid_helices_simulation;
                self.rigid_helices_button.running = b;
                self.rigid_body_factory.make_request(request);
            }
            Message::TabSelected(n) => self.selected_tab = n,
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
                self.organizer.notify_selection(keys);
            }
            Message::NewTreeApp(tree) => self.organizer.read_tree(tree),
            Message::UiSizePicked(ui_size) => {
                self.requests.lock().unwrap().new_ui_size = Some(ui_size)
            }
            Message::UiSizeChanged(ui_size) => self.ui_size = ui_size,
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        let width = self.logical_size.cast::<u16>().width;
        let tabs: Tabs<Message, Backend> = Tabs::new(self.selected_tab, Message::TabSelected)
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::Edit))),
                self.edition_tab.view(
                    self.action_mode,
                    self.selection_mode,
                    self.ui_size.clone(),
                    width,
                ),
            )
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::Apps))),
                self.grid_tab.view(
                    self.action_mode,
                    self.selection_mode,
                    self.ui_size.clone(),
                    width,
                ),
            )
            .push(
                TabLabel::Text(format!("{}", icon_to_char(MaterialIcon::Videocam))),
                self.camera_tab.view(self.ui_size.clone(), width),
            )
            .text_size(self.ui_size.icon())
            .text_font(ICONFONT)
            .tab_bar_height(Length::Units(self.ui_size.button()))
            .width(Length::Units(width))
            .height(Length::Fill);
        let contextual_menu = iced::Space::new(Length::Fill, Length::Fill);
        let organizer = self.organizer.view().map(|m| Message::OrganizerMessage(m));

        Container::new(
            Column::new()
                .width(Length::Fill)
                .push(Container::new(tabs).height(Length::FillPortion(1)))
                .push(iced::Rule::horizontal(5))
                .push(Container::new(contextual_menu).height(Length::FillPortion(1)))
                .push(iced::Rule::horizontal(5))
                .push(Container::new(organizer).height(Length::FillPortion(1))),
        )
        .style(TopBarStyle)
        .height(Length::Units(self.logical_size.height as u16))
        .into()
    }
    /*
    fn view(&mut self) -> Element<Message> {
        let width = self.logical_size.cast::<u16>().width;
        let ui_size = self.ui_size.clone();

        let mut global_scroll = Scrollable::new(&mut self.global_scroll)
            .spacing(5)
            .push(Text::new("SelectionMode"));

        global_scroll = global_scroll.push(self.grid_tab.view(
            self.action_mode,
            self.selection_mode,
            ui_size.clone(),
            width,
        ));

        let mut target_buttons: Vec<_> = self
            .camera_target_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, Text::new(target_text(i)).size(10))
                    .on_press(target_message(i))
                    .width(Length::Units(ui_size.button()))
            })
            .collect();
        global_scroll = global_scroll.spacing(5).push(Text::new("Camera Target"));
        while target_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(target_buttons.remove(0)).spacing(5);
            let mut space = self.ui_size.button() + 5;
            while space + self.ui_size.button() < width && target_buttons.len() > 0 {
                row = row.push(target_buttons.remove(0)).spacing(5);
                space += self.ui_size.button() + 5;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }

        let xz = self.xz;
        let yz = self.yz;

        let mut rotate_buttons: Vec<_> = self
            .camera_rotation_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, rotation_text(i, ui_size.clone()))
                    .on_press(rotation_message(i, xz, yz))
                    .width(Length::Units(ui_size.button()))
            })
            .collect();

        global_scroll = global_scroll.spacing(5).push(Text::new("Rotate Camera"));
        while rotate_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(rotate_buttons.remove(0)).spacing(5);
            let mut space = self.ui_size.button() + 5;
            while space + self.ui_size.button() < width && rotate_buttons.len() > 0 {
                row = row.push(rotate_buttons.remove(0)).spacing(5);
                space += self.ui_size.button() + 5;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }
        global_scroll = global_scroll
            .push(self.physical_simulation.view(&self.ui_size))
            .max_height(self.logical_size.height as u32);

        let mut widget = global_scroll
            .push(
                Checkbox::new(self.show_torsion, "Show Torsion", Message::ShowTorsion)
                    .size(self.ui_size.checkbox())
                    .spacing(CHECKBOXSPACING),
            )
            .width(Length::Units(width));

        let color_square = self.color_picker.color_square();
        if self.selection_mode == SelectionMode::Strand {
            widget = widget
                .spacing(5)
                .push(self.color_picker.view())
                .push(
                    Row::new()
                        .push(color_square)
                        .push(iced::Space::new(Length::FillPortion(4), Length::Shrink)),
                )
                .push(self.sequence_input.view());
        }
        widget = widget.push(self.fog.view(&self.ui_size));
        for view in self.scroll_sensitivity_factory.view().into_iter() {
            widget = widget.push(view);
        }
        widget = widget
            .push(self.rigid_grid_button.view())
            .push(self.rigid_helices_button.view());

        let volume_exclusion = self.rigid_body_factory.requestable.volume_exclusion;
        for view in self.rigid_body_factory.view().into_iter() {
            widget = widget.push(view);
        }
        widget = widget.push(
            Checkbox::new(
                volume_exclusion,
                "Volume exclusion",
                Message::VolumeExclusion,
            )
            .spacing(CHECKBOXSPACING)
            .size(self.ui_size.checkbox()),
        );

        widget = widget.push(PickList::new(
            &mut self.size_pick_list,
            &super::ALL_UI_SIZE[..],
            Some(self.ui_size.clone()),
            Message::UiSizePicked,
        ));

        let tabs: Tabs<Message, Backend> = Tabs::new(self.selected_tab, Message::TabSelected)
            .push(TabLabel::Text("Menu".to_owned()), widget)
            .push(
                TabLabel::Text("Organizer".to_owned()),
                self.organizer.view().map(|m| Message::OrganizerMessage(m)),
            )
            .width(Length::Units(width))
            .height(Length::Units(self.logical_size.height as u16 - 20));

        Container::new(tabs)
            .style(TopBarStyle)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }*/
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

pub struct ColorOverlay {
    logical_size: LogicalSize<f64>,
    color_picker: ColorPicker,
    close_button: iced::button::State,
    requests: Arc<Mutex<Requests>>,
}

impl ColorOverlay {
    pub fn new(requests: Arc<Mutex<Requests>>, logical_size: LogicalSize<f64>) -> Self {
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

impl Program for ColorOverlay {
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
                self.requests.lock().unwrap().strand_color_change = Some(color);
            }
            ColorMessage::HueChanged(x) => self.color_picker.change_hue(x),
            ColorMessage::Closed => {
                self.requests.lock().unwrap().overlay_closed = Some(OverlayType::Color)
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
            ..Default::default()
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
    ) -> BTreeMap<ActionMode, &'a mut button::State> {
        let mut ret = BTreeMap::new();
        ret.insert(ActionMode::Normal, &mut self.select);
        ret.insert(ActionMode::Translate, &mut self.translate);
        ret.insert(ActionMode::Rotate, &mut self.rotate);
        ret.insert(
            ActionMode::BuildHelix {
                position: position_helix,
                length: len_helix,
            },
            &mut self.build,
        );
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

fn rotation_message(i: usize, xz: isize, yz: isize) -> Message {
    let angle_xz = match i {
        0 => {
            if xz % 90 == 30 || xz % 90 == 45 {
                15f32.to_radians()
            } else {
                30f32.to_radians()
            }
        }
        1 => {
            if xz % 90 == 60 || xz % 90 == 45 {
                -15f32.to_radians()
            } else {
                -30f32.to_radians()
            }
        }
        _ => 0f32,
    };
    let angle_yz = match i {
        2 => {
            if yz % 90 == 30 || yz % 90 == 45 {
                -15f32.to_radians()
            } else {
                -30f32.to_radians()
            }
        }
        3 => {
            if yz % 90 == 60 || yz % 90 == 45 {
                15f32.to_radians()
            } else {
                30f32.to_radians()
            }
        }
        _ => 0f32,
    };
    Message::RotateCam(angle_xz, angle_yz)
}

fn rotation_text(i: usize, ui_size: UiSize) -> Text {
    match i {
        0 => icon(MaterialIcon::ArrowBack, &ui_size),
        1 => icon(MaterialIcon::ArrowForward, &ui_size),
        2 => icon(MaterialIcon::ArrowUpward, &ui_size),
        _ => icon(MaterialIcon::ArrowDownward, &ui_size),
    }
}

fn target_text(i: usize) -> String {
    match i {
        0 => "X+".to_string(),
        1 => "X-".to_string(),
        2 => "Y+".to_string(),
        3 => "Y-".to_string(),
        4 => "Z+".to_string(),
        _ => "Z-".to_string(),
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

#[derive(Default)]
struct PhysicalSimulation {
    go_stop_button: button::State,
    pub running: bool,
    pub roll: bool,
    pub springs: bool,
}

impl PhysicalSimulation {
    fn view(&mut self, ui_size: &UiSize) -> Row<Message> {
        let left_column = Column::new()
            .push(
                Checkbox::new(self.roll, "Roll", Message::SimRoll)
                    .size(ui_size.checkbox())
                    .spacing(CHECKBOXSPACING),
            )
            .push(
                Checkbox::new(self.springs, "Spring", Message::SimSprings)
                    .size(ui_size.checkbox())
                    .spacing(CHECKBOXSPACING),
            );
        let button_str = if self.running { "Stop" } else { "Go" };
        let right_column = Column::new().push(
            Button::new(&mut self.go_stop_button, Text::new(button_str))
                .on_press(Message::SimRequest),
        );
        Row::new().push(left_column).push(right_column)
    }

    fn request(&self) -> SimulationRequest {
        SimulationRequest {
            roll: self.roll,
            springs: self.springs,
        }
    }
}

struct GoStop {
    go_stop_button: button::State,
    pub running: bool,
    pub name: String,
    on_press: Box<dyn Fn(bool) -> Message>,
}

impl GoStop {
    fn new<F>(name: String, on_press: F) -> Self
    where
        F: 'static + Fn(bool) -> Message,
    {
        Self {
            go_stop_button: Default::default(),
            running: false,
            name,
            on_press: Box::new(on_press),
        }
    }

    fn view(&mut self) -> Row<Message> {
        let left_column = Column::new().push(Text::new(self.name.to_string()));
        let button_str = if self.running { "Stop" } else { "Go" };
        let right_column = Column::new().push(
            Button::new(&mut self.go_stop_button, Text::new(button_str))
                .on_press((self.on_press)(!self.running))
                .style(ButtonColor::red_green(self.running)),
        );
        Row::new().push(left_column).push(right_column)
    }
}

#[derive(Clone)]
pub struct SimulationRequest {
    pub roll: bool,
    pub springs: bool,
}

#[derive(Clone)]
pub struct HyperboloidRequest {
    pub radius: usize,
    pub length: f32,
    pub shift: f32,
    pub radius_shift: f32,
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
            1 => 100f32,
            2 => 0f32,
            3 => 1f32,
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
            1 => String::from("Length"),
            2 => String::from("Angle shift"),
            3 => String::from("Size"),
            _ => unreachable!(),
        }
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
            -20f32
        } else {
            unreachable!()
        }
    }
    fn max_val(&self, n: usize) -> f32 {
        if n == 0 {
            20f32
        } else {
            unreachable!()
        }
    }
    fn step_val(&self, n: usize) -> f32 {
        if n == 0 {
            1f32
        } else {
            unreachable!()
        }
    }
    fn name_val(&self, n: usize) -> String {
        if n == 0 {
            String::from("ScrollSentivity")
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
}

struct RigidBodyFactory {
    pub volume_exclusion: bool,
}

impl Requestable for RigidBodyFactory {
    type Request = RigidBodyParametersRequest;
    fn request_from_values(&self, values: &[f32]) -> RigidBodyParametersRequest {
        RigidBodyParametersRequest {
            k_springs: values[0],
            k_friction: values[1],
            mass_factor: values[2],
            volume_exclusion: self.volume_exclusion,
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
            0 => String::from("K spring"),
            1 => String::from("K friction"),
            2 => String::from("mass helix"),
            _ => unreachable!(),
        }
    }
}
