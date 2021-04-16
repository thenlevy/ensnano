use super::Requests;
use crate::mediator::{Operation, ParameterField, Selection};
use iced::{container, slider, Background, Container, Length};
use iced_native::{pick_list, text_input, Checkbox, Color, PickList, TextInput};
use iced_winit::{Column, Command, Element, Program, Row, Space, Text};
use std::str::FromStr;
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

pub struct StatusBar {
    parameters: Vec<StatusParameter>,
    info_values: Vec<String>,
    operation_values: Vec<String>,
    operation: Option<Arc<dyn Operation>>,
    requests: Arc<Mutex<Requests>>,
    selection: Selection,
    progress: Option<(String, f32)>,
    #[allow(dead_code)]
    slider_state: slider::State,
}

impl StatusBar {
    pub fn new(requests: Arc<Mutex<Requests>>) -> Self {
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

        let column = Column::new()
            .push(Space::new(Length::Fill, Length::Units(3)))
            .push(row);
        Container::new(column)
            .style(StatusBarStyle)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_selection(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        let selection = &self.selection;
        row = row.push(Text::new(selection.info()).size(STATUS_FONT_SIZE));

        match selection {
            Selection::Grid(_, _) => {
                row = row.push(
                    Checkbox::new(
                        bool::from_str(&self.info_values[0]).unwrap(),
                        "Persistent phantoms",
                        |b| Message::SelectionValueChanged(0, bool_to_string(b)),
                    )
                    .size(STATUS_FONT_SIZE)
                    .text_size(STATUS_FONT_SIZE),
                );
                row = row.push(
                    Checkbox::new(
                        bool::from_str(&self.info_values[1]).unwrap(),
                        "Small spheres",
                        |b| Message::SetSmallSpheres(b),
                    )
                    .size(STATUS_FONT_SIZE)
                    .text_size(STATUS_FONT_SIZE),
                );
                /*
                if let Some(current_angle) =
                    self.info_values.get(2).and_then(|s| s.parse::<f32>().ok())
                {
                    let min_angle = -std::f32::consts::PI + 1f32.to_radians();
                    let max_angle = std::f32::consts::PI - 1f32.to_radians();

                    row = row.push(Text::new("angle shift")).push(
                        Slider::new(
                            &mut self.slider_state,
                            min_angle..=max_angle,
                            current_angle,
                            Message::SetShift,
                        )
                        .step(1f32.to_radians())
                        .width(Length::Units(150)),
                    )
                }
                */
            }
            Selection::Strand(_, _) => {
                let s_id = self.info_values[2].parse::<usize>().unwrap();
                row = row.push(
                    Text::new(format!("length {}", &self.info_values[0])).size(STATUS_FONT_SIZE),
                );
                row = row.push(Checkbox::new(
                    bool::from_str(&self.info_values[1]).unwrap(),
                    "Scaffold",
                    move |b| Message::ScaffoldIdSet(s_id, b),
                ));
                row = row.push(Text::new(self.info_values[3].clone()).size(STATUS_FONT_SIZE - 2));
            }
            Selection::Nucleotide(_, _) => {
                let anchor = self.info_values[0].clone();
                row = row.push(Text::new(format!("Anchor {}", anchor)));
            }
            _ => (),
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

    fn view_progress(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        let progress = self.progress.as_ref().unwrap();
        row = row.push(
            Text::new(format!("{}, {:.1}%", progress.0, progress.1 * 100.)).size(STATUS_FONT_SIZE),
        );

        let column = Column::new()
            .push(Space::new(Length::Fill, Length::Units(3)))
            .push(row);
        Container::new(column)
            .style(StatusBarStyle)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
    SelectionValueChanged(usize, String),
    SetSmallSpheres(bool),
    ScaffoldIdSet(usize, bool),
    Progress(Option<(String, f32)>),
    #[allow(dead_code)]
    SetShift(f32),
    ClearOp,
}

impl Program for StatusBar {
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
            Message::SelectionValueChanged(n, s) => {
                self.info_values[n] = s.clone();
                self.requests.lock().unwrap().toggle_persistent_helices = bool::from_str(&s).ok();
            }
            Message::Selection(s, v) => {
                self.operation = None;
                self.selection = s;
                self.info_values = v;
            }
            Message::SetSmallSpheres(b) => {
                self.info_values[1] = if b {
                    "true".to_string()
                } else {
                    "false".to_string()
                };
                self.requests.lock().unwrap().small_spheres = Some(b);
            }
            Message::ClearOp => self.operation = None,
            Message::ScaffoldIdSet(n, b) => {
                self.info_values[1] = if b {
                    "true".to_string()
                } else {
                    "false".to_string()
                };
                if b {
                    self.requests.lock().unwrap().set_scaffold_id = Some(Some(n))
                } else {
                    self.requests.lock().unwrap().set_scaffold_id = Some(None)
                }
            }
            Message::SetShift(f) => {
                self.info_values[2] = f.to_string();
                self.requests.lock().unwrap().new_shift_hyperboloid = Some(f);
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        if self.progress.is_some() {
            self.view_progress()
        } else if self.operation.is_some() {
            self.view_op()
        } else {
            self.view_selection()
        }
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

fn bool_to_string(b: bool) -> String {
    if b {
        String::from("true")
    } else {
        String::from("false")
    }
}
