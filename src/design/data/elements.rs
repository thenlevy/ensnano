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
use ensnano_organizer::{
    AttributeDisplay, AttributeWidget, ElementKey, Icon, OrganizerAttribute,
    OrganizerAttributeRepr, OrganizerElement,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Clone, Debug)]
pub enum DnaElement {
    Grid {
        id: usize,
        visible: bool,
    },
    Strand {
        id: usize,
    },
    Helix {
        id: usize,
        group: Option<bool>,
        visible: bool,
    },
    Nucleotide {
        helix: usize,
        position: isize,
        forward: bool,
    },
    CrossOver {
        xover_id: usize,
        helix5prime: usize,
        position5prime: isize,
        forward5prime: bool,
        helix3prime: usize,
        position3prime: isize,
        forward3prime: bool,
    },
}

impl OrganizerElement for DnaElement {
    type Attribute = DnaAttribute;
    type Key = DnaElementKey;

    fn key(&self) -> DnaElementKey {
        match self {
            DnaElement::Grid { id, .. } => DnaElementKey::Grid(*id),
            DnaElement::Strand { id } => DnaElementKey::Strand(*id),
            DnaElement::Helix { id, .. } => DnaElementKey::Helix(*id),
            DnaElement::Nucleotide {
                helix,
                position,
                forward,
            } => DnaElementKey::Nucleotide {
                helix: *helix,
                position: *position,
                forward: *forward,
            },
            DnaElement::CrossOver { xover_id, .. } => DnaElementKey::CrossOver {
                xover_id: *xover_id,
            },
        }
    }

    fn display_name(&self) -> String {
        match self {
            DnaElement::Grid { id, .. } => format!("Grid {}", id),
            DnaElement::Strand { id } => format!("Strand {}", id),
            DnaElement::Helix { id, .. } => format!("Helix {}", id),
            DnaElement::Nucleotide {
                helix,
                position,
                forward,
            } => format!("Nucl {}:{}:{}", helix, position, forward),
            DnaElement::CrossOver {
                helix5prime,
                position5prime,
                forward5prime,
                helix3prime,
                position3prime,
                forward3prime,
                ..
            } => format!(
                "Xover ({}:{}:{}) -> ({}:{}:{})",
                helix5prime,
                position5prime,
                forward5prime,
                helix3prime,
                position3prime,
                forward3prime
            ),
        }
    }

