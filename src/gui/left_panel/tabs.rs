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
use iced::scrollable;
use iced::Color;

use crate::design::SimulationState;

pub(super) struct EditionTab {
    selection_mode_state: SelectionModeState,
    action_mode_state: ActionModeState,
    scroll: iced::scrollable::State,
    helix_roll_factory: RequestFactory<HelixRoll>,
    color_picker: ColorPicker,
    sequence_input: SequenceInput,
    redim_helices_button: button::State,
    redim_all_helices_button: button::State,
    roll_target_btn: GoStop,
    roll_target_helices: Vec<usize>,
}

impl EditionTab {
    pub(super) fn new() -> Self {
        Self {
            selection_mode_state: Default::default(),
            action_mode_state: Default::default(),
            scroll: Default::default(),
            helix_roll_factory: RequestFactory::new(FactoryId::HelixRoll, HelixRoll {}),
            color_picker: ColorPicker::new(),
            sequence_input: SequenceInput::new(),
            redim_helices_button: Default::default(),
            redim_all_helices_button: Default::default(),
            roll_target_btn: GoStop::new(
                "Autoroll selected helices".to_owned(),
                Message::RollTargeted,
            ),
            roll_target_helices: vec![],
        }
    }

    pub(super) fn view<'a>(
        &'a mut self,
        action_mode: ActionMode,
        selection_mode: SelectionMode,
        ui_size: UiSize,
        width: u16,
        app_state: &ApplicationState,
    ) -> Element<'a, Message> {
        let mut ret = Column::new().spacing(5);
        ret = ret.push(
            Text::new("Edition")
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .size(ui_size.head_text()),
        );
        let selection_modes = [
            SelectionMode::Nucleotide,
            SelectionMode::Strand,
            SelectionMode::Helix,
        ];

        let mut selection_buttons: Vec<Button<'a, Message>> = self
            .selection_mode_state
            .get_states()
            .into_iter()
            .rev()
            .filter(|(m, _)| selection_modes.contains(m))
            .map(|(mode, state)| selection_mode_btn(state, mode, selection_mode, ui_size.button()))
            .collect();

        ret = ret.push(Text::new("Selection Mode"));
        while selection_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(selection_buttons.pop().unwrap()).spacing(5);
            let mut space = ui_size.button() + 5;
            while space + ui_size.button() < width && selection_buttons.len() > 0 {
                row = row.push(selection_buttons.pop().unwrap()).spacing(5);
                space += ui_size.button() + 5;
            }
            ret = ret.push(row)
        }

        let action_modes = [
            ActionMode::Normal,
            ActionMode::Translate,
            ActionMode::Rotate,
        ];

        let mut action_buttons: Vec<Button<'a, Message>> = self
            .action_mode_state
            .get_states(0, 0, false)
            .into_iter()
            .filter(|(m, _)| action_modes.contains(m))
            .map(|(mode, state)| action_mode_btn(state, mode, action_mode, ui_size.button()))
            .collect();

        ret = ret.push(Text::new("Action Mode"));
        while action_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(action_buttons.remove(0)).spacing(5);
            let mut space = ui_size.button() + 5;
            while space + ui_size.button() < width && action_buttons.len() > 0 {
                row = row.push(action_buttons.remove(0)).spacing(5);
                space += ui_size.button() + 5;
            }
            ret = ret.push(row)
        }

        for view in self
            .helix_roll_factory
            .view(self.roll_target_helices.len() == 1)
            .into_iter()
        {
            ret = ret.push(view);
        }

        let sim_state = &app_state.simulation_state;
        let roll_target_active = sim_state.is_rolling() || self.roll_target_helices.len() > 0;
        ret = ret.push(
            self.roll_target_btn
                .view(roll_target_active, sim_state.is_rolling()),
        );

        let color_square = self.color_picker.color_square();
        if selection_mode == SelectionMode::Strand {
            ret = ret
                .push(self.color_picker.view())
                .push(
                    Row::new()
                        .push(color_square)
                        .push(iced::Space::new(Length::FillPortion(4), Length::Shrink)),
                )
                .push(self.sequence_input.view());
        }

        let mut tighten_helices_button =
            text_btn(&mut self.redim_helices_button, "Selected", ui_size.clone());
        if !self.roll_target_helices.is_empty() {
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

    pub(super) fn update_selection(&mut self, selection: &[DnaElementKey]) {
        self.roll_target_helices.clear();
        for s in selection.iter() {
            if let DnaElementKey::Helix(h) = s {
                self.roll_target_helices.push(*h)
            }
        }
    }

    pub(super) fn update_roll(&mut self, roll: f32) {
        self.helix_roll_factory.update_roll(roll);
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

    pub(super) fn notify_new_design(&mut self) {
        self.roll_target_helices = vec![];
    }

    pub(super) fn get_roll_request(&mut self) -> Option<SimulationRequest> {
        if self.roll_target_helices.len() > 0 {
            Some(SimulationRequest {
                roll: true,
                springs: false,
                target_helices: Some(self.roll_target_helices.clone()),
            })
        } else {
            None
        }
    }

    pub(super) fn strand_color_change(&mut self, color: Color, color_request: &mut Option<u32>) {
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
        *color_request = Some(color);
    }

    pub(super) fn change_hue(&mut self, hue: f32) {
        self.color_picker.change_hue(hue)
    }
}

pub(super) struct GridTab {
    action_mode_state: ActionModeState,
    scroll: iced::scrollable::State,
    helix_pos: isize,
    helix_length: usize,
    pos_str: String,
    length_str: String,
    builder_input: [text_input::State; 2],
    building_hyperboloid: bool,
    finalize_hyperboloid_btn: button::State,
    make_square_grid_btn: button::State,
    make_honeycomb_grid_btn: button::State,
    hyperboloid_factory: RequestFactory<Hyperboloid_>,
    start_hyperboloid_btn: button::State,
    show_strand_menu: bool,
    make_grid_btn: button::State,
    pub(super) can_make_grid: bool,
}

impl GridTab {
    pub fn new() -> Self {
        let default_helix_length = 48;
        Self {
            action_mode_state: Default::default(),
            scroll: Default::default(),
            helix_pos: 0,
            helix_length: default_helix_length,
            pos_str: "0".to_owned(),
            length_str: default_helix_length.to_string().to_owned(),
            builder_input: Default::default(),
            make_square_grid_btn: Default::default(),
            make_honeycomb_grid_btn: Default::default(),
            hyperboloid_factory: RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {}),
            finalize_hyperboloid_btn: Default::default(),
            building_hyperboloid: false,
            start_hyperboloid_btn: Default::default(),
            show_strand_menu: false,
            make_grid_btn: Default::default(),
            can_make_grid: false,
        }
    }

    pub(super) fn view<'a>(
        &'a mut self,
        action_mode: ActionMode,
        ui_size: UiSize,
        width: u16,
    ) -> Element<'a, Message> {
        let action_modes = [
            ActionMode::Normal,
            ActionMode::Translate,
            ActionMode::Rotate,
            self.get_build_helix_mode(),
        ];

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

        let mut inputs = self.builder_input.iter_mut();
        let position_input = TextInput::new(
            inputs.next().unwrap(),
            "Position",
            &self.pos_str,
            Message::PositionHelicesChanged,
        )
        .style(BadValue(self.pos_str == self.helix_pos.to_string()));

        let length_input = TextInput::new(
            inputs.next().unwrap(),
            "Length",
            &self.length_str,
            Message::LengthHelicesChanged,
        )
        .style(BadValue(self.length_str == self.helix_length.to_string()));

        ret = ret.push(right_checkbox(
            self.show_strand_menu,
            "Add double strand on helix",
            Message::AddDoubleStrandHelix,
            ui_size.clone(),
        ));
        let color_white = Color::WHITE;
        let color_gray = Color {
            r: 0.6,
            g: 0.6,
            b: 0.6,
            a: 1.0,
        };
        let color_choose_strand_start_length = if self.show_strand_menu {
            color_white
        } else {
            color_gray
        };
        let row = Row::new()
            .push(
                Column::new()
                    .push(Text::new("Starting nt").color(color_choose_strand_start_length))
                    .push(position_input)
                    .width(Length::Units(width / 2)),
            )
            .push(
                Column::new()
                    .push(Text::new("Length (nt)").color(color_choose_strand_start_length))
                    .push(length_input),
            );
        ret = ret.push(row);

        ret = ret.push(iced::Space::with_height(Length::Units(3)));

        let nanotube_title = Row::new().push(Text::new("New nanotube"));

        ret = ret.push(nanotube_title);
        let start_hyperboloid_btn = if !self.building_hyperboloid {
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

        if self.building_hyperboloid {
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
            .view(self.building_hyperboloid)
            .into_iter()
        {
            ret = ret.push(view);
        }

        let mut action_buttons: Vec<Button<'a, Message>> = self
            .action_mode_state
            .get_states(self.helix_length, self.helix_pos, self.show_strand_menu)
            .into_iter()
            .filter(|(m, _)| action_modes.contains(m))
            .map(|(mode, state)| action_mode_btn(state, mode, action_mode, ui_size.button()))
            .collect();

        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("Action Mode"));
        while action_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(action_buttons.remove(0)).spacing(5);
            let mut space = ui_size.button() + 5;
            while space + ui_size.button() < width && action_buttons.len() > 0 {
                row = row.push(action_buttons.remove(0)).spacing(5);
                space += ui_size.button() + 5;
            }
            ret = ret.push(row)
        }

        ret = ret.push(iced::Space::with_height(Length::Units(5)));
        ret = ret.push(Text::new("Guess grid").size(ui_size.intermediate_text()));
        let mut button_make_grid =
            Button::new(&mut self.make_grid_btn, iced::Text::new("From Selection"))
                .height(Length::Units(ui_size.button()));

        if self.can_make_grid {
            button_make_grid = button_make_grid.on_press(Message::MakeGrids);
        }

        ret = ret.push(button_make_grid);
        ret = ret.push(Text::new("Select ≥4 unattached helices").size(ui_size.main_text()));

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub(super) fn update_pos_str(&mut self, position_str: String) -> ActionMode {
        if let Ok(position) = position_str.parse::<isize>() {
            self.helix_pos = position;
        }
        self.pos_str = position_str;
        self.set_show_strand(true);
        ActionMode::BuildHelix {
            position: self.helix_pos,
            length: self.helix_length,
        }
    }

    pub(super) fn update_length_str(&mut self, length_str: String) -> ActionMode {
        if let Ok(length) = length_str.parse::<usize>() {
            self.helix_length = length
        }
        self.length_str = length_str;
        self.set_show_strand(true);
        ActionMode::BuildHelix {
            position: self.helix_pos,
            length: self.helix_length,
        }
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.builder_input.iter().any(|s| s.is_focused())
    }

    pub fn new_hyperboloid(&mut self, requests: &mut Option<HyperboloidRequest>) {
        self.hyperboloid_factory = RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {});
        self.hyperboloid_factory.make_request(requests);
        self.building_hyperboloid = true;
    }

    pub fn finalize_hyperboloid(&mut self) {
        self.building_hyperboloid = false;
    }

    pub fn is_building_hyperboloid(&self) -> bool {
        self.building_hyperboloid
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

    pub fn get_build_helix_mode(&self) -> ActionMode {
        let (length, position) = if self.show_strand_menu {
            (self.helix_length, self.helix_pos)
        } else {
            (0, 0)
        };
        ActionMode::BuildHelix { length, position }
    }

    pub fn set_show_strand(&mut self, show: bool) {
        self.show_strand_menu = show;
    }

    pub(super) fn notify_new_design(&mut self) {
        self.can_make_grid = false;
    }
}

