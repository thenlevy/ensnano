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

pub use ensnano_interactor::consts::*;
pub const MIN_NB_TURN: f32 = -5.0;
pub const MAX_NB_TURN: f32 = 5.0;
pub const NB_TURN_STEP: f32 = 0.05;

pub const NB_TURN_SLIDER_SPACING: u16 = 3;

use iced::Color;
pub const fn innactive_color() -> Color {
    Color::from_rgb(0.6, 0.6, 0.6)
}
