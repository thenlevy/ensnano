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
use super::color_picker::{ColorSquare, ColorState};
use super::*;
use ensnano_design::CameraId;
use ensnano_interactor::{RollRequest, SimulationState};
use iced::scrollable;
use std::collections::VecDeque;

const MEMORY_COLOR_ROWS: usize = 3;
const MEMORY_COLOR_COLUMN: usize = 8;
const NB_MEMORY_COLOR: usize = MEMORY_COLOR_ROWS * MEMORY_COLOR_COLUMN;
const JUMP_SIZE: u16 = 4;

use super::super::material_icons_light;
use material_icons_light::LightIcon;
const LIGHT_ICONFONT: iced::Font = iced::Font::External {
    name: "IconFontLight",
    bytes: material_icons_light::MATERIAL_ICON_LIGHT,
};
fn light_icon(icon: LightIcon, ui_size: UiSize) -> iced::Text {
    iced::Text::new(format!("{}", material_icons_light::icon_to_char(icon)))
        .font(LIGHT_ICONFONT)
        .size(ui_size.icon())
}

fn light_icon_btn<'a, Message: Clone>(
    state: &'a mut button::State,
    icon: LightIcon,
    ui_size: UiSize,
) -> Button<'a, Message> {
    let content = light_icon(icon, ui_size);
    Button::new(state, content).height(iced::Length::Units(ui_size.button()))
}

macro_rules! section {
    ($row:ident, $ui_size:ident, $text:tt) => {
        $row = $row.push(Text::new($text).size($ui_size.head_text()));
    };
}
macro_rules! subsection {
    ($row:ident, $ui_size:ident, $text:tt) => {
        $row = $row.push(Text::new($text).size($ui_size.intermediate_text()));
    };
}

macro_rules! extra_jump {
    ($row: ident) => {
        $row = $row.push(iced::Space::with_height(iced::Length::Units(JUMP_SIZE)))
    };
    ($nb: tt, $row: ident) => {
        $row = $row.push(iced::Space::with_height(iced::Length::Units($nb)))
    };
}

mod edition_tab;
pub use edition_tab::EditionTab;
mod grids_tab;
pub use grids_tab::GridTab;
mod camera_shortcut;
pub use camera_shortcut::CameraShortcut;

use ensnano_interactor::graphics::{
    Background3D, RenderingMode, ALL_BACKGROUND3D, ALL_RENDERING_MODE,
};

pub(super) struct CameraTab {
    fog: FogParameters,
    scroll: scrollable::State,
    selection_visibility_btn: button::State,
    compl_visibility_btn: button::State,
    all_visible_btn: button::State,
    pub background3d: Background3D,
    background3d_picklist: pick_list::State<Background3D>,
    pub rendering_mode: RenderingMode,
    rendering_mode_picklist: pick_list::State<RenderingMode>,
}

impl CameraTab {
    pub fn new() -> Self {
        Self {
            fog: Default::default(),
            scroll: Default::default(),
            selection_visibility_btn: Default::default(),
            compl_visibility_btn: Default::default(),
            all_visible_btn: Default::default(),
            background3d: Default::default(),
            background3d_picklist: Default::default(),
            rendering_mode: Default::default(),
            rendering_mode_picklist: Default::default(),
        }
    }

