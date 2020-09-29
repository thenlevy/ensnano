use native_dialog::Dialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use iced::Image;
use iced::{container, Background, Container};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::LogicalSize;
use iced_winit::{button, Button, Color, Command, Element, Length, Program, Row};

pub struct TopBar {
    button_fit: button::State,
    button_add_file: button::State,
    button_replace_file: button::State,
    pub fitting_requested: Arc<Mutex<bool>>,
    pub file_add_request: Arc<Mutex<Option<PathBuf>>>,
    pub file_replace_request: Arc<Mutex<Option<PathBuf>>>,
    logical_size: LogicalSize<f64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
    FileAddRequested,
    FileReplaceRequested,
    Resize(LogicalSize<f64>),
}

impl TopBar {
    pub fn new(
        fitting_requested: Arc<Mutex<bool>>,
        file_add_request: Arc<Mutex<Option<PathBuf>>>,
        file_replace_request: Arc<Mutex<Option<PathBuf>>>,
        logical_size: LogicalSize<f64>,
    ) -> TopBar {
        Self {
            button_fit: Default::default(),
            button_add_file: Default::default(),
            button_replace_file: Default::default(),
            fitting_requested,
            file_add_request,
            file_replace_request,
            logical_size,
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize<f64>) {
        self.logical_size = logical_size;
    }
}

impl Program for TopBar {
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
            Message::Resize(size) => self.resize(size),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let height = self.logical_size.cast::<u16>().height;
        let button_fit = Button::new(&mut self.button_fit, Image::new("icons/adjust_page.png"))
            .on_press(Message::SceneFitRequested)
            .height(Length::Units(height));
        let button_add_file = Button::new(
            &mut self.button_add_file,
            Image::new("icons/add_file.png").height(Length::Units(height)),
        )
        .on_press(Message::FileAddRequested)
        .height(Length::Units(height));
        let button_replace_file = Button::new(
            &mut self.button_replace_file,
            Image::new("icons/delete.png"),
        )
        .on_press(Message::FileReplaceRequested)
        .height(Length::Units(height));

        let buttons = Row::new()
            .width(Length::Fill)
            .height(Length::Units(height))
            .push(button_fit)
            .push(button_add_file)
            .push(button_replace_file);

        Container::new(buttons)
            .width(Length::Fill)
            .style(TopBarStyle)
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
    0x36 as f32 / 255.0,
    0x39 as f32 / 255.0,
    0x3F as f32 / 255.0,
);
