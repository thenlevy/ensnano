use iced::{Container, Element};
use iced_native::{
    event, layout, overlay, Alignment, Clipboard, Event, Hasher, Layout, Length, Point, Rectangle,
    Widget,
};
use std::hash::Hash;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Ord)]
pub(super) enum Identifier<K> {
    Group { id: super::NodeId },
    Section { key: K },
}

pub(super) struct DragDropTarget<'a, Message, K> {
    padding: u16,
    width: Length,
    height: Length,
    max_width: u32,
    max_height: u32,
    horizontal_alignment: Alignment,
    vertical_alignment: Alignment,
    content: Container<'a, Message>,
    identifier: Identifier<K>,
}

impl<'a, Message, K> DragDropTarget<'a, Message, K> {
    /// Creates an empty [`DragDropTarget`].
    pub fn new<T>(content: T, identifier: Identifier<K>) -> Self
    where
        T: Into<Element<'a, Message>>,
    {
        Self {
            padding: 0,
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: u32::MAX,
            max_height: u32::MAX,
            horizontal_alignment: Alignment::Start,
            vertical_alignment: Alignment::Start,
            content: Container::new(content).width(Length::Fill),
            identifier,
        }
    }

    /// Sets the width of the [`Container`] contained in self.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width.clone();
        self.content = self.content.width(width);
        self
    }
}

use super::OrganizerMessage;
use iced_wgpu::Renderer;

impl<'a, E: super::OrganizerElement> Widget<OrganizerMessage<E>, Renderer>
    for DragDropTarget<'a, OrganizerMessage<E>, E::Key>
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
            .loose()
            .max_width(self.max_width)
            .max_height(self.max_height)
            .width(self.width)
            .height(self.height)
            .pad(padding);

        let mut content = self.content.layout(renderer, &limits.loose());
        let size = limits.resolve(content.size());

        content.move_to(Point::new(self.padding as f32, self.padding as f32));
        content.align(self.horizontal_alignment, self.vertical_alignment, size);

        layout::Node::with_children(size.pad(padding), vec![content])
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        messages: &mut Vec<OrganizerMessage<E>>,
    ) -> event::Status {
        use iced::mouse;
        use iced::mouse::Event as MouseEvent;
        let status = self.content.on_event(
            event.clone(),
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            messages,
        );
        match event {
            Event::Mouse(MouseEvent::ButtonReleased(mouse::Button::Left)) => {
                if layout.bounds().contains(cursor_position) {
                    messages.push(OrganizerMessage::drag_dropped(self.identifier.clone()))
                }
            }
            Event::Mouse(MouseEvent::ButtonPressed(mouse::Button::Left)) => {
                if layout.bounds().contains(cursor_position) {
                    messages.push(OrganizerMessage::dragging(self.identifier.clone()))
                }
                return event::Status::Captured;
            }
            _ => (),
        };
        status
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        defaults: &<Renderer as iced_native::Renderer>::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) -> <Renderer as iced_native::Renderer>::Output {
        self.content.draw(
            renderer,
            defaults,
            layout.children().next().unwrap(),
            cursor_position,
            viewport,
        )
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.padding.hash(state);
        self.width.hash(state);
        self.height.hash(state);
        self.max_width.hash(state);
        self.max_height.hash(state);

        self.content.hash_layout(state);
    }

    fn overlay(
        &mut self,
        layout: Layout<'_>,
    ) -> Option<overlay::Element<'_, OrganizerMessage<E>, Renderer>> {
        self.content.overlay(layout.children().next().unwrap())
    }
}
