use std::sync::{Arc, Mutex};

use iced::{container, Background, Column, Container, Image, Row};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::{LogicalPosition, LogicalSize};
use iced_winit::{
    button, scrollable, slider, text_input, Button, Checkbox, Color, Command, Element, Length,
    Program, Scrollable, Slider, Text, TextInput,
};
use native_dialog::FileDialog;
use ultraviolet::Vec3;

use color_space::{Hsv, Rgb};

use crate::mediator::{ActionMode, SelectionMode};

use super::{FogParameters as Fog, OverlayType, Requests};
mod color_picker;
use color_picker::ColorPicker;
mod sequence_input;
use sequence_input::SequenceInput;
use text_input_style::BadValue;
mod discrete_value;
use discrete_value::{FactoryId, RequestFactory, Requestable, ValueId};

const BUTTON_SIZE: u16 = 40;

pub struct LeftPanel {
    selection_mode: SelectionMode,
    action_mode: ActionMode,
    global_scroll: scrollable::State,
    logical_size: LogicalSize<f64>,
    logical_position: LogicalPosition<f64>,
    #[allow(dead_code)]
    open_color: button::State,
    sequence_input: SequenceInput,
    requests: Arc<Mutex<Requests>>,
    action_mode_state: ActionModeState,
    selection_mode_state: SelectionModeState,
    color_picker: ColorPicker,
    camera_target_buttons: [button::State; 6],
    camera_rotation_buttons: [button::State; 4],
    xz: isize,
    yz: isize,
    length_helices: usize,
    position_helices: isize,
    length_str: String,
    position_str: String,
    builder_input: [text_input::State; 2],
    show_torsion: bool,
    fog: FogParameters,
    physical_simulation: PhysicalSimulation,
    scroll_sensitivity_factory: RequestFactory<ScrollSentivity>,
    hyperboloid_factory: RequestFactory<Hyperboloid_>,
    helix_roll_factory: RequestFactory<HelixRoll>,
    rigid_body_factory: RequestFactory<RigidBodyFactory>,
    building_hyperboloid: bool,
    finalize_hyperboloid: button::State,
    rigid_grid_button: GoStop,
    rigid_helices_button: GoStop,
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
            action_mode_state: Default::default(),
            selection_mode_state: Default::default(),
            color_picker: ColorPicker::new(),
            camera_rotation_buttons: Default::default(),
            camera_target_buttons: Default::default(),
            xz: 0,
            yz: 0,
            builder_input: Default::default(),
            length_helices: 0,
            position_helices: 0,
            length_str: "0".to_string(),
            position_str: "0".to_string(),
            show_torsion: false,
            fog: Default::default(),
            physical_simulation: Default::default(),
            scroll_sensitivity_factory: RequestFactory::new(0, ScrollSentivity {}),
            helix_roll_factory: RequestFactory::new(1, HelixRoll {}),
            hyperboloid_factory: RequestFactory::new(2, Hyperboloid_ {}),
            rigid_body_factory: RequestFactory::new(3, RigidBodyFactory {}),
            building_hyperboloid: false,
            finalize_hyperboloid: Default::default(),
            rigid_helices_button: GoStop::new(
                String::from("Rigid Helices"),
                Message::RigidHelicesSimulation,
            ),
            rigid_grid_button: GoStop::new(
                String::from("Rigid Grids"),
                Message::RigidGridSimulation,
            ),
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
}

