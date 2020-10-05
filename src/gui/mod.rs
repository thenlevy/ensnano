pub mod top_bar;
pub use top_bar::TopBar;
pub mod left_panel;
pub use left_panel::LeftPanel;

use crate::scene::{RotationMode, SelectionMode};
use std::path::PathBuf;

pub struct Requests {
    pub rotation_mode: Option<RotationMode>,
    pub selection_mode: Option<SelectionMode>,
    pub fitting: bool,
    pub file_add: Option<PathBuf>,
    pub file_clear: bool,
    pub file_save: Option<PathBuf>,
    pub strand_color_change: Option<u32>,
}

impl Requests {
    pub fn new() -> Self {
        Self {
            rotation_mode: None,
            selection_mode: None,
            fitting: false,
            file_add: None,
            file_clear: false,
            file_save: None,
            strand_color_change: None,
        }
    }
}
