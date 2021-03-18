use ensnano_organizer::{
    AttributeDisplay, AttributeWidget, ElementKey, Icon, OrganizerAttribute,
    OrganizerAttributeRepr, OrganizerElement,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Clone, Debug)]
pub enum DnaElement {
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
            DnaElement::CrossOver {
                helix5prime,
                position5prime,
                forward5prime,
                helix3prime,
                position3prime,
                forward3prime,
            } => DnaElementKey::CrossOver {
                helix5prime: *helix5prime,
                position5prime: *position5prime,
                forward5prime: *forward5prime,
                helix3prime: *helix3prime,
                position3prime: *position3prime,
                forward3prime: *forward3prime,
            },
        }
    }

    fn display_name(&self) -> String {
        match self {
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
            DnaElement::Helix { group, visible, .. } => vec![
                DnaAttribute::Visible(*visible),
                DnaAttribute::XoverGroup(*group),
            ],
            _ => vec![],
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum DnaElementKey {
    Strand(usize),
    Helix(usize),
    Nucleotide {
        helix: usize,
        position: isize,
        forward: bool,
    },
    CrossOver {
        helix5prime: usize,
        position5prime: isize,
        forward5prime: bool,
        helix3prime: usize,
        position3prime: isize,
        forward3prime: bool,
    },
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(usize)]
pub enum DnaElementSection {
    Strand,
    Helix,
    Nucleotide,
    CrossOver,
}

impl ElementKey for DnaElementKey {
    type Section = DnaElementSection;

    fn name(section: DnaElementSection) -> String {
        match section {
            DnaElementSection::Strand => "Strand".to_owned(),
            DnaElementSection::Helix => "Helix".to_owned(),
            DnaElementSection::Nucleotide => "Nucleotide".to_owned(),
            DnaElementSection::CrossOver => "CrossOver".to_owned(),
        }
    }

    fn section(&self) -> DnaElementSection {
        match self {
            Self::Strand(_) => DnaElementSection::Strand,
            Self::Helix(_) => DnaElementSection::Helix,
            Self::Nucleotide { .. } => DnaElementSection::Nucleotide,
            Self::CrossOver { .. } => DnaElementSection::CrossOver,
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
                None => AttributeDisplay::Text("B".to_owned()),
                Some(false) => AttributeDisplay::Text("R".to_owned()),
                Some(true) => AttributeDisplay::Text("G".to_owned()),
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
