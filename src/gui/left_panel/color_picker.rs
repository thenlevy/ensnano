use super::Message;
use iced::{
    slider, Color, Row
};

pub struct ColorPicker {
    hue_state: hue_column::State,
    light_sat_square_state: light_sat_square::State,
    color: Color,
    hue: f32,
}

use hue_column::HueColumn;
use light_sat_square::LightSatSquare;

impl ColorPicker {

    pub fn new() -> Self {
        Self {
            hue_state: Default::default(),
            light_sat_square_state: Default::default(),
            color: Color::BLACK,
            hue: 0.,
        }
    }

    pub fn update_color(&mut self, color: Color) {
        self.color = color
    }

    pub fn change_hue(&mut self, hue: f32) {
        self.hue = hue
    }
    
    pub fn view(&mut self) -> Row<Message> {
        let color_picker = Row::new()
            .spacing(5)
            .push(HueColumn::new(&mut self.hue_state, |x| Message::HueChanged(x)))
            .spacing(10)
            .push(LightSatSquare::new(self.hue as f64, &mut self.light_sat_square_state, |c| Message::StrandColorChanged(c)));
        color_picker
    }
}

mod hue_column {
    use iced_graphics::{
        triangle::{Mesh2D, Vertex2D},
        Backend, Defaults, Primitive, Renderer,
    };
    use iced_native::{
        layout, mouse, Element, Hasher, Layout, Length, Point, Size, Vector,
        Widget, Clipboard, Event,
    };

