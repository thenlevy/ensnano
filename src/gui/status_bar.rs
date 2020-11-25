use super::Requests;
use crate::mediator::{Operation, ParameterField};
use iced::{container, Background, Container, Length};
use iced_native::{pick_list, text_input, Color, PickList, TextInput};
use iced_winit::{Column, Command, Element, Program, Row, Space, Text};
use std::sync::{Arc, Mutex};

const STATUS_FONT_SIZE: u16 = 14;

#[derive(Debug)]
enum StatusParameter {
    Value(text_input::State),
    Choice(pick_list::State<String>),
}

impl StatusParameter {
    fn get_value(&mut self) -> &mut text_input::State {
        match self {
            StatusParameter::Value(ref mut state) => state,
            _ => panic!("wrong status parameter variant"),
        }
    }

    fn get_choice(&mut self) -> &mut pick_list::State<String> {
        match self {
            StatusParameter::Choice(ref mut state) => state,
            _ => panic!("wrong status parameter variant"),
        }
    }

    fn value() -> Self {
        Self::Value(Default::default())
    }

    fn choice() -> Self {
        Self::Choice(Default::default())
    }
}

pub struct StatusBar {
    parameters: Vec<StatusParameter>,
    values: Vec<String>,
    operation: Option<Arc<dyn Operation>>,
    info: Option<String>,
    requests: Arc<Mutex<Requests>>,
}

impl StatusBar {
    pub fn new(requests: Arc<Mutex<Requests>>) -> Self {
        Self {
            parameters: Vec::new(),
            values: Vec::new(),
            operation: None,
            info: None,
            requests,
        }
    }

    pub fn update_op(&mut self, operation: Arc<dyn Operation>) {
        let parameters = operation.parameters();
        let mut new_param = Vec::new();
        for p in parameters.iter() {
            match p.field {
                ParameterField::Choice(_) => new_param.push(StatusParameter::choice()),
                ParameterField::Value => new_param.push(StatusParameter::value()),
            }
        }
        self.values = operation.values().clone();
        self.parameters = new_param;
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Operation(Arc<dyn Operation>),
    Info(String),
    ValueChanged(usize, String),
    ClearOp,
}

impl Program for StatusBar {
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Operation(ref op) => {
                self.operation = Some(op.clone());
                self.update_op(op.clone());
            }
            Message::ValueChanged(n, s) => {
                self.values[n] = s.clone();
                let new_op = self
                    .operation
                    .as_ref()
                    .and_then(|op| op.with_new_value(n, s));
                if let Some(ref op) = new_op {
                    self.operation = Some(op.clone());
                }
                self.requests.lock().unwrap().operation_update = new_op;
            }
            Message::Info(s) => {
                self.operation = None;
                self.info = Some(s)
            }
            Message::ClearOp => self.operation = None,
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        if let Some(ref op) = self.operation {
            row = row.push(Text::new(op.description()).size(STATUS_FONT_SIZE));
            let values = &self.values;
            for (i, p) in self.parameters.iter_mut().enumerate() {
                let param = &op.parameters()[i];
                match param.field {
                    ParameterField::Value => {
                        row = row
                            .spacing(20)
                            .push(Text::new(param.name.clone()).size(STATUS_FONT_SIZE))
                            .push(
                                TextInput::new(
                                    p.get_value(),
                                    "",
                                    &format!("{0:.4}", values[i]),
                                    move |s| Message::ValueChanged(i, s),
                                )
                                .size(STATUS_FONT_SIZE)
                                .width(Length::Units(40)),
                            )
                    }
                    ParameterField::Choice(ref v) => {
                        row = row.spacing(20).push(
                            PickList::new(
                                p.get_choice(),
                                v.clone(),
                                Some(values[i].clone()),
                                move |s| Message::ValueChanged(i, s),
                            )
                            .text_size(STATUS_FONT_SIZE - 4),
                        )
                    }
                }
            }
        } else if let Some(ref info) = self.info {
            row = row.push(Text::new(info).size(STATUS_FONT_SIZE))
        }
        let column = Column::new()
            .push(Space::new(Length::Fill, Length::Units(3)))
            .push(row);
        Container::new(column)
            .style(StatusBarStyle)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct StatusBarStyle;
impl container::StyleSheet for StatusBarStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(BACKGROUND)),
            text_color: Some(Color::WHITE),
            ..container::Style::default()
        }
    }
}

pub const BACKGROUND: Color = Color::from_rgb(
    0x36 as f32 / 255.0,
    0x39 as f32 / 255.0,
    0x3F as f32 / 255.0,
);
