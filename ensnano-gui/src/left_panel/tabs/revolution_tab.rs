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
use ensnano_design::{ultraviolet::Rotor3, CurveDescriptor2D};
use ensnano_interactor::{
    EquadiffSolvingMethod, RevolutionSimulationParameters, RevolutionSurfaceRadius,
    RevolutionSurfaceSystemDescriptor, RootingParameters, ShiftGenerator,
    UnrootedRevolutionSurfaceDescriptor,
};
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
    NbSpiral,
    NbSectionPerSegment,
    ScaffoldLenTarget,
    SpringStiffness,
    TorsionStiffness,
    FluidFriction,
    BallMass,
    TimeSpan,
    SimulationStep,
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

pub type Frame = (ultraviolet::Vec3, ultraviolet::Rotor3);
#[derive(Clone)]
pub struct CurveDescriptorBuilder<S: AppState> {
    pub nb_parameters: usize,
    pub curve_name: &'static str,
    pub parameters: &'static [CurveDescriptorParameter],
    pub bezier_path_id: &'static (dyn Fn(&[InstanciatedParameter]) -> Option<usize> + Send + Sync),
    pub build:
        &'static (dyn Fn(&[InstanciatedParameter], &S) -> Option<CurveDescriptor2D> + Send + Sync),
    pub frame: &'static (dyn Fn(&[InstanciatedParameter], &S) -> Option<Frame> + Send + Sync),
}

use std::fmt;
impl<S: AppState> fmt::Debug for CurveDescriptorBuilder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CurveDecriptorBuilder")
            .field("curve_name", &self.curve_name)
            .finish()
    }
}

impl<S: AppState> ToString for CurveDescriptorBuilder<S> {
    fn to_string(&self) -> String {
        self.curve_name.to_string()
    }
}

impl<S: AppState> PartialEq for CurveDescriptorBuilder<S> {
    fn eq(&self, other: &Self) -> bool {
        self.curve_name == other.curve_name
    }
}

impl<S: AppState> Eq for CurveDescriptorBuilder<S> {}

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
        .width(iced::Length::Units(50))
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

struct CurveDescriptorWidget<S: AppState> {
    parameters: Vec<(&'static str, ParameterWidget)>,
    curve_name: &'static str,
    builder: CurveDescriptorBuilder<S>,
}

impl<S: AppState> CurveDescriptorWidget<S> {
    fn new(builder: CurveDescriptorBuilder<S>) -> Self {
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

    fn view<'a>(&'a mut self) -> Element<'a, Message<S>> {
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

    fn build_curve(&self, app_state: &S) -> Option<CurveDescriptor2D> {
        (self.builder.build)(&self.instanciated_parameters(), app_state)
    }

    fn get_bezier_path_id(&self) -> Option<usize> {
        (self.builder.bezier_path_id)(&self.instanciated_parameters())
    }

    fn get_frame(&self, app_state: &S) -> Option<Frame> {
        (self.builder.frame)(&self.instanciated_parameters(), app_state)
    }
}

pub(crate) struct RevolutionTab<S: AppState> {
    curve_descriptor_widget: Option<CurveDescriptorWidget<S>>,
    pick_curve_state: pick_list::State<CurveDescriptorBuilder<S>>,
    half_turn_count: ParameterWidget,
    radius_input: ParameterWidget,
    scaling: Option<RevolutionScaling>,
    nb_sprial_state_input: ParameterWidget,
    shift_generator: Option<ShiftGenerator>,
    pub shift_idx: isize,
    incr_shift: button::State,
    decr_shift: button::State,

    scaffold_len_target: ParameterWidget,

    nb_section_per_segment_input: ParameterWidget,
    spring_stiffness: ParameterWidget,
    torsion_stiffness: ParameterWidget,
    fluid_friction: ParameterWidget,
    ball_mass: ParameterWidget,
    time_span: ParameterWidget,
    simulation_step: ParameterWidget,
    pick_method_state: pick_list::State<EquadiffSolvingMethod>,
    equadiff_method: EquadiffSolvingMethod,
    scroll_state: scrollable::State,