    pub fn view<'a, S: AppState>(&'a mut self, ui_size: UiSize) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        section!(ret, ui_size, "Camera");
        subsection!(ret, ui_size, "Visibility");
        ret = ret.push(
            text_btn(
                &mut self.selection_visibility_btn,
                "Toggle Selected Visibility",
                ui_size.clone(),
            )
            .on_press(Message::ToggleVisibility(false)),
        );
        ret = ret.push(
            text_btn(
                &mut self.compl_visibility_btn,
                "Toggle NonSelected Visibility",
                ui_size.clone(),
            )
            .on_press(Message::ToggleVisibility(true)),
        );
        ret = ret.push(
            text_btn(
                &mut self.all_visible_btn,
                "Everything visible",
                ui_size.clone(),
            )
            .on_press(Message::AllVisible),
        );
        ret = ret.push(self.fog.view(&ui_size));

        subsection!(ret, ui_size, "Rendering");
        ret = ret.push(Text::new("Style"));
        ret = ret.push(PickList::new(
            &mut self.rendering_mode_picklist,
            &ALL_RENDERING_MODE[..],
            Some(self.rendering_mode),
            Message::RenderingMode,
        ));
        ret = ret.push(Text::new("Background"));
        ret = ret.push(PickList::new(
            &mut self.background3d_picklist,
            &ALL_BACKGROUND3D[..],
            Some(self.background3d),
            Message::Background3D,
        ));

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub(super) fn fog_visible(&mut self, visible: bool) {
        self.fog.visible = visible
    }

    pub(super) fn fog_length(&mut self, length: f32) {
        self.fog.length = length
    }

    pub(super) fn fog_radius(&mut self, radius: f32) {
        self.fog.radius = radius
    }

    pub(super) fn fog_camera(&mut self, from_camera: bool) {
        self.fog.from_camera = from_camera;
    }

    pub(super) fn get_fog_request(&self) -> Fog {
        self.fog.request()
    }
}

struct FogParameters {
    visible: bool,
    from_camera: bool,
    radius: f32,
    radius_slider: slider::State,
    length: f32,
    length_slider: slider::State,
    picklist: pick_list::State<FogChoice>,
}

impl FogParameters {
    fn view<S: AppState>(&mut self, ui_size: &UiSize) -> Column<Message<S>> {
        let mut column = Column::new()
            .push(Text::new("Fog").size(ui_size.intermediate_text()))
            .push(PickList::new(
                &mut self.picklist,
                &ALL_FOG_CHOICE[..],
                Some(FogChoice::from_param(self.visible, self.from_camera)),
                Message::FogChoice,
            ));

        let radius_text = if self.visible {
            Text::new("Radius")
        } else {
            Text::new("Radius").color([0.6, 0.6, 0.6])
        };

        let gradient_text = if self.visible {
            Text::new("Softness")
        } else {
            Text::new("Softness").color([0.6, 0.6, 0.6])
        };

        let length_slider = if self.visible {
            Slider::new(
                &mut self.length_slider,
                0f32..=100f32,
                self.length,
                Message::FogLength,
            )
        } else {
            Slider::new(&mut self.length_slider, 0f32..=100f32, self.length, |_| {
                Message::Nothing
            })
            .style(DesactivatedSlider)
        };

        let softness_slider = if self.visible {
            Slider::new(
                &mut self.radius_slider,
                0f32..=100f32,
                self.radius,
                Message::FogRadius,
            )
        } else {
            Slider::new(&mut self.radius_slider, 0f32..=100f32, self.radius, |_| {
                Message::Nothing
            })
            .style(DesactivatedSlider)
        };

        column = column
            .push(Row::new().spacing(5).push(radius_text).push(length_slider))
            .push(
                Row::new()
                    .spacing(5)
                    .push(gradient_text)
                    .push(softness_slider),
            );
        column
    }

    fn request(&self) -> Fog {
        Fog {
            radius: self.radius,
            active: self.visible,
            length: self.length,
            from_camera: self.from_camera,
            alt_fog_center: None,
        }
    }
}

impl Default for FogParameters {
    fn default() -> Self {
        Self {
            visible: false,
            length: 10.,
            radius: 10.,
            length_slider: Default::default(),
            radius_slider: Default::default(),
            from_camera: true,
            picklist: Default::default(),
        }
    }
}

pub(super) struct SimulationTab<S: AppState> {
    rigid_body_factory: RequestFactory<RigidBodyFactory>,
    brownian_factory: RequestFactory<BrownianParametersFactory>,
    rigid_grid_button: GoStop<S>,
    rigid_helices_button: GoStop<S>,
    scroll: scrollable::State,
    physical_simulation: PhysicalSimulation,
    reset_state: button::State,
}

