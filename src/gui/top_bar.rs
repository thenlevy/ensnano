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
use super::{AppState, UiSize};
use ensnano_interactor::{ActionMode, SelectionMode};
use iced::{container, Background, Container};
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::LogicalSize;
use iced_winit::{button, Button, Color, Command, Element, Length, Program, Row};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::material_icons_light;
use material_icons::{icon_to_char, Icon as MaterialIcon, FONT as MATERIALFONT};
use material_icons_light::LightIcon;

const ICONFONT: iced::Font = iced::Font::External {
    name: "IconFont",
    bytes: MATERIALFONT,
};

const LIGHT_ICONFONT: iced::Font = iced::Font::External {
    name: "IconFontLight",
    bytes: material_icons_light::MATERIAL_ICON_LIGHT,
};

const DARK_ICONFONT: iced::Font = iced::Font::External {
    name: "IconFontDark",
    bytes: material_icons_light::MATERIAL_ICON_DARK,
};

fn icon(icon: MaterialIcon, ui_size: UiSize) -> iced::Text {
    iced::Text::new(format!("{}", icon_to_char(icon)))
        .font(ICONFONT)
        .size(ui_size.icon())
}

fn light_icon(icon: LightIcon, ui_size: UiSize) -> iced::Text {
    iced::Text::new(format!("{}", material_icons_light::icon_to_char(icon)))
        .font(LIGHT_ICONFONT)
        .size(ui_size.icon())
}

fn dark_icon(icon: LightIcon, ui_size: UiSize) -> iced::Text {
    iced::Text::new(format!("{}", material_icons_light::icon_to_char(icon)))
        .font(DARK_ICONFONT)
        .size(ui_size.icon())
}

use super::{Requests, SplitMode};

pub struct TopBar<R: Requests, S: AppState> {
    button_fit: button::State,
    button_add_file: button::State,
    button_save_as: button::State,
    button_save: button::State,
    button_undo: button::State,
    button_redo: button::State,
    button_3d: button::State,
    button_2d: button::State,
    button_split: button::State,
    button_oxdna: button::State,
    button_split_2d: button::State,
    button_flip_split: button::State,
    button_help: button::State,
    button_tutorial: button::State,
    button_reload: button::State,
    button_new_empty_design: button::State,
    requests: Arc<Mutex<R>>,
    logical_size: LogicalSize<f64>,
    action_mode_state: ActionModeState,
    selection_mode_state: SelectionModeState,
    ui_size: UiSize,
    application_state: MainState<S>,
}

#[derive(Debug, Default, Clone)]
pub struct MainState<S: AppState> {
    pub app_state: S,
    pub can_undo: bool,
    pub can_redo: bool,
    pub need_save: bool,
    pub can_reload: bool,
    pub can_split2d: bool,
    pub splited_2d: bool,
}

#[derive(Debug, Clone)]
pub enum Message<S: AppState> {
    SceneFitRequested,
    OpenFileButtonPressed,
    FileSaveRequested,
    SaveAsRequested,
    Resize(LogicalSize<f64>),
    ToggleView(SplitMode),
    UiSizeChanged(UiSize),
    OxDNARequested,
    Split2d,
    NewApplicationState(MainState<S>),
    ForceHelp,
    ShowTutorial,
    Undo,
    Redo,
    ButtonNewEmptyDesignPressed,
    ActionModeChanged(ActionMode),
    SelectionModeChanged(SelectionMode),
    Reload,
    FlipSplitViews,
}

impl<R: Requests, S: AppState> TopBar<R, S> {
    pub fn new(requests: Arc<Mutex<R>>, logical_size: LogicalSize<f64>) -> Self {
        Self {
            button_fit: Default::default(),
            button_add_file: Default::default(),
            button_save_as: Default::default(),
            button_save: Default::default(),
            button_undo: Default::default(),
            button_redo: Default::default(),
            button_2d: Default::default(),
            button_3d: Default::default(),
            button_split: Default::default(),
            button_oxdna: Default::default(),
            button_split_2d: Default::default(),
            button_flip_split: Default::default(),
            button_help: Default::default(),
            button_tutorial: Default::default(),
            button_new_empty_design: Default::default(),
            button_reload: Default::default(),
            requests,
            logical_size,
            action_mode_state: Default::default(),
            selection_mode_state: Default::default(),
            ui_size: Default::default(),
            application_state: Default::default(),
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize<f64>) {
        self.logical_size = logical_size;
    }

    fn get_build_helix_mode(&self) -> ActionMode {
        self.application_state.app_state.get_build_helix_mode()
    }
}

impl<R: Requests, S: AppState> Program for TopBar<R, S> {
    type Renderer = Renderer;
    type Message = Message<S>;

