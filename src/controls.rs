use crate::design_handler::DesignHandler;
use crate::scene::{Scene};
use crate::PhySize;
use std::path::PathBuf;
use native_dialog::Dialog;
use iced_wgpu::wgpu;
use wgpu::Device;

use iced_wgpu::Renderer;
use iced_winit::{
    slider, Align, Color, Column, Command, Element, Length, Program, Row,
    Slider, Text, button, Button
};

pub struct Controls {
    button_fit: button::State,
    button_file: button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
    FileOpeningRequested,
}

impl Controls {
    pub fn new() -> Controls {
        Self {
            button_fit: Default::default(),
            button_file: Default::default(),
        }
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;
    

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SceneFitRequested => {
                self.design_handler.fit_design(&mut self.scene);
            }
            Message::FileOpeningRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        self.design_handler.get_design(&path);
                        self.design_handler.update_scene(&mut self.scene, true);
                        self.design_handler.fit_design(&mut self.scene);
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
