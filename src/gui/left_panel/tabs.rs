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
use ensnano_interactor::{RollRequest, SimulationState};
use iced::scrollable;
use iced::Color;

pub(super) struct EditionTab<S: AppState> {
    scroll: iced::scrollable::State,
    helix_roll_factory: RequestFactory<HelixRoll>,
    color_picker: ColorPicker,
    _sequence_input: SequenceInput,
    redim_helices_button: button::State,
    redim_all_helices_button: button::State,
    roll_target_btn: GoStop<S>,
}

impl<S: AppState> EditionTab<S> {
    pub(super) fn new() -> Self {
        Self {
            scroll: Default::default(),
            helix_roll_factory: RequestFactory::new(FactoryId::HelixRoll, HelixRoll {}),
            color_picker: ColorPicker::new(),
            _sequence_input: SequenceInput::new(),
            redim_helices_button: Default::default(),
            redim_all_helices_button: Default::default(),
            roll_target_btn: GoStop::new(
                "Autoroll selected helices".to_owned(),
                Message::RollTargeted,
            ),
        }
    }

    pub(super) fn view<'a>(
        &'a mut self,
        ui_size: UiSize,
        _width: u16,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        ret = ret.push(
            Text::new("Edition")
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .size(ui_size.head_text()),
        );
        let selection = app_state.get_selection_as_dnaelement();
        let roll_target_helices = self.get_roll_target_helices(&selection);

        for view in self
            .helix_roll_factory
            .view(roll_target_helices.len() >= 1)
            .into_iter()
        {
            ret = ret.push(view);
        }

        let sim_state = &app_state.get_simulation_state();
        let roll_target_active = sim_state.is_rolling() || roll_target_helices.len() > 0;
        ret = ret.push(
            self.roll_target_btn
                .view(roll_target_active, sim_state.is_rolling()),
        );

        let color_square = self.color_picker.color_square();
        if app_state.get_selection_mode() == SelectionMode::Strand {
            ret = ret.push(self.color_picker.view()).push(
                Row::new()
                    .push(color_square)
                    .push(iced::Space::new(Length::FillPortion(4), Length::Shrink)),
            )
            //.push(self.sequence_input.view());
        }

        let mut tighten_helices_button =
            text_btn(&mut self.redim_helices_button, "Selected", ui_size.clone());
        if !roll_target_helices.is_empty() {
            tighten_helices_button =
                tighten_helices_button.on_press(Message::Redim2dHelices(false));
        }
        ret = ret.push(Text::new("Tighten 2D helices"));
        ret = ret.push(
            Row::new()
                .push(tighten_helices_button)
                .push(
                    text_btn(&mut self.redim_all_helices_button, "All", ui_size.clone())
                        .on_press(Message::Redim2dHelices(true)),
                )
                .spacing(5),
        );

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    fn get_roll_target_helices(&self, selection: &[DnaElementKey]) -> Vec<usize> {
        let mut ret = vec![];
        for s in selection.iter() {
            if let DnaElementKey::Helix(h) = s {
                ret.push(*h)
            }
        }
        ret
    }

    pub(super) fn update_roll_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<f32>,
    ) {
        self.helix_roll_factory
            .update_request(value_id, value, request);
    }

    pub(super) fn get_roll_request(&mut self, selection: &[DnaElementKey]) -> Option<RollRequest> {
        let roll_target_helices = self.get_roll_target_helices(selection);
        if roll_target_helices.len() > 0 {
            Some(RollRequest {
                roll: true,
                springs: false,
                target_helices: Some(roll_target_helices.clone()),
            })
        } else {
            None
        }
    }

    pub(super) fn strand_color_change(&mut self, color: Color) -> u32 {
        let red = ((color.r * 255.) as u32) << 16;
        let green = ((color.g * 255.) as u32) << 8;
        let blue = (color.b * 255.) as u32;
        self.color_picker.update_color(color);
        let hue = Hsv::from(Rgb::new(
            color.r as f64 * 255.,
            color.g as f64 * 255.,
            color.b as f64 * 255.,
        ))
        .h;
        self.color_picker.change_hue(hue as f32);
        let color = red + green + blue;
        color
    }

    pub(super) fn change_hue(&mut self, hue: f32) {
        self.color_picker.change_hue(hue)
    }
}

pub(super) struct GridTab {
    scroll: iced::scrollable::State,
    finalize_hyperboloid_btn: button::State,
    make_square_grid_btn: button::State,
    make_honeycomb_grid_btn: button::State,
    hyperboloid_factory: RequestFactory<Hyperboloid_>,
    start_hyperboloid_btn: button::State,
    make_grid_btn: button::State,
}

