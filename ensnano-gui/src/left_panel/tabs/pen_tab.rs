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

const NEW_BEZIER_PLANE_ICON: LightIcon = LightIcon::HistoryEdu;
const EDIT_BEZIER_PATH_ICON: LightIcon = LightIcon::LinearScale;

#[derive(Default)]
pub struct PenTab {
    add_plane_btn: button::State,
    edit_path_btn: button::State,
}

macro_rules! add_buttons {
    ($ret: ident, $self:ident, $ui_size: ident) => {
        $ret = $ret.push(
            Row::new()
                .push(
                    light_icon_btn(&mut $self.add_plane_btn, NEW_BEZIER_PLANE_ICON, $ui_size)
                        .on_press(Message::NewBezierPlane),
                )
                .push(
                    light_icon_btn(&mut $self.edit_path_btn, EDIT_BEZIER_PATH_ICON, $ui_size)
                        .on_press(Message::StartBezierPath),
                ),
        );
    };
}

impl PenTab {
    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        _app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        section!(ret, ui_size, "Bezier Planes");
        add_buttons!(ret, self, ui_size);
        ret.into()
    }
}
