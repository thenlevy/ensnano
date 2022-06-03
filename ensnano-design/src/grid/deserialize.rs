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
use crate::BezierVertexId;

use super::*;

#[derive(Deserialize)]
enum NewGridTypeDescr {
    Square {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        twist: Option<f64>,
    },
    Honeycomb {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        twist: Option<f64>,
    },
    Hyperboloid {
        radius: usize,
        shift: f32,
        length: f32,
        radius_shift: f32,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        forced_radius: Option<f32>,
        #[serde(default)]
        nb_turn_per_100_nt: f64,
    },
}

impl NewGridTypeDescr {
    fn to_real(self) -> GridTypeDescr {
        match self {
            Self::Square { twist } => GridTypeDescr::Square { twist },
            Self::Honeycomb { twist } => GridTypeDescr::Honeycomb { twist },
            Self::Hyperboloid {
                radius,
                shift,
                length,
                radius_shift,
                forced_radius,
                nb_turn_per_100_nt,
            } => GridTypeDescr::Hyperboloid {
                radius,
                shift,
                length,
                radius_shift,
                forced_radius,
                nb_turn_per_100_nt,
            },
        }
    }
}

#[derive(Deserialize)]
enum OldGridTypeDescr {
    Square,
    Honeycomb,
    Hyperboloid {
        radius: usize,
        shift: f32,
        length: f32,
        radius_shift: f32,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        forced_radius: Option<f32>,
        #[serde(default)]
        nb_turn_per_100_nt: f64,
    },
}

impl OldGridTypeDescr {
    fn to_new(self) -> GridTypeDescr {
        match self {
            Self::Square => GridTypeDescr::Square { twist: None },
            Self::Honeycomb => GridTypeDescr::Honeycomb { twist: None },
            Self::Hyperboloid {
                radius,
                shift,
                length,
                radius_shift,
                forced_radius,
                nb_turn_per_100_nt,
            } => GridTypeDescr::Hyperboloid {
                radius,
                shift,
                length,
                radius_shift,
                forced_radius,
                nb_turn_per_100_nt,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NewOrOld {
    New(NewGridTypeDescr),
    Old(OldGridTypeDescr),
}

use serde::Deserialize;
impl<'de> Deserialize<'de> for GridTypeDescr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match NewOrOld::deserialize(deserializer) {
            Ok(NewOrOld::New(desc)) => Ok(desc.to_real()),
            Ok(NewOrOld::Old(desc)) => Ok(desc.to_new()),
            Err(e) => Err(e),
        }
    }
}

#[derive(Deserialize)]
enum NewGridId {
    FreeGrid(usize),
    BezierPathGrid(BezierVertexId),
}

impl NewGridId {
    fn to_real(self) -> GridId {
        match self {
            Self::FreeGrid(id) => GridId::FreeGrid(id),
            Self::BezierPathGrid(vertex) => GridId::BezierPathGrid(vertex),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NewOrOldGridId {
    New(NewGridId),
    Old(usize),
}

impl<'de> Deserialize<'de> for GridId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match NewOrOldGridId::deserialize(deserializer) {
            Ok(NewOrOldGridId::New(id)) => Ok(id.to_real()),
            Ok(NewOrOldGridId::Old(id)) => Ok(GridId::FreeGrid(id)),
            Err(e) => Err(e),
        }
    }
}
