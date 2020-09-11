use crate::design_handler::DesignHandler;
use crate::scene::Scene;
use native_dialog::Dialog;

use iced::{button, Align, Button, Column, Element, Length, Row, Text};

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
        Controls {
            button_fit: Default::default(),
            button_file: Default::default(),
        }
    }

    pub fn update(&self, message: Message, design_handler: &mut DesignHandler, scene: &mut Scene) {
        match message {
            Message::SceneFitRequested => {
                design_handler.fit_design(scene);
            }
            Message::FileOpeningRequested => {
                let dialog = native_dialog::OpenSingleFile {
                    dir: None,
                    filter: None,
                };
                let result = dialog.show();
                if let Ok(result) = result {
                    if let Some(path) = result {
                        design_handler.get_design(&path);
                        design_handler.update_scene(scene, true);
                        design_handler.fit_design(scene);
                    }
                }
            }
        }
    }

    pub fn view(&mut self) -> Element<Message> {
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
