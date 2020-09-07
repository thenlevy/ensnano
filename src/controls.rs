use crate::scene::Scene;

use iced_wgpu::Renderer;
use iced_winit::{slider, Align, Color, Column, Element, Length, Row, Slider, Text};

pub struct Controls {
    slider: slider::State,
}

#[derive(Debug)]
pub enum Message {
    NumberInstancesChanged(u32),
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            slider: Default::default(),
        }
    }

    pub fn update(&self, message: Message, scene: &mut Scene) {
        match message {
            Message::NumberInstancesChanged(number_instances) => {
                scene.number_instances = number_instances;
                //scene.update();
            }
        }
    }

    pub fn view(&mut self, scene: &Scene) -> Element<Message, Renderer> {
        let slider_n = &mut self.slider;
        let number_instances = scene.number_instances;

        let sliders = Row::new()
            .width(Length::Units(500))
            .spacing(20)
            .push(Slider::new(
                slider_n,
                0.0..=10.,
                number_instances as f32,
                move |n| Message::NumberInstancesChanged(n as u32),
            ));

        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Align::End)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Align::End)
                    .push(
                        Column::new()
                            .padding(10)
                            .spacing(10)
                            .push(Text::new("Number of cubes").color(Color::WHITE))
                            .push(sliders)
                            .push(
                                Text::new(format!("{:?}", number_instances))
                                    .size(14)
                                    .color(Color::WHITE),
                            ),
                    ),
            )
            .into()
    }
}
