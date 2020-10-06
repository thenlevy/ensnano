pub mod top_bar;
pub use top_bar::TopBar;
pub mod left_panel;
pub use left_panel::LeftPanel;

use crate::scene::{RotationMode, SelectionMode};
use std::path::PathBuf;

/// A structure that contains all the requests that can be made through the GUI.
pub struct Requests {
    /// A change of the rotation mode
    pub rotation_mode: Option<RotationMode>,
    /// A change of the selection mode
    pub selection_mode: Option<SelectionMode>,
    /// A request to move the camera so that the frustrum fits the desgin
    pub fitting: bool,
    /// A request to load a design into the scene
    pub file_add: Option<PathBuf>,
    /// A request to remove all designs
    pub file_clear: bool,
    /// A request to save the selected design
    pub file_save: Option<PathBuf>,
    /// A request to change the color of the selcted strand
    pub strand_color_change: Option<u32>,
}

impl Requests {
    /// Initialise the request structures with no requests
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