impl GridTab {
    pub fn new() -> Self {
        Self {
            scroll: Default::default(),
            make_square_grid_btn: Default::default(),
            make_honeycomb_grid_btn: Default::default(),
            hyperboloid_factory: RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {}),
            finalize_hyperboloid_btn: Default::default(),
            start_hyperboloid_btn: Default::default(),
            make_grid_btn: Default::default(),
        }
    }

    pub(super) fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        _width: u16,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        ret = ret.push(
            Text::new("Grids")
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .size(ui_size.head_text()),
        );

        ret = ret.push(Text::new("New Grid"));
        let make_square_grid_btn = icon_btn(
            &mut self.make_square_grid_btn,
            ICON_SQUARE_GRID,
            ui_size.clone(),
        )
        .on_press(Message::NewGrid(GridTypeDescr::Square));
        let make_honeycomb_grid_btn = icon_btn(
            &mut self.make_honeycomb_grid_btn,
            ICON_HONEYCOMB_GRID,
            ui_size.clone(),
        )
        .on_press(Message::NewGrid(GridTypeDescr::Honeycomb));

        let grid_buttons = Row::new()
            .push(make_square_grid_btn)
            .push(make_honeycomb_grid_btn)
            .spacing(5);
        ret = ret.push(grid_buttons);

        ret = ret.push(iced::Space::with_height(Length::Units(3)));

        let nanotube_title = Row::new().push(Text::new("New nanotube"));

        ret = ret.push(nanotube_title);
        let start_hyperboloid_btn = if !app_state.is_building_hyperboloid() {
            icon_btn(
                &mut self.start_hyperboloid_btn,
                ICON_NANOTUBE,
                ui_size.clone(),
            )
            .on_press(Message::NewHyperboloid)
        } else {
            text_btn(&mut self.start_hyperboloid_btn, "Finish", ui_size.clone())
                .on_press(Message::FinalizeHyperboloid)
        };

        let cancel_hyperboloid_btn = text_btn(
            &mut self.finalize_hyperboloid_btn,
            "Cancel",
            ui_size.clone(),
        )
        .on_press(Message::CancelHyperboloid);

        if app_state.is_building_hyperboloid() {
            ret = ret.push(
                Row::new()
                    .spacing(3)
                    .push(start_hyperboloid_btn)
                    .push(cancel_hyperboloid_btn),
            );
        } else {
            ret = ret.push(start_hyperboloid_btn);
        }

        for view in self
            .hyperboloid_factory
            .view(app_state.is_building_hyperboloid())
            .into_iter()
        {
            ret = ret.push(view);
        }

        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("Guess grid").size(ui_size.intermediate_text()));
        let mut button_make_grid =
            Button::new(&mut self.make_grid_btn, iced::Text::new("From Selection"))
                .height(Length::Units(ui_size.button()));

        if app_state.can_make_grid() {
            button_make_grid = button_make_grid.on_press(Message::MakeGrids);
        }

        ret = ret.push(button_make_grid);
        ret = ret.push(Text::new("Select ≥4 unattached helices").size(ui_size.main_text()));

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub fn new_hyperboloid(&mut self, requests: &mut Option<HyperboloidRequest>) {
        self.hyperboloid_factory = RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {});
        self.hyperboloid_factory.make_request(requests);
    }

    pub fn update_hyperboloid_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<HyperboloidRequest>,
    ) {
        self.hyperboloid_factory
            .update_request(value_id, value, request);
    }
}

pub(super) struct CameraShortcut {
    camera_target_buttons: [button::State; 6],
    camera_rotation_buttons: [button::State; 6],
    xz: isize,
    yz: isize,
    xy: isize,
    scroll: scrollable::State,
}

impl CameraShortcut {
    pub fn new() -> Self {
        Self {
            camera_target_buttons: Default::default(),
            camera_rotation_buttons: Default::default(),
            xz: 0,
            yz: 0,
            xy: 0,
            scroll: Default::default(),
        }
    }

    pub(super) fn reset_angles(&mut self) {
        self.xz = 0;
        self.yz = 0;
        self.xy = 0
    }

