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

use std::collections::HashMap;
use std::hash::Hash;

use iced_native::{text_input, Column, Element, Row, Text, TextInput};
use iced_wgpu::Renderer;

pub trait BuilderMessage: Clone + 'static {
    fn value_changed(kind: ValueKind, n: usize, value: String) -> Self;
    fn value_submitted(kind: ValueKind) -> Self;
}

use ultraviolet::{Rotor3, Vec3};

macro_rules! type_builder {
    ($builder_name:ident, $initializer:tt, $func:path, $($param: ident: $param_type: tt) , *) => {
        paste! {
            pub struct $builder_name {
                $(
                    $param: $param_type,
                    [<$param _string>]: String,
                    [<$param _input>]: text_input::State,
                )*
                    value_to_modify: ValueKind,
            }

            impl $builder_name {
                const PARAMETER_NAMES: &'static [&'static str] = &[$(stringify!($param),)*];
                pub fn new(value_to_modify: ValueKind, initial_value: $initializer) -> Self {
                    Self {
                        value_to_modify,
                        $(
                            $param: initial_value.$param,
                            [<$param _string>]: initial_value.$param.to_string(),
                            [<$param _input>]: Default::default(),
                        )*
                    }

                }
                fn update_str_value(&mut self, n: usize, value_str: String) {
                    let mut refs = [$(&mut self.[<$param _string>],)*];
                    if let Some(val) = refs.get_mut(n) {
                        **val = value_str;
                    }
                }

                fn view<'a ,Message: BuilderMessage>(&'a mut self) -> Element<'a, Message, Renderer> {
                    let str_values = [$(& self.[<$param _string>],)*];
                    let states = vec![$(&mut self.[<$param _input>],)*];
                    let mut ret = Column::new();
                    let value_to_modify = self.value_to_modify;
                    for (i, s) in states.into_iter().enumerate() {
                        let mut row = Row::new();
                        row = row.push(Text::new(Self::PARAMETER_NAMES[i]));
                        row = row.push(
                            TextInput::new(s, "", str_values[i], move |string| Message::value_changed(value_to_modify, i, string))
                            .on_submit(Message::value_submitted(value_to_modify))
                        );
                        ret = ret.push(row)
                    }
                    ret.into()
                }

                fn submit_value(&mut self) -> Option<$initializer> {
                    $(
                        let $param = self.[<$param _string>].parse::<$param_type>().ok()?;
                    )*
                        Some($func($($param),*))
                }
            }
        }
    }
}

type_builder!(Vec3Builder, Vec3, Vec3::new, x: f32, y: f32, z: f32);

#[derive(Clone, Copy, Debug)]
pub enum ValueKind {
    GridPosition,
    GridOrientation,
}

#[derive(Debug, Clone)]
pub enum InstanciatedValue {
    GridPosition(Vec3),
    GridOrientation(Rotor3),
}

pub enum GridPositionBuilder {
    Cartesian(Vec3Builder),
}

impl GridPositionBuilder {
    pub fn new_cartesian(position: Vec3) -> Self {
        Self::Cartesian(Vec3Builder::new(ValueKind::GridPosition, position))
    }

    fn view<'a, Message: BuilderMessage>(&'a mut self) -> Element<'a, Message, Renderer> {
        match self {
            Self::Cartesian(builder) => builder.view(),
        }
    }

    fn update_str_value(&mut self, n: usize, value_str: String) {
        match self {
            Self::Cartesian(builder) => builder.update_str_value(n, value_str),
        }
    }

    fn submit_value(&mut self) -> Option<InstanciatedValue> {
        match self {
            Self::Cartesian(builder) => builder.submit_value().map(InstanciatedValue::GridPosition),
        }
    }
}

pub struct GridBuilder {
    position_builder: GridPositionBuilder,
    //TODO add an orientation builder
}

impl<S: AppState> Builder<S> for GridBuilder {
    fn view<'a>(&'a mut self) -> Element<'a, super::Message<S>, Renderer> {
        let mut ret = Row::new();
        let position_builder_view = match &mut self.position_builder {
            GridPositionBuilder::Cartesian(builder) => builder.view(),
        };
        ret = ret.push(position_builder_view);
        ret.into()
    }

    fn update_str_value(&mut self, value_kind: ValueKind, n: usize, value_str: String) {
        match value_kind {
            ValueKind::GridPosition => self.position_builder.update_str_value(n, value_str),
            ValueKind::GridOrientation => todo!(),
        }
    }

    fn submit_value(&mut self, value_kind: ValueKind) -> Option<InstanciatedValue> {
        match value_kind {
            ValueKind::GridPosition => self.position_builder.submit_value(),
            ValueKind::GridOrientation => todo!(),
        }
    }
}

use super::AppState;

pub trait Builder<S: AppState> {
    fn view<'a>(&'a mut self) -> Element<'a, super::Message<S>, Renderer>;
    fn update_str_value(&mut self, value_kind: ValueKind, n: usize, value_str: String);
    fn submit_value(&mut self, value_kind: ValueKind) -> Option<InstanciatedValue>;
}