impl Program for LeftPanel {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
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
                let result = FileDialog::new().show_open_single_file();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        let content = std::fs::read_to_string(path);
                        if let Ok(content) = content {
                            self.update(Message::SequenceChanged(content));
                        }
                    }
                }
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
                self.xz = xz as isize;
                self.yz = yz as isize;
                self.requests.lock().unwrap().camera_rotation = Some((xz, yz));
            }
            Message::FixPoint(point, up) => {
                self.requests.lock().unwrap().camera_target = Some((point, up));
                self.xz = 0;
                self.yz = 0;
            }
            Message::LengthHelicesChanged(length_str) => {
                if let Ok(length) = length_str.parse::<usize>() {
                    self.length_helices = length
                }
                self.length_str = length_str;
                let action_mode = ActionMode::BuildHelix {
                    position: self.position_helices,
                    length: self.length_helices,
                };
                if self.action_mode != action_mode {
                    self.action_mode = action_mode;
                    self.requests.lock().unwrap().action_mode = Some(action_mode)
                }
            }
            Message::PositionHelicesChanged(position_str) => {
                if let Ok(position) = position_str.parse::<isize>() {
                    self.position_helices = position
                }
                self.position_str = position_str;
                let action_mode = ActionMode::BuildHelix {
                    position: self.position_helices,
                    length: self.length_helices,
                };
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
                self.fog.visible = b;
                self.requests.lock().unwrap().fog = Some(self.fog.request());
            }
            Message::FogLength(length) => {
                self.fog.length = length;
                self.requests.lock().unwrap().fog = Some(self.fog.request());
            }
            Message::FogRadius(radius) => {
                self.fog.radius = radius;
                self.requests.lock().unwrap().fog = Some(self.fog.request());
            }
            Message::NewDesign => {
                self.show_torsion = false;
                self.physical_simulation.running = false;
                self.fog = Default::default();
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
                self.fog.from_camera = b;
                self.requests.lock().unwrap().fog = Some(self.fog.request());
            }
            Message::DescreteValue {
                factory_id,
                value_id,
                value,
            } => match factory_id.0 {
                0 => {
                    let request = &mut self.requests.lock().unwrap().scroll_sensitivity;
                    self.scroll_sensitivity_factory
                        .update_request(value_id, value, request);
                }
                1 => {
                    let request = &mut self.requests.lock().unwrap().helix_roll;
                    self.helix_roll_factory
                        .update_request(value_id, value, request);
                }
                2 => {
                    let request = &mut self.requests.lock().unwrap().hyperboloid_update;
                    self.hyperboloid_factory
                        .update_request(value_id, value, request);
                }
                3 => {
                    let request = &mut self.requests.lock().unwrap().rigid_body_parameters;
                    self.rigid_body_factory
                        .update_request(value_id, value, request)
                }
                _ => unreachable!(),
            },
            Message::HelixRoll(roll) => {
                self.helix_roll_factory.update_roll(roll);
            }
            Message::NewHyperboloid => {
                let request = &mut self.requests.lock().unwrap().new_hyperboloid;
                self.hyperboloid_factory = RequestFactory::new(2, Hyperboloid_ {});
                self.hyperboloid_factory.make_request(request);
                self.building_hyperboloid = true;
            }
            Message::FinalizeHyperboloid => {
                self.requests.lock().unwrap().finalize_hyperboloid = true;
                self.building_hyperboloid = false;
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
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let width = self.logical_size.cast::<u16>().width;

        let mut selection_buttons = vec![
            Button::new(
                &mut self.selection_mode_state.grid,
                if self.selection_mode == SelectionMode::Grid {
                    Image::new(format!(
                        "{}/icons/icons/Grid-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Grid-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::SelectionModeChanged(SelectionMode::Grid))
            .style(ButtonStyle(self.selection_mode == SelectionMode::Grid))
            .width(Length::Units(BUTTON_SIZE)),
            Button::new(
                &mut self.selection_mode_state.helix,
                if self.selection_mode == SelectionMode::Helix {
                    Image::new(format!(
                        "{}/icons/icons/Helix-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Helix-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::SelectionModeChanged(SelectionMode::Helix))
            .style(ButtonStyle(self.selection_mode == SelectionMode::Helix))
            .width(Length::Units(BUTTON_SIZE)),
            Button::new(
                &mut self.selection_mode_state.strand,
                if self.selection_mode == SelectionMode::Strand {
                    Image::new(format!(
                        "{}/icons/icons/Strand-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Strand-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::SelectionModeChanged(SelectionMode::Strand))
            .style(ButtonStyle(self.selection_mode == SelectionMode::Strand))
            .width(Length::Units(BUTTON_SIZE)),
            Button::new(
                &mut self.selection_mode_state.nucleotide,
                if self.selection_mode == SelectionMode::Nucleotide {
                    Image::new(format!(
                        "{}/icons/icons/Nucleotide-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Nucleotide-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::SelectionModeChanged(SelectionMode::Nucleotide))
            .style(ButtonStyle(
                self.selection_mode == SelectionMode::Nucleotide,
            ))
            .width(Length::Units(BUTTON_SIZE)),
        ];

        let mut global_scroll = Scrollable::new(&mut self.global_scroll)
            .spacing(5)
            .push(Text::new("SelectionMode"));
        while selection_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(selection_buttons.pop().unwrap()).spacing(5);
            let mut space = BUTTON_SIZE + 5;
            while space + BUTTON_SIZE < width && selection_buttons.len() > 0 {
                row = row.push(selection_buttons.pop().unwrap()).spacing(5);
                space += BUTTON_SIZE + 5;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }

        let mut action_buttons = vec![
            Button::new(
                &mut self.action_mode_state.select,
                if self.action_mode == ActionMode::Normal {
                    Image::new(format!(
                        "{}/icons/icons/Select-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Select-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::ActionModeChanged(ActionMode::Normal))
            .style(ButtonStyle(self.action_mode == ActionMode::Normal))
            .width(Length::Units(40)),
            Button::new(
                &mut self.action_mode_state.translate,
                if self.action_mode == ActionMode::Translate {
                    Image::new(format!(
                        "{}/icons/icons/Move-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Move-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::ActionModeChanged(ActionMode::Translate))
            .style(ButtonStyle(self.action_mode == ActionMode::Translate))
            .width(Length::Units(40)),
            Button::new(
                &mut self.action_mode_state.rotate,
                if self.action_mode == ActionMode::Rotate {
                    Image::new(format!(
                        "{}/icons/icons/Rotate-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Rotate-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::ActionModeChanged(ActionMode::Rotate))
            .style(ButtonStyle(self.action_mode == ActionMode::Rotate))
            .width(Length::Units(40)),
            Button::new(
                &mut self.action_mode_state.build,
                if self.action_mode.is_build() {
                    Image::new(format!(
                        "{}/icons/icons/Build-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Build-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                },
            )
            .on_press(Message::ActionModeChanged(ActionMode::Build(false)))
            .style(ButtonStyle(self.action_mode.is_build()))
            .width(Length::Units(40)),
            Button::new(
                &mut self.action_mode_state.cut,
                if self.action_mode == ActionMode::Cut {
                    Image::new(format!(
                        "{}/icons/icons/Cut-on.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                    .width(Length::Units(BUTTON_SIZE))
                } else {
                    Image::new(format!(
                        "{}/icons/icons/Cut-off.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                    .width(Length::Units(BUTTON_SIZE))
                },
            )
            .on_press(Message::ActionModeChanged(ActionMode::Cut))
            .width(Length::Units(40))
            .style(ButtonStyle(self.action_mode == ActionMode::Cut)),
            Button::new(
                &mut self.action_mode_state.add_grid,
                Image::new(format!(
                    "{}/icons/icons/NewGrid-on.png",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .width(Length::Units(BUTTON_SIZE)),
            )
            .on_press(Message::NewGrid)
            .width(Length::Units(40)),
            Button::new(&mut self.action_mode_state.add_hyperboloid, Text::new("H"))
                .on_press(Message::NewHyperboloid)
                .width(Length::Units(BUTTON_SIZE)),
        ];

        let mut inputs = self.builder_input.iter_mut();

        let position_input = TextInput::new(
            inputs.next().unwrap(),
            "Position",
            &self.position_str,
            Message::PositionHelicesChanged,
        )
        .style(BadValue(
            self.position_str == self.position_helices.to_string(),
        ));

        let length_input = TextInput::new(
            inputs.next().unwrap(),
            "Length",
            &self.length_str,
            Message::LengthHelicesChanged,
        )
        .style(BadValue(self.length_str == self.length_helices.to_string()));

        global_scroll = global_scroll.spacing(5).push(Text::new("Action Mode"));
        while action_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(action_buttons.remove(0)).spacing(5);
            let mut space = BUTTON_SIZE + 5;
            while space + BUTTON_SIZE < width && action_buttons.len() > 0 {
                row = row.push(action_buttons.remove(0)).spacing(5);
                space += BUTTON_SIZE + 5;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }

        if let ActionMode::Build(b) = self.action_mode {
            global_scroll = global_scroll.spacing(5).push(
                Checkbox::new(b, "Stick", |b| {
                    Message::ActionModeChanged(ActionMode::Build(b))
                })
                .size(12)
                .text_size(12),
            )
        } else if let ActionMode::BuildHelix { .. } = self.action_mode {
            let row = Row::new()
                .push(
                    Column::new()
                        .push(Text::new("Position strand").size(14).color(Color::BLACK))
                        .push(position_input)
                        .width(Length::Units(width / 2)),
                )
                .push(
                    Column::new()
                        .push(Text::new("Length strands").size(14).color(Color::BLACK))
                        .push(length_input),
                );
            global_scroll = global_scroll.push(row);
        }
        if self.building_hyperboloid {
            for view in self.hyperboloid_factory.view().into_iter() {
                global_scroll = global_scroll.push(view);
            }
            global_scroll = global_scroll.push(
                Button::new(&mut self.finalize_hyperboloid, Text::new("Finish"))
                    .on_press(Message::FinalizeHyperboloid),
            );
        }

        if self.selection_mode == SelectionMode::Helix {
            for view in self.helix_roll_factory.view().into_iter() {
                global_scroll = global_scroll.push(view);
            }
        }

        let mut target_buttons: Vec<_> = self
            .camera_target_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, Text::new(target_text(i)).size(10))
                    .on_press(target_message(i))
                    .width(Length::Units(30))
            })
            .collect();
        global_scroll = global_scroll.spacing(5).push(Text::new("Camera Target"));
        while target_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(target_buttons.remove(0)).spacing(5);
            let mut space = 30 + 5;
            while space + 30 < width && target_buttons.len() > 0 {
                row = row.push(target_buttons.remove(0)).spacing(5);
                space += 30 + 5;
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
                Button::new(s, Text::new(rotation_text(i))).on_press(rotation_message(i, xz, yz))
            })
            .collect();

        global_scroll = global_scroll.spacing(5).push(Text::new("Rotate Camera"));
        while rotate_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(rotate_buttons.remove(0)).spacing(5);
            let mut space = 30 + 5;
            while space + 30 < width && rotate_buttons.len() > 0 {
                row = row.push(rotate_buttons.remove(0)).spacing(5);
                space += 30 + 5;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }
        global_scroll = global_scroll
            .push(self.physical_simulation.view())
            .max_height(self.logical_size.height as u32);

        let mut widget = global_scroll
            .push(Checkbox::new(
                self.show_torsion,
                "Show Torsion",
                Message::ShowTorsion,
            ))
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
        widget = widget.push(self.fog.view());
        for view in self.scroll_sensitivity_factory.view().into_iter() {
            widget = widget.push(view);
        }
        widget = widget
            .push(self.rigid_grid_button.view())
            .push(self.rigid_helices_button.view());

        for view in self.rigid_body_factory.view().into_iter() {
            widget = widget.push(view);
        }

        Container::new(widget)
            .style(TopBarStyle)
            .height(Length::Fill)
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
    0xA4 as f32 / 255.0,
    0xD4 as f32 / 255.0,
    0xFF as f32 / 255.0,
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

    fn update(&mut self, message: ColorMessage) -> Command<ColorMessage> {
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

    fn view(&mut self) -> Element<ColorMessage, Renderer> {
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

fn rotation_text(i: usize) -> String {
    match i {
        0 => "←".to_string(),
        1 => "→".to_string(),
        2 => "↑".to_string(),
        _ => "↓".to_string(),
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

struct FogParameters {
    visible: bool,
    from_camera: bool,
    radius: f32,
    radius_slider: slider::State,
    length: f32,
    length_slider: slider::State,
}

impl FogParameters {
    fn view(&mut self) -> Column<Message> {
        let mut column = Column::new()
            .push(Text::new("Fog"))
            .push(Checkbox::new(
                self.visible,
                "Visible",
                Message::FogVisibility,
            ))
            .push(Checkbox::new(
                self.from_camera,
                "From Camera",
                Message::FogCamera,
            ));

        if self.visible {
            column = column
                .push(Text::new("Radius"))
                .push(Slider::new(
                    &mut self.radius_slider,
                    0f32..=100f32,
                    self.radius,
                    Message::FogRadius,
                ))
                .push(Text::new("Length"))
                .push(Slider::new(
                    &mut self.length_slider,
                    0f32..=100f32,
                    self.length,
                    Message::FogLength,
                ));
        }
        column
    }

    fn request(&self) -> Fog {
        Fog {
            radius: self.radius,
            active: self.visible,
            length: self.length,
            from_camera: self.from_camera,
            alt_fog_center: None,
        }
    }
}

impl Default for FogParameters {
    fn default() -> Self {
        Self {
            visible: false,
            length: 10.,
            radius: 10.,
            length_slider: Default::default(),
            radius_slider: Default::default(),
            from_camera: false,
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
    fn view(&mut self) -> Row<Message> {
        let left_column = Column::new()
            .push(Checkbox::new(self.roll, "Roll", Message::SimRoll))
            .push(Checkbox::new(self.springs, "Spring", Message::SimSprings));
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
            0 => 30f32,
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
}

struct RigidBodyFactory {}

impl Requestable for RigidBodyFactory {
    type Request = RigidBodyParametersRequest;
    fn request_from_values(&self, values: &[f32]) -> RigidBodyParametersRequest {
        RigidBodyParametersRequest {
            k_springs: values[0],
            k_friction: values[1],
            mass_factor: values[2],
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
