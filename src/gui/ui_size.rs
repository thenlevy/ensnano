pub const ALL_UI_SIZE: [UiSize; 3] = [UiSize::Small, UiSize::Medium, UiSize::Large];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiSize {
    Small,
    Medium,
    Large,
}

impl UiSize {
    pub fn main_text(&self) -> u16 {
        match self {
            Self::Small => 12,
            Self::Medium => 16,
            Self::Large => 20,
        }
    }

    pub fn icon(&self) -> u16 {
        match self {
            Self::Small => 14,
            Self::Medium => 20,
            Self::Large => 30,
        }
    }

    pub fn checkbox(&self) -> u16 {
        match self {
            Self::Small => 15,
            Self::Medium => 17,
            Self::Large => 19,
        }
    }

    pub fn button(&self) -> u16 {
        match self {
            Self::Small => 25,
            Self::Medium => 35,
            Self::Large => 45,
        }
    }

    pub fn top_bar(&self) -> u16 {
        self.button()
    }
}

impl std::fmt::Display for UiSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ret = match self {
            UiSize::Small => "Small",
            UiSize::Medium => "Medium",
            UiSize::Large => "Large",
        };
        write!(f, "{}", ret)
    }
}
