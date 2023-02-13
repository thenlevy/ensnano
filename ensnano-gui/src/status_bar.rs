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
use super::{AppState, Requests, UiSize};
use ensnano_interactor::operation::{Operation, ParameterField};
pub use ensnano_interactor::StrandBuildingStatus;
use iced::{container, slider, Background, Container, Length};
use iced_native::{
    widget::{pick_list, text_input, PickList, TextInput},
    Color,
};
use iced_winit::{
    widget::{Column, Row, Space, Text},
    winit, Command, Element, Program,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use winit::dpi::LogicalSize;

const GOLD_ORANGE: iced::Color = iced::Color::from_rgb(0.84, 0.57, 0.20);

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

    fn focus(&mut self) -> bool {
        if let Self::Value(state) = self {
            state.focus();
            state.select_all();
            true
        } else {
            false
        }
    }

    fn unfocus(&mut self) {
        if let Self::Value(state) = self {
            state.unfocus()
        }
    }
}

pub struct StatusBar<R: Requests, S: AppState> {
    info_values: Vec<String>,
    operation: Option<OperationInput>,
    requests: Arc<Mutex<R>>,
    progress: Option<(String, f32)>,
    #[allow(dead_code)]
    slider_state: slider::State,
    app_state: S,
    ui_size: UiSize,
    message: Option<String>,
    logical_size: LogicalSize<f64>,
}

impl<R: Requests, S: AppState> StatusBar<R, S> {
    pub fn new(
        requests: Arc<Mutex<R>>,
        state: &S,
        logical_size: LogicalSize<f64>,
        ui_size: UiSize,
    ) -> Self {
        Self {
            info_values: Vec::new(),
            operation: None,
            requests,
            progress: None,
            slider_state: Default::default(),
            app_state: state.clone(),
            ui_size,
            message: None,
            logical_size,
        }
    }

    pub fn set_ui_size(&mut self, ui_size: UiSize) {
        self.ui_size = ui_size;
    }

    fn update_operation(&mut self) {
        if let Some(new_operation) = self.app_state.get_curent_operation_state() {
            if let Some(operation) = self.operation.as_mut() {
                operation.update(new_operation);
            } else {
                self.operation = Some(OperationInput::new(new_operation));
            }
        } else {
            self.operation = None;
        }
    }

    fn view_progress(&mut self) -> Row<Message<S>, iced_wgpu::Renderer> {
        let row = Row::new();
        let progress = self.progress.as_ref().unwrap();
        row.push(
            Text::new(format!("{}, {:.1}%", progress.0, progress.1 * 100.))
                .size(self.ui_size.main_text()),
        )
    }

    /* TODO
    fn view_overed_strand(&self) -> Element<Message<S>, iced_wgpu::Renderer> {
        let mut row = Row::new();
        if let Some(strand) = self.app_state.get_overed_strand() {
            row = row.push(Text::new(strand.info()).size(self.ui_size.status_font())
        }
        row.into()
    }*/

    pub fn has_keyboard_priority(&self) -> bool {
        self.operation
            .as_ref()
            .map(|op| op.has_keyboard_priority())
            .unwrap_or(false)
    }

