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
use super::Requests;
use crate::mediator::{Operation, ParameterField, Selection};
use iced::{container, slider, Background, Container, Length};
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

    fn has_keyboard_priority(&self) -> bool {
        match self {
            Self::Choice(_) => false,
            Self::Value(state) => state.is_focused(),
        }
    }
}

pub struct StatusBar<R: Requests> {
    parameters: Vec<StatusParameter>,
    info_values: Vec<String>,
    operation_values: Vec<String>,
    operation: Option<Arc<dyn Operation>>,
    requests: Arc<Mutex<R>>,
    selection: Selection,
    progress: Option<(String, f32)>,
    #[allow(dead_code)]
    slider_state: slider::State,
}

impl<R: Requests> StatusBar<R> {
    pub fn new(requests: Arc<Mutex<R>>) -> Self {
        Self {
            parameters: Vec::new(),
            info_values: Vec::new(),
            operation_values: Vec::new(),
            operation: None,
            requests,
            selection: Selection::Nothing,
            progress: None,
            slider_state: Default::default(),
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
        self.operation_values = operation.values().clone();
        self.parameters = new_param;
    }

    fn view_op(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        let op = self.operation.as_ref().unwrap(); // the function view op is only called when op is some.
        row = row.push(Text::new(op.description()).size(STATUS_FONT_SIZE));
        let values = &self.operation_values;
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
        row.into()
    }

    fn view_selection(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        if self.selection != Selection::Nothing {
            row = row.push(Text::new(self.selection.info()).size(STATUS_FONT_SIZE));
        }
        row.into()
    }

    fn view_progress(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        let progress = self.progress.as_ref().unwrap();
        row = row.push(
            Text::new(format!("{}, {:.1}%", progress.0, progress.1 * 100.)).size(STATUS_FONT_SIZE),
        );

        row.into()
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.parameters.iter().any(|p| p.has_keyboard_priority())
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Operation(Arc<dyn Operation>),
    Selection(Selection, Vec<String>),
    ValueChanged(usize, String),
    Progress(Option<(String, f32)>),
    #[allow(dead_code)]
    SetShift(f32),
    ClearOp,
}

impl<R: Requests> Program for StatusBar<R> {
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;
    type Clipboard = iced_native::clipboard::Null;

    fn update(
        &mut self,
        message: Message,
        _cb: &mut iced_native::clipboard::Null,
    ) -> Command<Message> {
        match message {
            Message::Operation(ref op) => {
                self.operation = Some(op.clone());
                self.update_op(op.clone());
            }
            Message::ValueChanged(n, s) => {
                self.operation_values[n] = s.clone();
                let new_op = self
                    .operation
                    .as_ref()
                    .and_then(|op| op.with_new_value(n, s));
                if let Some(ref op) = new_op {
                    self.operation = Some(op.clone());
                }
                self.requests.lock().unwrap().operation_update = new_op;
            }
            Message::Progress(progress) => self.progress = progress,
            Message::Selection(s, v) => {
                self.operation = None;
                self.selection = s;
                self.info_values = v;
            }
            Message::ClearOp => self.operation = None,
            Message::SetShift(f) => {
                self.info_values[2] = f.to_string();
                self.requests.lock().unwrap().new_shift_hyperboloid = Some(f);
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let content = if self.progress.is_some() {
            self.view_progress()
        } else if self.operation.is_some() {
            self.view_op()
        } else {
            self.view_selection()
        };

        let column = Column::new()
            .push(Space::new(Length::Fill, Length::Units(3)))
            .push(content);
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
    0x12 as f32 / 255.0,
    0x12 as f32 / 255.0,
    0x30 as f32 / 255.0,
);