fn selection_mode_btn<'a>(
    state: &'a mut button::State,
    mode: SelectionMode,
    fixed_mode: SelectionMode,
    button_size: u16,
) -> Button<'a, Message> {
    let icon_path = if fixed_mode == mode {
        mode.icon_on()
    } else {
        mode.icon_off()
    };

    Button::new(state, Image::new(icon_path))
        .on_press(Message::SelectionModeChanged(mode))
        .style(ButtonStyle(fixed_mode == mode))
        .width(Length::Units(button_size))
}

fn action_mode_btn<'a>(
    state: &'a mut button::State,
    mode: ActionMode,
    fixed_mode: ActionMode,
    button_size: u16,
) -> Button<'a, Message> {
    let icon_path = if fixed_mode == mode {
        mode.icon_on()
    } else {
        mode.icon_off()
    };

    Button::new(state, Image::new(icon_path))
        .on_press(Message::ActionModeChanged(mode))
        .style(ButtonStyle(fixed_mode == mode))
        .width(Length::Units(button_size))
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

    pub fn view<'a>(&'a mut self, ui_size: UiSize, width: u16) -> Element<'a, Message> {
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
        ret = ret.push(Text::new("Camera Target"));
        while target_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(target_buttons.remove(0)).spacing(5);
            let mut space = 2 * ui_size.button() + 5;
            while space + 2 * ui_size.button() < width && target_buttons.len() > 0 {
                row = row.push(target_buttons.remove(0)).spacing(5);
                space += 2 * ui_size.button() + 5;
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

use crate::mediator::{Background3D, RenderingMode, ALL_BACKGROUND3D, ALL_RENDERING_MODE};

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

    pub fn view<'a>(&'a mut self, ui_size: UiSize) -> Element<'a, Message> {
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

    pub(super) fn notify_new_design(&mut self) {
        self.fog = Default::default();
    }
}

