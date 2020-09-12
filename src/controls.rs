use std::path::PathBuf;
use native_dialog::Dialog;
use std::sync::{Arc, Mutex};

use iced_wgpu::Renderer;
use iced_winit::{
    Align, Column, Command, Element, Length, Program, Row,
    Text, button, Button
};

pub struct Controls {
    button_fit: button::State,
    button_file: button::State,
    pub fitting_requested: Arc<Mutex<bool>>,
    pub file_opening_request: Arc<Mutex<Option<PathBuf>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
    FileOpeningRequested,
}

impl Controls {
    pub fn new(fitting_requested: &Arc<Mutex<bool>>, file_opening_request: &Arc<Mutex<Option<PathBuf>>>) -> Controls {
        Self {
            button_fit: Default::default(),
            button_file: Default::default(),
            fitting_requested: fitting_requested.clone(),
            file_opening_request: file_opening_request.clone(),
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
            Message::FileOpeningRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        *self.file_opening_request.lock().expect("file_opening_request") = Some(PathBuf::from(path));
                    }
                }
            }
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let button_fit = Button::new(&mut self.button_fit, Text::new("Fit Scene"))
            .on_press(Message::SceneFitRequested);
        let button_file = Button::new(&mut self.button_file, Text::new("Open design"))
            .on_press(Message::FileOpeningRequested);
        let buttons = Row::new()
            .width(Length::Units(500))
            .spacing(20)
            .push(button_fit)
            .push(button_file);

        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Align::Start)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Align::Start)
                    .push(Column::new().padding(10).spacing(10).push(buttons)),
            )
            .into()
    }
}
