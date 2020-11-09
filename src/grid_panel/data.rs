use std::sync::{Arc, Mutex};
use super::{Design, ViewPtr};

pub struct Data {
    view: ViewPtr,
    designs: Vec<Arc<Mutex<Design>>>,
    selected_grid: Option<usize>,
}

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self {
            view,
            designs: Vec::new(),
            selected_grid: None,
        }
    }
}
