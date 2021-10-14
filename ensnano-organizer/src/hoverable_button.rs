//! Allow your users to perform actions by pressing a button.
//!
//! A [`HoverableButton`] is an `iced_native::Button` that produces a messages when hovered.
use iced_native::event::{self, Event};
use iced_native::layout;
use iced_native::mouse;
use iced_native::overlay;
use iced_native::touch;
use iced_native::{Clipboard, Element, Hasher, Layout, Length, Point, Rectangle, Widget};
use std::hash::Hash;

#[allow(missing_debug_implementations)]
pub struct Button<'a, Message, Renderer: iced_native::button::Renderer> {
    state: &'a mut State,
    content: Element<'a, Message, Renderer>,
    on_press: Option<Message>,
    on_hovered_in: Option<Message>,
    on_hovered_out: Option<Message>,
    width: Length,
    height: Length,
    min_width: u32,
    min_height: u32,
    padding: u16,
    style: Renderer::Style,
}

impl<'a, Message, Renderer> Button<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: iced_native::button::Renderer,
{
    /// Creates a new [`Button`] with some local [`State`] and the given
    /// content.
    pub fn new<E>(state: &'a mut State, content: E) -> Self
    where
        E: Into<Element<'a, Message, Renderer>>,
    {
        Button {
            state,
            content: content.into(),
            on_press: None,
            on_hovered_in: None,
            on_hovered_out: None,
            width: Length::Shrink,
            height: Length::Shrink,
            min_width: 0,
            min_height: 0,
            padding: Renderer::DEFAULT_PADDING.left,
            style: Renderer::Style::default(),
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the minimum width of the [`Button`].
    pub fn min_width(mut self, min_width: u32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Sets the minimum height of the [`Button`].
    pub fn min_height(mut self, min_height: u32) -> Self {
        self.min_height = min_height;
        self
    }

    /// Sets the padding of the [`Button`].
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    pub fn on_press(mut self, msg: Message) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn on_hovered_in(mut self, msg: Message) -> Self {
        self.on_hovered_in = Some(msg);
        self
    }

    pub fn on_hovered_out(mut self, msg: Message) -> Self {
        self.on_hovered_out = Some(msg);
        self
    }

    /// Sets the style of the [`Button`].
    pub fn style(mut self, style: impl Into<Renderer::Style>) -> Self {
        self.style = style.into();
        self
    }
}

/// The local state of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    is_pressed: bool,
    is_hovered: bool,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> State {
        State::default()
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Button<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: iced_native::button::Renderer,
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let padding = iced_native::Padding::from(self.padding);
        let limits = limits
            .min_width(self.min_width)
            .min_height(self.min_height)
            .width(self.width)
            .height(self.height)
            .pad(padding);

        let mut content = self.content.layout(renderer, &limits);
        content.move_to(Point::new(self.padding as f32, self.padding as f32));

        let size = limits.resolve(content.size()).pad(padding);

        layout::Node::with_children(size, vec![content])
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        messages: &mut Vec<Message>,
    ) -> event::Status {
        if let event::Status::Captured = self.content.on_event(
            event.clone(),
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            messages,
        ) {
            return event::Status::Captured;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if self.on_press.is_some() {
                    let bounds = layout.bounds();

                    if bounds.contains(cursor_position) {
                        self.state.is_pressed = true;

                        return event::Status::Captured;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = self.on_press.clone() {
                    let bounds = layout.bounds();

                    if self.state.is_pressed {
                        self.state.is_pressed = false;

                        if bounds.contains(cursor_position) {
                            messages.push(on_press);
                        }

                        return event::Status::Captured;
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => {
                self.state.is_pressed = false;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let bounds = layout.bounds();
                if bounds.contains(cursor_position) {
                    if !self.state.is_hovered {
                        if let Some(on_hovered_in) = self.on_hovered_in.clone() {
                            messages.push(on_hovered_in)
                        }
                        self.state.is_hovered = true;
                    }
                } else {
                    if self.state.is_hovered {
                        if let Some(on_hovered_out) = self.on_hovered_out.clone() {
                            messages.push(on_hovered_out)
                        }
                        self.state.is_hovered = false;
                    }
                }
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        defaults: &Renderer::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) -> Renderer::Output {
        renderer.draw(
            defaults,
            layout.bounds(),
            cursor_position,
            self.on_press.is_none(),
            self.state.is_pressed,
            &self.style,
            &self.content,
            layout.children().next().unwrap(),
        )
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.content.hash_layout(state);
    }

    fn overlay(&mut self, layout: Layout<'_>) -> Option<overlay::Element<'_, Message, Renderer>> {
        self.content.overlay(layout.children().next().unwrap())
    }
}

impl<'a, Message, Renderer> From<Button<'a, Message, Renderer>> for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced_native::button::Renderer,
{
    fn from(button: Button<'a, Message, Renderer>) -> Element<'a, Message, Renderer> {
        Element::new(button)
    }
}
