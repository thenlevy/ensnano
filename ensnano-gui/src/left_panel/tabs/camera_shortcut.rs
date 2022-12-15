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

struct TargetShortcut {
    name: &'static str,
    target_axis: (Vec3, Vec3),
}

impl TargetShortcut {
    fn message<S: AppState>(&self) -> Message<S> {
        Message::FixPoint(self.target_axis.0, self.target_axis.1)
    }
}

macro_rules! vec3 {
    ($x: expr, $y: expr, $z: expr) => {
        Vec3 {
            x: $x,
            y: $y,
            z: $z,
        }
    };
}

const TARGETS: [TargetShortcut; 6] = [
    TargetShortcut {
        name: "Left",
        target_axis: (vec3!(-1., 0., 0.), vec3!(0., 1., 0.)),
    },
    TargetShortcut {
        name: "Right",
        target_axis: (vec3!(1., 0., 0.), vec3!(0., 1., 0.)),
    },
    TargetShortcut {
        name: "Top",
        target_axis: (vec3!(0., 1., 0.), vec3!(0., 0., 1.)),
    },
    TargetShortcut {
        name: "Back",
        target_axis: (vec3!(0., 0., 1.), vec3!(0., 1., 0.)),
    },
    TargetShortcut {
        name: "Front",
        target_axis: (vec3!(0., 0., -1.), vec3!(0., 1., 0.)),
    },
    TargetShortcut {
        name: "Bottom",
        target_axis: (vec3!(0., -1., 0.), vec3!(0., 0., -1.)),
    },
];

macro_rules! add_target_buttons {
    ($ret: ident, $self:ident, $ui_size: ident, $width: ident) => {
        let mut target_buttons: Vec<_> = $self
            .camera_target_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, Text::new(TARGETS[i].name).size($ui_size.main_text()))
                    .on_press(TARGETS[i].message())
                    .width(Length::Units(2 * $ui_size.button()))
            })
            .collect();
        while target_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(target_buttons.remove(0)).spacing(5);
            let mut nb_button_row = 1;
            let mut space = 2 * $ui_size.button() + 5;
            while space + 2 * $ui_size.button() < $width
                && target_buttons.len() > 0
                && nb_button_row < 3
            {
                row = row.push(target_buttons.remove(0)).spacing(5);
                space += 2 * $ui_size.button() + 5;
                nb_button_row += 1;
            }
            $ret = $ret.push(row)
        }
    };
}

macro_rules! add_rotate_buttons {
    ($ret: ident, $self: ident, $ui_size: ident, $width: ident) => {
        let xz = $self.xz;
        let yz = $self.yz;
        let xy = $self.xy;

        let mut rotate_buttons: Vec<_> = $self
            .camera_rotation_buttons
            .iter_mut()
            .enumerate()
            .map(|(i, s)| {
                Button::new(s, rotation_text(i, $ui_size))
                    .on_press(rotation_message(i, xz, yz, xy))
                    .width(Length::Units($ui_size.button()))
            })
            .collect();

        $ret = $ret.push(Text::new("Rotate Camera"));
        while rotate_buttons.len() > 0 {
            let mut row = Row::new();
            row = row.push(rotate_buttons.remove(0)).spacing(5);
            let mut space = $ui_size.button() + 5;
            while space + $ui_size.button() < $width && rotate_buttons.len() > 0 {
                row = row.push(rotate_buttons.remove(0)).spacing(5);
                space += $ui_size.button() + 5;
            }
            $ret = $ret.spacing(5).push(row)
        }
    };
}

macro_rules! add_screenshot_button {
    ($ret: ident, $self: ident, $ui_size: ident, $width: ident) => {
        let screenshot_button = Button::new(
            &mut $self.screenshot_button,
            Text::new("3D").size($ui_size.main_text()),
        )
        .on_press(Message::ScreenShot3D)
        .width(Length::Units($ui_size.button()));

        $ret = $ret.push(Text::new("Screenshot"));
        $ret = $ret.spacing(5).push(screenshot_button);
    };
}

macro_rules! add_custom_camera_row {
    ($ret: ident, $self: ident, $ui_size: ident) => {
        let new_camera_button =
            light_icon_btn(&mut $self.new_camera_button, LightIcon::AddAPhoto, $ui_size)
                .on_press(Message::NewCustomCamera);
        let custom_cameras_row = Row::new()
            .push(Text::new("Custom cameras").size($ui_size.head_text()))
            .push(iced::Space::with_width(Length::Fill))
            .push(new_camera_button);

        $ret = $ret.push(custom_cameras_row);
    };
}