    fn update(&mut self, message: Message<S>) -> Command<Message<S>> {
        match message {
            Message::SceneFitRequested => {
                self.requests.lock().unwrap().fit_design_in_scenes();
            }
            Message::OpenFileButtonPressed => {
                self.requests.lock().unwrap().open_file();
            }
            Message::SaveAsRequested => {
                self.requests.lock().unwrap().save_as();
            }
            Message::FileSaveRequested => {
                self.requests.lock().unwrap().save();
            }
            Message::Resize(size) => self.resize(size),
            Message::ToggleView(b) => self.requests.lock().unwrap().change_split_mode(b),
            Message::UiSizeChanged(ui_size) => self.ui_size = ui_size,
            Message::OxDNARequested => self.requests.lock().unwrap().export_to_oxdna(),
            Message::Split2d => self.requests.lock().unwrap().toggle_2d_view_split(),
            Message::NewApplicationState(state) => self.application_state = state,
            Message::Undo => self.requests.lock().unwrap().undo(),
            Message::Redo => self.requests.lock().unwrap().redo(),
            Message::ForceHelp => self.requests.lock().unwrap().force_help(),
            Message::ShowTutorial => self.requests.lock().unwrap().show_tutorial(),
            Message::ButtonNewEmptyDesignPressed => self.requests.lock().unwrap().new_design(),
            Message::Reload => self.requests.lock().unwrap().reload_file(),
            Message::SelectionModeChanged(selection_mode) => {
                if selection_mode != self.application_state.app_state.get_selection_mode() {
                    self.requests
                        .lock()
                        .unwrap()
                        .change_selection_mode(selection_mode);
                }
            }
            Message::ActionModeChanged(action_mode) => {
                if self.application_state.app_state.get_action_mode() != action_mode {
                    self.requests
                        .lock()
                        .unwrap()
                        .change_action_mode(action_mode)
                } else {
                    match action_mode {
                        ActionMode::Rotate | ActionMode::Translate => {
                            self.requests.lock().unwrap().toggle_widget_basis();
                        }
                        _ => (),
                    }
                }
            }
            Message::FlipSplitViews => self.requests.lock().unwrap().flip_split_views(),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message<S>, Renderer> {
        let build_helix_mode = self.get_build_helix_mode();
        let action_modes = [
            ActionMode::Normal,
            ActionMode::Translate,
            ActionMode::Rotate,
            build_helix_mode.clone(),
        ];
        let height = self.logical_size.cast::<u16>().height;
        let button_fit = Button::new(
            &mut self.button_fit,
            light_icon(LightIcon::ViewInAr, self.ui_size.clone()),
        )
        .on_press(Message::SceneFitRequested)
        .height(Length::Units(height));

        let button_new_empty_design = Button::new(
            &mut self.button_new_empty_design,
            light_icon(LightIcon::InsertDriveFile, self.ui_size.clone()),
        )
        .on_press(Message::ButtonNewEmptyDesignPressed);

        let button_add_file = Button::new(
            &mut self.button_add_file,
            light_icon(LightIcon::FolderOpen, self.ui_size.clone()),
        )
        .on_press(Message::OpenFileButtonPressed);

        let mut button_reload = Button::new(
            &mut self.button_reload,
            light_icon(LightIcon::RestorePage, self.ui_size),
        );

        if self.application_state.can_reload {
            button_reload = button_reload.on_press(Message::Reload);
        }

        let save_message = Message::FileSaveRequested;
        /*
        let button_save = bottom_tooltip_icon_btn(
            &mut self.button_save,
            MaterialIcon::Save,
            &top_size_info,
            "Save As..",
            Some(save_message),
        );*/
        let button_save = if self.application_state.need_save {
            Button::new(
                &mut self.button_save,
                icon(MaterialIcon::Save, self.ui_size.clone()),
            )
            .on_press(save_message)
        } else {
            Button::new(
                &mut self.button_save,
                light_icon(LightIcon::Save, self.ui_size.clone()),
            )
            .on_press(save_message)
        };

        let button_save_as = if self.application_state.need_save {
            Button::new(
                &mut self.button_save_as,
                dark_icon(LightIcon::DriveFileMove, self.ui_size.clone()),
            )
            .on_press(Message::SaveAsRequested)
        } else {
            Button::new(
                &mut self.button_save_as,
                light_icon(LightIcon::DriveFileMove, self.ui_size.clone()),
            )
            .on_press(Message::SaveAsRequested)
        };

        let mut button_undo = Button::new(
            &mut self.button_undo,
            icon(MaterialIcon::Undo, self.ui_size.clone()),
        );
        if self.application_state.can_undo {
            button_undo = button_undo.on_press(Message::Undo)
        }

        let mut button_redo = Button::new(
            &mut self.button_redo,
            icon(MaterialIcon::Redo, self.ui_size.clone()),
        );
        if self.application_state.can_redo {
            button_redo = button_redo.on_press(Message::Redo)
        }

        let button_2d = Button::new(&mut self.button_2d, iced::Text::new("2D"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ToggleView(SplitMode::Flat));
        let button_3d = Button::new(&mut self.button_3d, iced::Text::new("3D"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ToggleView(SplitMode::Scene3D));
        let button_split = Button::new(&mut self.button_split, iced::Text::new("3D+2D"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ToggleView(SplitMode::Both));

        let button_oxdna = Button::new(&mut self.button_oxdna, iced::Text::new("To OxView"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::OxDNARequested);
        let oxdna_tooltip = button_oxdna;

        let split_icon = if self.application_state.splited_2d {
            LightIcon::BorderOuter
        } else {
            LightIcon::BorderHorizontal
        };

        let mut button_split_2d = Button::new(
            &mut self.button_split_2d,
            light_icon(split_icon, self.ui_size),
        )
        .height(Length::Units(self.ui_size.button()));

        if self.application_state.can_split2d {
            button_split_2d = button_split_2d.on_press(Message::Split2d);
        }

        let mut button_flip_split = Button::new(
            &mut self.button_flip_split,
            light_icon(LightIcon::SwapVert, self.ui_size),
        )
        .height(Length::Units(self.ui_size.button()));
        if self.application_state.splited_2d {
            button_flip_split = button_flip_split.on_press(Message::FlipSplitViews);
        }

        let button_help = Button::new(&mut self.button_help, iced::Text::new("Help"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ForceHelp);

        let button_tutorial = Button::new(&mut self.button_tutorial, iced::Text::new("Tutorials"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ShowTutorial);

        let app_state = &self.application_state.app_state;
        let ui_size = self.ui_size.clone();
        let action_buttons: Vec<Button<Message<S>, _>> = self
            .action_mode_state
            .get_states(build_helix_mode)
            .into_iter()
            .filter(|(m, _)| action_modes.contains(m))
            .map(|(mode, state)| {
                action_mode_btn(
                    state,
                    mode,
                    app_state.get_action_mode(),
                    ui_size.button(),
                    app_state.get_widget_basis().is_axis_aligned(),
                )
            })
            .collect();

        let selection_modes = [
            SelectionMode::Helix,
            SelectionMode::Strand,
            SelectionMode::Nucleotide,
        ];

        let selection_buttons: Vec<_> = self
            .selection_mode_state
            .get_states()
            .into_iter()
            .filter(|(m, _)| selection_modes.contains(m))
            .map(|(mode, state)| {
                selection_mode_btn(
                    state,
                    mode,
                    app_state.get_selection_mode(),
                    ui_size.button(),
                )
            })
            .collect();

        let mut buttons = Row::new()
            .width(Length::Fill)
            .height(Length::Units(height))
            .push(button_new_empty_design)
            .push(button_add_file)
            .push(button_reload)
            .push(button_save)
            .push(button_save_as)
            .push(oxdna_tooltip)
            .push(iced::Space::with_width(Length::Units(10)))
            .push(button_3d)
            .push(button_2d)
            .push(button_split)
            .push(button_split_2d)
            .push(button_flip_split)
            .push(iced::Space::with_width(Length::Units(10)))
            .push(button_fit)
            .push(iced::Space::with_width(Length::Units(10)))
            .push(button_undo)
            .push(button_redo)
            .push(iced::Space::with_width(Length::Units(10)));

        for button in action_buttons.into_iter() {
            buttons = buttons.push(button);
        }

        buttons = buttons.push(iced::Space::with_width(Length::Units(10)));

        for button in selection_buttons.into_iter() {
            buttons = buttons.push(button);
        }

        buttons = buttons.push(iced::Space::with_width(Length::Units(10)));

        buttons = buttons
            .push(button_help)
            .push(iced::Space::with_width(Length::Units(2)))
            .push(button_tutorial)
            .push(
                iced::Text::new("\u{e91c}")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Right)
                    .vertical_alignment(iced::alignment::Vertical::Center),
            )
            .push(iced::Space::with_width(Length::Units(10)));

        Container::new(buttons)
            .width(Length::Units(self.logical_size.width as u16))
            .style(TopBarStyle)
            .into()
    }
}

struct TopBarStyle;
impl container::StyleSheet for TopBarStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(BACKGROUND)),
            text_color: Some(Color::WHITE),
            ..container::Style::default()
        }
    }
}

pub const BACKGROUND: Color = Color::from_rgb(
    0x36 as f32 / 255.0,
    0x39 as f32 / 255.0,
    0x3F as f32 / 255.0,
);

#[derive(Clone)]
struct TopSizeInfo {
    ui_size: UiSize,
    height: iced::Length,
}

struct ToolTipStyle;
impl iced::container::StyleSheet for ToolTipStyle {
    fn style(&self) -> iced::container::Style {
        iced::container::Style {
            text_color: Some(iced::Color::BLACK),
            ..Default::default()
        }
    }
}

#[derive(Default, Debug, Clone)]
struct SelectionModeState {
    pub nucleotide: button::State,
    pub strand: button::State,
    pub helix: button::State,
    pub grid: button::State,
}

impl SelectionModeState {
    fn get_states<'a>(&'a mut self) -> BTreeMap<SelectionMode, &'a mut button::State> {
        let mut ret = BTreeMap::new();
        ret.insert(SelectionMode::Nucleotide, &mut self.nucleotide);
        ret.insert(SelectionMode::Strand, &mut self.strand);
        ret.insert(SelectionMode::Helix, &mut self.helix);
        ret.insert(SelectionMode::Grid, &mut self.grid);
        ret
    }
}

#[derive(Default, Debug, Clone)]
struct ActionModeState {
    pub select: button::State,
    pub translate: button::State,
    pub rotate: button::State,
    pub build: button::State,
    pub cut: button::State,
    pub add_grid: button::State,
    pub add_hyperboloid: button::State,
}

impl ActionModeState {
    fn get_states<'a>(
        &'a mut self,
        build_helix_mode: ActionMode,
    ) -> BTreeMap<ActionMode, &'a mut button::State> {
        let mut ret = BTreeMap::new();
        ret.insert(ActionMode::Normal, &mut self.select);
        ret.insert(ActionMode::Translate, &mut self.translate);
        ret.insert(ActionMode::Rotate, &mut self.rotate);
        ret.insert(build_helix_mode, &mut self.build);
        ret
    }
}

