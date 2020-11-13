use iced_winit::{Text, Command, Element, Program, Row};
use iced_native::{text_input, TextInput, pick_list, PickList};
use crate::mediator::{ParameterField, Operation};
use std::sync::Arc;

enum StatusParameter {
    Value(text_input::State),
    Choice(pick_list::State<String>),
}

impl StatusParameter {
    fn get_value(&mut self) -> &mut text_input::State {
        match self {
            StatusParameter::Value(ref mut state) => state,
            _ => panic!("wrong status parameter variant")
        }
    }

    fn get_choice(&mut self) -> &mut pick_list::State<String> {
        match self {
            StatusParameter::Choice(ref mut state) => state,
            _ => panic!("wrong status parameter variant")
        }
    }

    fn value() -> Self {
        Self::Value(Default::default())
    }

    fn choice() -> Self {
        Self::Choice(Default::default())
    }


}

pub struct StatusBar {
    parameters: Vec<StatusParameter>,
    operation: Option<Arc<dyn Operation>>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { 
            parameters: Vec::new(),
            operation: None,
        }
    }

    pub fn update_op(&mut self, operation: Arc<dyn Operation>) {
        let parameters = operation.parameters();
        let mut new_param = Vec::new();
        for p in parameters.iter() {
            match p.field {
                ParameterField::Choice(_) => {
                    new_param.push(StatusParameter::choice())
                }
                ParameterField::Value => {
                    new_param.push(StatusParameter::value())
                }
            }
        }
        self.parameters = new_param;
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Operation(Arc<dyn Operation>),
    ValueChanged(usize, String),
}

impl Program for StatusBar {
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Operation(ref op) => {
                self.operation = Some(op.clone());
                self.update_op(op.clone());
            }
            _ => ()
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        let mut row = Row::new();
        if let Some(ref op) = self.operation {
            row = row.push(Text::new(op.description()));
            let values = op.values();
            for (i, p) in self.parameters.iter_mut().enumerate() {
                let param = &op.parameters()[i];
                match param.field {
                    ParameterField::Value => {
                        row = row
                            .push(Text::new(param.name.clone()))
                            .push(TextInput::new(p.get_value(), values[i].as_str(), "", move |s| Message::ValueChanged(i, s)))
                    }
                    ParameterField::Choice(_) => unimplemented!()
                }
            }
        }
        row.into()
    }

}
