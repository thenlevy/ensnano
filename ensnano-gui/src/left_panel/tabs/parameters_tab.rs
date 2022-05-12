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
use ensnano_design::NamedParameter;

pub struct ParametersTab {
    size_pick_list: pick_list::State<UiSize>,
    scroll: scrollable::State,
    scroll_sensitivity_factory: RequestFactory<ScrollSentivity>,
    dna_parameters_picklist: pick_list::State<NamedParameter>,
    pub invert_y_scroll: bool,
}

impl ParametersTab {
    pub fn new<S: AppState>(app_state: &S) -> Self {
        Self {
            size_pick_list: Default::default(),
            scroll: Default::default(),
            scroll_sensitivity_factory: RequestFactory::new(
                FactoryId::Scroll,
                ScrollSentivity {
                    initial_value: app_state.get_scroll_sensitivity(),
                },
            ),
            dna_parameters_picklist: Default::default(),
            invert_y_scroll: false,
        }
    }

    pub fn view<'a, S: AppState>(
        &'a mut self,
        ui_size: UiSize,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new();
        section!(ret, ui_size, "Parameters");
        extra_jump!(ret);
        subsection!(ret, ui_size, "Font size");
        ret = ret.push(PickList::new(
            &mut self.size_pick_list,
            &super::super::super::ALL_UI_SIZE[..],
            Some(ui_size.clone()),
            Message::UiSizePicked,
        ));

        extra_jump!(ret);
        subsection!(ret, ui_size, "Scrolling");
        for view in self
            .scroll_sensitivity_factory
            .view(true, ui_size.main_text())
            .into_iter()
        {
            ret = ret.push(view);
        }

        ret = ret.push(right_checkbox(
            app_state.get_invert_y_scroll(),
            "Inverse direction",
            Message::InvertScroll,
            ui_size.clone(),
        ));

        extra_jump!(10, ret);
        section!(ret, ui_size, "P-stick model");
        ret = ret.push(PickList::new(
            &mut self.dna_parameters_picklist,
            &ensnano_design::NAMED_DNA_PARAMETERS[..],
            Some(app_state.get_dna_parameters().name().clone()),
            Message::NewDnaParameters,
        ));
        for line in app_state.get_dna_parameters().formated_string().lines() {
            ret = ret.push(Text::new(line));
        }
        ret = ret.push(iced::Space::with_height(Length::Units(10)));
        ret = ret.push(Text::new("About").size(ui_size.head_text()));
        ret = ret.push(Text::new(format!(
            "Version {}",
            ensnano_design::ensnano_version()
        )));

        subsection!(ret, ui_size, "Development:");
        ret = ret.push(Text::new("Nicolas Levy"));
        extra_jump!(ret);
        subsection!(ret, ui_size, "Conception:");
        ret = ret.push(Text::new("Nicolas Levy"));
        ret = ret.push(Text::new("Nicolas Schabanel"));
        extra_jump!(ret);
        subsection!(ret, ui_size, "License:");
        ret = ret.push(Text::new("GPLv3"));

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    pub fn update_scroll_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<f32>,
    ) {
        self.scroll_sensitivity_factory
            .update_request(value_id, value, request);
    }
}
