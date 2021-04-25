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
        let mut column = Column::new().max_width(self.width);
        let selection = &self.selection;
        column = column.push(Text::new(selection.info()).size(ui_size.main_text()));

        match selection {
            Selection::Grid(_, _) => {
                column = add_grid_content(column, self.info_values.as_slice(), ui_size.clone())
            }
            Selection::Strand(_, _) => {
                column = add_strand_content(column, self.info_values.as_slice(), ui_size.clone())
            }
            Selection::Nucleotide(_, _) => {
                let anchor = self.info_values[0].clone();
                column = column.push(Text::new(format!("Anchor {}", anchor)));
            }
            _ => (),
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
