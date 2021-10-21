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

trait BuilderMessage: Clone + 'static {
    fn value_changed(n: usize, value: String) -> Self;
}

macro_rules! type_builder {
    ($builder_name:ident, $($param: ident: $param_type: tt) , *) => {
        paste! {
            struct $builder_name {
                $(
                    $param: $param_type,
                    [<$param _string>]: String,
                    [<$param _input>]: text_input::State,
                )*
            }

            impl $builder_name {
                const PARAMETER_NAMES: &'static [&'static str] = &[$(stringify!($param),)*];
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
                    for (i, s) in states.into_iter().enumerate() {
                        let mut row = Row::new();
                        row = row.push(Text::new(Self::PARAMETER_NAMES[i]));
                        row = row.push(TextInput::new(s, "", str_values[i], move |string| Message::value_changed(i, string)));
                        ret = ret.push(row)
                    }
                    ret.into()
                }
            }
        }
    }
}

type_builder!(Vec3Builder, x: f32, y: f32, z: f32);
