use super::Message;
use iced::{
    slider, Color, Column, Slider
};


pub struct ColorPicker {
    sliders: [slider::State; 3],
    color: Color,
}

impl ColorPicker {

    pub fn new() -> Self {
        Self {
            sliders: Default::default(),
            color: Color::BLACK,
        }
    }

    pub fn update_color(&mut self, color: Color) {
        self.color = color
    }
    
    pub fn view(&mut self) -> Column<Message> {
        let [r, g, b] = &mut self.sliders;
        let color = self.color;

        let sliders = Column::new()
            .spacing(20)
            .push(
                Slider::new(r, 0.0..=1.0, color.r, move |r| {
                    Message::StrandColorChanged(Color {
                        r,
                        ..color
                    })
                })
                .step(0.01),
            )
            .push(
                Slider::new(g, 0.0..=1.0, color.g, move |g| {
                    Message::StrandColorChanged(Color {
                        g,
                        ..color
                    })
                })
                .step(0.01),
            )
            .push(
                Slider::new(b, 0.0..=1.0, color.b, move |b| {
                    Message::StrandColorChanged(Color {
                        b,
                        ..color
                    })
                })
                .step(0.01),
            );

        sliders
    }
}
