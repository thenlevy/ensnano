use iced::pick_list::Style as PickListStyle;
use iced::Element;
use iced::{button, pick_list, Button, PickList, Text};

/// A key identifing an element
pub trait ElementKey:
    Clone + std::cmp::Ord + std::fmt::Debug + serde::Serialize + serde::Deserialize<'static>
{
    type Section: std::cmp::Eq
        + std::cmp::Ord
        + core::convert::TryFrom<usize>
        + std::fmt::Debug
        + core::convert::Into<usize>;

    fn name(section: Self::Section) -> String;
    fn section(&self) -> Self::Section;
}

/// A root node of the organizer tree.
pub trait OrganizerElement: Clone + std::fmt::Debug + 'static {
    /// A type that describes all the attributes of an element that can be changed through
    /// interaction with the organizer.
    type Attribute: OrganizerAttribute;
    /// A type that is used to store the elements in a BTreeMap
    type Key: ElementKey;

    type AutoGroup: ToString + std::cmp::Ord + std::cmp::Eq + Clone + std::fmt::Debug;

    /// The name that will be displayed to represent the element
    fn display_name(&self) -> String;
    /// The key that will be used to store self in a BTreeMap
    fn key(&self) -> Self::Key;

    /// The aliases of the element that can be used to search it
    fn aliases(&self) -> Vec<String> {
        vec![self.display_name()]
    }

    fn attributes(&self) -> Vec<Self::Attribute>;

    fn all_repr() -> &'static [<Self::Attribute as OrganizerAttribute>::Repr] {
        Self::Attribute::all_repr()
    }

    fn auto_groups(&self) -> Vec<Self::AutoGroup>;
}

pub trait OrganizerAttributeRepr:
    std::cmp::Ord
    + std::cmp::Eq
    + core::convert::TryFrom<usize>
    + core::convert::Into<usize>
    + std::fmt::Debug
    + Clone
{
    fn all_repr() -> &'static [Self];
}

pub trait OrganizerAttribute: Clone + std::fmt::Debug + 'static + Ord + std::fmt::Display {
    /// A type used to represent the different values of self
    type Repr: OrganizerAttributeRepr;

    /// Map any value to its representent
    fn repr(&self) -> Self::Repr;
    /// The widget that will be used to change the value of self
    fn widget(&self) -> AttributeWidget<Self>;
    /// Map any value to a char that represents it
    fn char_repr(&self) -> AttributeDisplay;

    fn all_repr() -> &'static [Self::Repr] {
        Self::Repr::all_repr()
    }
}

pub enum AttributeDisplay {
    Icon(char),
    Text(String),
}

#[derive(Clone)]
pub enum AttributeWidget<E: OrganizerAttribute> {
    PickList { choices: &'static [E] },
    FlipButton { value_if_pressed: E },
}

#[derive(Default, Clone)]
pub(crate) struct AttributeDisplayer<A: OrganizerAttribute> {
    pick_list_state: pick_list::State<A>,
    button_state: button::State,
    being_modified: bool,
    widget: Option<AttributeWidget<A>>,
    attribute: Option<A>,
}

impl<A: OrganizerAttribute> AttributeDisplayer<A> {
    pub fn new() -> Self {
        Self {
            pick_list_state: Default::default(),
            button_state: Default::default(),
            being_modified: false,
            widget: None,
            attribute: None,
        }
    }

    pub fn update_attribute(&mut self, attribute: Option<A>) {
        self.update_widget(attribute.as_ref().map(|a| a.widget()));
        self.attribute = attribute;
    }

    pub fn update_widget(&mut self, widget: Option<AttributeWidget<A>>) {
        // If the widget is no longer a picklist, reset self.being_modified
        if let Some(AttributeWidget::PickList { .. }) = widget {
            ()
        } else {
            self.being_modified = false;
        }
        self.widget = widget;
    }

    pub fn view(&mut self) -> Option<Element<A>> {
        if let Some(widget) = self.widget.as_mut() {
            match widget {
                AttributeWidget::PickList { choices } => {
                    let mut picklist = PickList::new(
                        &mut self.pick_list_state,
                        *choices,
                        self.attribute.clone(),
                        |a| a,
                    )
                    .style(NoIcon {});
                    if let Some(AttributeDisplay::Icon(_)) =
                        self.attribute.as_ref().map(|a| a.char_repr())
                    {
                        picklist = picklist.font(super::ICONS).text_size(super::ICON_SIZE);
                    }
                    Some(picklist.into())
                }
                AttributeWidget::FlipButton { value_if_pressed } => {
                    let content = match self.attribute.as_ref().map(|a| a.char_repr()) {
                        Some(AttributeDisplay::Icon(c)) => super::icon(c),
                        Some(AttributeDisplay::Text(s)) => {
                            Text::new(s.clone()).size(super::ICON_SIZE)
                        }
                        _ => Text::new("???"),
                    };
                    Some(
                        Button::new(&mut self.button_state, content)
                            .on_press(value_if_pressed.clone())
                            .into(),
                    )
                }
            }
        } else {
            None
        }
    }
}

struct NoIcon {}

impl iced::pick_list::StyleSheet for NoIcon {
    fn menu(&self) -> iced::pick_list::Menu {
        Default::default()
    }

    fn active(&self) -> PickListStyle {
        PickListStyle {
            icon_size: 0.,
            ..Default::default()
        }
    }

    fn hovered(&self) -> PickListStyle {
        PickListStyle {
            icon_size: 0.,
            ..Default::default()
        }
    }
}
