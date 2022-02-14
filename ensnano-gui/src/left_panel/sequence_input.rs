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
use super::{AppState, Message};
use iced::{button, text_input, Button, Row, Text, TextInput};

pub struct SequenceInput {
    input: text_input::State,
    #[allow(dead_code)]
    button_state: button::State,
    sequence: String,
}

impl SequenceInput {
    pub fn new() -> Self {
        Self {
            input: Default::default(),
            sequence: String::new(),
            button_state: Default::default(),
        }
    }

    #[allow(dead_code)]
    pub fn view<S: AppState>(&mut self) -> Row<Message<S>> {
        let sequence_input = Row::new()
            .spacing(5)
            .push(TextInput::new(
                &mut self.input,
                "Sequence",
                &self.sequence,
                Message::SequenceChanged,
            ))
            .push(
                Button::new(&mut self.button_state, Text::new("Load File"))
                    .on_press(Message::SequenceFileRequested),
            );
        sequence_input
    }

    pub fn update_sequence(&mut self, sequence: String) {
        self.sequence = sequence;
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.input.is_focused()
    }
}
