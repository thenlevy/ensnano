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

#[derive(Debug, Clone, Copy)]
pub enum InstanciatedParameter {
    Float(f64),
}

impl InstanciatedParameter {
    pub fn get_float(&self) -> Option<f64> {
        #[allow(irrefutable_let_patterns)]
        if let Self::Float(x) = self {
            Some(*x)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurveDescriptorParameter {
    pub name: &'static str,
    pub kind: ParameterKind,
    pub default_value: InstanciatedParameter,
}

#[derive(Clone)]
pub struct CurveDescriptorBuilder {
    pub nb_parameters: usize,
    pub curve_name: &'static str,
    pub parameters: &'static [CurveDescriptorParameter],
    pub build:
        &'static (dyn Fn(&[InstanciatedParameter]) -> Option<CurveDescriptor2D> + Send + Sync),
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

impl ParameterWidget {
    fn new_float(initial_value: f64) -> Self {
        Self::Float {
            current_text: format!("{:.3}", initial_value),
            state: Default::default(),
        }
    }

    fn input_view<S: AppState>(&mut self, id: usize) -> Element<Message<S>> {
        match self {
            Self::Float {
                current_text,
                state,
            } => TextInput::new(state, "", current_text, move |s| {
                Message::CurveBuilderParameterUpdate {
                    parameter_id: id,
                    text: s,
                }
            })
            .into(),
        }
    }

    fn set_text(&mut self, text: String) {
        match self {
            Self::Float { current_text, .. } => *current_text = text,
        }
    }

    fn has_keyboard_priority(&self) -> bool {
        match self {
            Self::Float { state, .. } => state.is_focused(),
        }
    }
}

struct CurveDescriptorWidget {
    parameters: Vec<(&'static str, ParameterWidget)>,
    curve_name: &'static str,
    builder: CurveDescriptorBuilder,
}

impl CurveDescriptorWidget {
    fn new(builder: CurveDescriptorBuilder) -> Self {
        let parameters = builder
            .parameters
            .iter()
            .map(|builder| match builder.default_value {
                InstanciatedParameter::Float(x) => (builder.name, ParameterWidget::new_float(x)),
            })
            .collect();

        Self {
            parameters,
            curve_name: builder.curve_name,
            builder,
        }
    }

    fn view<'a, S: AppState>(&'a mut self) -> Element<'a, Message<S>> {
        let column: Column<'a, Message<S>> =
            self.parameters
                .iter_mut()
                .enumerate()
                .fold(Column::new(), |col, (param_id, param)| {
                    let row = Row::new()
                        .push(Text::new(param.0))
                        .push(param.1.input_view(param_id));
                    col.push(row)
                });
        column.into()
    }

    fn update_builder_parameter(&mut self, param_id: usize, text: String) {
        self.parameters
            .get_mut(param_id)
            .map(|p| p.1.set_text(text));
    }

    fn has_keyboard_priority(&self) -> bool {
        self.parameters
            .iter()
            .any(|(_, p)| p.has_keyboard_priority())
    }
}

#[derive(Default)]
pub(crate) struct RevolutionTab {
    curve_descriptor_widget: Option<CurveDescriptorWidget>,
    pick_curve_state: pick_list::State<CurveDescriptorBuilder>,
}

impl RevolutionTab {
    pub fn set_builder(&mut self, builder: CurveDescriptorBuilder) {
        if self.curve_descriptor_widget.as_ref().map(|w| w.curve_name) != Some(builder.curve_name) {
            self.curve_descriptor_widget = Some(CurveDescriptorWidget::new(builder))
        }
    }

    pub fn update_builder_parameter(&mut self, param_id: usize, text: String) {
        self.curve_descriptor_widget
            .as_mut()
            .map(|widget| widget.update_builder_parameter(param_id, text));
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
            self.curve_descriptor_widget
                .as_ref()
                .map(|w| w.builder.clone()),
            |curve| Message::CurveBuilderPicked(curve),
        )
        .placeholder("Pick..");

        let pick_curve_row = Row::new()
            .push(Text::new("Curve type"))
            .push(curve_pick_list);

        ret = ret.push(pick_curve_row);

        if let Some(widget) = self.curve_descriptor_widget.as_mut() {
            ret = ret.push(widget.view())
        }

        ret.into()
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.curve_descriptor_widget
            .as_ref()
            .map(CurveDescriptorWidget::has_keyboard_priority)
            .unwrap_or(false)
    }
}
