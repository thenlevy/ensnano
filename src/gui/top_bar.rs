use native_dialog::Dialog;
use std::sync::{Arc, Mutex};

use iced::Image;
use iced::{container, Background, Container};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::LogicalSize;
use iced_winit::{button, Button, Checkbox, Color, Command, Element, Length, Program, Row};

use super::{SplitMode, Requests};

pub struct TopBar {
    button_fit: button::State,
    button_add_file: button::State,
    button_replace_file: button::State,
    button_save: button::State,
    button_3d: button::State,
    button_2d: button::State,
    button_split: button::State,
    button_make_grid: button::State,
    toggle_text_value: bool,
    requests: Arc<Mutex<Requests>>,
    logical_size: LogicalSize<f64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
    FileAddRequested,
    FileReplaceRequested,
    FileSaveRequested,
    Resize(LogicalSize<f64>),
    ToggleText(bool),
    ToggleView(SplitMode),
    MakeGrids,
}

impl TopBar {
    pub fn new(requests: Arc<Mutex<Requests>>, logical_size: LogicalSize<f64>) -> TopBar {
        Self {
            button_fit: Default::default(),
            button_add_file: Default::default(),
            button_replace_file: Default::default(),
            button_save: Default::default(),
            button_2d: Default::default(),
            button_3d: Default::default(),
            button_split: Default::default(),
            button_make_grid: Default::default(),
            toggle_text_value: false,
            requests,
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
                self.requests.lock().expect("fitting_requested").fitting = true;
            }
            Message::FileAddRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        self.requests.lock().expect("file_opening_request").file_add = Some(path);
                    }
                }
            }
            Message::FileReplaceRequested => {
                self.requests
                    .lock()
                    .expect("file_opening_request")
                    .file_clear = false;
            }
            Message::FileSaveRequested => {
                let dialog = native_dialog::OpenSingleDir { dir: None };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(mut path) = result {
                        path.push("icednano.json");
                        if path.exists() {
                            let mut addon = 1;
                            while path.exists() {
                                path.pop();
                                path.push(format!("icednano{}.json", addon));
                                addon += 1;
                            }
                        }
                        self.requests
                            .lock()
                            .expect("file_opening_request")
                            .file_save = Some(path);
                    }
                }
            }
            Message::Resize(size) => self.resize(size),
            Message::ToggleText(b) => {
                self.requests.lock().unwrap().toggle_text = Some(b);
                self.toggle_text_value = b;
            }
            Message::MakeGrids => self.requests.lock().unwrap().make_grids = true,
            Message::ToggleView(b) => self.requests.lock().unwrap().toggle_scene = Some(b),
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
        let button_save = Button::new(&mut self.button_save, Image::new("icons/save.png"))
            .on_press(Message::FileSaveRequested)
            .height(Length::Units(height));

        let button_2d = Button::new(&mut self.button_2d, iced::Text::new("2D"))
            .on_press(Message::ToggleView(SplitMode::Flat));
        let button_3d = Button::new(&mut self.button_3d, iced::Text::new("3D"))
            .on_press(Message::ToggleView(SplitMode::Scene3D));
        let button_split = Button::new(&mut self.button_split, iced::Text::new("Split"))
            .on_press(Message::ToggleView(SplitMode::Both));

        let button_make_grid =
            Button::new(&mut self.button_make_grid, iced::Text::new("Make grids"))
                .on_press(Message::MakeGrids);

        let buttons = Row::new()
            .width(Length::Fill)
            .height(Length::Units(height))
            .push(button_fit)
            .push(button_add_file)
            .push(button_replace_file)
            .push(button_save)
            .push(Checkbox::new(
                self.toggle_text_value,
                "Show Sequences",
                Message::ToggleText,
            ))
            .push(button_2d)
            .push(button_3d)
            .push(button_split)
            .push(button_make_grid);

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
