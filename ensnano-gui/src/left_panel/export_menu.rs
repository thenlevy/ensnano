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
use iced_native::widget::scrollable;

#[derive(Default)]
pub struct ExportMenu {
    scroll: scrollable::State,
    button_cancel: button::State,
    button_oxdna: button::State,
    button_pdb: button::State,
    button_cadnano: button::State,
}

impl ExportMenu {
    pub fn view<'a, S: AppState>(&'a mut self) -> Element<'a, Message<S>> {
        let ret = Column::new()
            .push(
                Button::new(&mut self.button_cancel, Text::new("Cancel"))
                    .on_press(Message::CancelExport),
            )
            .push(
                Button::new(&mut self.button_oxdna, Text::new("Oxdna"))
                    .on_press(Message::Export(ExportType::Oxdna)),
            )
            .push(
                Button::new(&mut self.button_pdb, Text::new("Pdb"))
                    .on_press(Message::Export(ExportType::Pdb)),
            )
            .push(
                Button::new(&mut self.button_cadnano, Text::new("Cadnano"))
                    .on_press(Message::Export(ExportType::Cadnano)),
            );

        Scrollable::new(&mut self.scroll).push(ret).into()
    }
}
