/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::{AppState, ColorMessage, Message};
use iced::{Color, Row};

pub struct ColorPicker {
    hue_state: hue_column::State,
    light_sat_square_state: light_sat_square::State,
    color: Color,
    hue: f64,
    saturation: f64,
    hsv_value: f64,
}

pub use color_square::{ColorSquare, State as ColorState};
use hue_column::HueColumn;
use light_sat_square::LightSatSquare;

impl ColorPicker {
    pub fn new() -> Self {
        Self {
            hue_state: Default::default(),
            light_sat_square_state: Default::default(),
            color: Color::BLACK,
            hue: 0.,
            saturation: 1.,
            hsv_value: 1.,
        }
    }

    pub fn update_color(&mut self) -> Color {
        use color_space::{Hsv, Rgb};
        let hsv = Hsv::new(self.hue, self.saturation, self.hsv_value);
        let rgb = Rgb::from(hsv);
        let color: Color = [
            rgb.r as f32 / 255.,
            rgb.g as f32 / 255.,
            rgb.b as f32 / 255.,
            1.,
        ]
        .into();
        self.color = color;
        color
    }

    pub fn change_hue(&mut self, hue: f64) {
        self.hue = hue
    }

    pub fn set_saturation(&mut self, saturation: f64) {
        self.saturation = saturation
    }

    pub fn set_hsv_value(&mut self, hsv_value: f64) {
        self.hsv_value = hsv_value
    }

    pub fn view<S: AppState>(&mut self) -> Row<Message<S>> {
        let color_picker = Row::new()
            .spacing(5)
            .push(HueColumn::new(&mut self.hue_state, Message::HueChanged))
            .spacing(10)
            .push(LightSatSquare::new(
                self.hue as f64,
                &mut self.light_sat_square_state,
                Message::HsvSatValueChanged,
                Message::FinishChangingColor,
            ));
        color_picker
    }

    pub fn color_square<'a, S: AppState>(
        &self,
        state: &'a mut color_square::State,
    ) -> ColorSquare<'a, Message<S>> {
        ColorSquare::new(
            self.color,
            state,
            Message::ColorPicked,
            Message::FinishChangingColor,
        )
    }

    pub fn new_view(&mut self) -> Row<ColorMessage> {
        let color_picker = Row::new()
            .spacing(5)
            .push(HueColumn::new(
                &mut self.hue_state,
                ColorMessage::HueChanged,
            ))
            .spacing(10)
            .push(LightSatSquare::new(
                self.hue as f64,
                &mut self.light_sat_square_state,
                ColorMessage::HsvSatValueChanged,
                ColorMessage::FinishChangingColor,
            ));
        color_picker
    }
}

mod hue_column {
    use iced_graphics::{
        triangle::{Mesh2D, Vertex2D},
        Backend, Defaults, Primitive, Rectangle, Renderer,
    };
    use iced_native::{
        layout, mouse, Clipboard, Element, Event, Hasher, Layout, Length, Point, Size, Vector,
        Widget,
    };

    use color_space::{Hsv, Rgb};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct State {
        is_dragging: bool,
    }

    impl State {
        /// Creates a new [`State`].
        ///
        pub fn new() -> State {
            State::default()
        }
    }