    pub fn process_tab(&mut self) {
        let op = self.operation.as_mut().and_then(|op| op.process_tab());
        if !self.has_keyboard_priority() {
            log::info!("Updating operation");
            if let Some(op) = op {
                self.requests.lock().unwrap().update_current_operation(op)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message<S: AppState> {
    ValueStrChanged(usize, String),
    ValueSet(usize, String),
    Progress(Option<(String, f32)>),
    #[allow(dead_code)]
    SetShift(f32),
    NewApplicationState(S),
    UiSizeChanged(UiSize),
    TabPressed,
    Message(Option<String>),
    Resize(LogicalSize<f64>),
}

impl<R: Requests, S: AppState> Program for StatusBar<R, S> {
    type Message = Message<S>;
    type Renderer = iced_wgpu::Renderer;

    fn update(&mut self, message: Message<S>) -> Command<Message<S>> {
        match message {
            Message::ValueStrChanged(n, s) => {
                if let Some(operation) = self.operation.as_mut() {
                    operation.update_input_str(n, s)
                }
            }
            Message::ValueSet(n, s) => {
                if let Some(operation) = self.operation.as_mut() {
                    if let Some(new_operation) = operation.update_value(n, s) {
                        self.requests
                            .lock()
                            .unwrap()
                            .update_current_operation(new_operation);
                    }
                }
            }
            Message::Progress(progress) => self.progress = progress,
            Message::SetShift(f) => {
                self.info_values[2] = f.to_string();
                self.requests.lock().unwrap().update_hyperboloid_shift(f);
            }
            Message::NewApplicationState(state) => self.app_state = state,
            Message::UiSizeChanged(ui_size) => self.set_ui_size(ui_size),
            Message::TabPressed => self.process_tab(),
            Message::Message(message) => self.message = message,
            Message::Resize(size) => self.logical_size = size,
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message<S>, iced_wgpu::Renderer> {
        self.update_operation();
        let clipboard_text = format!(
            "Clipboard: {}",
            self.app_state.get_clipboard_content().to_string()
        );
        let pasting_text = match self.app_state.get_pasting_status() {
            ensnano_interactor::PastingStatus::Copy => "Pasting",
            ensnano_interactor::PastingStatus::None => "",
            ensnano_interactor::PastingStatus::Duplication => "Duplicating",
        }
        .to_string();

        let size = self.logical_size.clone();
        let mut content = if self.progress.is_some() {
            self.operation = None;
            self.message = None;
            self.view_progress()
        } else if let Some(building_info) = self.app_state.get_strand_building_state() {
            self.operation = None;
            self.message = None;
            Row::new().push(Text::new(building_info.to_info()).size(self.ui_size.main_text()))
        } else if let Some(ref message) = self.message {
            self.operation = None;
            Row::new().push(Text::new(message).size(self.ui_size.main_text()))
        } else if let Some(operation) = self.operation.as_mut() {
            log::trace!("operation is some");
            operation.view(self.ui_size)
        } else {
            log::trace!("operation is none");
            Row::new().into() //TODO
        };

        content = Row::new()
            .push(content)
            .push(Space::with_width(Length::Fill)) // To right align the clipboard text
            .push(Text::new(clipboard_text))
            .push(Space::with_width(Length::Units(5)))
            .align_items(iced_winit::Alignment::End);

        let pasting_status_row = Row::new()
            .push(Space::with_width(Length::Fill))
            .push(Text::new(pasting_text))
            .push(Space::with_width(Length::Units(5)));

        let column = Column::new()
            .push(Space::new(Length::Fill, Length::Units(3)))
            .push(content)
            .push(pasting_status_row);
        Container::new(column)
            .style(StatusBarStyle)
            .width(Length::Units(size.width as u16))
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

pub struct CurentOpState {
    pub current_operation: Arc<dyn Operation>,
    pub operation_id: usize,
}

struct OperationInput {
    /// The values obatained with Operation::values
    values: Vec<String>,
    /// The String in the text inputs,
    values_str: Vec<String>,
    parameters: Vec<StatusParameter>,
    op_id: usize,
    operation: Arc<dyn Operation>,
    inputed_values: HashMap<usize, String>,
}

impl OperationInput {
    pub fn new(operation_state: CurentOpState) -> Self {
        let operation = operation_state.current_operation;
        let parameters = operation.parameters();
        let mut status_parameters = Vec::new();
        for p in parameters.iter() {
            match p.field {
                ParameterField::Choice(_) => status_parameters.push(StatusParameter::choice()),
                ParameterField::Value => status_parameters.push(StatusParameter::value()),
            }
        }
        let values = operation.values().clone();
        let values_str = values.clone();
        let op_id = operation_state.operation_id;
        Self {
            parameters: status_parameters,
            op_id,
            values,
            values_str,
            operation,
            inputed_values: HashMap::new(),
        }
    }

    #[must_use = "Do not forget to apply the oppertaion"]
    pub fn process_tab(&mut self) -> Option<Arc<dyn Operation>> {
        let mut was_focus = false;
        let mut old_foccussed_idx: Option<usize> = None;
        for (i, p) in self.parameters.iter_mut().enumerate() {
            if was_focus {
                was_focus ^= p.focus()
            } else {
                if p.has_keyboard_priority() {
                    p.unfocus();
                    old_foccussed_idx = Some(i);
                    was_focus = true;
                }
            }
        }

        old_foccussed_idx.and_then(|i| {
            self.inputed_values.insert(i, self.values_str[i].clone());
            self.update_value(i, self.values_str[i].clone())
        })
    }

    pub fn update(&mut self, operation_state: CurentOpState) {
        let op_is_new = self.op_id != operation_state.operation_id;
        let operation = operation_state.current_operation;
        self.values = operation.values().clone();
        if op_is_new {
            self.values_str = self.values.clone();
            self.op_id = operation_state.operation_id;

            let mut status_parameters = Vec::new();
            for p in operation.parameters().iter() {
                match p.field {
                    ParameterField::Choice(_) => status_parameters.push(StatusParameter::choice()),
                    ParameterField::Value => status_parameters.push(StatusParameter::value()),
                }
            }
            self.parameters = status_parameters;
        } else {
            for (v_id, v) in self.values.iter().enumerate() {
                let foccused_parameter = self
                    .parameters
                    .get(v_id)
                    .map(|p| p.has_keyboard_priority())
                    .unwrap_or(false);
                if !foccused_parameter {
                    self.values_str[v_id] =
                        self.inputed_values.get(&v_id).cloned().unwrap_or(v.clone())
                }
            }
        }
        self.operation = operation;
    }

    fn view<S: AppState>(&mut self, ui_size: UiSize) -> Row<Message<S>, iced_wgpu::Renderer> {
        let mut row = Row::new();
        let op = self.operation.as_ref();
        row = row.push(Text::new(op.description()).size(ui_size.main_text()));
        let values = &self.values;
        let str_values = &self.values_str;
        let active_input = (0..values.len())
            .map(|i| self.active_input(i))
            .collect::<Vec<_>>();
        let mut need_validation = false;
        for (i, p) in self.parameters.iter_mut().enumerate() {
            if let Some(param) = op.parameters().get(i) {
                match param.field {
                    ParameterField::Value => {
                        let mut input = TextInput::new(
                            p.get_value(),
                            "",
                            &format!("{0:.4}", str_values[i]),
                            move |s| Message::ValueStrChanged(i, s),
                        )
                        .size(ui_size.main_text())
                        .width(Length::Units(40))
                        .on_submit(Message::ValueSet(i, str_values[i].clone()));
                        if active_input.get(i) == Some(&true) {
                            use input_color::InputValueState;
                            let state = if values.get(i) == str_values.get(i) {
                                InputValueState::Normal
                            } else if op.with_new_value(i, str_values[i].clone()).is_some() {
                                need_validation = true;
                                InputValueState::BeingTyped
                            } else {
                                InputValueState::Invalid
                            };
                            input = input.style(state);
                        }
                        row = row
                            .spacing(20)
                            .push(Text::new(param.name.clone()).size(ui_size.main_text()))
                            .push(input)
                    }
                    ParameterField::Choice(ref v) => {
                        row = row.spacing(20).push(
                            PickList::new(
                                p.get_choice(),
                                v.clone(),
                                Some(values[i].clone()),
                                move |s| Message::ValueSet(i, s),
                            )
                            .text_size(ui_size.main_text() - 4),
                        )
                    }
                }
            }
        }
        if need_validation {
            row = row.push(Text::new("(Press enter to validate change)").size(ui_size.main_text()));
        }
        row
    }

    fn active_input(&self, i: usize) -> bool {
        self.parameters
            .get(i)
            .map(|p| p.has_keyboard_priority())
            .unwrap_or(false)
    }

    fn update_input_str(&mut self, value_id: usize, new_str: String) {
        if let Some(s) = self.values_str.get_mut(value_id) {
            *s = new_str.clone()
        } else {
            log::error!(
                "Changing str of value_id {} but self has {} values",
                value_id,
                self.values_str.len()
            );
        }
    }

    fn update_value(&mut self, value_id: usize, values_str: String) -> Option<Arc<dyn Operation>> {
        if let Some(op) = self.operation.as_ref().with_new_value(value_id, values_str) {
            self.operation = op.clone();
            Some(op)
        } else {
            None
        }
    }

    fn has_keyboard_priority(&self) -> bool {
        self.parameters.iter().any(|p| p.has_keyboard_priority())
    }
}

mod input_color {
    use iced::{Background, Color};
    use iced_native::widget::text_input::*;
    pub enum InputValueState {
        Normal,
        BeingTyped,
        Invalid,
    }

    impl iced_native::widget::text_input::StyleSheet for InputValueState {
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
            match self {
                Self::Normal => Color::from_rgb(0.3, 0.3, 0.3),
                Self::Invalid => Color::from_rgb(1., 0.3, 0.3),
                Self::BeingTyped => super::GOLD_ORANGE,
            }
        }

        fn selection_color(&self) -> Color {
            Color::from_rgb(0.8, 0.8, 1.0)
        }
    }
}

trait ToInfo {
    fn to_info(&self) -> String;
}

impl ToInfo for StrandBuildingStatus {
    fn to_info(&self) -> String {
        format!(
            "Current domain length: {} nt ({:.2} nm). 5': {}, 3': {}",
            self.nt_length, self.nm_length, self.prime5.position, self.prime3.position
        )
    }
}

pub enum ClipboardContent {
    Empty,
    Xovers(usize),
    Strands(usize),
    Grids(usize),
    Helices(usize),
}

impl ToString for ClipboardContent {
    fn to_string(&self) -> String {
        match self {
            Self::Empty => "Empty".into(),
            Self::Xovers(n) => format!("{n} xover(s)"),
            Self::Strands(n) => format!("{n} strand(s)"),
            Self::Grids(n) => format!("{n} grid(s)"),
            Self::Helices(n) => format!("{n} helice(s)"),
        }
    }
}
