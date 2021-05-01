use std::sync::{Arc, Mutex};
use std::thread;

use super::UiSize;
use iced::{container, Background, Container};
use iced_native::clipboard::Null as NullClipBoard;
use iced_wgpu::Renderer;
use iced_winit::winit::dpi::LogicalSize;
use iced_winit::{button, Button, Color, Command, Element, Length, Program, Row};

use material_icons::{icon_to_char, Icon as MaterialIcon, FONT as MATERIALFONT};

const ICONFONT: iced::Font = iced::Font::External {
    name: "IconFont",
    bytes: MATERIALFONT,
};

fn icon(icon: MaterialIcon, ui_size: UiSize) -> iced::Text {
    iced::Text::new(format!("{}", icon_to_char(icon)))
        .font(ICONFONT)
        .size(ui_size.icon())
}

use super::{Requests, SplitMode};

pub struct TopBar {
    button_fit: button::State,
    button_add_file: button::State,
    #[allow(dead_code)]
    button_replace_file: button::State,
    button_save: button::State,
    button_undo: button::State,
    button_redo: button::State,
    button_3d: button::State,
    button_2d: button::State,
    button_split: button::State,
    button_oxdna: button::State,
    button_split_2d: button::State,
    requests: Arc<Mutex<Requests>>,
    logical_size: LogicalSize<f64>,
    dialoging: Arc<Mutex<bool>>,
    ui_size: UiSize,
    can_undo: bool,
    can_redo: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SceneFitRequested,
    FileAddRequested,
    #[allow(dead_code)]
    FileReplaceRequested,
    FileSaveRequested,
    Resize(LogicalSize<f64>),
    ToggleView(SplitMode),
    UiSizeChanged(UiSize),
    OxDNARequested,
    Split2d,
    CanUndo(bool),
    CanRedo(bool),
    Undo,
    Redo,
}

impl TopBar {
    pub fn new(
        requests: Arc<Mutex<Requests>>,
        logical_size: LogicalSize<f64>,
        dialoging: Arc<Mutex<bool>>,
    ) -> TopBar {
        Self {
            button_fit: Default::default(),
            button_add_file: Default::default(),
            button_replace_file: Default::default(),
            button_save: Default::default(),
            button_undo: Default::default(),
            button_redo: Default::default(),
            button_2d: Default::default(),
            button_3d: Default::default(),
            button_split: Default::default(),
            button_oxdna: Default::default(),
            button_split_2d: Default::default(),
            requests,
            logical_size,
            dialoging,
            ui_size: Default::default(),
            can_undo: false,
            can_redo: false,
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize<f64>) {
        self.logical_size = logical_size;
    }
}

impl Program for TopBar {
    type Renderer = Renderer;
    type Message = Message;
    type Clipboard = NullClipBoard;

