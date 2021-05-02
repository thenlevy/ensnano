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
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum RenderingMode {
    Normal,
    Cartoon,
}

pub const ALL_RENDERING_MODE: [RenderingMode; 2] = [RenderingMode::Normal, RenderingMode::Cartoon];

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum Background3D {
    Sky,
    White,
}

pub const ALL_BACKGROUND3D: [Background3D; 2] = [Background3D::Sky, Background3D::White];

impl Default for Background3D {
    fn default() -> Self {
        Self::Sky
    }
}

impl std::fmt::Display for Background3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ret = match self {
            Self::White => "White",
            Self::Sky => "Sky",
        };
        write!(f, "{}", ret)
    }
}

impl std::fmt::Display for RenderingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ret = match self {
            Self::Normal => "Normal",
            Self::Cartoon => "Cartoon",
        };
        write!(f, "{}", ret)
    }
}