    pub struct HueColumn<'a, Message> {
        state: &'a mut State,
        on_slide: Box<dyn Fn(f64) -> Message>,
    }

    impl<'a, Message> HueColumn<'a, Message> {
        pub fn new<F>(state: &'a mut State, on_slide: F) -> Self
        where
            F: 'static + Fn(f64) -> Message,
        {
            Self {
                state,
                on_slide: Box::new(on_slide),
            }
        }
    }

    impl<'a, Message, B> Widget<Message, Renderer<B>> for HueColumn<'a, Message>
    where
        B: Backend,
    {
        fn width(&self) -> Length {
            Length::FillPortion(1)
        }

        fn height(&self) -> Length {
            Length::Shrink
        }

        fn layout(&self, _renderer: &Renderer<B>, limits: &layout::Limits) -> layout::Node {
            let size = limits
                .width(Length::Fill)
                .height(Length::Fill)
                .resolve(Size::ZERO);

            layout::Node::new(Size::new(size.width, 4. * size.width))
        }

        fn hash_layout(&self, _state: &mut Hasher) {}

        fn draw(
            &self,
            _renderer: &mut Renderer<B>,
            _defaults: &Defaults,
            layout: Layout<'_>,
            _cursor_position: Point,
            _viewport: &Rectangle,
        ) -> (Primitive, mouse::Interaction) {
            let b = layout.bounds();

            let x_max = b.width;
            let y_max = b.height;

            let nb_row = 10;

            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for i in 0..=nb_row {
                let hsv = Hsv::new(i as f64 / nb_row as f64 * 360., 1., 1.);
                let rgb = Rgb::from(hsv);
                let color = [
                    rgb.r as f32 / 255.,
                    rgb.g as f32 / 255.,
                    rgb.b as f32 / 255.,
                    1.,
                ];
                vertices.push(Vertex2D {
                    position: [0., y_max * (i as f32 / nb_row as f32)],
                    color,
                });
                vertices.push(Vertex2D {
                    position: [x_max, y_max * (i as f32 / nb_row as f32)],
                    color,
                });
                if i > 0 {
                    indices.push(2 * i - 2);
                    indices.push(2 * i + 1);
                    indices.push(2 * i);
                    indices.push(2 * i - 2);
                    indices.push(2 * i + 1);
                    indices.push(2 * i - 1);
                }
            }

            (
                Primitive::Translate {
                    translation: Vector::new(b.x, b.y),
                    content: Box::new(Primitive::Mesh2D {
                        size: b.size(),
                        buffers: Mesh2D { vertices, indices },
                    }),
                },
                mouse::Interaction::default(),
            )
        }

        fn on_event(
            &mut self,
            event: Event,
            layout: Layout<'_>,
            cursor_position: Point,
            _renderer: &Renderer<B>,
            _clipboard: &mut dyn Clipboard,
            messages: &mut Vec<Message>,
        ) -> iced_native::event::Status {
            let mut change = || {
                let bounds = layout.bounds();
                if cursor_position.y <= bounds.y {
                    messages.push((self.on_slide)(0.));
                } else if cursor_position.y >= bounds.y + bounds.height {
                    messages.push((self.on_slide)(360.));
                } else {
                    let percent = (cursor_position.y - bounds.y) / bounds.height;
                    let value = percent * 360.;
                    messages.push((self.on_slide)(value.into()));
                }
            };

            if let Event::Mouse(mouse_event) = event {
                match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if layout.bounds().contains(cursor_position) {
                            change();
                            self.state.is_dragging = true;
                        }
                        iced_native::event::Status::Captured
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        if self.state.is_dragging {
                            self.state.is_dragging = false;
                        }
                        iced_native::event::Status::Captured
                    }
                    mouse::Event::CursorMoved { .. } => {
                        if self.state.is_dragging {
                            change();
                            iced_native::event::Status::Captured
                        } else {
                            iced_native::event::Status::Ignored
                        }
                    }
                    _ => iced_native::event::Status::Ignored,
                }
            } else {
                iced_native::event::Status::Ignored
            }
        }
    }

    impl<'a, Message, B> From<HueColumn<'a, Message>> for Element<'a, Message, Renderer<B>>
    where
        B: Backend,
        Message: 'a + Clone,
    {
        fn from(hue_column: HueColumn<'a, Message>) -> Element<'a, Message, Renderer<B>> {
            Element::new(hue_column)
        }
    }
}

mod light_sat_square {
    use iced_graphics::{
        triangle::{Mesh2D, Vertex2D},
        Backend, Defaults, Primitive, Rectangle, Renderer,
    };
    use iced_native::{
        layout, mouse, Clipboard, Element, Event, Hasher, Layout, Length, Point, Size, Vector,
        Widget,
    };

    use color_space::{Hsv, Rgb};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct State {
        is_dragging: bool,
    }

    fn hsv_to_linear(hue: f64, sat: f64, light: f64) -> [f32; 4] {
        let hsv = Hsv::new(hue, sat, light);
        let rgb = Rgb::from(hsv);
        [
            rgb.r as f32 / 255.,
            rgb.g as f32 / 255.,
            rgb.b as f32 / 255.,
            1.,
        ]
    }

