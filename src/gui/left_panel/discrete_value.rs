use super::{button, slider, Button, Element, HelixRoll, Row, Slider, Text};

use super::Message;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValueId(pub usize);

pub trait Requestable {
    type Request;
    fn request_from_values(&self, values: &[f32]) -> Self::Request;
    fn nb_values(&self) -> usize;
    fn initial_value(&self, n: usize) -> f32;
    fn min_val(&self, n: usize) -> f32;
    fn max_val(&self, n: usize) -> f32;
    fn step_val(&self, n: usize) -> f32;
    fn name_val(&self, n: usize) -> String;

    fn make_request(&self, values: &[f32], request: &mut Option<Self::Request>) {
        *request = Some(self.request_from_values(values))
    }

    fn hidden(&self, _: usize) -> bool {
        false
    }
}

pub struct RequestFactory<R: Requestable> {
    values: BTreeMap<ValueId, DiscreteValue>,
    pub requestable: R,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum FactoryId {
    HelixRoll,
    Hyperboloid,
    Scroll,
    RigidBody,
    Brownian,
}

impl<R: Requestable> RequestFactory<R> {
    pub fn new(factory_id: FactoryId, requestable: R) -> Self {
        let mut values = BTreeMap::new();
        for id in 0..requestable.nb_values() {
            let default = requestable.initial_value(id);
            let min_val = requestable.min_val(id);
            let max_val = requestable.max_val(id);
            let step_val = requestable.step_val(id);
            let name = requestable.name_val(id);
            values.insert(
                ValueId(id),
                DiscreteValue::new(
                    default,
                    step_val,
                    min_val,
                    max_val,
                    name,
                    factory_id,
                    ValueId(id),
                    requestable.hidden(id),
                ),
            );
        }
        Self {
            values,
            requestable,
        }
    }

    pub fn view(&mut self, active: bool) -> Vec<Element<Message>> {
        self.values
            .values_mut()
            .filter(|v| !v.hidden)
            .map(|v| v.view(active))
            .collect()
    }

    pub fn update_request(
        &mut self,
        value_id: ValueId,
        new_val: f32,
        request: &mut Option<R::Request>,
    ) {
        self.values
            .get_mut(&value_id)
            .unwrap()
            .update_value(new_val);
        let values: Vec<f32> = self.values.values().map(|v| v.get_value()).collect();
        self.requestable.make_request(&values, request)
    }

    pub fn update_value(&mut self, value_id: ValueId, new_val: f32) -> R::Request {
        self.values
            .get_mut(&value_id)
            .unwrap()
            .update_value(new_val);
        let values: Vec<f32> = self.values.values().map(|v| v.get_value()).collect();
        self.requestable.request_from_values(&values)
    }

    pub fn make_request(&self, request: &mut Option<R::Request>) {
        let values: Vec<f32> = self.values.values().map(|v| v.get_value()).collect();
        self.requestable.make_request(&values, request)
    }
}

struct DiscreteValue {
    value: f32,
    step: f32,
    min_val: f32,
    max_val: f32,
    name: String,
    owner_id: FactoryId,
    value_id: ValueId,
    incr_button: button::State,
    decr_button: button::State,
    slider: slider::State,
    hidden: bool,
}

impl DiscreteValue {
    fn new(
        default: f32,
        step: f32,
        min_val: f32,
        max_val: f32,
        name: String,
        owner_id: FactoryId,
        value_id: ValueId,
        hidden: bool,
    ) -> Self {
        Self {
            value: default,
            step,
            min_val,
            max_val,
            name,
            owner_id,
            value_id,
            incr_button: Default::default(),
            decr_button: Default::default(),
            slider: Default::default(),
            hidden,
        }
    }

    fn view(&mut self, active: bool) -> Element<Message> {
        let decr_button = if active && self.value - self.step > self.min_val {
            Button::new(&mut self.decr_button, Text::new("-")).on_press(Message::DescreteValue {
                factory_id: self.owner_id,
                value_id: self.value_id,
                value: self.value - self.step,
            })
        } else {
            Button::new(&mut self.decr_button, Text::new("-"))
        };
        let incr_button = if active && self.value + self.step < self.max_val {
            Button::new(&mut self.incr_button, Text::new("+")).on_press(Message::DescreteValue {
                factory_id: self.owner_id,
                value_id: self.value_id,
                value: self.value + self.step,
            })
        } else {
            Button::new(&mut self.incr_button, Text::new("+"))
        };
        let factory_id = self.owner_id.clone();
        let value_id = self.value_id.clone();
        let slider = if active {
            Slider::new(
                &mut self.slider,
                self.min_val..=self.max_val,
                self.value,
                move |value| Message::DescreteValue {
                    factory_id,
                    value_id,
                    value,
                },
            )
            .step(self.step)
        } else {
            Slider::new(
                &mut self.slider,
                self.min_val..=self.max_val,
                self.value,
                |_| Message::Nothing,
            )
        };

        let mut name_text = Text::new(self.name.clone());

        if !active {
            name_text = name_text.color([0.6, 0.6, 0.6]);
        }

        let left = Row::new()
            .push(name_text)
            .push(iced::Space::with_width(iced::Length::Fill))
            .align_items(iced::Align::Center)
            .width(iced::Length::FillPortion(4));

        let middle = Row::new()
            .push(Text::new(format!("{:.1}", self.value)))
            .width(iced::Length::FillPortion(1));
        let right = Row::new()
            .push(decr_button)
            .push(incr_button)
            .push(iced::Space::with_width(iced::Length::Units(2)))
            .push(slider)
            .width(iced::Length::FillPortion(5));

        Row::new()
            .push(left)
            .push(middle)
            .push(right)
            .align_items(iced::Align::Center)
            .into()
    }

    fn get_value(&self) -> f32 {
        self.value
    }

    fn update_value(&mut self, new_val: f32) {
        self.value = new_val
    }
}

impl RequestFactory<HelixRoll> {
    pub fn update_roll(&mut self, roll: f32) {
        self.values.get_mut(&ValueId(0)).unwrap().update_value(roll);
    }
}
