use crate::design_handler::DesignHandler;
use crate::scene::Scene;

use iced::{button, slider, Align, Button, Column, Element, Length, Row, Text};

pub struct Controls {
    slider: slider::State,
    button: button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            slider: Default::default(),
            button: Default::default(),
        }
    }

    pub fn update(&self, message: Message, design_handler: &DesignHandler, scene: &mut Scene) {
        match message {
            Message::SceneFitRequested => {
                design_handler.fit_design(scene);
            }
        }
    }

    pub fn view(&mut self, scene: &Scene) -> Element<Message> {
        let slider_n = &mut self.slider;
        let number_instances = scene.number_instances;

        let button = Button::new(&mut self.button, Text::new("Fit Scene"))
            .on_press(Message::SceneFitRequested);
        let buttons = Row::new()
            .width(Length::Units(500))
            .spacing(20)
            .push(button);

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
