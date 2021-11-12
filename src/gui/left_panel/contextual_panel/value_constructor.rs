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

use ultraviolet::{Bivec3, Mat3, Rotor3, Vec3};

macro_rules! type_builder {
    ($builder_name:ident, $initializer:tt, $internal:tt, $convert_in:path, $convert_out:path, $($param: ident: $param_type: tt) , *) => {
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
                    let initial: $internal = $convert_in(initial_value);
                    Self {
                        value_to_modify,
                        $(
                            $param: initial.$param,
                            [<$param _string>]: initial.$param.to_string(),
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
                    let out: $internal = $internal {
                        $(
                            $param,
                        )*
                    };

                    Some($convert_out(out))
                }

                fn has_keyboard_priority(&self) -> bool {
                    let states = [$(&self.[<$param _input>],)*];
                    states.iter().any(|s| s.is_focused())
                }
            }
        }
    }
}

type_builder!(
    Vec3Builder,
    Vec3,
    Vec3,
    std::convert::identity,
    std::convert::identity,
    x: f32,
    y: f32,
    z: f32
);

type_builder!(
    DirectionAngleBuilder,
    Rotor3,
    DirectionAngle,
    DirectionAngle::from_rotor,
    DirectionAngle::to_rotor,
    x: f32,
    y: f32,
    z: f32,
    angle: f32
);

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

    fn has_keyboard_priority(&self) -> bool {
        match self {
            Self::Cartesian(b) => b.has_keyboard_priority(),
        }
    }
}

pub enum GridOrientationBuilder {
    DirectionAngle(DirectionAngleBuilder),
}

impl GridOrientationBuilder {
    pub fn new_direction_angle(orientation: Rotor3) -> Self {
        Self::DirectionAngle(DirectionAngleBuilder::new(
            ValueKind::GridOrientation,
            orientation,
        ))
    }

    fn view<'a, Message: BuilderMessage>(&'a mut self) -> Element<'a, Message, Renderer> {
        match self {
            Self::DirectionAngle(builder) => builder.view(),
        }
    }

    fn update_str_value(&mut self, n: usize, value_str: String) {
        match self {
            Self::DirectionAngle(builder) => builder.update_str_value(n, value_str),
        }
    }

    fn submit_value(&mut self) -> Option<InstanciatedValue> {
        match self {
            Self::DirectionAngle(builder) => builder
                .submit_value()
                .map(InstanciatedValue::GridOrientation),
        }
    }

    fn has_keyboard_priority(&self) -> bool {
        match self {
            Self::DirectionAngle(b) => b.has_keyboard_priority(),
        }
    }
}

pub struct GridBuilder {
    position_builder: GridPositionBuilder,
    orientation_builder: GridOrientationBuilder,
}

impl GridBuilder {
    pub fn new(position: Vec3, orientation: Rotor3) -> Self {
        Self {
            position_builder: GridPositionBuilder::new_cartesian(position),
            orientation_builder: GridOrientationBuilder::new_direction_angle(orientation),
        }
    }
}

impl<S: AppState> Builder<S> for GridBuilder {
    fn view<'a>(&'a mut self) -> Element<'a, super::Message<S>, Renderer> {
        let mut ret = Column::new();
        let position_builder_view = self.position_builder.view();
        let orientation_builder_view = self.orientation_builder.view();
        ret = ret.push(position_builder_view);
        ret = ret.push(orientation_builder_view);
        ret.into()
    }

    fn update_str_value(&mut self, value_kind: ValueKind, n: usize, value_str: String) {
        match value_kind {
            ValueKind::GridPosition => self.position_builder.update_str_value(n, value_str),
            ValueKind::GridOrientation => self.orientation_builder.update_str_value(n, value_str),
        }
    }

    fn submit_value(&mut self, value_kind: ValueKind) -> Option<InstanciatedValue> {
        match value_kind {
            ValueKind::GridPosition => self.position_builder.submit_value(),
            ValueKind::GridOrientation => self.orientation_builder.submit_value(),
        }
    }

    fn has_keyboard_priority(&self) -> bool {
        self.position_builder.has_keyboard_priority()
            || self.orientation_builder.has_keyboard_priority()
    }
}

use super::AppState;

pub trait Builder<S: AppState> {
    fn view<'a>(&'a mut self) -> Element<'a, super::Message<S>, Renderer>;
    fn update_str_value(&mut self, value_kind: ValueKind, n: usize, value_str: String);
    fn submit_value(&mut self, value_kind: ValueKind) -> Option<InstanciatedValue>;
    fn has_keyboard_priority(&self) -> bool;
}

#[derive(Debug, Clone, Copy)]
struct DirectionAngle {
    x: f32,
    y: f32,
    z: f32,
    angle: f32,
}

impl DirectionAngle {
    const CONVERSION_ESPILON: f32 = 1e-6;

    fn from_rotor(rotor: Rotor3) -> Self {
        let direction = Vec3::unit_z().rotated_by(rotor);

        let real_x = Self::real_x(direction);
        let real_y = direction.cross(-real_x);

        let cos_angle = Vec3::unit_x().rotated_by(rotor).dot(real_x);
        let sin_angle = Vec3::unit_x().rotated_by(rotor).dot(real_y);
        let angle = sin_angle.atan2(cos_angle).to_degrees();

        Self {
            x: direction.x,
            y: direction.y,
            z: direction.z,
            angle,
        }
    }

    fn to_rotor(self) -> Rotor3 {
        let direction = Vec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
        .normalized();

        let angle = self.angle.to_radians();
        let real_x = Self::real_x(direction);
        let real_y = direction.cross(-real_x);
        let x = real_x.rotated_by(Rotor3::from_angle_plane(
            angle,
            Bivec3::from_normalized_axis(real_y),
        ));
        let y = direction.cross(-x);
        Mat3::new(x, y, direction).into_rotor3()
    }

    fn real_x(direction: Vec3) -> Vec3 {
        let phi = direction.y.asin();

        if direction.y.abs() < 1. - Self::CONVERSION_ESPILON {
            let radius = phi.cos();
            let theta = if direction.x > 0. {
                (direction.z / radius).acos()
            } else {
                -(direction.z / radius).acos()
            };

            Vec3::unit_x()
                .rotated_by(Rotor3::from_angle_plane(
                    phi,
                    Bivec3::from_normalized_axis(Vec3::unit_z()),
                ))
                .rotated_by(Rotor3::from_angle_plane(
                    theta,
                    Bivec3::from_normalized_axis(Vec3::unit_y()),
                ))
        } else {
            Vec3::unit_x()
        }
    }
}