struct FogParameters {
    visible: bool,
    from_camera: bool,
    radius: f32,
    radius_slider: slider::State,
    length: f32,
    length_slider: slider::State,
    visible_btn: button::State,
    center_btn: button::State,
}

impl FogParameters {
    fn view(&mut self, ui_size: &UiSize) -> Column<Message> {
        let visible_text = if self.visible {
            "Desactivate"
        } else {
            "Activate"
        };
        let center_text = if self.from_camera {
            "Centered on camera"
        } else {
            "Centered on pivot"
        };
        let mut column = Column::new()
            .push(Text::new("Fog").size(ui_size.intermediate_text()))
            .push(
                Row::new()
                    .push(
                        text_btn(&mut self.visible_btn, visible_text, ui_size.clone())
                            .on_press(Message::FogVisibility(!self.visible)),
                    )
                    .push(
                        text_btn(&mut self.center_btn, center_text, ui_size.clone())
                            .on_press(Message::FogCamera(!self.from_camera)),
                    ),
            );

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
            visible_btn: Default::default(),
            center_btn: Default::default(),
        }
    }
}

pub(super) struct SimulationTab {
    rigid_body_factory: RequestFactory<RigidBodyFactory>,
    brownian_factory: RequestFactory<BrownianParametersFactory>,
    rigid_grid_button: GoStop,
    rigid_helices_button: GoStop,
    scroll: scrollable::State,
    physical_simulation: PhysicalSimulation,
}