    pub struct LightSatSquare<'a, Message: Clone> {
        hue: f64,
        state: &'a mut State,
        on_slide: Box<dyn Fn(f64, f64) -> Message>,
        on_finish: Message,
    }

    impl<'a, Message: Clone> LightSatSquare<'a, Message> {
        pub fn new<F>(hue: f64, state: &'a mut State, on_slide: F, on_finish: Message) -> Self
        where
            F: 'static + Fn(f64, f64) -> Message,
        {
            Self {
                hue,
                state,
                on_slide: Box::new(on_slide),
                on_finish,
            }
        }
    }

    impl<'a, Message: Clone, B> Widget<Message, Renderer<B>> for LightSatSquare<'a, Message>
    where
        B: Backend,
    {
        fn width(&self) -> Length {
            Length::FillPortion(4)
        }

        fn height(&self) -> Length {
            Length::Shrink
        }

        fn layout(&self, _renderer: &Renderer<B>, limits: &layout::Limits) -> layout::Node {
            let size = limits
                .width(Length::Fill)
                .height(Length::Fill)
                .resolve(Size::ZERO);

            layout::Node::new(Size::new(size.width, size.width))
        }

        fn hash_layout(&self, _state: &mut Hasher) {}

        fn draw(
            &self,
            _renderer: &mut Renderer<B>,
            _defaults: &Defaults,
            layout: Layout<'_>,
            _cursor_position: Point,
            _viewport: &Rectangle,
        ) -> (Primitive, mouse::Interaction) {
            let b = layout.bounds();

            let x_max = b.width;
            let y_max = b.height;

            let nb_row = 100;
            let nb_column = 100;

            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for i in 0..nb_row {
                let value = 1. - (i as f64 / nb_row as f64);
                for j in 0..nb_column {
                    let sat = 1. - (j as f64 / nb_column as f64);
                    let color = hsv_to_linear(self.hue, sat, value);
                    vertices.push(Vertex2D {
                        position: [
                            x_max * (j as f32 / nb_column as f32),
                            y_max * (i as f32 / nb_row as f32),
                        ],
                        color,
                    });
                    if i > 0 && j > 0 {
                        indices.push(nb_row * (i - 1) + j - 1);
                        indices.push(nb_row * i + j);
                        indices.push(nb_row * i + j - 1);
                        indices.push(nb_row * (i - 1) + j - 1);
                        indices.push(nb_row * i + j);
                        indices.push(nb_row * (i - 1) + j);
                    }
                }
            }

            (
                Primitive::Translate {
                    translation: Vector::new(b.x, b.y),
                    content: Box::new(Primitive::Mesh2D {
                        size: b.size(),
                        buffers: Mesh2D { vertices, indices },
                    }),
                },
                mouse::Interaction::default(),
            )
        }

        fn on_event(
            &mut self,
            event: Event,
            layout: Layout<'_>,
            cursor_position: Point,
            _renderer: &Renderer<B>,
            _clipboard: &mut dyn Clipboard,
            messages: &mut Vec<Message>,
        ) -> iced_native::event::Status {
            let mut change = || {
                let bounds = layout.bounds();
                let percent_x = if cursor_position.x <= bounds.x {
                    0.
                } else if cursor_position.x >= bounds.x + bounds.width {
                    1.
                } else {
                    f64::from(cursor_position.x - bounds.x) / f64::from(bounds.width)
                };

                let percent_y = if cursor_position.y <= bounds.y {
                    0.
                } else if cursor_position.y >= bounds.y + bounds.height {
                    1.
                } else {
                    f64::from(cursor_position.y - bounds.y) / f64::from(bounds.height)
                };

                let saturation = 1. - percent_x;
                let value = 1. - percent_y;
                messages.push((self.on_slide)(saturation, value));
            };

            if let Event::Mouse(mouse_event) = event {
                match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if layout.bounds().contains(cursor_position) {
                            change();
                            self.state.is_dragging = true;
                            iced_native::event::Status::Captured
                        } else {
                            iced_native::event::Status::Ignored
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        if self.state.is_dragging {
                            self.state.is_dragging = false;
                        }
                        messages.push(self.on_finish.clone());
                        iced_native::event::Status::Captured
                    }
                    mouse::Event::CursorMoved { .. } => {
                        if self.state.is_dragging {
                            change();
                        }
                        iced_native::event::Status::Captured
                    }
                    _ => iced_native::event::Status::Ignored,
                }
            } else {
                iced_native::event::Status::Ignored
            }
        }
    }

    impl<'a, Message, B> Into<Element<'a, Message, Renderer<B>>> for LightSatSquare<'a, Message>
    where
        B: Backend,
        Message: 'a + Clone,
    {
        fn into(self) -> Element<'a, Message, Renderer<B>> {
            Element::new(self)
        }
    }
}