    pub(super) fn set_angles(&mut self, xz: isize, yz: isize, xy: isize) {
        self.xz += xz;
        self.yz += yz;
        self.xy += xy;
    }

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        width: u16,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new();
        ret = ret.push(
            Text::new("Camera")
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .size(ui_size.head_text()),
        );
        let mut target_buttons: Vec<_> = self
            .camera_target_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, Text::new(target_text(i)).size(ui_size.main_text()))
                    .on_press(target_message(i))
                    .width(Length::Units(2 * ui_size.button()))
            })
            .collect();
        while target_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(target_buttons.remove(0)).spacing(5);
            let mut nb_button_row = 1;
            let mut space = 2 * ui_size.button() + 5;
            while space + 2 * ui_size.button() < width
                && target_buttons.len() > 0
                && nb_button_row < 3
            {
                row = row.push(target_buttons.remove(0)).spacing(5);
                space += 2 * ui_size.button() + 5;
                nb_button_row += 1;
            }
            ret = ret.push(row)
        }

        let xz = self.xz;
        let yz = self.yz;
        let xy = self.xy;

        let mut rotate_buttons: Vec<_> = self
            .camera_rotation_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, rotation_text(i, ui_size.clone()))
                    .on_press(rotation_message(i, xz, yz, xy))
                    .width(Length::Units(ui_size.button()))
            })
            .collect();

        ret = ret.push(Text::new("Rotate Camera"));
        while rotate_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(rotate_buttons.remove(0)).spacing(5);
            let mut space = ui_size.button() + 5;
            while space + ui_size.button() < width && rotate_buttons.len() > 0 {
                row = row.push(rotate_buttons.remove(0)).spacing(5);
                space += ui_size.button() + 5;
            }
            ret = ret.spacing(5).push(row)
        }

        Scrollable::new(&mut self.scroll).push(ret).into()
    }
}

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
        let mut ret = Column::new().spacing(2);
        ret = ret.push(
            Text::new("Camera")
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .size(ui_size.head_text()),
        );
        ret = ret.push(iced::Space::with_height(Length::Units(2)));
        ret = ret.push(Text::new("Visibility").size(ui_size.intermediate_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(2)));
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

        ret = ret.push(iced::Space::with_height(Length::Units(2)));
        ret = ret.push(Text::new("Rendering").size(ui_size.intermediate_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(2)));
        ret = ret.push(Text::new("Style"));
        ret = ret.push(PickList::new(
            &mut self.rendering_mode_picklist,
            &ALL_RENDERING_MODE[..],
            Some(self.rendering_mode),
            Message::RenderingMode,
        ));
        ret = ret.push(iced::Space::with_height(Length::Units(2)));
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
        let mut ret = Column::new().spacing(2);
        ret = ret.push(Text::new("Simulation (Beta)").size(ui_size.head_text()));
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
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
        ret = ret
            .push(Text::new("Parameters for helices simulation").size(ui_size.intermediate_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(2)));
        for view in self.rigid_body_factory.view(true).into_iter() {
            ret = ret.push(view);
        }
        ret = ret.push(right_checkbox(
            volume_exclusion,
            "Volume exclusion",
            Message::VolumeExclusion,
            ui_size.clone(),
        ));
        ret = ret.push(right_checkbox(
            brownian_motion,
            "Unmatched nt jiggling",
            Message::BrownianMotion,
            ui_size.clone(),
        ));
        for view in self.brownian_factory.view(brownian_motion).into_iter() {
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
        ret = ret.push(Text::new("Parameters").size(ui_size.head_text()));
        ret = ret.push(Text::new("Font size"));
        ret = ret.push(PickList::new(
            &mut self.size_pick_list,
            &super::super::ALL_UI_SIZE[..],
            Some(ui_size.clone()),
            Message::UiSizePicked,
        ));

        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("Scrolling").size(ui_size.intermediate_text()));
        for view in self.scroll_sensitivity_factory.view(true).into_iter() {
            ret = ret.push(view);
        }

        ret = ret.push(right_checkbox(
            self.invert_y_scroll,
            "Inverse direction",
            Message::InvertScroll,
            ui_size.clone(),
        ));

        ret = ret.push(iced::Space::with_height(Length::Units(10)));
        ret = ret.push(Text::new("DNA parameters").size(ui_size.head_text()));
        for line in app_state.get_dna_parameters().formated_string().lines() {
            ret = ret.push(Text::new(line));
        }
        ret = ret.push(iced::Space::with_height(Length::Units(10)));
        ret = ret.push(Text::new("About").size(ui_size.head_text()));
        ret = ret.push(Text::new(format!(
            "Version {}",
            std::env!("CARGO_PKG_VERSION")
        )));
        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("Development:").size(ui_size.intermediate_text()));
        ret = ret.push(Text::new("Nicolas Levy"));
        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("Conception:").size(ui_size.intermediate_text()));
        ret = ret.push(Text::new("Nicolas Levy"));
        ret = ret.push(Text::new("Nicolas Schabanel"));
        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("License:").size(ui_size.intermediate_text()));
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
        ret = ret.push(Text::new("Sequences").size(ui_size.head_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
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
        ret = ret.push(iced::Space::with_height(Length::Units(3)));

        ret = ret.push(Text::new("Scaffold").size(ui_size.intermediate_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
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
        let mut button_selection_to_scaffold = text_btn(
            &mut self.button_selection_to_scaffold,
            "From selection",
            ui_size.clone(),
        );
        let mut button_selection_from_scaffold = text_btn(
            &mut self.button_selection_from_scaffold,
            "To selection",
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
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
        ret = ret.push(
            Row::new()
                .push(button_selection_from_scaffold)
                .push(iced::Space::with_width(Length::Units(5)))
                .push(button_selection_to_scaffold),
        );
        ret = ret.push(iced::Space::with_height(Length::Units(3)));

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
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
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

        ret = ret.push(iced::Space::with_height(Length::Units(3)));
        ret = ret.push(Text::new("Stapples").size(ui_size.head_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
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
