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
use super::super::DesignReader;
use super::*;
use ensnano_interactor::Selection;
use iced::{scrollable, Scrollable};

mod value_constructor;
use value_constructor::{Builder, GridBuilder};
pub use value_constructor::{BuilderMessage, InstanciatedValue, ValueKind};

use ultraviolet::{Rotor3, Vec3};
pub enum ValueRequest {
    GridPosition { grid_id: usize, position: Vec3 },
    GridOrientation { grid_id: usize, orientation: Rotor3 },
}

impl ValueRequest {
    fn from_value_and_selection(selection: &Selection, value: InstanciatedValue) -> Option<Self> {
        match value {
            InstanciatedValue::GridPosition(v) => {
                if let Selection::Grid(_, g_id) = selection {
                    Some(Self::GridPosition {
                        grid_id: *g_id,
                        position: v,
                    })
                } else {
                    log::error!("Recieved value {:?} with selection {:?}", value, selection);
                    None
                }
            }
            InstanciatedValue::GridOrientation(orientation) => {
                if let Selection::Grid(_, g_id) = selection {
                    Some(Self::GridOrientation {
                        grid_id: *g_id,
                        orientation,
                    })
                } else {
                    log::error!("Recieved value {:?} with selection {:?}", value, selection);
                    None
                }
            }
        }
    }

    pub(super) fn make_request(&self, request: Arc<Mutex<dyn Requests>>) {
        match self {
            Self::GridPosition { grid_id, position } => request
                .lock()
                .unwrap()
                .set_grid_position(*grid_id, *position),
            Self::GridOrientation {
                grid_id,
                orientation,
            } => request
                .lock()
                .unwrap()
                .set_grid_orientation(*grid_id, *orientation),
        }
    }
}

struct InstantiatedBuilder<S: AppState> {
    selection: Selection,
    builder: Box<dyn Builder<S>>,
}

