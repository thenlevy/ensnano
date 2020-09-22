use native_dialog::Dialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use iced::{container, Background, Container};
use iced_wgpu::Renderer;
use iced_winit::{
    button, Button, Color, Command, Element, Length, Program, Row, Text,
};

pub struct Controls {
    button_fit: button::State,
    button_add_file: button::State,
    button_replace_file: button::State,
    pub fitting_requested: Arc<Mutex<bool>>,
    pub file_add_request: Arc<Mutex<Option<PathBuf>>>,
    pub file_replace_request: Arc<Mutex<Option<PathBuf>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
    FileAddRequested,
    FileReplaceRequested,
}

impl Controls {
    pub fn new(
        fitting_requested: Arc<Mutex<bool>>,
        file_add_request: Arc<Mutex<Option<PathBuf>>>,
        file_replace_request: Arc<Mutex<Option<PathBuf>>>,
    ) -> Controls {
        Self {
            button_fit: Default::default(),
            button_add_file: Default::default(),
            button_replace_file: Default::default(),
            fitting_requested,
            file_add_request,
            file_replace_request,
        }
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SceneFitRequested => {
                *self.fitting_requested.lock().expect("fitting_requested") = true;
            }
            Message::FileAddRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        *self.file_add_request.lock().expect("file_opening_request") =
                            Some(PathBuf::from(path));
                    }
                }
            }
            Message::FileReplaceRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        *self
                            .file_replace_request
                            .lock()
                            .expect("file_opening_request") = Some(PathBuf::from(path));
                    }
                }
            }
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let button_fit = Button::new(&mut self.button_fit, Text::new("Fit Scene"))
            .on_press(Message::SceneFitRequested);
        let button_add_file = Button::new(&mut self.button_add_file, Text::new("Add design"))
            .on_press(Message::FileAddRequested);
        let button_replace_file =
            Button::new(&mut self.button_replace_file, Text::new("Replace designs"))
                .on_press(Message::FileReplaceRequested);
        let buttons = Row::new()
            .width(Length::Fill)
            .push(button_fit)
            .push(button_add_file)
            .push(button_replace_file);

        /*Row::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Align::Start)
        .push(
            Column::new()
                .width(Length::Fill)
                .align_items(Align::Start)
                .push(Column::new().padding(10).spacing(10).push(buttons)),
        )
        .into()*/
        Container::new(buttons)
            .width(Length::Fill)
            .style(TopBar)
            .into()
    }
}

struct TopBar;
impl container::StyleSheet for TopBar {
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