macro_rules! add_camera_widgets {
    ($ret: ident, $self: ident, $ui_size: ident) => {
        if $self.camera_widget_states.len() < $self.camera_widgets.len() {
            $self.camera_widget_states.extend(vec![
                CameraWidgetState::default();
                $self.camera_widgets.len()
            ]);
        }
        for (c, s) in $self
            .camera_widgets
            .iter_mut()
            .zip($self.camera_widget_states.iter_mut())
        {
            $ret = $ret.push(c.view($ui_size, s));
        }
    };
}
pub struct CameraShortcut {
    camera_target_buttons: [button::State; 6],
    camera_rotation_buttons: [button::State; 6],
    // Camera angles
    xz: isize,
    yz: isize,
    xy: isize,
    scroll: scrollable::State,
    camera_input_name: Option<String>,
    camera_being_edited: Option<CameraId>,
    camera_widgets: Vec<CameraWidget>,
    new_camera_button: button::State,
    camera_widget_states: Vec<CameraWidgetState>,
    screenshot_button: button::State,
}

impl CameraShortcut {
    pub fn new() -> Self {
        Self {
            camera_target_buttons: Default::default(),
            camera_rotation_buttons: Default::default(),
            xz: 0,
            yz: 0,
            xy: 0,
            scroll: Default::default(),
            camera_input_name: None,
            camera_being_edited: None,
            camera_widgets: vec![],
            new_camera_button: Default::default(),
            camera_widget_states: vec![],
            screenshot_button: Default::default(),
        }
    }

    pub fn reset_angles(&mut self) {
        self.xz = 0;
        self.yz = 0;
        self.xy = 0
    }

    pub fn set_angles(&mut self, xz: isize, yz: isize, xy: isize) {
        self.xz += xz;
        self.yz += yz;
        self.xy += xy;
    }

    pub fn set_camera_input_name(&mut self, name: String) {
        self.camera_input_name = Some(name);
    }

    pub fn stop_editing(&mut self) -> Option<(CameraId, String)> {
        let name = self.camera_input_name.take();
        let id = self.camera_being_edited.take();
        for s in self.camera_widget_states.iter_mut() {
            s.name_input.unfocus();
        }
        id.zip(name)
    }

    pub fn start_editing(&mut self, id: CameraId) {
        for (c, s) in self
            .camera_widgets
            .iter()
            .zip(self.camera_widget_states.iter_mut())
        {
            if c.camera_id == id {
                self.camera_being_edited = Some(id);
                s.name_input.focus();
                s.name_input.select_all();
            }
        }
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.camera_widget_states
            .iter()
            .any(|s| s.name_input.is_focused())
    }

    fn set_camera_widget<S: AppState>(&mut self, app: &S) {
        self.camera_widgets = app
            .get_reader()
            .get_all_cameras()
            .iter()
            .map(|cam| {
                let being_edited = self.camera_being_edited == Some(cam.0);
                let name = if being_edited {
                    self.camera_input_name
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or(cam.1)
                } else {
                    cam.1
                };
                CameraWidget::new(name.to_string(), being_edited, cam.0)
            })
            .collect();
    }

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        width: u16,
        app: &S,
    ) -> Element<'a, Message<S>> {
        self.set_camera_widget(app);
        let mut ret = Column::new();
        section!(ret, ui_size, "Camera");
        add_target_buttons!(ret, self, ui_size, width);

        add_rotate_buttons!(ret, self, ui_size, width);

        add_screenshot_button!(ret, self, ui_size, width);

        add_custom_camera_row!(ret, self, ui_size);

        add_camera_widgets!(ret, self, ui_size);

        Scrollable::new(&mut self.scroll)
            .push(ret)
            .width(iced::Length::Units(width))
            .into()
    }

    pub fn scroll_down(&mut self) {
        self.scroll.snap_to(1.);
    }
}

struct CameraWidget {
    name: String,
    being_edited: bool,
    camera_id: CameraId,
}

#[derive(Debug, Clone, Default)]
struct CameraWidgetState {
    select_camera_btn: button::State,
    edit_name_btn: button::State,
    delete_btn: button::State,
    name_input: text_input::State,
}

impl CameraWidget {
    fn new(name: String, being_edited: bool, camera_id: CameraId) -> Self {
        Self {
            name,
            being_edited,
            camera_id,
        }
    }

    fn view<'a, S: AppState>(
        &self,
        ui_size: UiSize,
        state: &'a mut CameraWidgetState,
    ) -> Element<'a, Message<S>> {
        let name: Element<Message<S>> = if self.being_edited {
            TextInput::new(
                &mut state.name_input,
                "Camera name",
                &self.name,
                Message::EditCameraName,
            )
            .on_submit(Message::SubmitCameraName)
            .into()
        } else {
            Text::new(&self.name).into()
        };

        let select_camera_btn =
            light_icon_btn(&mut state.select_camera_btn, LightIcon::Visibility, ui_size)
                .on_press(Message::SelectCamera(self.camera_id));

        let edit_button = light_icon_btn(&mut state.edit_name_btn, LightIcon::Edit, ui_size)
            .on_press(Message::StartEditCameraName(self.camera_id));

        let delete_button = light_icon_btn(&mut state.delete_btn, LightIcon::Delete, ui_size)
            .on_press(Message::DeleteCamera(self.camera_id));

        Row::new()
            .push(name)
            .push(iced::Space::with_width(iced::Length::Units(3)))
            .push(edit_button)
            .push(iced::Space::with_width(iced::Length::Fill))
            .push(select_camera_btn)
            .push(delete_button)
            .into()
    }
}