impl SimulationTab {
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
        }
    }

    pub(super) fn view<'a>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &ApplicationState,
    ) -> Element<'a, Message> {
        let sim_state = &app_state.simulation_state;
        let grid_active = sim_state.is_none() || sim_state.simulating_grid();
        let helices_active = sim_state.is_none() || sim_state.simulating_helices();
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
            .push(
                self.rigid_helices_button
                    .view(helices_active, sim_state.simulating_helices()),
            );

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

    pub(super) fn get_physical_simulation_request(&self) -> SimulationRequest {
        self.physical_simulation.request()
    }

    pub(super) fn leave_tab(
        &mut self,
        requests: Arc<Mutex<Requests>>,
        app_state: &ApplicationState,
    ) {
        if app_state.simulation_state == SimulationState::RigidGrid {
            let request = &mut requests.lock().unwrap().rigid_grid_simulation;
            self.make_rigid_body_request(request);
            println!("stop grids");
        } else if app_state.simulation_state == SimulationState::RigidHelices {
            let request = &mut requests.lock().unwrap().rigid_helices_simulation;
            self.make_rigid_body_request(request);
            println!("stop helices");
        }
    }
}

struct GoStop {
    go_stop_button: button::State,
    pub name: String,
    on_press: Box<dyn Fn(bool) -> Message>,
}