impl<S: AppState> InstantiatedBuilder<S> {
    /// If a builder can be made from the selection, update the builder and return true. Otherwise,
    /// return false.
    fn update(&mut self, selection: &Selection, reader: &dyn DesignReader) -> bool {
        if *selection != self.selection {
            self.selection = selection.clone();
            if let Some(builder) = Self::new_builder(selection, reader) {
                self.builder = builder;
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    fn new(selection: &Selection, reader: &dyn DesignReader) -> Option<Self> {
        if let Some(builder) = Self::new_builder(selection, reader) {
            Some(Self {
                builder,
                selection: selection.clone(),
            })
        } else {
            None
        }
    }

    fn new_builder(
        selection: &Selection,
        reader: &dyn DesignReader,
    ) -> Option<Box<dyn Builder<S>>> {
        match selection {
            Selection::Grid(_, g_id) => {
                if let Some((position, orientation)) =
                    reader.get_grid_position_and_orientation(*g_id)
                {
                    Some(Box::new(GridBuilder::new(position, orientation)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub(super) struct ContextualPanel<S: AppState> {
    scroll: scrollable::State,
    width: u32,
    pub force_help: bool,
    pub show_tutorial: bool,
    help_btn: button::State,
    ens_nano_website: button::State,
    add_strand_menu: AddStrandMenu,
    strand_name_state: text_input::State,
    builder: Option<InstantiatedBuilder<S>>,
}

impl<S: AppState> ContextualPanel<S> {
    pub fn new(width: u32) -> Self {
        Self {
            scroll: Default::default(),
            width,
            force_help: false,
            show_tutorial: false,
            help_btn: Default::default(),
            ens_nano_website: Default::default(),
            add_strand_menu: Default::default(),
            strand_name_state: Default::default(),
            builder: None,
        }
    }

    pub fn new_width(&mut self, width: u32) {
        self.width = width;
    }

    fn update_builder(&mut self, selection: Option<&Selection>, reader: &dyn DesignReader) {
        if let Some(s) = selection {
            if let Some(builder) = &mut self.builder {
                if !builder.update(s, reader) {
                    self.builder = None;
                }
            } else {
                self.builder = InstantiatedBuilder::new(s, reader)
            }
        } else {
            self.builder = None;
        }
    }

    pub fn view(&mut self, ui_size: UiSize, app_state: &S) -> Element<Message<S>> {
        let mut column = Column::new().max_width(self.width - 2);
        let selection = app_state
            .get_selection()
            .get(0)
            .unwrap_or(&Selection::Nothing);
        let nb_selected = app_state
            .get_selection()
            .iter()
            .filter(|s| !matches!(s, Selection::Nothing))
            .count();

        self.update_builder(
            Some(selection).filter(|_| nb_selected == 1),
            app_state.get_reader().as_ref(),
        );
        let info_values = values_of_selection(selection, app_state.get_reader().as_ref());
        if self.show_tutorial {
            column = column.push(
                Text::new("Tutorials")
                    .size(ui_size.head_text())
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            );
            column = column.push(Text::new("ENSnano website"));
            column = column.push(link_row(
                &mut self.ens_nano_website,
                "http://ens-lyon.fr/ensnano",
                ui_size.clone(),
            ));
        } else if self.force_help {
            column = turn_into_help_column(column, ui_size)
        } else if app_state.get_action_mode().is_build() {
            let strand_menu = self.add_strand_menu.view(ui_size, self.width as u16);
            column = column.push(strand_menu);
        } else if *selection == Selection::Nothing {
            column = turn_into_help_column(column, ui_size)
        } else if nb_selected > 1 {
            let help_btn =
                text_btn(&mut self.help_btn, "Help", ui_size.clone()).on_press(Message::ForceHelp);
            column = column.push(
                Row::new()
                    .width(Length::Fill)
                    .push(iced::Space::with_width(Length::FillPortion(1)))
                    .align_items(iced::Alignment::Center)
                    .push(Column::new().width(Length::FillPortion(1)).push(help_btn))
                    .push(iced::Space::with_width(Length::FillPortion(1))),
            );
            column = column.push(Text::new(format!("{} objects selected", nb_selected)));
        } else {
            let help_btn =
                text_btn(&mut self.help_btn, "Help", ui_size.clone()).on_press(Message::ForceHelp);
            column = column.push(
                Row::new()
                    .width(Length::Fill)
                    .push(iced::Space::with_width(Length::FillPortion(1)))
                    .align_items(iced::Alignment::Center)
                    .push(Column::new().width(Length::FillPortion(1)).push(help_btn))
                    .push(iced::Space::with_width(Length::FillPortion(1))),
            );
            column = column.push(Text::new(selection.info()).size(ui_size.main_text()));

            match selection {
                Selection::Grid(_, _) => {
                    column = add_grid_content(column, info_values.as_slice(), ui_size.clone())
                }
                Selection::Strand(_, _) => {
                    column = add_strand_content(
                        column,
                        &mut self.strand_name_state,
                        info_values.as_slice(),
                        ui_size.clone(),
                    )
                }
                Selection::Nucleotide(_, _) => {
                    let anchor = info_values[0].clone();
                    column = column.push(Text::new(format!("Anchor {}", anchor)));
                }
                _ => (),
            }
            if let Some(builder) = &mut self.builder {
                column = column.push(builder.builder.view(ui_size))
            }
        }

        Scrollable::new(&mut self.scroll).push(column).into()
    }

    pub fn selection_value_changed<R: Requests>(
        &mut self,
        _n: usize,
        s: String,
        requests: Arc<Mutex<R>>,
    ) {
        if let Ok(g_id) = s.parse() {
            requests
                .lock()
                .unwrap()
                .toggle_helices_persistance_of_grid(g_id);
        }
    }

    pub fn set_small_sphere<R: Requests>(&mut self, b: bool, requests: Arc<Mutex<R>>) {
        requests.lock().unwrap().set_small_sphere(b);
    }

    pub fn scaffold_id_set<R: Requests>(&mut self, n: usize, b: bool, requests: Arc<Mutex<R>>) {
        if b {
            requests.lock().unwrap().set_scaffold_id(Some(n))
        } else {
            requests.lock().unwrap().set_scaffold_id(None)
        }
    }

    pub fn state_updated(&mut self) {
        self.force_help = false;
        self.show_tutorial = false;
    }

    pub(super) fn update_pos_str(&mut self, position_str: String) -> (isize, usize) {
        self.add_strand_menu.update_pos_str(position_str)
    }

    pub(super) fn update_length_str(&mut self, length_str: String) -> (isize, usize) {
        self.add_strand_menu.update_length_str(length_str)
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.add_strand_menu.has_keyboard_priority()
            || self.strand_name_state.is_focused()
            || self.builder_has_keyboard_priority()
    }

    fn builder_has_keyboard_priority(&self) -> bool {
        self.builder
            .as_ref()
            .map(|b| b.builder.has_keyboard_priority())
            .unwrap_or(false)
    }

    pub fn get_build_helix_mode(&self) -> ActionMode {
        self.add_strand_menu.get_build_helix_mode()
    }

    pub fn get_new_strand_parameters(&self) -> Option<(isize, usize)> {
        self.add_strand_menu.get_new_strand_parameters()
    }

    pub fn set_show_strand(&mut self, show: bool) {
        self.add_strand_menu.set_show_strand(show)
    }

    pub fn update_builder_value(&mut self, kind: ValueKind, n: usize, value: String) {
        if let Some(b) = &mut self.builder {
            b.builder.update_str_value(kind, n, value)
        } else {
            log::error!("Cannot update value: No instanciated builder");
        }
    }

    pub fn submit_value(&mut self, kind: ValueKind) -> Option<ValueRequest> {
        if let Some(b) = &mut self.builder {
            if let Some(value) = b.builder.submit_value(kind) {
                ValueRequest::from_value_and_selection(&b.selection, value)
            } else {
                None
            }
        } else {
            log::error!("Cannot submit value: No instanciated builder");
            None
        }
    }
}

fn add_grid_content<'a, S: AppState, I: std::ops::Deref<Target = str>>(
    mut column: Column<'a, Message<S>>,
    info_values: &[I],
    ui_size: UiSize,
) -> Column<'a, Message<S>> {
    column = column.push(
        Checkbox::new(
            info_values[0].parse::<bool>().unwrap(),
            "Persistent phantoms",
            |b| Message::SelectionValueChanged(0, bool_to_string(b)),
        )
        .size(ui_size.checkbox())
        .text_size(ui_size.main_text()),
    );
    column = column.push(
        Checkbox::new(info_values[1].parse::<bool>().unwrap(), "No sphere", |b| {
            Message::SetSmallSpheres(b)
        })
        .size(ui_size.checkbox())
        .text_size(ui_size.main_text()),
    );
    column
}

fn add_strand_content<'a, S: AppState, I: std::ops::Deref<Target = str>>(
    mut column: Column<'a, Message<S>>,
    strand_name_state: &'a mut text_input::State,
    info_values: &[I],
    ui_size: UiSize,
) -> Column<'a, Message<S>> {
    let s_id = info_values[2].parse::<usize>().unwrap();
    let name_row = Row::new()
        .push(Text::new(format!("Name")).size(ui_size.main_text()))
        .push(
            TextInput::new(
                strand_name_state,
                "Name",
                &info_values[4],
                move |new_name| Message::StrandNameChanged(s_id, new_name),
            )
            .size(ui_size.main_text()),
        );
    column = column.push(name_row);
    column = column
        .push(Text::new(format!("length {}", info_values[0].deref())).size(ui_size.main_text()));
    column = column.push(Checkbox::new(
        info_values[1].parse().unwrap(),
        "Scaffold",
        move |b| Message::ScaffoldIdSet(s_id, b),
    ));
    column = column.push(Text::new(info_values[3].deref()).size(ui_size.main_text()));
    column
}

fn bool_to_string(b: bool) -> String {
    if b {
        String::from("true")
    } else {
        String::from("false")
    }
}

fn add_help_to_column<'a, M: 'static>(
    mut column: Column<'a, M>,
    help_title: impl Into<String>,
    help: Vec<(String, String)>,
    ui_size: UiSize,
) -> Column<'a, M> {
    column = column.push(Text::new(help_title).size(ui_size.intermediate_text()));
    for (l, r) in help {
        if l.is_empty() {
            column = column.push(iced::Space::with_height(Length::Units(10)));
        } else if r.is_empty() {
            column = column.push(
                Text::new(l)
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            );
        } else {
            column = column.push(
                Row::new()
                    .push(
                        Text::new(l)
                            .width(Length::FillPortion(5))
                            .horizontal_alignment(iced::alignment::Horizontal::Right),
                    )
                    .push(iced::Space::with_width(Length::FillPortion(1)))
                    .push(Text::new(r).width(Length::FillPortion(5))),
            );
        }
    }
    column
}

fn turn_into_help_column<'a, M: 'static>(
    mut column: Column<'a, M>,
    ui_size: UiSize,
) -> Column<'a, M> {
    column = column.push(
        Text::new("Help")
            .size(ui_size.head_text())
            .width(Length::Fill)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    );
    column = add_help_to_column(column, "3D view", view_3d_help(), ui_size.clone());
    column = column.push(iced::Space::with_height(Length::Units(15)));
    column = add_help_to_column(column, "2D/3D view", view_2d_3d_help(), ui_size.clone());
    column = column.push(iced::Space::with_height(Length::Units(15)));
    column = add_help_to_column(column, "2D view", view_2d_help(), ui_size.clone());
    column
}

fn view_3d_help() -> Vec<(String, String)> {
    vec![
        (
            format!("{}", LCLICK),
            "Select\nnt → strand → helix".to_owned(),
        ),
        (
            format!("{}+{}", SHIFT, LCLICK),
            "Multiple select".to_owned(),
        ),
        (String::new(), String::new()),
        (
            format!("2x{}", LCLICK),
            "Center selection in 2D view".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("{} Drag", MCLICK), "Translate camera".to_owned()),
        (
            format!("{}+{} Drag", ALT, LCLICK),
            "Translate camera".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("{}", RCLICK), "Set pivot".to_owned()),
        (
            format!("{} Drag", RCLICK),
            "Rotate camera around pivot".to_owned(),
        ),
        (
            format!("{}+{} Drag", CTRL, LCLICK),
            "Rotate camera around pivot".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("{} Drag", LCLICK), "Edit strand".to_owned()),
        (
            format!("long {} Drag", LCLICK),
            "Make crossover (drop on nt)".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("When in 3D {} mode", MOVECHAR), String::new()),
        (
            format!("{} on handle", LCLICK),
            "Move selected object".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("When in 3D {} mode", ROTCHAR), String::new()),
        (
            format!("{} on handle", LCLICK),
            "Rotate selected object".to_owned(),
        ),
    ]
}

fn view_2d_3d_help() -> Vec<(String, String)> {
    vec![
        (format!("{} + C", CTRL), "Copy selection".to_owned()),
        (format!("{} + V", CTRL), "Paste".to_owned()),
        (format!("{} + J", CTRL), "Paste & repeat".to_owned()),
        (String::new(), String::new()),
        (
            format!("{} or {}", SUPPRCHAR, BACKSPACECHAR),
            "Delete selected strands".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("{} + S", CTRL), "Save design".to_owned()),
        (format!("{} + O", CTRL), "Open design".to_owned()),
        (format!("{} + Z", CTRL), "Undo".to_owned()),
        (format!("{} + R", CTRL), "Redo".to_owned()),
        (String::new(), String::new()),
        ("Selection mode shortcuts".to_owned(), "".to_owned()),
        ("'N' key".to_owned(), format!("Nucleotide, ({})", NUCLCHAR)),
        ("'S' key".to_owned(), format!("Strand ({})", STRANDCHAR)),
        ("'H' key".to_owned(), format!("Helix ({})", HELIXCHAR)),
        (String::new(), String::new()),
        ("Action mode shortcuts".to_owned(), "".to_owned()),
        ("ESC".to_owned(), format!("Select ({})", SELECTCHAR)),
        ("'T' key".to_owned(), format!("Translation ({})", MOVECHAR)),
        ("'R' key".to_owned(), format!("Rotation ({})", ROTCHAR)),
    ]
}

fn view_2d_help() -> Vec<(String, String)> {
    vec![
        (format!("{} Drag", MCLICK), "Translate camera".to_owned()),
        (
            format!("{} + {} Drag", ALT, LCLICK),
            "Translate camera".to_owned(),
        ),
        (String::new(), String::new()),
        (format!("{}", LCLICK), "Select".to_owned()),
        (
            format!("{} + {}", SHIFT, LCLICK),
            "Multiple Select".to_owned(),
        ),
        (
            format!("{} Drag", LCLICK),
            "Rectangular selection".to_owned(),
        ),
        (String::new(), String::new()),
        ("On helix numbers".to_owned(), String::new()),
        (format!("{}", LCLICK), "Select helix".to_owned()),
        (
            format!("{} + {}", SHIFT, LCLICK),
            "Multiple select".to_owned(),
        ),
        (
            format!("{} Drag", LCLICK),
            "Translate selected helices".to_owned(),
        ),
        (
            format!("{} Drag", RCLICK),
            "Rotate selected helices".to_owned(),
        ),
        (String::new(), String::new()),
        ("On nucleotides".to_owned(), String::new()),
        (
            format!("{}", RCLICK),
            "cut/glue strand or double xover".to_owned(),
        ),
        (
            format!("{} Drag", LCLICK),
            "edit strand/crossover".to_owned(),
        ),
        (
            format!("{} + {}", CTRL, LCLICK),
            "Make suggested crossover".to_owned(),
        ),
    ]
}

fn link_row<'a, S: AppState>(
    button_state: &'a mut button::State,
    link: &'static str,
    ui_size: UiSize,
) -> Row<'a, Message<S>> {
    Row::new()
        .push(
            Column::new()
                .push(Text::new(link))
                .width(Length::FillPortion(3)),
        )
        .push(
            Column::new()
                .push(text_btn(button_state, "Go", ui_size).on_press(Message::OpenLink(link)))
                .width(Length::FillPortion(1)),
        )
}

fn values_of_selection(selection: &Selection, reader: &dyn DesignReader) -> Vec<String> {
    match selection {
        Selection::Grid(_, g_id) => {
            let b1 = reader.grid_has_persistent_phantom(*g_id);
            let b2 = reader.grid_has_small_spheres(*g_id);
            let mut ret: Vec<String> = vec![b1, b2]
                .iter()
                .map(|b| {
                    if *b {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    }
                })
                .collect();
            if let Some(f) = reader.get_grid_shift(*g_id) {
                ret.push(f.to_string());
            }
            ret
        }
        Selection::Strand(_, s_id) => vec![
            format!(
                "{:?}",
                reader.get_strand_length(*s_id as usize).unwrap_or(0)
            ),
            format!("{:?}", reader.is_id_of_scaffold(*s_id as usize)),
            s_id.to_string(),
            reader.length_decomposition(*s_id as usize),
            reader.strand_name(*s_id as usize),
        ],
        Selection::Nucleotide(_, nucl) => {
            vec![format!("{}", reader.nucl_is_anchor(*nucl))]
        }
        _ => Vec::new(),
    }
}

struct AddStrandMenu {
    helix_pos: isize,
    helix_length: usize,
    pos_str: String,
    length_str: String,
    text_inputs_are_active: bool,
    builder_input: [text_input::State; 2],
}

impl Default for AddStrandMenu {
    fn default() -> Self {
        Self {
            helix_pos: 0,
            helix_length: 0,
            pos_str: "0".into(),
            length_str: "0".into(),
            text_inputs_are_active: false,
            builder_input: Default::default(),
        }
    }
}

impl AddStrandMenu {
    fn update_pos_str(&mut self, position_str: String) -> (isize, usize) {
        if let Ok(position) = position_str.parse::<isize>() {
            self.helix_pos = position;
        }
        self.pos_str = position_str;
        self.set_show_strand(true);
        (self.helix_pos, self.helix_length)
    }

    fn update_length_str(&mut self, length_str: String) -> (isize, usize) {
        if let Ok(length) = length_str.parse::<usize>() {
            self.helix_length = length
        }
        self.length_str = length_str;
        self.set_show_strand(true);
        (self.helix_pos, self.helix_length)
    }

    fn has_keyboard_priority(&self) -> bool {
        self.builder_input.iter().any(|s| s.is_focused())
    }

    fn get_build_helix_mode(&self) -> ActionMode {
        let (length, position) = if self.text_inputs_are_active {
            (self.helix_length, self.helix_pos)
        } else {
            (0, 0)
        };
        ActionMode::BuildHelix { length, position }
    }

    fn get_new_strand_parameters(&self) -> Option<(isize, usize)> {
        if self.text_inputs_are_active {
            Some((self.helix_pos, self.helix_length))
        } else {
            None
        }
    }

    fn set_show_strand(&mut self, show: bool) {
        self.text_inputs_are_active = show;
    }

    fn view<'a, S: AppState>(&'a mut self, ui_size: UiSize, width: u16) -> Element<'a, Message<S>> {
        let mut ret = Column::new();
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
            self.text_inputs_are_active,
            "Add double strand on helix",
            Message::AddDoubleStrandHelix,
            ui_size,
        ));
        let color_white = Color::WHITE;
        let color_gray = Color {
            r: 0.6,
            g: 0.6,
            b: 0.6,
            a: 1.0,
        };
        let color_choose_strand_start_length = if self.text_inputs_are_active {
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
        ret.into()
    }
}
