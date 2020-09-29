use std::sync::{Arc, Mutex};

use iced::{container, Background, Container};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::{LogicalPosition, LogicalSize};
use iced_winit::{
    pick_list, scrollable, Color, Column, Command, Element, Length, PickList, Program, Scrollable,
    Space, Text,
};

use crate::scene::SelectionMode;

pub struct LeftPanel {
    pick_selection_mode: pick_list::State<SelectionMode>,
    scroll_selection_mode: scrollable::State,
    selection_mode: SelectionMode,
    global_scroll: scrollable::State,
    pub selection_mode_request: Arc<Mutex<Option<SelectionMode>>>,
    logical_size: LogicalSize<f64>,
    logical_position: LogicalPosition<f64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectionModeChanged(SelectionMode),
    Resized(LogicalSize<f64>, LogicalPosition<f64>),
}

impl LeftPanel {
    pub fn new(
        selection_mode_request: Arc<Mutex<Option<SelectionMode>>>,
        logical_size: LogicalSize<f64>,
        logical_position: LogicalPosition<f64>,
    ) -> Self {
        Self {
            pick_selection_mode: Default::default(),
            scroll_selection_mode: Default::default(),
            selection_mode: Default::default(),
            global_scroll: Default::default(),
            selection_mode_request,
            logical_size,
            logical_position,
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize<f64>, logical_position: LogicalPosition<f64>) {
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
                *self.selection_mode_request.lock().unwrap() = Some(selection_mode);
            }
            Message::Resized(size, position) => self.resize(size, position)
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

        let global_scroll = Scrollable::new(&mut self.global_scroll)
            .width(Length::Units(width))
            .push(selection_mode_scroll);

        let empty_space = Space::new(Length::Units(width), Length::Units(position_top));

        let widget = Column::new()
            .push(empty_space)
            .push(global_scroll)
            .width(Length::Units(width))
            .height(Length::Fill);

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
