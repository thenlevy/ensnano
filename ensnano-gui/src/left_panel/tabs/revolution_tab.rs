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

use super::*;
use ensnano_design::CurveDescriptor2D;
use iced_native::widget::{
    pick_list::{self, PickList},
    text_input::{self, TextInput},
};

#[derive(Debug, Clone, Copy)]
pub enum ParameterKind {
    Float,
}

pub enum InstanciatedParameters {
    Float(f64),
}

#[derive(Debug, Clone)]
pub struct CurveDescriptorParameter {
    name: &'static str,
    kind: ParameterKind,
}

#[derive(Clone)]
pub struct CurveDescriptorBuilder {
    nb_parameters: usize,
    curve_name: &'static str,
    parameters: &'static [CurveDescriptorParameter],
    build: &'static (dyn Fn(&[InstanciatedParameters]) -> CurveDescriptor2D + Send + Sync),
}

use std::fmt;
impl fmt::Debug for CurveDescriptorBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CurveDecriptorBuilder")
            .field("curve_name", &self.curve_name)
            .finish()
    }
}

impl ToString for CurveDescriptorBuilder {
    fn to_string(&self) -> String {
        self.curve_name.to_string()
    }
}

impl PartialEq for CurveDescriptorBuilder {
    fn eq(&self, other: &Self) -> bool {
        self.curve_name == other.curve_name
    }
}

impl Eq for CurveDescriptorBuilder {}

enum ParameterWidget {
    Float {
        current_text: String,
        state: text_input::State,
    },
}

struct CurveDescriptorWidget {
    parameters: Vec<ParameterWidget>,
}

impl CurveDescriptorWidget {
    fn view<'a, S: AppState>(&'a mut self) -> Element<'a, Message<S>> {
        todo!()
    }
}

#[derive(Default)]
pub(crate) struct RevolutionTab {
    curve_descriptor_widget: Option<CurveDescriptorWidget>,
    pick_curve_state: pick_list::State<CurveDescriptorBuilder>,
}

impl RevolutionTab {
    pub fn set_builder(&mut self, builder: CurveDescriptorBuilder) {
        println!("set {}", builder.to_string());
    }

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new();
        section!(ret, ui_size, "Revolution Surfaces");

        let curve_pick_list = PickList::new(
            &mut self.pick_curve_state,
            S::POSSIBLE_CURVES,
            None,
            |curve| Message::CurveBuilderPicked(curve),
        );

        let mut pick_curve_row = Row::new()
            .push(Text::new("Curve type"))
            .push(curve_pick_list);

        ret = ret.push(pick_curve_row);

        ret.into()
    }
}