    go_button: button::State,
    abbort_button: button::State,
    finish_button: button::State,
}

impl<S: AppState> Default for RevolutionTab<S> {
    fn default() -> Self {
        let init_parameter = RevolutionSimulationParameters::default();
        Self {
            scroll_state: Default::default(),
            curve_descriptor_widget: None,
            pick_curve_state: Default::default(),
            half_turn_count: ParameterWidget::new(InstanciatedParameter::Int(0)),
            radius_input: ParameterWidget::new(InstanciatedParameter::Float(0.)),
            scaling: None,
            nb_sprial_state_input: ParameterWidget::new(InstanciatedParameter::Uint(2)),
            shift_generator: None,
            shift_idx: 0,
            incr_shift: Default::default(),
            decr_shift: Default::default(),
            nb_section_per_segment_input: ParameterWidget::new(InstanciatedParameter::Uint(
                init_parameter.nb_section_per_segment,
            )),
            spring_stiffness: ParameterWidget::new(InstanciatedParameter::Float(
                init_parameter.spring_stiffness,
            )),
            torsion_stiffness: ParameterWidget::new(InstanciatedParameter::Float(
                init_parameter.torsion_stiffness,
            )),
            fluid_friction: ParameterWidget::new(InstanciatedParameter::Float(
                init_parameter.fluid_friction,
            )),
            ball_mass: ParameterWidget::new(InstanciatedParameter::Float(init_parameter.ball_mass)),
            time_span: ParameterWidget::new(InstanciatedParameter::Float(init_parameter.time_span)),
            simulation_step: ParameterWidget::new(InstanciatedParameter::Float(
                init_parameter.simulation_step,
            )),
            pick_method_state: Default::default(),
            equadiff_method: init_parameter.method,
            scaffold_len_target: ParameterWidget::new(InstanciatedParameter::Uint(7249)),
            go_button: Default::default(),
            abbort_button: Default::default(),
            finish_button: Default::default(),
        }
    }
}

impl<S: AppState> RevolutionTab<S> {
    pub fn set_builder(&mut self, builder: CurveDescriptorBuilder<S>) {
        if self.curve_descriptor_widget.as_ref().map(|w| w.curve_name) != Some(builder.curve_name) {
            self.curve_descriptor_widget = Some(CurveDescriptorWidget::new(builder))
        }
    }

    pub fn set_method(&mut self, method: EquadiffSolvingMethod) {
        self.equadiff_method = method;
    }

    pub fn get_current_bezier_path_id(&self) -> Option<usize> {
        self.curve_descriptor_widget
            .as_ref()
            .and_then(|w| w.get_bezier_path_id())
    }

    pub fn update_builder_parameter(&mut self, param_id: RevolutionParameterId, text: String) {
        match param_id {
            RevolutionParameterId::SectionParameter(id) => {
                if let Some(widget) = self.curve_descriptor_widget.as_mut() {
                    widget.update_builder_parameter(id, text)
                }
            }
            param => {
                use RevolutionParameterId::*;
                let widget = match param {
                    SectionParameter(_) => unreachable!(),
                    HalfTurnCount => &mut self.half_turn_count,
                    NbSpiral => &mut self.nb_sprial_state_input,
                    RevolutionRadius => &mut self.radius_input,
                    ScaffoldLenTarget => &mut self.scaffold_len_target,
                    NbSectionPerSegment => &mut self.nb_section_per_segment_input,
                    SpringStiffness => &mut self.spring_stiffness,
                    TorsionStiffness => &mut self.torsion_stiffness,
                    FluidFriction => &mut self.fluid_friction,
                    BallMass => &mut self.ball_mass,
                    TimeSpan => &mut self.time_span,
                    SimulationStep => &mut self.simulation_step,
                };
                widget.set_text(text);
            }
        }
    }

    pub fn get_current_unrooted_surface(
        &self,
        app_state: &S,
    ) -> Option<UnrootedRevolutionSurfaceDescriptor> {
        let curve = self
            .curve_descriptor_widget
            .as_ref()
            .and_then(|w| w.build_curve(app_state))?;
        let revolution_radius = self
            .radius_input
            .get_value()
            .and_then(InstanciatedParameter::get_float)
            .map(RevolutionSurfaceRadius::from_signed_f64)?;
        let half_turn_count = self
            .half_turn_count
            .get_value()
            .and_then(InstanciatedParameter::get_int)?;

        let (curve_plane_position, curve_plane_orientation) = self
            .curve_descriptor_widget
            .as_ref()
            .and_then(|w| w.get_frame(app_state))
            .unwrap_or_else(|| (Vec3::zero(), Rotor3::identity()));

        Some(UnrootedRevolutionSurfaceDescriptor {
            curve,
            revolution_radius,
            half_turn_count,
            curve_plane_orientation,
            curve_plane_position,
        })
    }