impl GoStop {
    fn new<F>(name: String, on_press: F) -> Self
    where
        F: 'static + Fn(bool) -> Message,
    {
        Self {
            go_stop_button: Default::default(),
            name,
            on_press: Box::new(on_press),
        }
    }

    fn view(&mut self, active: bool, running: bool) -> Row<Message> {
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
    fn view<'a, 'b>(
        &'a mut self,
        _ui_size: &'b UiSize,
        name: &'static str,
        active: bool,
        running: bool,
    ) -> Row<'a, Message> {
        let button_str = if running { "Stop" } else { name };
        let mut button = Button::new(&mut self.go_stop_button, Text::new(button_str))
            .style(ButtonColor::red_green(running));
        if active {
            button = button.on_press(Message::SimRequest);
        }
        Row::new().push(button)
    }

    fn request(&self) -> SimulationRequest {
        SimulationRequest {
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

    pub(super) fn view<'a>(&'a mut self, ui_size: UiSize) -> Element<'a, Message> {
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
    pub scaffold_info: Option<ScaffoldInfo>,
    scaffold_input: text_input::State,
    button_selection_from_scaffold: button::State,
    button_selection_to_scaffold: button::State,
    candidate_scaffold_id: Option<usize>,
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
            scaffold_info: None,
            scaffold_input: Default::default(),
            button_selection_from_scaffold: Default::default(),
            button_selection_to_scaffold: Default::default(),
            candidate_scaffold_id: None,
            button_show_sequence: Default::default(),
        }
    }

    pub(super) fn view<'a>(&'a mut self, ui_size: UiSize) -> Element<'a, Message> {
        let mut ret = Column::new();
        ret = ret.push(Text::new("Sequences").size(ui_size.head_text()));
        ret = ret.push(iced::Space::with_height(Length::Units(3)));
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
        let (scaffold_text, length_text) = if let Some(info) = self.scaffold_info.as_ref() {
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
        if self.scaffold_info.is_none() {
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
        if self.scaffold_info.is_some() {
            button_selection_from_scaffold =
                button_selection_from_scaffold.on_press(Message::SelectScaffold);
        }
        if let Some(n) = self.candidate_scaffold_id {
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
        .on_press(Message::ScaffoldSequenceFile);
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
        let starting_nucl = self
            .scaffold_info
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

    pub(super) fn get_scaffold_pos(&self) -> usize {
        self.scaffold_position
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.scaffold_input.is_focused()
    }

    pub(super) fn update_selection(&mut self, selection: &[DnaElementKey]) {
        self.candidate_scaffold_id = None;
        if selection.len() == 1 {
            if let DnaElementKey::Strand(n) = selection[0] {
                self.candidate_scaffold_id = Some(n);
            }
        }
    }

    pub(super) fn set_scaffold_info(&mut self, info: Option<ScaffoldInfo>) {
        if !self.scaffold_input.is_focused() {
            if let Some(n) = info.as_ref().and_then(|info| info.shift) {
                self.update_pos_str(n.to_string());
            }
        }
        self.scaffold_info = info;
    }
}

fn right_checkbox<'a, F>(
    is_checked: bool,
    label: impl Into<String>,
    f: F,
    ui_size: UiSize,
) -> Element<'a, Message>
where
    F: 'static + Fn(bool) -> Message,
{
    Row::new()
        .push(Text::new(label))
        .push(Checkbox::new(is_checked, "", f).size(ui_size.checkbox()))
        .spacing(CHECKBOXSPACING)
        .into()
}
