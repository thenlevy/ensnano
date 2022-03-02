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

#[derive(Default)]
pub struct PenTab {
    add_plane_btn: button::State,
}

macro_rules! add_new_plane_button {
    ($ret: ident, $self:ident, $ui_size: ident) => {
        $ret = $ret.push(
            light_icon_btn(&mut $self.add_plane_btn, NEW_BEZIER_PLANE_ICON, $ui_size)
                .on_press(Message::NewBezierPlane),
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
        add_new_plane_button!(ret, self, ui_size);
        ret.into()
    }
}