impl<S: AppState> SimulationTab<S> {
    pub(super) fn new() -> Self {
        let init_brownian = BrownianParametersFactory {
            rate: 0.,
            amplitude: 0.08,
        };
        Self {
            rigid_body_factory: RequestFactory::new(
                FactoryId::RigidBody,
                RigidBodyFactory {
                    volume_exclusion: false,
                    brownian_motion: false,
                    brownian_parameters: init_brownian.clone(),
                },
            ),
            brownian_factory: RequestFactory::new(FactoryId::Brownian, init_brownian),
            rigid_helices_button: GoStop::new(
                String::from("Rigid Helices"),
                Message::RigidHelicesSimulation,
            ),
            rigid_grid_button: GoStop::new(
                String::from("Rigid Grids"),
                Message::RigidGridSimulation,
            ),
            scroll: Default::default(),
            physical_simulation: Default::default(),
            reset_state: Default::default(),
        }
    }

    pub(super) fn view<'a>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let sim_state = &app_state.get_simulation_state();
        let grid_active = sim_state.is_none() || sim_state.simulating_grid();
        let roll_active = sim_state.is_none() || sim_state.is_rolling();
        let mut ret = Column::new().spacing(5);
        section!(ret, ui_size, "Simulation (Beta)");
        ret = ret.push(self.physical_simulation.view(
            &ui_size,
            "Roll",
            roll_active,
            sim_state.is_rolling(),
        ));
        ret = ret
            .push(
                self.rigid_grid_button
                    .view(grid_active, sim_state.simulating_grid()),
            )
            .push(Self::helix_btns(
                &mut self.rigid_helices_button,
                &mut self.reset_state,
                app_state,
                ui_size.clone(),
            ));

        let volume_exclusion = self.rigid_body_factory.requestable.volume_exclusion;
        let brownian_motion = self.rigid_body_factory.requestable.brownian_motion;
        subsection!(ret, ui_size, "Parameters for helices simulation");
        for view in self
            .rigid_body_factory
            .view(true, ui_size.main_text())
            .into_iter()
        {
            ret = ret.push(view);
        }
        ret = ret.push(right_checkbox(
            volume_exclusion,
            "Volume exclusion",
            Message::VolumeExclusion,
            ui_size,
        ));
        ret = ret.push(right_checkbox(
            brownian_motion,
            "Unmatched nt jiggling",
            Message::BrownianMotion,
            ui_size,
        ));
        for view in self
            .brownian_factory
            .view(brownian_motion, ui_size.main_text())
            .into_iter()
        {
            ret = ret.push(view);
        }

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    fn helix_btns<'a>(
        go_stop: &'a mut GoStop<S>,
        reset_state: &'a mut button::State,
        app_state: &S,
        ui_size: UiSize,
    ) -> Element<'a, Message<S>> {
        let sim_state = app_state.get_simulation_state();
        if sim_state.is_paused() {
            Row::new()
                .push(go_stop.view(true, false))
                .spacing(3)
                .push(text_btn(reset_state, "Reset", ui_size).on_press(Message::ResetSimulation))
                .into()
        } else {
            let helices_active = sim_state.is_none() || sim_state.simulating_helices();
            go_stop
                .view(helices_active, sim_state.simulating_helices())
                .into()
        }
    }

    pub(super) fn set_volume_exclusion(&mut self, volume_exclusion: bool) {
        self.rigid_body_factory.requestable.volume_exclusion = volume_exclusion;
    }

    pub(super) fn set_brownian_motion(&mut self, brownian_motion: bool) {
        self.rigid_body_factory.requestable.brownian_motion = brownian_motion;
    }

    pub(super) fn make_rigid_body_request(
        &mut self,
        request: &mut Option<RigidBodyParametersRequest>,
    ) {
        self.rigid_body_factory.make_request(request)
    }

    pub(super) fn update_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<RigidBodyParametersRequest>,
    ) {
        self.rigid_body_factory
            .update_request(value_id, value, request)
    }

    pub(super) fn update_brownian(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<RigidBodyParametersRequest>,
    ) {
        let new_brownian = self.brownian_factory.update_value(value_id, value);
        self.rigid_body_factory.requestable.brownian_parameters = new_brownian;
        self.rigid_body_factory.make_request(request)
    }

    pub(super) fn get_physical_simulation_request(&self) -> RollRequest {
        self.physical_simulation.request()
    }

    pub(super) fn leave_tab<R: Requests>(&mut self, requests: Arc<Mutex<R>>, app_state: &S) {
        if app_state.get_simulation_state() == SimulationState::RigidGrid {
            self.request_stop_rigid_body_simulation(requests);
            println!("stop grids");
        } else if app_state.get_simulation_state() == SimulationState::RigidHelices {
            self.request_stop_rigid_body_simulation(requests);
            println!("stop helices");
        }
    }

    fn request_stop_rigid_body_simulation<R: Requests>(&mut self, requests: Arc<Mutex<R>>) {
        let mut request = None;
        self.make_rigid_body_request(&mut request);
        if let Some(request) = request {
            requests
                .lock()
                .unwrap()
                .update_rigid_body_simulation_parameters(request)
        }
    }
}

