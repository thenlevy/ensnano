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

macro_rules! add_show_sequence_button {
    ($ret: ident, $self: ident, $ui_size: ident) => {
        let button_show_sequence = if $self.toggle_text_value {
            text_btn(&mut $self.button_show_sequence, "Hide Sequences", $ui_size)
                .on_press(Message::ToggleText(false))
        } else {
            text_btn(&mut $self.button_show_sequence, "Show Sequences", $ui_size)
                .on_press(Message::ToggleText(true))
        };
        $ret = $ret.push(button_show_sequence);
    };
}

macro_rules! add_scaffold_from_to_selection_buttons {
    ($ret: ident, $self:ident, $ui_size: ident, $app_state: ident) => {
        let mut button_selection_to_scaffold = text_btn(
            &mut $self.button_selection_to_scaffold,
            "From selection",
            $ui_size,
        );
        let mut button_selection_from_scaffold =
            text_btn(&mut $self.button_selection_from_scaffold, "Show", $ui_size);
        if $app_state.get_scaffold_info().is_some() {
            button_selection_from_scaffold =
                button_selection_from_scaffold.on_press(Message::SelectScaffold);
        }
        let selection = $app_state.get_selection_as_dnaelement();
        if let Some(n) = Self::get_candidate_scaffold(&selection) {
            button_selection_to_scaffold =
                button_selection_to_scaffold.on_press(Message::ScaffoldIdSet(n, true));
        }
        $ret = $ret.push(
            Row::new()
                .push(button_selection_to_scaffold)
                .push(iced::Space::with_width(Length::Units(5)))
                .push(button_selection_from_scaffold),
        );
    };
}

macro_rules! scaffold_length_fmt {
    () => {
        "Length: {} nt"
    };
}

macro_rules! nucl_text_fmt {
    () => {
        "   Helix #{}\n   Strand: {}\n   Nt #{}"
    };
}

macro_rules! add_scaffold_info {
    ($ret: ident, $self: ident, $ui_size: ident, $app_state: ident) => {
        let (scaffold_text, length_text) = if let Some(info) = $app_state.get_scaffold_info() {
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
        if $app_state.get_scaffold_info().is_none() {
            length_text = length_text.color(innactive_color())
        }
        $ret = $ret.push(Text::new(scaffold_text).size($ui_size.main_text()));
        $ret = $ret.push(length_text);
    };
}

macro_rules! add_set_scaffold_sequence_button {
    ($ret: ident, $self: ident, $ui_size: ident) => {
        let button_scaffold = Button::new(
            &mut $self.button_scaffold,
            iced::Text::new("Set scaffold sequence"),
        )
        .height(Length::Units($ui_size.button()))
        .on_press(Message::SetScaffoldSeqButtonPressed);
        $ret = $ret.push(button_scaffold);
    };
}

macro_rules! add_scaffold_position_input_row {
    ($ret: ident, $self: ident) => {
        let scaffold_position_text = "Starting position";
        let scaffold_row = Row::new()
            .push(Text::new(scaffold_position_text).width(Length::FillPortion(2)))
            .push(
                TextInput::new(
                    &mut $self.scaffold_input,
                    "Scaffold position",
                    &$self.scaffold_position_str,
                    Message::ScaffoldPositionInput,
                )
                .style(BadValue(
                    $self.scaffold_position_str == $self.scaffold_position.to_string(),
                ))
                .width(iced::Length::FillPortion(1)),
            );
        $ret = $ret.push(scaffold_row);
    };
}
macro_rules! add_scaffold_start_position {
    ($ret: ident, $ui_size: ident, $app_state: ident) => {
        let starting_nucl = $app_state
            .get_scaffold_info()
            .as_ref()
            .and_then(|info| info.starting_nucl);
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
        let mut nucl_text = Text::new(nucl_text).size($ui_size.main_text());
        if starting_nucl.is_none() {
            nucl_text = nucl_text.color(innactive_color())
        }
        $ret = $ret.push(nucl_text);
    };
}

macro_rules! add_download_staples_button {
    ($ret: ident, $self: ident, $ui_size: ident) => {
        let button_stapples = Button::new(
            &mut $self.button_stapples,
            iced::Text::new("Export Staples"),
        )
        .height(Length::Units($ui_size.button()))
        .on_press(Message::StapplesRequested);
        $ret = $ret.push(button_stapples);
    };
}

impl SequenceTab {
    pub fn new() -> Self {
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

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &'a S,
    ) -> Element<'a, Message<S>> {
        if !self.scaffold_input.is_focused() {
            if let Some(n) = app_state.get_scaffold_info().and_then(|info| info.shift) {
                self.update_pos_str(n.to_string());
            }
        }

        let mut ret = Column::new();
        section!(ret, ui_size, "Sequence");
        extra_jump!(ret);
        add_show_sequence_button!(ret, self, ui_size);
        extra_jump!(ret);
        section!(ret, ui_size, "Scaffold");
        extra_jump!(ret);
        add_scaffold_from_to_selection_buttons!(ret, self, ui_size, app_state);
        extra_jump!(ret);
        add_scaffold_info!(ret, self, ui_size, app_state);
        extra_jump!(ret);

        add_set_scaffold_sequence_button!(ret, self, ui_size);
        extra_jump!(ret);
        add_scaffold_position_input_row!(ret, self);

        add_scaffold_start_position!(ret, ui_size, app_state);
        extra_jump!(ret);
        section!(ret, ui_size, "Staples");
        extra_jump!(ret);
        add_download_staples_button!(ret, self, ui_size);
        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub fn toggle_text_value(&mut self, b: bool) {
        self.toggle_text_value = b;
    }

    pub fn update_pos_str(&mut self, position_str: String) -> Option<usize> {
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