    pub fn view<'a>(&'a mut self, ui_size: UiSize, app_state: &S) -> Element<'a, Message<S>> {
        let desc = self.get_revolution_system(app_state, false);
        let nb_shift = self.get_shift_per_turn(app_state);

        let mut ret = Scrollable::new(&mut self.scroll_state);
        section!(ret, ui_size, "Revolution Surfaces");
        ret = ret.push(Checkbox::new(
            app_state.get_show_bezier_paths(),
            "Show bezier paths",
            Message::SetShowBezierPaths,
        ));

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
        let helix_text = if let Some(RevolutionScaling { nb_helix, .. }) = self.scaling {
            format!("Nb helix: {nb_helix}")
        } else {
            "Nb helix: ###".into()
        };

        ret = ret.push(Text::new(helix_text));

        ret = ret.push(
            Row::new().push(Text::new("Nb spiral")).push(
                self.nb_sprial_state_input
                    .input_view(RevolutionParameterId::NbSpiral),
            ),
        );
        let shift_txt = if let Some(shift) = nb_shift {
            format!("Nb shift: {shift}")
        } else {
            "Nb shift: ###".into()
        };
        let mut button_incr = Button::new(&mut self.incr_shift, Text::new("+"));
        let mut button_decr = Button::new(&mut self.decr_shift, Text::new("-"));
        if nb_shift.is_some() {
            button_decr = button_decr.on_press(Message::DecrRevolutionShift);
            button_incr = button_incr.on_press(Message::IncrRevolutionShift);
        }
        ret = ret.push(
            Row::new()
                .push(button_decr)
                .push(button_incr)
                .push(Text::new(shift_txt)),
        );

        ret = ret.push(
            Row::new().push(Text::new("Revolution Radius")).push(
                self.radius_input
                    .input_view(RevolutionParameterId::RevolutionRadius),
            ),
        );

        extra_jump!(ret);
        subsection!(ret, ui_size, "Discretization parameters");
        ret = ret.push(
            Row::new().push(Text::new("Nb section per segments")).push(
                self.nb_section_per_segment_input
                    .input_view(RevolutionParameterId::NbSectionPerSegment),
            ),
        );
        ret = ret.push(
            Row::new().push(Text::new("Target length")).push(
                self.scaffold_len_target
                    .input_view(RevolutionParameterId::ScaffoldLenTarget),
            ),
        );

        extra_jump!(ret);
        subsection!(ret, ui_size, "Simulation parameters");
        ret = ret.push(
            Row::new().push(Text::new("Spring Stiffness")).push(
                self.spring_stiffness
                    .input_view(RevolutionParameterId::SpringStiffness),
            ),
        );
        ret = ret.push(
            Row::new().push(Text::new("Torsion Stiffness")).push(
                self.torsion_stiffness
                    .input_view(RevolutionParameterId::TorsionStiffness),
            ),
        );
        ret = ret.push(
            Row::new().push(Text::new("Fluid Friction")).push(
                self.fluid_friction
                    .input_view(RevolutionParameterId::FluidFriction),
            ),
        );
        ret = ret.push(
            Row::new()
                .push(Text::new("Ball Mass"))
                .push(self.ball_mass.input_view(RevolutionParameterId::BallMass)),
        );
        let method_pick_list = PickList::new(
            &mut self.pick_method_state,
            EquadiffSolvingMethod::ALL_METHODS,
            Some(self.equadiff_method),
            |method| Message::RevolutionEquadiffSolvingMethodPicked(method),
        );

        let pick_method_row = Row::new()
            .push(Text::new("Solving Method"))
            .push(method_pick_list);

        ret = ret.push(pick_method_row);

