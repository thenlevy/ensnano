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
use crate::mediator::Selection;
use iced::{scrollable, Scrollable};
use std::borrow::Cow;

pub(super) struct ContextualPanel {
    selection: Selection,
    info_values: Vec<Cow<'static, str>>,
    scroll: scrollable::State,
    width: u32,
    pub force_help: bool,
    pub show_tutorial: bool,
    help_btn: button::State,
    ens_nano_website: button::State,
}

impl ContextualPanel {
    pub fn new(width: u32) -> Self {
        Self {
            selection: Selection::Nothing,
            info_values: vec![],
            scroll: Default::default(),
            width,
            force_help: false,
            show_tutorial: false,
            help_btn: Default::default(),
            ens_nano_website: Default::default(),
        }
    }

    pub fn new_width(&mut self, width: u32) {
        self.width = width;
    }

    pub fn view(&mut self, ui_size: UiSize) -> Element<Message> {
        let mut column = Column::new().max_width(self.width - 2);
        let selection = &self.selection;
        if self.show_tutorial {
            column = column.push(
                Text::new("Tutorials")
                    .size(ui_size.head_text())
                    .width(Length::Fill)
                    .horizontal_alignment(iced::HorizontalAlignment::Center),
            );
            column = column.push(Text::new("ENSnano website"));
            column = column.push(link_row(
                &mut self.ens_nano_website,
                "http://ens-lyon.fr/ensnano",
                ui_size.clone(),
            ));
        } else if *selection == Selection::Nothing || self.force_help {
            column = column.push(
                Text::new("Help")
                    .size(ui_size.head_text())
                    .width(Length::Fill)
                    .horizontal_alignment(iced::HorizontalAlignment::Center),
            );
            column = add_help_to_column(column, "3D view", view_3d_help(), ui_size.clone());
            column = column.push(iced::Space::with_height(Length::Units(15)));
            column = add_help_to_column(column, "2D/3D view", view_2d_3d_help(), ui_size.clone());
            column = column.push(iced::Space::with_height(Length::Units(15)));
            column = add_help_to_column(column, "2D view", view_2d_help(), ui_size.clone());
        } else {
            let help_btn =
                text_btn(&mut self.help_btn, "Help", ui_size.clone()).on_press(Message::ForceHelp);
            column = column.push(
                Row::new()
                    .width(Length::Fill)
                    .push(iced::Space::with_width(Length::FillPortion(1)))
                    .align_items(iced::Align::Center)
                    .push(Column::new().width(Length::FillPortion(1)).push(help_btn))
                    .push(iced::Space::with_width(Length::FillPortion(1))),
            );
            column = column.push(Text::new(selection.info()).size(ui_size.main_text()));

            match selection {
                Selection::Grid(_, _) => {
                    column = add_grid_content(column, self.info_values.as_slice(), ui_size.clone())
                }
                Selection::Strand(_, _) => {
                    column =
                        add_strand_content(column, self.info_values.as_slice(), ui_size.clone())
                }
                Selection::Nucleotide(_, _) => {
                    let anchor = self.info_values[0].clone();
                    column = column.push(Text::new(format!("Anchor {}", anchor)));
                }
                _ => (),
            }
        }

        Scrollable::new(&mut self.scroll).push(column).into()
    }

    pub fn selection_value_changed(&mut self, n: usize, s: String, requests: Arc<Mutex<Requests>>) {
        requests.lock().unwrap().toggle_persistent_helices = s.parse().ok();
        self.info_values[n] = s.into();
    }

    pub fn set_small_sphere(&mut self, b: bool, requests: Arc<Mutex<Requests>>) {
        self.info_values[1] = if b { "true".into() } else { "false".into() };
        requests.lock().unwrap().small_spheres = Some(b);
    }

    pub fn scaffold_id_set(&mut self, n: usize, b: bool, requests: Arc<Mutex<Requests>>) {
        self.info_values[1] = if b { "true".into() } else { "false".into() };
        if b {
            requests.lock().unwrap().set_scaffold_id = Some(Some(n))
        } else {
            requests.lock().unwrap().set_scaffold_id = Some(None)
        }
    }

    pub fn update_selection(&mut self, selection: Selection, info_values: Vec<String>) {
        self.selection = selection;
        self.info_values = info_values.into_iter().map(|s| s.into()).collect();
        self.force_help = false;
    }
}

fn add_grid_content<'a>(
    mut column: Column<'a, Message>,
    info_values: &[Cow<'static, str>],
    ui_size: UiSize,
) -> Column<'a, Message> {
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
        Checkbox::new(
            info_values[1].parse::<bool>().unwrap(),
            "Small spheres",
            |b| Message::SetSmallSpheres(b),
        )
        .size(ui_size.checkbox())
        .text_size(ui_size.main_text()),
    );
    column
}

fn add_strand_content<'a>(
    mut column: Column<'a, Message>,
    info_values: &[Cow<'static, str>],
    ui_size: UiSize,
) -> Column<'a, Message> {
    let s_id = info_values[2].parse::<usize>().unwrap();
    column = column.push(Text::new(format!("length {}", info_values[0])).size(ui_size.main_text()));
    column = column.push(Checkbox::new(
        info_values[1].parse().unwrap(),
        "Scaffold",
        move |b| Message::ScaffoldIdSet(s_id, b),
    ));
    column = column.push(Text::new(info_values[3].clone()).size(ui_size.main_text()));
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
                    .horizontal_alignment(iced::HorizontalAlignment::Center),
            );
        } else {
            column = column.push(
                Row::new()
                    .push(
                        Text::new(l)
                            .width(Length::FillPortion(5))
                            .horizontal_alignment(iced::HorizontalAlignment::Right),
                    )
                    .push(iced::Space::with_width(Length::FillPortion(1)))
                    .push(Text::new(r).width(Length::FillPortion(5))),
            );
        }
    }
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
        (format!("{} + J", CTRL), "Magic Paste".to_owned()),
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
        (format!("{}", RCLICK), "Select".to_owned()),
        (
            format!("{} + {}", SHIFT, RCLICK),
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
            format!("{} Drac", RCLICK),
            "Rotate selected helices".to_owned(),
        ),
        (String::new(), String::new()),
        ("On nucleotides".to_owned(), String::new()),
        (
            format!("{}", LCLICK),
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

fn link_row<'a>(
    button_state: &'a mut button::State,
    link: &'static str,
    ui_size: UiSize,
) -> Row<'a, Message> {
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