struct GoStop<S: AppState> {
    go_stop_button: button::State,
    pub name: String,
    on_press: Box<dyn Fn(bool) -> Message<S>>,
}

impl<S: AppState> GoStop<S> {
    fn new<F>(name: String, on_press: F) -> Self
    where
        F: 'static + Fn(bool) -> Message<S>,
    {
        Self {
            go_stop_button: Default::default(),
            name,
            on_press: Box::new(on_press),
        }
    }

    fn view(&mut self, active: bool, running: bool) -> Row<Message<S>> {
        let button_str = if running {
            "Stop".to_owned()
        } else {
            self.name.clone()
        };
        let mut button = Button::new(&mut self.go_stop_button, Text::new(button_str))
            .style(ButtonColor::red_green(running));
        if active {
            button = button.on_press((self.on_press)(!running));
        }
        Row::new().push(button)
    }
}

#[derive(Default)]
struct PhysicalSimulation {
    go_stop_button: button::State,
}

impl PhysicalSimulation {
    fn view<'a, 'b, S: AppState>(
        &'a mut self,
        _ui_size: &'b UiSize,
        name: &'static str,
        active: bool,
        running: bool,
    ) -> Row<'a, Message<S>> {
        let button_str = if running { "Stop" } else { name };
        let mut button = Button::new(&mut self.go_stop_button, Text::new(button_str))
            .style(ButtonColor::red_green(running));
        if active {
            button = button.on_press(Message::SimRequest);
        }
        Row::new().push(button)
    }

    fn request(&self) -> RollRequest {
        RollRequest {
            roll: true,
            springs: false,
            target_helices: None,
        }
    }
}

pub struct ParametersTab {
    size_pick_list: pick_list::State<UiSize>,
    scroll: scrollable::State,
    scroll_sensitivity_factory: RequestFactory<ScrollSentivity>,
    pub invert_y_scroll: bool,
}

impl ParametersTab {
    pub(super) fn new() -> Self {
        Self {
            size_pick_list: Default::default(),
            scroll: Default::default(),
            scroll_sensitivity_factory: RequestFactory::new(FactoryId::Scroll, ScrollSentivity {}),
            invert_y_scroll: false,
        }
    }

    pub(super) fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new();
        section!(ret, ui_size, "Parameters");
        extra_jump!(ret);
        subsection!(ret, ui_size, "Font size");
        ret = ret.push(PickList::new(
            &mut self.size_pick_list,
            &super::super::ALL_UI_SIZE[..],
            Some(ui_size.clone()),
            Message::UiSizePicked,
        ));

        extra_jump!(ret);
        subsection!(ret, ui_size, "Scrolling");
        for view in self
            .scroll_sensitivity_factory
            .view(true, ui_size.main_text())
            .into_iter()
        {
            ret = ret.push(view);
        }

        ret = ret.push(right_checkbox(
            self.invert_y_scroll,
            "Inverse direction",
            Message::InvertScroll,
            ui_size.clone(),
        ));

