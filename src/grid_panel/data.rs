use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use super::{Design, ViewPtr};
use super::view::{CharInstance, CircleInstance};

use ultraviolet::{Mat2, Vec2};

pub struct Data {
    view: ViewPtr,
    designs: Vec<Arc<Mutex<Design>>>,
    selected_grid: usize,
    selected_design: usize,
}

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self {
            view,
            designs: Vec::new(),
            selected_grid: 0,
            selected_design: 0,
        }
    }

    pub fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        self.designs.push(design);
    }

    pub fn update_view(&self) {
        let grid_opt = self.designs.get(self.selected_grid).and_then(|d| d.lock().unwrap().get_grid2d(self.selected_grid));
        let mut circles = Vec::new();
        {
            let mut view = self.view.borrow_mut();
            let char_map = view.get_char_map();
            if let Some(grid) = grid_opt {
                for ((x, y), h_id) in grid.read().unwrap().helices().iter() {
                    let position = grid.read().unwrap().helix_position(*x, *y);
                    circles.push(CircleInstance {
                        center: position
                    });
                    add_char_instances(char_map, position, *h_id);
                }
            }
            // drop view
        }
        self.view.borrow_mut().update_circles(circles);
    }

}
fn add_char_instances(
    char_map: &mut HashMap<char, Vec<CharInstance>>,
    position: Vec2,
    id: usize,
) {
    let nb_chars = id.to_string().len(); // ok to use len because digits are ascii
    for (c_idx, c) in id.to_string().chars().enumerate() {
        let instances = char_map.get_mut(&c).unwrap();
        instances.push(CharInstance {
            center: position + (c_idx as f32 - (nb_chars - 1) as f32 / 2.) * (1. / nb_chars as f32) * Vec2::unit_x(),
            rotation: Mat2::identity(),
            size: 0.7 / nb_chars as f32,
            z_index: -1,
        })
    }
}