        ret = ret.push(
            Row::new()
                .push(Text::new("Time Span"))
                .push(self.time_span.input_view(RevolutionParameterId::TimeSpan)),
        );
        ret = ret.push(
            Row::new().push(Text::new("Simulation Step")).push(
                self.simulation_step
                    .input_view(RevolutionParameterId::SimulationStep),
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
                if desc.is_some() {
                    button = button.on_press(Message::InitRevolutionRelaxation);
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
            || self.scaffold_len_target.has_keyboard_priority()
            || self.nb_sprial_state_input.has_keyboard_priority()
            || self.spring_stiffness.has_keyboard_priority()
            || self.torsion_stiffness.has_keyboard_priority()
            || self.fluid_friction.has_keyboard_priority()
            || self.ball_mass.has_keyboard_priority()
            || self.time_span.has_keyboard_priority()
            || self.simulation_step.has_keyboard_priority()
    }

    pub fn get_revolution_system(
        &self,
        app_state: &S,
        compute_area: bool,
    ) -> Option<RevolutionSurfaceSystemDescriptor> {
        let unrooted_surface = self.get_current_unrooted_surface(app_state)?;

        let rooting_parameters = RootingParameters {
            dna_parameters: app_state.get_dna_parameters(),
            nb_helix_per_half_section: self.scaling.as_ref()?.nb_helix / 2,
            shift_per_turn: self.try_get_shift_per_turn(app_state)?,
            junction_smoothening: 0.,
        };

        let surface_descriptor = unrooted_surface.rooted(rooting_parameters, compute_area);

        let simulation_parameters = self.get_simulation_parameters()?;

        let system = RevolutionSurfaceSystemDescriptor {
            target: surface_descriptor,
            scaffold_len_target: self
                .scaffold_len_target
                .get_value()
                .and_then(InstanciatedParameter::get_uint)?,
            dna_parameters: app_state.get_dna_parameters(),
            simulation_parameters,
        };

        Some(system)
    }

    /// Get the number of shift per turn, updating `self.shift_generator` if needed.
    fn get_shift_per_turn(&mut self, app_state: &S) -> Option<isize> {
        self.try_get_shift_per_turn(app_state).or_else(|| {
            let unrooted_surface = self.get_current_unrooted_surface(app_state)?;
            let nb_spiral = self
                .nb_sprial_state_input
                .get_value()
                .and_then(InstanciatedParameter::get_uint)?;
            let half_nb_helix = self.scaling.as_ref()?.nb_helix / 2;
            self.shift_generator =
                unrooted_surface.shifts_to_get_n_spirals(half_nb_helix, nb_spiral);
            self.try_get_shift_per_turn(app_state)
        })
    }

    /// Return the number of shift per turn if `self.shift_generator` if up-to-date, and `None`
    /// otherwise.
    fn try_get_shift_per_turn(&self, app_state: &S) -> Option<isize> {
        let unrooted_surface = self.get_current_unrooted_surface(app_state)?;
        let nb_spiral = self
            .nb_sprial_state_input
            .get_value()
            .and_then(InstanciatedParameter::get_uint)?;
        let half_nb_helix = self.scaling.as_ref()?.nb_helix / 2;
        self.shift_generator
            .as_ref()
            .and_then(|g| g.ith_value(self.shift_idx, nb_spiral, &unrooted_surface, half_nb_helix))
    }

    fn get_simulation_parameters(&self) -> Option<RevolutionSimulationParameters> {
        let nb_section_per_segment = self
            .nb_section_per_segment_input
            .get_value()
            .and_then(InstanciatedParameter::get_uint)?;
        let spring_stiffness = self
            .spring_stiffness
            .get_value()
            .and_then(InstanciatedParameter::get_float)?;
        let torsion_stiffness = self
            .torsion_stiffness
            .get_value()
            .and_then(InstanciatedParameter::get_float)?;
        let fluid_friction = self
            .fluid_friction
            .get_value()
            .and_then(InstanciatedParameter::get_float)?;
        let ball_mass = self
            .ball_mass
            .get_value()
            .and_then(InstanciatedParameter::get_float)?;
        let time_span = self
            .time_span
            .get_value()
            .and_then(InstanciatedParameter::get_float)?;
        let simulation_step = self
            .simulation_step
            .get_value()
            .and_then(InstanciatedParameter::get_float)?;
        let method = self.equadiff_method;

        let rescaling = self.scaling.as_ref()?.scale;

        Some(RevolutionSimulationParameters {
            nb_section_per_segment,
            spring_stiffness,
            torsion_stiffness,
            fluid_friction,
            ball_mass,
            simulation_step,
            time_span,
            method,
            rescaling,
        })
    }

    pub fn modifying_radius(&self) -> bool {
        self.radius_input.state.is_focused()
    }

    pub fn update(&mut self, app_state: &S) {
        if let Some(r) = app_state.get_current_revoultion_radius() {
            if !self.modifying_radius() {
                self.update_builder_parameter(
                    RevolutionParameterId::RevolutionRadius,
                    format!("{:.3}", r),
                )
            }
        }

        self.scaling = self
            .scaffold_len_target
            .get_value()
            .and_then(InstanciatedParameter::get_uint)
            .and_then(|len_scaffold| {
                app_state.get_recommended_scaling_revolution_surface(len_scaffold)
            });
    }
}

pub struct RevolutionScaling {
    pub nb_helix: usize,
    pub scale: f64,
}
