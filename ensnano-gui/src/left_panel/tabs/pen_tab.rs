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
    make_square_grid_btn: button::State,
    make_honeycomb_grid_btn: button::State,
    load_svg_btn: button::State,
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

macro_rules! add_grid_buttons {
    ($ret: ident, $self: ident, $ui_size: ident, $app_state: ident) => {
        if let Some(path_id) = $app_state.get_selected_bezier_path() {
            let make_square_grid_btn =
                icon_btn(&mut $self.make_square_grid_btn, ICON_SQUARE_GRID, $ui_size).on_press(
                    Message::TurnPathIntoGrid {
                        path_id,
                        grid_type: GridTypeDescr::Square { twist: None },
                    },
                );
            let make_honeycomb_grid_btn = icon_btn(
                &mut $self.make_honeycomb_grid_btn,
                ICON_HONEYCOMB_GRID,
                $ui_size,
            )
            .on_press(Message::TurnPathIntoGrid {
                path_id,
                grid_type: GridTypeDescr::Honeycomb { twist: None },
            });

            let grid_buttons = Row::new()
                .push(make_square_grid_btn)
                .push(make_honeycomb_grid_btn)
                .spacing(5);
            $ret = $ret.push(grid_buttons);
        }
    };
}

impl PenTab {
    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        section!(ret, ui_size, "Bezier Planes");
        ret = ret.push(
            light_icon_btn(&mut self.load_svg_btn, LightIcon::FileOpen, ui_size)
                .on_press(Message::LoadSvgFile),
        );
        add_buttons!(ret, self, ui_size);
        add_grid_buttons!(ret, self, ui_size, app_state);
        let selected_path_id = app_state.get_selected_bezier_path();
        let path_txt = selected_path_id
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "None".to_string());
        ret = ret.push(Text::new(format!("Selected path {path_txt}")));

        if let Some(b) =
            selected_path_id.and_then(|p_id| app_state.get_reader().is_bezier_path_cyclic(p_id))
        {
            ret = ret.push(Checkbox::new(b, "Cyclic", move |cyclic| {
                Message::MakeBezierPathCyclic {
                    path_id: selected_path_id.unwrap(),
                    cyclic,
                }
            }));
        }

        extra_jump!(ret);
        ret = ret.push(Checkbox::new(
            app_state.get_show_bezier_paths(),
            "Show bezier paths",
            Message::SetShowBezierPaths,
        ));
        ret.into()
    }
}
