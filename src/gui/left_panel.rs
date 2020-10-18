use std::sync::{Arc, Mutex};

use iced::{container, Background, Container};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::{LogicalPosition, LogicalSize};
use iced_winit::{
    pick_list, scrollable, Color, Column, Command, Element, Length, PickList, Program, Scrollable,
    Space, Text,
};

use color_space::{Hsv, Rgb};

use crate::scene::{ActionMode, SelectionMode};

use super::Requests;
mod color_picker;
use color_picker::ColorPicker;
mod sequence_input;
use sequence_input::SequenceInput;

pub struct LeftPanel {
    pick_selection_mode: pick_list::State<SelectionMode>,
    pick_action_mode: pick_list::State<ActionMode>,
    scroll_selection_mode: scrollable::State,
    scroll_action_mode: scrollable::State,
    selection_mode: SelectionMode,
    action_mode: ActionMode,
    global_scroll: scrollable::State,
    logical_size: LogicalSize<f64>,
    logical_position: LogicalPosition<f64>,
    color_picker: ColorPicker,
    sequence_input: SequenceInput,
    requests: Arc<Mutex<Requests>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectionModeChanged(SelectionMode),
    Resized(LogicalSize<f64>, LogicalPosition<f64>),
    StrandColorChanged(Color),
    HueChanged(f32),
    ActionModeChanged(ActionMode),
    SequenceChanged(String),
}

impl LeftPanel {
    pub fn new(
        requests: Arc<Mutex<Requests>>,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
    ) -> Self {
        Self {
            pick_selection_mode: Default::default(),
            scroll_action_mode: Default::default(),
            scroll_selection_mode: Default::default(),
            selection_mode: Default::default(),
            pick_action_mode: Default::default(),
            action_mode: Default::default(),
            global_scroll: Default::default(),
            logical_size,
            logical_position,
            color_picker: ColorPicker::new(),
            sequence_input: SequenceInput::new(),
            requests,
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
            Message::SequenceChanged(s) => {
                self.requests.lock().unwrap().sequence_change = Some(s.clone());
                self.sequence_input.update_sequence(s);
            }
            Message::Resized(size, position) => self.resize(size, position),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let width = self.logical_size.cast::<u16>().width;
        let position_top = self.logical_position.cast::<u16>().y;
        let selection_mode_list = PickList::new(
            &mut self.pick_selection_mode,
            &SelectionMode::ALL[..],
            Some(self.selection_mode),
            Message::SelectionModeChanged,
        );

        let selection_mode_scroll = Scrollable::new(&mut self.scroll_selection_mode)
            .push(Text::new("Selection mode"))
            .push(selection_mode_list);

        let action_mode_list = PickList::new(
            &mut self.pick_action_mode,
            &ActionMode::ALL[..],
            Some(self.action_mode),
            Message::ActionModeChanged,
        );

        let action_mode_scroll = Scrollable::new(&mut self.scroll_action_mode)
            .push(Text::new("Action mode"))
            .push(action_mode_list);

        let global_scroll = Scrollable::new(&mut self.global_scroll)
            .width(Length::Units(width))
            .push(selection_mode_scroll)
            .push(action_mode_scroll);

        let empty_space = Space::new(Length::Units(width), Length::Units(position_top));

        let mut widget = Column::new()
            .push(empty_space)
            .push(global_scroll)
            .width(Length::Units(width))
            .height(Length::Fill);

        if self.selection_mode == SelectionMode::Strand {
            widget = widget.spacing(5).push(self.color_picker.view()).spacing(5).push(self.sequence_input.view());
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
