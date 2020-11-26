use std::sync::{Arc, Mutex};

use iced::{container, Background, Container, Image};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::{LogicalPosition, LogicalSize};
use iced_winit::{
    button, scrollable, slider, Button, Checkbox, Color, Column, Command, Element,
    Length, Program, Scrollable, Slider, Text, Row,
};
use native_dialog::Dialog;

use color_space::{Hsv, Rgb};

use crate::mediator::{ActionMode, SelectionMode};

use super::{OverlayType, Requests};
mod color_picker;
use color_picker::ColorPicker;
mod sequence_input;
use sequence_input::SequenceInput;

const BUTTON_SIZE: u16 = 40;

pub struct LeftPanel {
    scroll_sensitivity_slider: slider::State,
    selection_mode: SelectionMode,
    action_mode: ActionMode,
    global_scroll: scrollable::State,
    logical_size: LogicalSize<f64>,
    logical_position: LogicalPosition<f64>,
    scroll_sensitivity: f32,
    open_color: button::State,
    sequence_input: SequenceInput,
    requests: Arc<Mutex<Requests>>,
    action_mode_state: ActionModeState,
    selection_mode_state: SelectionModeState,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectionModeChanged(SelectionMode),
    Resized(LogicalSize<f64>, LogicalPosition<f64>),
    OpenColor,
    ActionModeChanged(ActionMode),
    SequenceChanged(String),
    SequenceFileRequested,
    ScrollSensitivityChanged(f32),
}

