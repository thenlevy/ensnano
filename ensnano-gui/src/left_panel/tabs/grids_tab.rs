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

pub struct GridTab {
    scroll: iced::scrollable::State,
    finalize_hyperboloid_btn: button::State,
    make_square_grid_btn: button::State,
    make_honeycomb_grid_btn: button::State,
    hyperboloid_factory: RequestFactory<Hyperboloid_>,
    start_hyperboloid_btn: button::State,
    make_grid_btn: button::State,
}

macro_rules! add_grid_buttons {
    ($ret: ident, $self:ident, $ui_size: ident) => {
        let make_square_grid_btn =
            icon_btn(&mut $self.make_square_grid_btn, ICON_SQUARE_GRID, $ui_size)
                .on_press(Message::NewGrid(GridTypeDescr::Square { twist: None }));
        let make_honeycomb_grid_btn = icon_btn(
            &mut $self.make_honeycomb_grid_btn,
            ICON_HONEYCOMB_GRID,
            $ui_size,
        )
        .on_press(Message::NewGrid(GridTypeDescr::Honeycomb { twist: None }));

        let grid_buttons = Row::new()
            .push(make_square_grid_btn)
            .push(make_honeycomb_grid_btn)
            .spacing(5);
        $ret = $ret.push(grid_buttons);
    };
}

macro_rules! add_start_cancel_hyperboloid_button {
    ($ret:ident, $self:ident, $ui_size: ident, $app_state: ident) => {
        let start_hyperboloid_btn = if !$app_state.is_building_hyperboloid() {
            icon_btn(
                &mut $self.start_hyperboloid_btn,
                ICON_NANOTUBE,
                $ui_size.clone(),
            )
            .on_press(Message::NewHyperboloid)
        } else {
            text_btn(&mut $self.start_hyperboloid_btn, "Finish", $ui_size.clone())
                .on_press(Message::FinalizeHyperboloid)
        };

        let cancel_hyperboloid_btn = text_btn(
            &mut $self.finalize_hyperboloid_btn,
            "Cancel",
            $ui_size.clone(),
        )
        .on_press(Message::CancelHyperboloid);

        if $app_state.is_building_hyperboloid() {
            $ret = $ret.push(
                Row::new()
                    .spacing(3)
                    .push(start_hyperboloid_btn)
                    .push(cancel_hyperboloid_btn),
            );
        } else {
            $ret = $ret.push(start_hyperboloid_btn);
        }
    };
}

macro_rules! add_hyperboloid_sliders {
    ($ret: ident, $self: ident, $ui_size: ident, $app_state: ident) => {
        for view in $self
            .hyperboloid_factory
            .view($app_state.is_building_hyperboloid(), $ui_size.main_text())
            .into_iter()
        {
            $ret = $ret.push(view);
        }
    };
}

macro_rules! add_guess_grid_button {
    ($ret: ident, $self: ident, $ui_size: ident, $app_state: ident) => {
        let mut button_make_grid =
            Button::new(&mut $self.make_grid_btn, iced::Text::new("From Selection"))
                .height(Length::Units($ui_size.button()));

        if $app_state.can_make_grid() {
            button_make_grid = button_make_grid.on_press(Message::MakeGrids);
        }

        $ret = $ret.push(button_make_grid);
        $ret = $ret.push(Text::new("Select ≥4 unattached helices").size($ui_size.main_text()));
    };
}

impl GridTab {
    pub fn new() -> Self {
        Self {
            scroll: Default::default(),
            make_square_grid_btn: Default::default(),
            make_honeycomb_grid_btn: Default::default(),
            hyperboloid_factory: RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {}),
            finalize_hyperboloid_btn: Default::default(),
            start_hyperboloid_btn: Default::default(),
            make_grid_btn: Default::default(),
        }
    }

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        _width: u16,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        section!(ret, ui_size, "Grids");

        subsection!(ret, ui_size, "New Grid");

        add_grid_buttons!(ret, self, ui_size);

        extra_jump!(ret);

        subsection!(ret, ui_size, "New nanotube");

        add_start_cancel_hyperboloid_button!(ret, self, ui_size, app_state);

        add_hyperboloid_sliders!(ret, self, ui_size, app_state);

        extra_jump!(ret);

        subsection!(ret, ui_size, "Guess grid");

        add_guess_grid_button!(ret, self, ui_size, app_state);

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub fn new_hyperboloid(&mut self, requests: &mut Option<HyperboloidRequest>) {
        self.hyperboloid_factory = RequestFactory::new(FactoryId::Hyperboloid, Hyperboloid_ {});
        self.hyperboloid_factory.make_request(requests);
    }

    pub fn update_hyperboloid_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<HyperboloidRequest>,
    ) {
        self.hyperboloid_factory
            .update_request(value_id, value, request);
    }
}