        extra_jump!(10, ret);
        section!(ret, ui_size, "DNA parameters");
        for line in app_state.get_dna_parameters().formated_string().lines() {
            ret = ret.push(Text::new(line));
        }
        ret = ret.push(iced::Space::with_height(Length::Units(10)));
        ret = ret.push(Text::new("About").size(ui_size.head_text()));
        ret = ret.push(Text::new(format!(
            "Version {}",
            std::env!("CARGO_PKG_VERSION")
        )));

        subsection!(ret, ui_size, "Development:");
        ret = ret.push(Text::new("Nicolas Levy"));
        extra_jump!(ret);
        subsection!(ret, ui_size, "Conception:");
        ret = ret.push(Text::new("Nicolas Levy"));
        ret = ret.push(Text::new("Nicolas Schabanel"));
        extra_jump!(ret);
        subsection!(ret, ui_size, "License:");
        ret = ret.push(Text::new("GPLv3"));

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub(super) fn update_scroll_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<f32>,
    ) {
        self.scroll_sensitivity_factory
            .update_request(value_id, value, request);
    }
}

pub struct SequenceTab {
    scroll: scrollable::State,
    button_scaffold: button::State,
    button_stapples: button::State,
    toggle_text_value: bool,
    scaffold_position_str: String,
    scaffold_position: usize,
    scaffold_input: text_input::State,
    button_selection_from_scaffold: button::State,
    button_selection_to_scaffold: button::State,
    button_show_sequence: button::State,
}

impl SequenceTab {
    pub(super) fn new() -> Self {
        Self {
            scroll: Default::default(),
            button_stapples: Default::default(),
            button_scaffold: Default::default(),
            toggle_text_value: false,
            scaffold_position_str: "0".to_string(),
            scaffold_position: 0,
            scaffold_input: Default::default(),
            button_selection_from_scaffold: Default::default(),
            button_selection_to_scaffold: Default::default(),
            button_show_sequence: Default::default(),
        }
    }

    pub(super) fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &'a S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new();
        section!(ret, ui_size, "Sequence");
        extra_jump!(ret);
        if !self.scaffold_input.is_focused() {
            if let Some(n) = app_state.get_scaffold_info().and_then(|info| info.shift) {
                self.update_pos_str(n.to_string());
            }
        }
        let button_show_sequence = if self.toggle_text_value {
            text_btn(
                &mut self.button_show_sequence,
                "Hide Sequences",
                ui_size.clone(),
            )
            .on_press(Message::ToggleText(false))
        } else {
            text_btn(
                &mut self.button_show_sequence,
                "Show Sequences",
                ui_size.clone(),
            )
            .on_press(Message::ToggleText(true))
        };
        ret = ret.push(button_show_sequence);
        extra_jump!(ret);
        section!(ret, ui_size, "Scaffold");
        extra_jump!(ret);
        let mut button_selection_to_scaffold = text_btn(
            &mut self.button_selection_to_scaffold,
            "From selection",
            ui_size.clone(),
        );
        let mut button_selection_from_scaffold = text_btn(
            &mut self.button_selection_from_scaffold,
            "Show",
            ui_size.clone(),
        );
        if app_state.get_scaffold_info().is_some() {
            button_selection_from_scaffold =
                button_selection_from_scaffold.on_press(Message::SelectScaffold);
        }
        let selection = app_state.get_selection_as_dnaelement();
        if let Some(n) = Self::get_candidate_scaffold(&selection) {
            button_selection_to_scaffold =
                button_selection_to_scaffold.on_press(Message::ScaffoldIdSet(n, true));
        }
        ret = ret.push(
            Row::new()
                .push(button_selection_to_scaffold)
                .push(iced::Space::with_width(Length::Units(5)))
                .push(button_selection_from_scaffold),
        );
        extra_jump!(ret);
        macro_rules! scaffold_length_fmt {
            () => {
                "Length: {} nt"
            };
        }
        let (scaffold_text, length_text) = if let Some(info) = app_state.get_scaffold_info() {
            (
                format!("Strand #{}", info.id),
                format!(scaffold_length_fmt!(), info.length),
            )
        } else {
            (
                "NOT SET".to_owned(),
                format!(scaffold_length_fmt!(), "—").to_owned(),
            )
        };
        let mut length_text = Text::new(length_text);
        if app_state.get_scaffold_info().is_none() {
            length_text = length_text.color(innactive_color())
        }
        ret = ret.push(Text::new(scaffold_text).size(ui_size.main_text()));
        ret = ret.push(length_text);
        extra_jump!(ret);

