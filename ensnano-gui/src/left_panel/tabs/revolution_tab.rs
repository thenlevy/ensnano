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
use ensnano_interactor::{RevolutionSurfaceDescriptor, RevolutionSurfaceSystemDescriptor};
use iced_native::widget::{
    button::{self, Button},
    pick_list::{self, PickList},
    scrollable::{self, Scrollable},
    text_input::{self, TextInput},
};

#[derive(Debug, Clone, Copy)]
pub enum ParameterKind {
    Float,
    Int,
    Uint,
}

#[derive(Debug, Clone, Copy)]
pub enum InstanciatedParameter {
    Float(f64),
    Int(isize),
    Uint(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum RevolutionParameterId {
    SectionParameter(usize),
    HalfTurnCount,
    RevolutionRadius,
    NbHelixHalfSection,
    ShiftPerTurn,
    NbSectionPerSegment,
    ScaffoldLenTarget,
}

impl InstanciatedParameter {
    pub fn get_float(self) -> Option<f64> {
        if let Self::Float(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn get_int(self) -> Option<isize> {
        if let Self::Int(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn get_uint(self) -> Option<usize> {
        if let Self::Uint(x) = self {
            Some(x)
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

struct ParameterWidget {
    current_text: String,
    state: text_input::State,
    parameter_kind: ParameterKind,
}

impl ParameterWidget {
    fn new(initial_value: InstanciatedParameter) -> Self {
        let (current_text, parameter_kind) = match initial_value {
            InstanciatedParameter::Float(x) => (format!("{:.3}", x), ParameterKind::Float),
            InstanciatedParameter::Int(x) => (x.to_string(), ParameterKind::Int),
            InstanciatedParameter::Uint(x) => (x.to_string(), ParameterKind::Uint),
        };
        Self {
            current_text,
            state: Default::default(),
            parameter_kind,
        }
    }

    fn input_view<S: AppState>(&mut self, id: RevolutionParameterId) -> Element<Message<S>> {
        let style = super::BadValue(self.contains_valid_input());
        TextInput::new(&mut self.state, "", &self.current_text, move |s| {
            Message::RevolutionParameterUpdate {
                parameter_id: id,
                text: s,
            }
        })
        .style(style)
        .into()
    }

    fn set_text(&mut self, text: String) {
        self.current_text = text;
    }

    fn has_keyboard_priority(&self) -> bool {
        self.state.is_focused()
    }

    fn contains_valid_input(&self) -> bool {
        self.get_value().is_some()
    }

    fn get_value(&self) -> Option<InstanciatedParameter> {
        match self.parameter_kind {
            ParameterKind::Float => self
                .current_text
                .parse::<f64>()
                .ok()
                .map(InstanciatedParameter::Float),
            ParameterKind::Int => self
                .current_text
                .parse::<isize>()
                .ok()
                .map(InstanciatedParameter::Int),
            ParameterKind::Uint => self
                .current_text
                .parse::<usize>()
                .ok()
                .map(InstanciatedParameter::Uint),
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
            .map(|builder| (builder.name, ParameterWidget::new(builder.default_value)))
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
                    let row = Row::new().push(Text::new(param.0)).push(
                        param
                            .1
                            .input_view(RevolutionParameterId::SectionParameter(param_id)),
                    );
                    col.push(row)
                });
        column.into()
    }

    fn update_builder_parameter(&mut self, param_id: usize, text: String) {
        if let Some(p) = self.parameters.get_mut(param_id) {
            p.1.set_text(text)
        }
    }

    fn has_keyboard_priority(&self) -> bool {
        self.parameters
            .iter()
            .any(|(_, p)| p.has_keyboard_priority())
    }

    fn instanciated_parameters(&self) -> Vec<InstanciatedParameter> {
        self.parameters
            .iter()
            .filter_map(|p| p.1.get_value())
            .collect()
    }

    fn build_curve(&self) -> Option<CurveDescriptor2D> {
        (self.builder.build)(&self.instanciated_parameters())
    }
}

pub(crate) struct RevolutionTab {
    curve_descriptor_widget: Option<CurveDescriptorWidget>,
    pick_curve_state: pick_list::State<CurveDescriptorBuilder>,
    half_turn_count: ParameterWidget,
    radius_input: ParameterWidget,
    nb_helix_per_half_section_input: ParameterWidget,
    shift_per_turn_state_input: ParameterWidget,

    nb_section_per_segment_input: ParameterWidget,
    scaffold_len_target: ParameterWidget,

    scroll_state: scrollable::State,

    go_button: button::State,
    abbort_button: button::State,
    finish_button: button::State,
}

impl Default for RevolutionTab {
    fn default() -> Self {
        Self {
            scroll_state: Default::default(),
            curve_descriptor_widget: None,
            pick_curve_state: Default::default(),
            half_turn_count: ParameterWidget::new(InstanciatedParameter::Int(0)),
            radius_input: ParameterWidget::new(InstanciatedParameter::Float(0.)),
            nb_helix_per_half_section_input: ParameterWidget::new(InstanciatedParameter::Uint(1)),
            shift_per_turn_state_input: ParameterWidget::new(InstanciatedParameter::Int(0)),
            nb_section_per_segment_input: ParameterWidget::new(InstanciatedParameter::Uint(100)),
            scaffold_len_target: ParameterWidget::new(InstanciatedParameter::Uint(7249)),
            go_button: Default::default(),
            abbort_button: Default::default(),
            finish_button: Default::default(),
        }
    }
}

impl RevolutionTab {
    pub fn set_builder(&mut self, builder: CurveDescriptorBuilder) {
        if self.curve_descriptor_widget.as_ref().map(|w| w.curve_name) != Some(builder.curve_name) {
            self.curve_descriptor_widget = Some(CurveDescriptorWidget::new(builder))
        }
    }

    pub fn update_builder_parameter(&mut self, param_id: RevolutionParameterId, text: String) {
        match param_id {
            RevolutionParameterId::SectionParameter(id) => {
                if let Some(widget) = self.curve_descriptor_widget.as_mut() {
                    widget.update_builder_parameter(id, text)
                }
            }
            param => {
                let widget = match param {
                    RevolutionParameterId::SectionParameter(_) => unreachable!(),
                    RevolutionParameterId::HalfTurnCount => &mut self.half_turn_count,
                    RevolutionParameterId::ShiftPerTurn => &mut self.shift_per_turn_state_input,
                    RevolutionParameterId::RevolutionRadius => &mut self.radius_input,
                    RevolutionParameterId::ScaffoldLenTarget => &mut self.scaffold_len_target,
                    RevolutionParameterId::NbHelixHalfSection => {
                        &mut self.nb_helix_per_half_section_input
                    }
                    RevolutionParameterId::NbSectionPerSegment => {
                        &mut self.nb_section_per_segment_input
                    }
                };
                widget.set_text(text);
            }
        }
    }

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let desc = self.get_revolution_system(app_state);
        let mut ret = Scrollable::new(&mut self.scroll_state);
        section!(ret, ui_size, "Revolution Surfaces");

        subsection!(ret, ui_size, "Section parameters");
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

        extra_jump!(ret);
        subsection!(ret, ui_size, "Revolution parameter");

        ret = ret.push(
            Row::new().push(Text::new("Nb Half Turns")).push(
                self.half_turn_count
                    .input_view(RevolutionParameterId::HalfTurnCount),
            ),
        );
        ret = ret.push(
            Row::new()
                .push(Text::new("Nb Helix per Half section"))
                .push(
                    self.nb_helix_per_half_section_input
                        .input_view(RevolutionParameterId::NbHelixHalfSection),
                ),
        );
        ret = ret.push(
            Row::new().push(Text::new("Shift per turn")).push(
                self.shift_per_turn_state_input
                    .input_view(RevolutionParameterId::ShiftPerTurn),
            ),
        );
        ret = ret.push(
            Row::new().push(Text::new("Revolution Radius")).push(
                self.radius_input
                    .input_view(RevolutionParameterId::RevolutionRadius),
            ),
        );

        extra_jump!(ret);
        subsection!(ret, ui_size, "Discretisation paramters");
        ret = ret.push(
            Row::new().push(Text::new("Nb section per segments")).push(
                self.nb_section_per_segment_input
                    .input_view(RevolutionParameterId::NbSectionPerSegment),
            ),
        );
        ret = ret.push(
            Row::new().push(Text::new("Targent length")).push(
                self.scaffold_len_target
                    .input_view(RevolutionParameterId::ScaffoldLenTarget),
            ),
        );

        extra_jump!(ret);
        section!(ret, ui_size, "Relaxation computation");
        if let SimulationState::Relaxing = app_state.get_simulation_state() {
            let button_abbort = Button::new(&mut self.abbort_button, Text::new("Abort"))
                .on_press(Message::StopSimulation);
            ret = ret.push(button_abbort);
            extra_jump!(2, ret);
            if let Some(len) = app_state.get_reader().get_current_length_of_relaxed_shape() {
                ret = ret.push(Text::new(format!("Current total length: {len}")));
            }
            let button_relaxation = Button::new(&mut self.finish_button, Text::new("Finish"))
                .on_press(Message::FinishRelaxation);
            ret = ret.push(button_relaxation);
        } else {
            let mut button = Button::new(&mut self.go_button, Text::new("Start"));
            if let SimulationState::None = app_state.get_simulation_state() {
                if let Some(desc) = desc {
                    button = button.on_press(Message::InitRevolutionRelaxation(desc));
                }
            }
            ret = ret.push(button);
        }
        ret.into()
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.curve_descriptor_widget
            .as_ref()
            .map(CurveDescriptorWidget::has_keyboard_priority)
            .unwrap_or(false)
            || self.radius_input.has_keyboard_priority()
            || self.nb_section_per_segment_input.has_keyboard_priority()
            || self.half_turn_count.has_keyboard_priority()
            || self.nb_helix_per_half_section_input.has_keyboard_priority()
            || self.scaffold_len_target.has_keyboard_priority()
            || self.shift_per_turn_state_input.has_keyboard_priority()
    }

    fn get_revolution_system<S: AppState>(
        &self,
        app_state: &S,
    ) -> Option<RevolutionSurfaceSystemDescriptor> {
        let surface_descriptor = RevolutionSurfaceDescriptor {
            dna_paramters: app_state.get_dna_parameters(),
            curve: self
                .curve_descriptor_widget
                .as_ref()
                .and_then(|w| w.build_curve())?,
            half_turns_count: self
                .half_turn_count
                .get_value()
                .and_then(InstanciatedParameter::get_int)?,
            nb_helix_per_half_section: self
                .nb_helix_per_half_section_input
                .get_value()
                .and_then(InstanciatedParameter::get_uint)?,
            revolution_radius: self
                .radius_input
                .get_value()
                .and_then(InstanciatedParameter::get_float)?,
            shift_per_turn: self
                .shift_per_turn_state_input
                .get_value()
                .and_then(InstanciatedParameter::get_int)?,
            junction_smoothening: 0.,
        };

        let system = RevolutionSurfaceSystemDescriptor {
            target: surface_descriptor,
            nb_section_per_segment: self
                .nb_section_per_segment_input
                .get_value()
                .and_then(InstanciatedParameter::get_uint)?,
            dna_parameters: app_state.get_dna_parameters(),
            scaffold_len_target: self
                .scaffold_len_target
                .get_value()
                .and_then(InstanciatedParameter::get_uint)?,
        };

        Some(system)
    }
}