impl LeftPanel {
    pub fn new(
        requests: Arc<Mutex<Requests>>,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
    ) -> Self {
        Self {
            selection_mode: Default::default(),
            scroll_sensitivity_slider: Default::default(),
            action_mode: Default::default(),
            global_scroll: Default::default(),
            logical_size,
            logical_position,
            scroll_sensitivity: 0f32,
            open_color: Default::default(),
            sequence_input: SequenceInput::new(),
            requests,
            action_mode_state: Default::default(),
            selection_mode_state: Default::default(),
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
                self.selection_mode = selection_mode;
                self.requests.lock().unwrap().selection_mode = Some(selection_mode);
            }
            Message::ActionModeChanged(action_mode) => {
                self.action_mode = action_mode;
                self.requests.lock().unwrap().action_mode = Some(action_mode)
            }
            Message::SequenceChanged(s) => {
                self.requests.lock().unwrap().sequence_change = Some(s.clone());
                self.sequence_input.update_sequence(s);
            }
            Message::ScrollSensitivityChanged(x) => {
                self.requests.lock().unwrap().scroll_sensitivity = Some(x);
                self.scroll_sensitivity = x;
            }
            Message::SequenceFileRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
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
            Message::Resized(size, position) => self.resize(size, position),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let width = self.logical_size.cast::<u16>().width;

        let slider = Slider::new(
            &mut self.scroll_sensitivity_slider,
            -20f32..=20f32,
            self.scroll_sensitivity,
            Message::ScrollSensitivityChanged,
        );

        let mut global_scroll = Scrollable::new(&mut self.global_scroll)
            .width(Length::Units(width))
            .push(Text::new("Scroll sensitivity"))
            .push(slider);

        let mut selection_buttons = vec![
            Button::new(&mut self.selection_mode_state.grid, if self.selection_mode == SelectionMode::Grid {
                Image::new("icons/icons/Grid-on.png")
            } else {
                Image::new("icons/icons/Grid-off.png")
            }).on_press(Message::SelectionModeChanged(SelectionMode::Grid)).width(Length::Units(BUTTON_SIZE)),
            Button::new(&mut self.selection_mode_state.helix, if self.selection_mode == SelectionMode::Helix {
                Image::new("icons/icons/Helix-on.png")
            } else {
                Image::new("icons/icons/Helix-off.png")
            }).on_press(Message::SelectionModeChanged(SelectionMode::Helix)).width(Length::Units(BUTTON_SIZE)),
            Button::new(&mut self.selection_mode_state.strand, if self.selection_mode == SelectionMode::Strand {
                Image::new("icons/icons/Strand-on.png")
            } else {
                Image::new("icons/icons/Strand-off.png")
            }).on_press(Message::SelectionModeChanged(SelectionMode::Strand)).width(Length::Units(BUTTON_SIZE)),
            Button::new(&mut self.selection_mode_state.nucleotide, if self.selection_mode == SelectionMode::Nucleotide {
                Image::new("icons/icons/Nucleotide-on.png")
            } else {
                Image::new("icons/icons/Nucleotide-off.png")
            }).on_press(Message::SelectionModeChanged(SelectionMode::Nucleotide)).width(Length::Units(BUTTON_SIZE))
        ];

        global_scroll = global_scroll.spacing(5).push(Text::new("SelectionMode"));
        while selection_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(selection_buttons.pop().unwrap()).spacing(5);
            let mut space = BUTTON_SIZE + 5;
            while space + BUTTON_SIZE < width  && selection_buttons.len() > 0{
                row = row.push(selection_buttons.pop().unwrap()).spacing(5);
                space += BUTTON_SIZE;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }

        let mut action_buttons = vec![
            Button::new(&mut self.action_mode_state.select, if self.action_mode == ActionMode::Normal {
                Image::new("icons/icons/Select-on.png")
            } else {
                Image::new("icons/icons/Select-off.png")
            }
            ).on_press(Message::ActionModeChanged(ActionMode::Normal)).width(Length::Units(40)),
            Button::new(&mut self.action_mode_state.translate, if self.action_mode == ActionMode::Translate {
                Image::new("icons/icons/Move-on.png")
            } else {
                Image::new("icons/icons/Move-off.png")
            }
            ).on_press(Message::ActionModeChanged(ActionMode::Translate)).width(Length::Units(40)),
            Button::new(&mut self.action_mode_state.rotate, if self.action_mode == ActionMode::Rotate {
                Image::new("icons/icons/Rotate-on.png")
            } else {
                Image::new("icons/icons/Rotate-off.png")
            }
            ).on_press(Message::ActionModeChanged(ActionMode::Rotate)).width(Length::Units(40)),
            Button::new(&mut self.action_mode_state.build, if self.action_mode.is_build() {
                Image::new("icons/icons/Build-on.png")
            } else {
                Image::new("icons/icons/Build-off.png")
            }
            ).on_press(Message::ActionModeChanged(ActionMode::Build(false))).width(Length::Units(40)),
            Button::new(&mut self.action_mode_state.cut, if self.action_mode == ActionMode::Cut {
                Image::new("icons/icons/Cut-on.png")
            } else {
                Image::new("icons/icons/Cut-off.png")
            }
            ).on_press(Message::ActionModeChanged(ActionMode::Cut)).width(Length::Units(40))];

        global_scroll = global_scroll.spacing(5).push(Text::new("Action Mode"));
        while action_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(action_buttons.remove(0)).spacing(5);
            let mut space = BUTTON_SIZE + 5;
            while space + BUTTON_SIZE < width  && action_buttons.len() > 0{
                row = row.push(action_buttons.remove(0)).spacing(5);
                space += BUTTON_SIZE;
            }
            global_scroll = global_scroll.spacing(5).push(row)
        }

        let mut widget = Column::new()
            .push(global_scroll)
            .width(Length::Units(width))
            .height(Length::Fill);

        if self.selection_mode == SelectionMode::Strand {
            widget = widget
                .spacing(5)
                .push(
                    Button::new(&mut self.open_color, Text::new("Change color"))
                        .on_press(Message::OpenColor),
                )
                .spacing(5)
                .push(self.sequence_input.view());
        }

        if let ActionMode::Build(b) = self.action_mode {
            widget = widget.spacing(5).push(Checkbox::new(b, "Stick", |b| {
                Message::ActionModeChanged(ActionMode::Build(b))
            }))
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
            border_width: 3,
            border_radius: 3,
            border_color: Color::BLACK,
            ..container::Style::default()
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
}