        let button_scaffold = Button::new(
            &mut self.button_scaffold,
            iced::Text::new("Set scaffold sequence"),
        )
        .height(Length::Units(ui_size.button()))
        .on_press(Message::SetScaffoldSeqButtonPressed);
        let scaffold_position_text = "Starting position";
        let scaffold_row = Row::new()
            .push(Text::new(scaffold_position_text).width(Length::FillPortion(2)))
            .push(
                TextInput::new(
                    &mut self.scaffold_input,
                    "Scaffold position",
                    &self.scaffold_position_str,
                    Message::ScaffoldPositionInput,
                )
                .style(BadValue(
                    self.scaffold_position_str == self.scaffold_position.to_string(),
                ))
                .width(iced::Length::FillPortion(1)),
            );
        ret = ret.push(button_scaffold);
        extra_jump!(ret);
        ret = ret.push(scaffold_row);
        let starting_nucl = app_state
            .get_scaffold_info()
            .as_ref()
            .and_then(|info| info.starting_nucl);
        macro_rules! nucl_text_fmt {
            () => {
                "   Helix #{}\n   Strand: {}\n   Nt #{}"
            };
        }
        let nucl_text = if let Some(nucl) = starting_nucl {
            format!(
                nucl_text_fmt!(),
                nucl.helix,
                if nucl.forward {
                    "→ forward"
                } else {
                    "← backward"
                }, // Pourquoi pas "→" et "←" ?
                nucl.position
            )
        } else {
            format!(nucl_text_fmt!(), " —", " —", " —")
        };
        let mut nucl_text = Text::new(nucl_text).size(ui_size.main_text());
        if starting_nucl.is_none() {
            nucl_text = nucl_text.color(innactive_color())
        }
        ret = ret.push(nucl_text);

        extra_jump!(ret);
        section!(ret, ui_size, "Stapples");
        extra_jump!(ret);
        let button_stapples = Button::new(
            &mut self.button_stapples,
            iced::Text::new("Export Stapples"),
        )
        .height(Length::Units(ui_size.button()))
        .on_press(Message::StapplesRequested);
        ret = ret.push(button_stapples);
        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub(super) fn toggle_text_value(&mut self, b: bool) {
        self.toggle_text_value = b;
    }

    pub(super) fn update_pos_str(&mut self, position_str: String) -> Option<usize> {
        self.scaffold_position_str = position_str;
        if let Ok(pos) = self.scaffold_position_str.parse::<usize>() {
            self.scaffold_position = pos;
            Some(pos)
        } else {
            None
        }
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.scaffold_input.is_focused()
    }

    fn get_candidate_scaffold(selection: &[DnaElementKey]) -> Option<usize> {
        if selection.len() == 1 {
            if let DnaElementKey::Strand(n) = selection[0] {
                Some(n)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_scaffold_shift(&self) -> usize {
        self.scaffold_position
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum FogChoice {
    None,
    FromCamera,
    FromPivot,
}

impl Default for FogChoice {
    fn default() -> Self {
        Self::None
    }
}

const ALL_FOG_CHOICE: [FogChoice; 3] =
    [FogChoice::None, FogChoice::FromCamera, FogChoice::FromPivot];

impl std::fmt::Display for FogChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ret = match self {
            Self::None => "None",
            Self::FromCamera => "From Camera",
            Self::FromPivot => "From Pivot",
        };
        write!(f, "{}", ret)
    }
}

impl FogChoice {
    fn from_param(visible: bool, from_camera: bool) -> Self {
        if visible {
            if from_camera {
                Self::FromCamera
            } else {
                Self::FromPivot
            }
        } else {
            Self::None
        }
    }

    pub fn to_param(&self) -> (bool, bool) {
        match self {
            Self::None => (false, false),
            Self::FromPivot => (true, false),
            Self::FromCamera => (true, true),
        }
    }
}