    use color_space::{Rgb, Hsv};

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
        on_slide: Box<dyn Fn(f32) -> Message>
    }

    impl<'a, Message> HueColumn<'a, Message> {
        pub fn new<F>(state: &'a mut State, on_slide: F) -> Self
            where F: 'static + Fn(f32) -> Message, {
            Self {
                state,
                on_slide: Box::new(on_slide)
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

        fn layout(
            &self,
            _renderer: &Renderer<B>,
            limits: &layout::Limits,
        ) -> layout::Node {
            let size = limits.width(Length::Fill).height(Length::Fill).resolve(Size::ZERO);

            layout::Node::new(Size::new(size.width, 4. * size.width))
        }

        fn hash_layout(&self, _state: &mut Hasher) {}

        fn draw(
            &self,
            _renderer: &mut Renderer<B>,
            _defaults: &Defaults,
            layout: Layout<'_>,
            _cursor_position: Point,
        ) -> (Primitive, mouse::Interaction) {
            let b = layout.bounds();

            let x_max = b.width;
            let y_max = b.height;
            

            let nb_row = 10;
            
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for i in 0..nb_row {
                let hsv = Hsv::new(i as f64 / nb_row as f64 * 360., 1., 1.);
                let rgb = Rgb::from(hsv);
                let color = [rgb.r as f32 / 255., rgb.g as f32 / 255., rgb.b as f32 / 255., 1.];
                vertices.push(Vertex2D {
                    position:[0., y_max * (i as f32/ nb_row as f32)],
                    color: color.clone()
                });
                vertices.push(Vertex2D {
                    position:[x_max, y_max * (i as f32/ nb_row as f32)],
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
                        buffers: Mesh2D {
                            vertices,
                            indices,
                        },
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
            messages: &mut Vec<Message>,
            _renderer: &Renderer<B>,
            _clipboard: Option<&dyn Clipboard>,
        ) {
            let mut change = || {
                let bounds = layout.bounds();
                if cursor_position.y <= bounds.y {
                    messages.push((self.on_slide)(0.));
                } else if cursor_position.y >= bounds.y + bounds.height {
                    messages.push((self.on_slide)(360.));
                } else {
                    let percent = f32::from(cursor_position.y - bounds.y)
                        / f32::from(bounds.height);

                    let value = percent * 360.;
                    messages.push((self.on_slide)(value));
                }
            };

            match event {
                Event::Mouse(mouse_event) => match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if layout.bounds().contains(cursor_position) {
                            change();
                            self.state.is_dragging = true;
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        if self.state.is_dragging {
                            self.state.is_dragging = false;
                        }
                    }
                    mouse::Event::CursorMoved { .. } => {
                        if self.state.is_dragging {
                            change();
                        }
                    }
                    _ => {}
                },
                _ => {}
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
        Backend, Defaults, Primitive, Renderer,
    };
    use iced_native::{
        layout, mouse, Element, Hasher, Layout, Length, Point, Size, Vector,
        Widget, Event, Clipboard
    };
    use super::Color;

    use color_space::{Rgb, Hsv};

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

    fn hsv_to_linear(hue: f64, sat: f64, light: f64) -> [f32; 4] {
        let hsv = Hsv::new(hue, sat, light);
        let rgb = Rgb::from(hsv);
        [rgb.r as f32 / 255., rgb.g as f32 / 255., rgb.b as f32 / 255., 1.]
    }

    pub struct LightSatSquare<'a, Message> {
        hue: f64,
        state: &'a mut State,
        on_slide: Box<dyn Fn(Color) -> Message>
    }

    impl<'a, Message> LightSatSquare<'a, Message> {
        pub fn new<F>(hue: f64, state: &'a mut State, on_slide: F) -> Self
        where F: 'static + Fn(Color) -> Message {
            Self {
                hue,
                state,
                on_slide: Box::new(on_slide)
            }
        }
    }


    impl<'a, Message, B> Widget<Message, Renderer<B>> for LightSatSquare<'a, Message>
        where
            B: Backend,
        {
            fn width(&self) -> Length {
                Length::FillPortion(4)
            }

            fn height(&self) -> Length {
                Length::Shrink
            }

            fn layout(
                &self,
                _renderer: &Renderer<B>,
                limits: &layout::Limits,
            ) -> layout::Node {
                let size = limits.width(Length::Fill).height(Length::Fill).resolve(Size::ZERO);

                layout::Node::new(Size::new(size.width, size.width))
            }

            fn hash_layout(&self, _state: &mut Hasher) {}

            fn draw(
                &self,
                _renderer: &mut Renderer<B>,
                _defaults: &Defaults,
                layout: Layout<'_>,
                _cursor_position: Point,
            ) -> (Primitive, mouse::Interaction) {
                let b = layout.bounds();

                let x_max = b.width;
                let y_max = b.height;

                let nb_row = 100;
                let nb_column = 100;

                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                for i in 0..nb_row {
                    let light = 1. - (i as f64 / nb_row as f64);
                    for j in 0..nb_column {
                        let sat = j as f64 / nb_column as f64;
                        let color = hsv_to_linear(self.hue, sat, light);
                        vertices.push(Vertex2D {
                            position:[x_max * (j as f32 / nb_column as f32), y_max * (i as f32/ nb_row as f32)],
                            color: color.clone()
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
                            buffers: Mesh2D {
                                vertices,
                                indices,
                            },
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
                messages: &mut Vec<Message>,
                _renderer: &Renderer<B>,
                _clipboard: Option<&dyn Clipboard>,
            ) {
                let mut change = || {
                    let bounds = layout.bounds();
                    let percent_x = if cursor_position.x <= bounds.x {
                        0.
                    } else if cursor_position.x >= bounds.x + bounds.width {
                        1.
                    } else {
                        f64::from(cursor_position.x - bounds.x)
                            / f64::from(bounds.width)
                    };

                    let percent_y = if cursor_position.y <= bounds.y {
                        0.
                    } else if cursor_position.y >= bounds.y + bounds.height {
                        1.
                    } else {
                        f64::from(cursor_position.y - bounds.y)
                            / f64::from(bounds.height)
                    };

                    let color = Rgb::from(Hsv::new(self.hue, percent_x, 1. - percent_y));
                    let value = Color::from_rgb(color.r as f32 / 255., color.g as f32 / 255., color.b as f32 / 255.);
                    messages.push((self.on_slide)(value));
                };

                match event {
                    Event::Mouse(mouse_event) => match mouse_event {
                        mouse::Event::ButtonPressed(mouse::Button::Left) => {
                            if layout.bounds().contains(cursor_position) {
                                change();
                                self.state.is_dragging = true;
                            }
                        }
                        mouse::Event::ButtonReleased(mouse::Button::Left) => {
                            if self.state.is_dragging {
                                self.state.is_dragging = false;
                            }
                        }
                        mouse::Event::CursorMoved { .. } => {
                            if self.state.is_dragging {
                                change();
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

    impl<'a, Message, B> Into<Element<'a, Message, Renderer<B>>> for LightSatSquare<'a, Message>
        where
            B: Backend,
            Message: 'a + Clone
        {
            fn into(self) -> Element<'a, Message, Renderer<B>> {
                Element::new(self)
            }
        }
}