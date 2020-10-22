use super::Message;
use iced::{button, text_input, Button, Row, Text, TextInput};

pub struct SequenceInput {
    input: text_input::State,
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
    pub fn view(&mut self) -> Row<Message> {
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
}
