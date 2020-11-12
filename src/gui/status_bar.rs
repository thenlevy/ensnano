use iced_winit::{Text, Command, Element, Program, Row};

pub struct StatusBar {

}

impl StatusBar {
    pub fn new() -> Self {
        Self { }
    }
}

#[derive(Clone, Debug)]
pub enum Message {

}

impl Program for StatusBar {
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn update(&mut self, message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&mut self) -> Element<Message, iced_wgpu::Renderer> {
        Row::new().push(Text::new("This is the status bar")).into()
    }

}
