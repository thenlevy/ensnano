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

use super::UiSize;

use iced_native::{text_input, Column, Element, Row, Text, TextInput};
use iced_wgpu::Renderer;

pub trait BuilderMessage: Clone + 'static {
    fn value_changed(kind: ValueKind, n: usize, value: String) -> Self;
    fn value_submitted(kind: ValueKind) -> Self;
}

use ultraviolet::{Bivec3, Mat3, Rotor3, Vec3};

macro_rules! type_builder {
    ($builder_name:ident, $initializer:tt, $internal:tt, $convert_in:path, $convert_out:path, $($param: ident: $param_type: tt %$formatter:path) , *) => {
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
                            [<$param _string>]: $formatter::fmt(&initial.$param),
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
                    let mut ret = Column::new().width(iced::Length::Fill).align_items(iced::Alignment::End);
                    let value_to_modify = self.value_to_modify;
                    for (i, s) in states.into_iter().enumerate() {
                        let mut row = Row::new().width(iced::Length::Fill);
                        row = row.push(Text::new(Self::PARAMETER_NAMES[i]));
                        row = row.push(iced::Space::with_width(iced::Length::Units(5)));
                        row = row.push(
                            TextInput::new(s, "", str_values[i], move |string| Message::value_changed(value_to_modify, i, string))
                            .on_submit(Message::value_submitted(value_to_modify))
                            .width(iced::Length::Units(50))
                        );
                        ret = ret.push(row)
                    }
                    ret.into()
                }

                fn submit_value(&mut self) -> Option<$initializer> {
                    $(
                        let $param = $formatter::parse(&self.[<$param _string>])?;
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

struct DegreeAngleFormater;

impl DegreeAngleFormater {
    fn fmt(angle: &f32) -> String {
        format!("{:.1}°", angle.to_degrees())
    }

    fn parse(angle_str: &str) -> Option<f32> {
        angle_str
            .trim_end_matches("°")
            .parse::<f32>()
            .ok()
            .map(f32::to_radians)
    }
}

struct FloatFormatter;

impl FloatFormatter {
    fn fmt(float: &f32) -> String {
        format!("{:.2}", float)
    }

    fn parse(float_str: &str) -> Option<f32> {
        float_str.parse::<f32>().ok()
    }
}

type_builder!(
    Vec3Builder,
    Vec3,
    Vec3,
    std::convert::identity,
    std::convert::identity,
    x: f32 % FloatFormatter,
    y: f32 % FloatFormatter,
    z: f32 % FloatFormatter
);

type_builder!(
    DirectionAngleBuilder,
    Rotor3,
    DirectionAngle,
    DirectionAngle::from_rotor,
    DirectionAngle::to_rotor,
    x: f32 % FloatFormatter,
    y: f32 % FloatFormatter,
    z: f32 % FloatFormatter,
    angle: f32 % DegreeAngleFormater
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
    fn view<'a>(&'a mut self, ui_size: UiSize) -> Element<'a, super::Message<S>, Renderer> {
        let mut ret = Column::new().width(iced::Length::Fill);
        let position_builder_view = self.position_builder.view();
        let orientation_builder_view = self.orientation_builder.view();
        ret = ret.push(Text::new("Position").size(ui_size.intermediate_text()));
        ret = ret.push(position_builder_view);
        ret = ret.push(Text::new("Orientation").size(ui_size.intermediate_text()));
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
    fn view<'a>(&'a mut self, ui_size: UiSize) -> Element<'a, super::Message<S>, Renderer>;
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
        let direction = Vec3::unit_x().rotated_by(rotor);
        log::info!("direction {:?}", direction);

        let real_z = Self::real_z(direction);
        log::info!("real z {:?}", real_z);
        let real_y = real_z.cross(direction);
        log::info!("real y {:?}", real_y);

        let cos_angle = Vec3::unit_z().rotated_by(rotor).dot(real_z);
        let sin_angle = -Vec3::unit_z().rotated_by(rotor).dot(real_y);
        log::info!("cos = {}, sin = {}", cos_angle, sin_angle);
        let angle = sin_angle.atan2(cos_angle);

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

        let angle = self.angle;
        let real_z = Self::real_z(direction);
        log::info!("real z {:?}", real_z);
        let z = real_z.rotated_by(Rotor3::from_angle_plane(
            angle,
            Bivec3::from_normalized_axis(direction),
        ));
        let y = z.cross(direction);
        log::info!(" x {:?}", direction);
        log::info!(" y {:?}", y);
        log::info!(" z {:?}", real_z);

        Mat3::new(direction, y, z).into_rotor3()
    }

    fn real_z(direction: Vec3) -> Vec3 {
        let z_angle = direction.y.asin();
        log::info!("z angle {}", z_angle.to_degrees());

        if direction.y.abs() < 1. - Self::CONVERSION_ESPILON {
            let radius = z_angle.cos();
            log::info!("radius {}", radius);
            log::info!("direction.x / radius {}", direction.x / radius);
            let y_angle = if direction.z > 0. {
                -(direction.x / radius).min(1.).max(-1.).acos()
            } else {
                (direction.x / radius).min(1.).max(-1.).acos()
            };
            log::info!("y angle {}", y_angle.to_degrees());

            Vec3::unit_z().rotated_by(Rotor3::from_angle_plane(
                y_angle,
                Bivec3::from_normalized_axis(Vec3::unit_y()),
            ))
        } else {
            Vec3::unit_z()
        }
    }
}
