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

use iced::image::Handle;

use ensnano_interactor::{ActionMode, SelectionMode};

pub trait HasIcon {
    fn icon_on(&self) -> Handle;
    fn icon_off(&self) -> Handle;
}

impl HasIcon for SelectionMode {
    fn icon_on(&self) -> Handle {
        let bytes = match self {
            Self::Grid { .. } => include_bytes!("../../icons/icons/Grid-on32.png").to_vec(),
            Self::Helix => include_bytes!("../../icons/icons/Helix-on32.png").to_vec(),
            Self::Nucleotide => include_bytes!("../../icons/icons/Nucleotide-on32.png").to_vec(),
            Self::Strand => include_bytes!("../../icons/icons/Strand-on32.png").to_vec(),
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }

    fn icon_off(&self) -> Handle {
        let bytes = match self {
            Self::Grid { .. } => include_bytes!("../../icons/icons/Grid-off32.png").to_vec(),
            Self::Helix => include_bytes!("../../icons/icons/Helix-off32.png").to_vec(),
            Self::Nucleotide => include_bytes!("../../icons/icons/Nucleotide-off32.png").to_vec(),
            Self::Strand => include_bytes!("../../icons/icons/Strand-off32.png").to_vec(),
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }
}

pub trait HasIconDependentOnAxis {
    fn icon_on(&self, on_axis: bool) -> Handle;
    fn icon_off(&self, on_axis: bool) -> Handle;
}

impl HasIconDependentOnAxis for ActionMode {
    fn icon_on(&self, axis_aligned: bool) -> Handle {
        let bytes = match self {
            Self::BuildHelix { .. } => {
                include_bytes!("../../icons/icons/NewHelix-on32.png").to_vec()
            }
            Self::Normal => include_bytes!("../../icons/icons/Select-on32.png").to_vec(),
            Self::Translate => {
                if axis_aligned {
                    include_bytes!("../../icons/icons/Move-on32.png").to_vec()
                } else {
                    include_bytes!("../../icons/icons/Move-on-in32.png").to_vec()
                }
            }
            Self::Rotate => {
                if axis_aligned {
                    include_bytes!("../../icons/icons/Rotate-on32.png").to_vec()
                } else {
                    include_bytes!("../../icons/icons/Rotate-on-in32.png").to_vec()
                }
            }
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }

    fn icon_off(&self, axis_aligned: bool) -> Handle {
        let bytes = match self {
            Self::BuildHelix { .. } => {
                include_bytes!("../../icons/icons/NewHelix-off32.png").to_vec()
            }
            Self::Normal => include_bytes!("../../icons/icons/Select-off32.png").to_vec(),
            Self::Translate => {
                if axis_aligned {
                    include_bytes!("../../icons/icons/Move-off32.png").to_vec()
                } else {
                    include_bytes!("../../icons/icons/Move-off-in32.png").to_vec()
                }
            }
            Self::Rotate => {
                if axis_aligned {
                    include_bytes!("../../icons/icons/Rotate-off32.png").to_vec()
                } else {
                    include_bytes!("../../icons/icons/Rotate-off-in32.png").to_vec()
                }
            }
            _ => vec![],
        };
        Handle::from_memory(bytes)
    }
}
