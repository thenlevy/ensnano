use std::rc::Rc;
use std::cell::RefCell;

enum LayoutNode {
    Area(f64, f64, f64, f64, usize),
    VSplit(f64, Rc<RefCell<LayoutNode>>, Rc<RefCell<LayoutNode>>),
    HSplit(f64, Rc<RefCell<LayoutNode>>, Rc<RefCell<LayoutNode>>),
}

type LayoutNodePtr = Rc<RefCell<LayoutNode>>;

pub struct LayoutTree {
    root: LayoutNodePtr,
    area: Vec<LayoutNodePtr>,
}

impl LayoutTree {
    pub fn new() -> Self {
        let root = Rc::new(RefCell::new(LayoutNode::Area(0., 0., 1., 1., 0)));
        let mut area = Vec::new();
        area.push(root.clone());
        Self {
            root,
            area,
        }
    }

    pub fn vsplit(&mut self, area_idx: usize, top_proportion: f64) -> (usize, usize) {
        let bottom_idx = self.area.len();
        let (top, bottom) = {
            let mut area = self.area[area_idx].borrow_mut();
            area.vsplit(top_proportion, bottom_idx)
        };
        self.area[area_idx] = top;
        self.area.push(bottom);
        (area_idx, bottom_idx)
    }

    pub fn hsplit(&mut self, area_idx: usize, left_proportion: f64) -> (usize, usize) {
        let right_idx = self.area.len();
        let (left, right) = {
            let mut area = self.area[area_idx].borrow_mut();
            area.hsplit(left_proportion, right_idx)
        };
        self.area[area_idx] = left;
        self.area.push(right);
        (area_idx, right_idx)
    }

    pub fn get_area_pixel(&self, x: f64, y: f64) -> usize {
        self.root.borrow().get_area_pixel(x, y)
    }

    pub fn get_area(&self, area: usize) -> (f64, f64, f64, f64) {
        match *self.area[area].borrow() {
            LayoutNode::Area(left, top, right, bottom, _) => (left, top, right, bottom),
            _ => panic!("got split_node")
        }
    }
}

impl LayoutNode {
    pub fn vsplit(&mut self, top_proportion: f64, bottom_idx: usize) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(top_proportion >= 0. && top_proportion <= 1.);
        match self {
            LayoutNode::Area(left, top, right, bottom, idx) => {
                let separation = top_proportion * (*top + *bottom);
                println!("separation {}", separation);
                let top_area = Rc::new(RefCell::new(LayoutNode::Area(*left, *top, *right, separation, *idx)));
                let bottom_area = Rc::new(RefCell::new(LayoutNode::Area(*left, separation, *right, *bottom, bottom_idx)));
                *self = LayoutNode::VSplit(separation, top_area.clone(), bottom_area.clone());
                (top_area, bottom_area)
            }
            _ => {
                panic!("splitting a node");
            }
        }
    }

    pub fn hsplit(&mut self, left_proportion: f64, right_idx: usize) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(left_proportion >= 0. && left_proportion <= 1.);
        match self {
            LayoutNode::Area(left, top, right, bottom, idx) => {
                let separation = left_proportion * (*left + *right);
                let left_area = Rc::new(RefCell::new(LayoutNode::Area(*left, *top, separation, *bottom, *idx)));
                let right_area = Rc::new(RefCell::new(LayoutNode::Area(separation, *top, *right, *bottom, right_idx)));
                *self = LayoutNode::HSplit(separation, left_area.clone(), right_area.clone());
                (left_area, right_area)
            }
            _ => {
                panic!("splitting a node");
            }
        }
    }

    pub fn get_area_pixel(&self, x: f64, y: f64) -> usize {
        match self {
            LayoutNode::Area(_, _, _, _, idx) => *idx,
            LayoutNode::VSplit(separation, top, bottom) => {
                if y <= *separation {
                    top.borrow().get_area_pixel(x, y)
                } else {
                    bottom.borrow().get_area_pixel(x, y)
                }
            }
            LayoutNode::HSplit(separation, left, right) => {
                if x <= *separation {
                    left.borrow().get_area_pixel(x, y)
                } else {
                    right.borrow().get_area_pixel(x, y)
                }
            }
        }
    }

}


