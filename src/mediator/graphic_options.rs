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