    fn update(&mut self, message: Message, _cb: &mut NullClipBoard) -> Command<Message> {
        match message {
            Message::SceneFitRequested => {
                self.requests.lock().expect("fitting_requested").fitting = true;
            }
            Message::FileAddRequested => {
                if !*self.dialoging.lock().unwrap() {
                    *self.dialoging.lock().unwrap() = true;
                    let requests = self.requests.clone();
                    let dialog = rfd::AsyncFileDialog::new().pick_file();
                    let dialoging = self.dialoging.clone();
                    thread::spawn(move || {
                        let load_op = async move {
                            let file = dialog.await;
                            if let Some(handle) = file {
                                let path_buf: std::path::PathBuf = handle.path().clone().into();
                                requests.lock().unwrap().file_add = Some(path_buf);
                            }
                            *dialoging.lock().unwrap() = false;
                        };
                        futures::executor::block_on(load_op);
                    });
                    /*
                    if cfg!(target_os = "macos") {
                        // do not spawn a new thread on macos
                        let result = match nfd2::open_file_dialog(None, None).expect("oh no") {
                            Response::Okay(file_path) => Some(file_path),
                            Response::OkayMultiple(_) => {
                                println!("Please open only one file");
                                None
                            }
                            Response::Cancel => None,
                        };
                        *self.dialoging.lock().unwrap() = false;
                        if let Some(path) = result {
                            requests.lock().expect("file_opening_request").file_add = Some(path);
                        }
                    } else {
                        let dialoging = self.dialoging.clone();
                        thread::spawn(move || {
                            let result = match nfd2::open_file_dialog(None, None).expect("oh no") {
                                Response::Okay(file_path) => Some(file_path),
                                Response::OkayMultiple(_) => {
                                    println!("Please open only one file");
                                    None
                                }
                                Response::Cancel => None,
                            };
                            *dialoging.lock().unwrap() = false;
                            if let Some(path) = result {
                                requests.lock().expect("file_opening_request").file_add =
                                    Some(path);
                            }
                        });
                    }*/
                }
            }
            Message::FileReplaceRequested => {
                self.requests
                    .lock()
                    .expect("file_opening_request")
                    .file_clear = false;
            }
            Message::FileSaveRequested => {
                if !*self.dialoging.lock().unwrap() {
                    *self.dialoging.lock().unwrap() = true;
                    let requests = self.requests.clone();
                    let dialog = rfd::AsyncFileDialog::new().save_file();
                    let dialoging = self.dialoging.clone();
                    thread::spawn(move || {
                        let save_op = async move {
                            let file = dialog.await;
                            if let Some(handle) = file {
                                let mut path_buf: std::path::PathBuf = handle.path().clone().into();
                                let extension = path_buf.extension().clone();
                                if extension.is_none() {
                                    path_buf.set_extension("json");
                                } else if extension.and_then(|e| e.to_str()) != Some("json".into())
                                {
                                    let extension = extension.unwrap();
                                    let new_extension =
                                        format!("{}.json", extension.to_str().unwrap());
                                    path_buf.set_extension(new_extension);
                                }
                                requests.lock().unwrap().file_save = Some(path_buf);
                            }
                            *dialoging.lock().unwrap() = false;
                        };
                        futures::executor::block_on(save_op);
                    });
                    /*
                    if cfg!(target_os = "macos") {
                        // do not spawn a new thread for macos
                        let result = match nfd2::open_save_dialog(None, None).expect("oh no") {
                            Response::Okay(file_path) => Some(file_path),
                            Response::OkayMultiple(_) => {
                                println!("Please open only one file");
                                None
                            }
                            Response::Cancel => None,
                        };
                        *self.dialoging.lock().unwrap() = false;
                        if let Some(path) = result {
                            requests.lock().expect("file_opening_request").file_save = Some(path);
                        }
                    } else {
                        let dialoging = self.dialoging.clone();
                        thread::spawn(move || {
                            let result = match nfd2::open_save_dialog(None, None).expect("oh no") {
                                Response::Okay(file_path) => Some(file_path),
                                Response::OkayMultiple(_) => {
                                    println!("Please open only one file");
                                    None
                                }
                                Response::Cancel => None,
                            };
                            *dialoging.lock().unwrap() = false;
                            if let Some(path) = result {
                                requests.lock().expect("file_opening_request").file_save =
                                    Some(path);
                            }
                        });
                    }
                    */
                }
            }
            Message::Resize(size) => self.resize(size),
            Message::ToggleView(b) => self.requests.lock().unwrap().toggle_scene = Some(b),
            Message::UiSizeChanged(ui_size) => self.ui_size = ui_size,
            Message::OxDNARequested => self.requests.lock().unwrap().oxdna = true,
            Message::Split2d => self.requests.lock().unwrap().split2d = true,
            Message::CanUndo(b) => self.can_undo = b,
            Message::CanRedo(b) => self.can_redo = b,
            Message::Undo => self.requests.lock().unwrap().undo = Some(()),
            Message::Redo => self.requests.lock().unwrap().redo = Some(()),
        };
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let height = self.logical_size.cast::<u16>().height;
        let button_fit = Button::new(
            &mut self.button_fit,
            icon(MaterialIcon::CenterFocusStrong, self.ui_size.clone()),
        )
        .on_press(Message::SceneFitRequested)
        .height(Length::Units(height));
        let button_add_file = Button::new(
            &mut self.button_add_file,
            icon(MaterialIcon::FolderOpen, self.ui_size.clone()),
        )
        .on_press(Message::FileAddRequested)
        .height(Length::Units(height));
        /*let button_replace_file = Button::new(
            &mut self.button_replace_file,
            Image::new("icons/delete.png"),
        )
        .on_press(Message::FileReplaceRequested)
        .height(Length::Units(height));*/
        let button_save = Button::new(
            &mut self.button_save,
            icon(MaterialIcon::Save, self.ui_size.clone()),
        )
        .on_press(Message::FileSaveRequested)
        .height(Length::Units(height));

        let mut button_undo = Button::new(
            &mut self.button_undo,
            icon(MaterialIcon::Undo, self.ui_size.clone()),
        );
        if self.can_undo {
            button_undo = button_undo.on_press(Message::Undo)
        }

        let mut button_redo = Button::new(
            &mut self.button_redo,
            icon(MaterialIcon::Redo, self.ui_size.clone()),
        );
        if self.can_redo {
            button_redo = button_redo.on_press(Message::Redo)
        }

        let button_2d = Button::new(&mut self.button_2d, iced::Text::new("2D"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ToggleView(SplitMode::Flat));
        let button_3d = Button::new(&mut self.button_3d, iced::Text::new("3D"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ToggleView(SplitMode::Scene3D));
        let button_split = Button::new(&mut self.button_split, iced::Text::new("3D+2D"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::ToggleView(SplitMode::Both));

        let button_oxdna = Button::new(&mut self.button_oxdna, iced::Text::new("To OxDNA"))
            .height(Length::Units(self.ui_size.button()))
            .on_press(Message::OxDNARequested);

        let button_split_2d =
            Button::new(&mut self.button_split_2d, iced::Text::new("Split 2d view"))
                .height(Length::Units(self.ui_size.button()))
                .on_press(Message::Split2d);

        let buttons = Row::new()
            .width(Length::Fill)
            .height(Length::Units(height))
            .push(button_fit)
            .push(button_add_file)
            //.push(button_replace_file)
            .push(button_save)
            .push(button_oxdna)
            .push(iced::Space::with_width(Length::Units(10)))
            .push(button_undo)
            .push(button_redo)
            .push(iced::Space::with_width(Length::Units(30)))
            .push(button_3d)
            .push(button_2d)
            .push(button_split)
            .push(button_split_2d)
            .push(
                iced::Text::new("\u{e91c}")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::HorizontalAlignment::Right)
                    .vertical_alignment(iced::VerticalAlignment::Bottom),
            ).push(iced::Space::with_width(Length::Units(10)));

        Container::new(buttons)
            .width(Length::Units(self.logical_size.width as u16))
            .style(TopBarStyle)
            .into()
    }
}

struct TopBarStyle;
impl container::StyleSheet for TopBarStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(BACKGROUND)),
            text_color: Some(Color::WHITE),
            ..container::Style::default()
        }
    }
}

pub const BACKGROUND: Color = Color::from_rgb(
    0x36 as f32 / 255.0,
    0x39 as f32 / 255.0,
    0x3F as f32 / 255.0,
);