mod color_square {
    #[derive(Default, Clone, Eq, PartialEq)]
    pub struct State {
        clicked: bool,
    }
    use super::Color;
    use iced_graphics::{
        triangle::{Mesh2D, Vertex2D},
        Backend, Defaults, Primitive, Rectangle, Renderer,
    };
    use iced_native::{
        layout, mouse, Clipboard, Element, Event, Hasher, Layout, Length, Point, Size, Vector,
        Widget,
    };

    pub struct ColorSquare<'a, Message: Clone> {
        state: &'a mut State,
        color: Color,
        on_click: Box<dyn Fn(Color) -> Message>,
        on_release: Message,
    }

    impl<'a, Message: Clone> ColorSquare<'a, Message> {
        pub fn new<F>(color: Color, state: &'a mut State, on_click: F, on_release: Message) -> Self
        where
            F: 'static + Fn(Color) -> Message,
        {
            Self {
                state,
                color,
                on_click: Box::new(on_click),
                on_release,
            }
        }
    }

    impl<'a, Message: Clone, B> Widget<Message, Renderer<B>> for ColorSquare<'a, Message>
    where
        B: Backend,
    {
        fn width(&self) -> Length {
            Length::FillPortion(1)
        }

        fn height(&self) -> Length {
            Length::FillPortion(1)
        }

        fn layout(&self, _renderer: &Renderer<B>, limits: &layout::Limits) -> layout::Node {
            let size = limits
                .width(Length::Fill)
                .height(Length::Fill)
                .resolve(Size::ZERO);

            layout::Node::new(Size::new(size.width, size.width))
        }

        fn hash_layout(&self, _state: &mut Hasher) {}

        fn draw(
            &self,
            _renderer: &mut Renderer<B>,
            _defaults: &Defaults,
            layout: Layout<'_>,
            _cursor_position: Point,
            _viewport: &Rectangle,
        ) -> (Primitive, mouse::Interaction) {
            let b = layout.bounds();
            let x_max = b.width;
            let y_max = b.height;
            let color = [self.color.r, self.color.g, self.color.b, self.color.a];
            let vertices = vec![
                Vertex2D {
                    position: [0., 0.],
                    color,
                },
                Vertex2D {
                    position: [0., y_max],
                    color,
                },
                Vertex2D {
                    position: [x_max, 0.],
                    color,
                },
                Vertex2D {
                    position: [x_max, y_max],
                    color,
                },
            ];
            let indices = vec![0, 1, 2, 1, 2, 3];
            (
                Primitive::Translate {
                    translation: Vector::new(b.x, b.y),
                    content: Box::new(Primitive::Mesh2D {
                        size: b.size(),
                        buffers: Mesh2D { vertices, indices },
                    }),
                },
                mouse::Interaction::default(),
            )
        }

        fn on_event(
            &mut self,
            event: Event,
            layout: Layout<'_>,
            cursor_position: Point,
            _renderer: &Renderer<B>,
            _clipboard: &mut dyn Clipboard,
            messages: &mut Vec<Message>,
        ) -> iced_native::event::Status {
            if let Event::Mouse(mouse_event) = event {
                match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if layout.bounds().contains(cursor_position) {
                            self.state.clicked = true;
                            messages.push((self.on_click)(self.color));
                            iced_native::event::Status::Captured
                        } else {
                            iced_native::event::Status::Ignored
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) if self.state.clicked => {
                        if layout.bounds().contains(cursor_position) {
                            self.state.clicked = false;
                            messages.push(self.on_release.clone());
                            iced_native::event::Status::Captured
                        } else {
                            iced_native::event::Status::Ignored
                        }
                    }
                    mouse::Event::CursorMoved { .. } if self.state.clicked => {
                        if layout.bounds().contains(cursor_position) {
                            iced_native::event::Status::Ignored
                        } else {
                            self.state.clicked = false;
                            messages.push(self.on_release.clone());
                            iced_native::event::Status::Captured
                        }
                    }
                    _ => iced_native::event::Status::Ignored,
                }
            } else {
                iced_native::event::Status::Ignored
            }
        }
    }

    impl<'a, Message, B> Into<Element<'a, Message, Renderer<B>>> for ColorSquare<'a, Message>
    where
        B: Backend,
        Message: 'a + Clone,
    {
        fn into(self) -> Element<'a, Message, Renderer<B>> {
            Element::new(self)
        }
    }
}