struct ButtonStyle(bool);

impl iced_wgpu::button::StyleSheet for ButtonStyle {
    fn active(&self) -> iced_wgpu::button::Style {
        iced_wgpu::button::Style {
            border_width: if self.0 { 3_f32 } else { 1_f32 },
            border_radius: if self.0 { 3_f32 } else { 2_f32 },
            border_color: if self.0 {
                Color::BLACK
            } else {
                [0.7, 0.7, 0.7].into()
            },
            background: Some(Background::Color([0.87, 0.87, 0.87].into())),
            //background: Some(Background::Color(BACKGROUND)),
            ..Default::default()
        }
    }
}

use super::icon::{HasIcon, HasIconDependentOnAxis};
use iced::Image;
fn action_mode_btn<'a, S: AppState>(
    state: &'a mut button::State,
    mode: ActionMode,
    fixed_mode: ActionMode,
    button_size: u16,
    axis_aligned: bool,
) -> Button<'a, Message<S>, Renderer> {
    let icon_path = if fixed_mode == mode {
        mode.icon_on(axis_aligned)
    } else {
        mode.icon_off(axis_aligned)
    };

    Button::new(state, Image::new(icon_path))
        .on_press(Message::ActionModeChanged(mode))
        .style(ButtonStyle(fixed_mode == mode))
        .width(Length::Units(button_size))
}

fn selection_mode_btn<'a, S: AppState>(
    state: &'a mut button::State,
    mode: SelectionMode,
    fixed_mode: SelectionMode,
    button_size: u16,
) -> Button<'a, Message<S>, Renderer> {
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