    fn attributes(&self) -> Vec<DnaAttribute> {
        match self {
            DnaElement::Helix { group, .. } => vec![DnaAttribute::XoverGroup(*group)],
            DnaElement::Grid { visible, .. } => vec![DnaAttribute::Visible(*visible)],
            _ => vec![],
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum DnaElementKey {
    Grid(usize),
    Strand(usize),
    Helix(usize),
    Nucleotide {
        helix: usize,
        position: isize,
        forward: bool,
    },
    CrossOver {
        xover_id: usize,
    },
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(usize)]
pub enum DnaElementSection {
    Grid,
    Helix,
    Strand,
    CrossOver,
    Nucleotide,
}

impl ElementKey for DnaElementKey {
    type Section = DnaElementSection;

    fn name(section: DnaElementSection) -> String {
        match section {
            DnaElementSection::Grid => "Grid".to_owned(),
            DnaElementSection::Helix => "Helix".to_owned(),
            DnaElementSection::Strand => "Strand".to_owned(),
            DnaElementSection::CrossOver => "CrossOver".to_owned(),
            DnaElementSection::Nucleotide => "Nucleotide".to_owned(),
        }
    }

    fn section(&self) -> DnaElementSection {
        match self {
            Self::Strand(_) => DnaElementSection::Strand,
            Self::Helix(_) => DnaElementSection::Helix,
            Self::Nucleotide { .. } => DnaElementSection::Nucleotide,
            Self::CrossOver { .. } => DnaElementSection::CrossOver,
            Self::Grid { .. } => DnaElementSection::Grid,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DnaAttribute {
    Visible(bool),
    XoverGroup(Option<bool>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(usize)]
pub enum DnaAttributeRepr {
    Visible,
    XoverGroup,
}

const ALL_DNA_ATTRIBUTE_REPR: [DnaAttributeRepr; 2] =
    [DnaAttributeRepr::Visible, DnaAttributeRepr::XoverGroup];

impl OrganizerAttributeRepr for DnaAttributeRepr {
    fn all_repr() -> &'static [Self] {
        &ALL_DNA_ATTRIBUTE_REPR
    }
}

impl OrganizerAttribute for DnaAttribute {
    type Repr = DnaAttributeRepr;

    fn repr(&self) -> DnaAttributeRepr {
        match self {
            DnaAttribute::Visible(_) => DnaAttributeRepr::Visible,
            DnaAttribute::XoverGroup(_) => DnaAttributeRepr::XoverGroup,
        }
    }

    fn widget(&self) -> AttributeWidget<DnaAttribute> {
        match self {
            DnaAttribute::Visible(b) => AttributeWidget::FlipButton {
                value_if_pressed: DnaAttribute::Visible(!b),
            },
            DnaAttribute::XoverGroup(None) => AttributeWidget::FlipButton {
                value_if_pressed: DnaAttribute::XoverGroup(Some(false)),
            },
            DnaAttribute::XoverGroup(Some(b)) => AttributeWidget::FlipButton {
                value_if_pressed: if *b {
                    DnaAttribute::XoverGroup(None)
                } else {
                    DnaAttribute::XoverGroup(Some(true))
                },
            },
        }
    }

    fn char_repr(&self) -> AttributeDisplay {
        match self {
            DnaAttribute::Visible(b) => {
                let c = if *b {
                    Icon::EyeFill.into()
                } else {
                    Icon::EyeSlash.into()
                };
                AttributeDisplay::Icon(c)
            }
            DnaAttribute::XoverGroup(group) => match group {
                None => AttributeDisplay::Text("\u{2205}".to_owned()),
                Some(false) => AttributeDisplay::Text("G".to_owned()),
                Some(true) => AttributeDisplay::Text("R".to_owned()),
            },
        }
    }
}

impl std::fmt::Display for DnaAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.char_repr() {
                AttributeDisplay::Icon(c) => format!("{}", c),
                AttributeDisplay::Text(s) => s,
            }
        )
    }
}

use super::Nucl;
use crate::mediator::Selection;

impl DnaElementKey {
    pub fn from_selection(selection: &Selection, d_id: u32) -> Option<Self> {
        if selection.get_design() == Some(d_id) {
            match selection {
                Selection::Grid(_, g_id) => Some(Self::Grid(*g_id)),
                Selection::Design(_) => None,
                Selection::Helix(_, h_id) => Some(Self::Helix(*h_id as usize)),
                Selection::Strand(_, s_id) => Some(Self::Strand(*s_id as usize)),
                Selection::Nucleotide(_, nucl) => Some(Self::Nucleotide {
                    helix: nucl.helix,
                    position: nucl.position,
                    forward: nucl.forward,
                }),
                Selection::Bound(_, _, _) => None,
                Selection::Xover(_, xover_id) => Some(Self::CrossOver {
                    xover_id: *xover_id,
                }),
                Selection::Phantom(pe) => {
                    if pe.bound {
                        None
                    } else {
                        let nucl = pe.to_nucl();
                        Some(Self::Nucleotide {
                            helix: nucl.helix,
                            position: nucl.position,
                            forward: nucl.forward,
                        })
                    }
                }
                Selection::Nothing => None,
            }
        } else {
            None
        }
    }

    pub fn to_selection(&self, d_id: u32) -> Selection {
        match self {
            Self::Nucleotide {
                helix,
                position,
                forward,
            } => Selection::Nucleotide(
                d_id,
                Nucl {
                    helix: *helix,
                    position: *position,
                    forward: *forward,
                },
            ),
            Self::CrossOver { xover_id } => Selection::Xover(d_id, *xover_id),
            Self::Helix(h_id) => Selection::Helix(d_id, *h_id as u32),
            Self::Strand(s_id) => Selection::Strand(d_id, *s_id as u32),
            Self::Grid(g_id) => Selection::Grid(d_id, *g_id),
        }
    }
}
