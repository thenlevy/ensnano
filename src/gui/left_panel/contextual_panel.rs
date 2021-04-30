use super::*;
use crate::mediator::Selection;
use iced::{scrollable, Scrollable};
use std::borrow::Cow;

pub(super) struct ContextualPanel {
    selection: Selection,
    info_values: Vec<Cow<'static, str>>,
    scroll: scrollable::State,
    width: u32,
}

impl ContextualPanel {
    pub fn new(width: u32) -> Self {
        Self {
            selection: Selection::Nothing,
            info_values: vec![],
            scroll: Default::default(),
            width,
        }
    }

    pub fn new_width(&mut self, width: u32) {
        self.width = width;
    }

    pub fn view(&mut self, ui_size: UiSize) -> Element<Message> {
        let mut column = Column::new().max_width(self.width - 2);
        let selection = &self.selection;
        if *selection == Selection::Nothing {
            column = add_help_to_column(column, "3D view", view_3d_help(), ui_size.clone());
            column = column.push(iced::Space::with_height(Length::Units(15)));
            column = add_help_to_column(column, "2D/3D view", view_2d_3d_help(), ui_size.clone());
            column = column.push(iced::Space::with_height(Length::Units(15)));
            column = add_help_to_column(column, "2D view", view_2d_help(), ui_size.clone());
        } else {
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
            format!("{} + {}", ALT, LCLICK),
            "Make suggested crossover".to_owned(),
        ),
    ]
}
