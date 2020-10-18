use super::Message;
use iced::{text_input, TextInput, Row};

pub struct SequenceInput {
    input: text_input::State,
    sequence: String,
}

impl SequenceInput {
    pub fn new() -> Self {
        Self {
            input: Default::default(),
            sequence: String::new(),
        }
    }
    pub fn view(&mut self) -> Row<Message> {
        let sequence_input = Row::new()
            .spacing(5)
            .push(TextInput::new(
                    &mut self.input,
                    "Sequence",
                    &mut self.sequence,
                    Message::SequenceChanged));
        sequence_input
    }

    pub fn update_sequence(&mut self, sequence: String) {
        self.sequence = sequence;
    }

}
