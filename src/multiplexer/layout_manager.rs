use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::ElementType;

/// A node of a `LayoutTree`
enum LayoutNode {
    /// A leaf of a `LayoutTree`. Represents an area that can be drawn on.
    /// The first 4 attributes represents the boundaries of the area, expressed between 0. and 1.,
    /// the last attribute is the identifier of the area.
    Area(f64, f64, f64, f64, usize),

    /// A Node representing an horizontal splitting of an area.
    /// In `VSplit(y, t, b)` `y` represents the proportion of the area that is given to the top
    /// area, `t` is a pointer to the node representing the top area, and `b` is a pointer to the node
    /// representing the bottomg area.
    VSplit(f64, Rc<RefCell<LayoutNode>>, Rc<RefCell<LayoutNode>>),

    /// A Node representing a vertical splitting of an area.
    /// In `HSplit(x, l, r)` `x` represents the proportion of the area that is given to the left
    /// area, `l` is a pointer to the node representing the left area, and `r` is a pointer to the node
    /// representing the right area.
    #[allow(dead_code)]
    HSplit(f64, Rc<RefCell<LayoutNode>>, Rc<RefCell<LayoutNode>>),
}

type LayoutNodePtr = Rc<RefCell<LayoutNode>>;

pub struct LayoutTree {
    /// The root of the LayoutTree
    root: LayoutNodePtr,
    /// An array mapping area identifier to leaves of the LayoutTree
    area: Vec<LayoutNodePtr>,
    /// An array mapping area identifier to ElementType
    element_type: Vec<ElementType>,
    /// A HashMap mapping element types to area identifer
    area_identifer: HashMap<ElementType, usize>,
}

impl LayoutTree {
    /// Create a new Layout Tree.
    pub fn new() -> Self {
        let root = Rc::new(RefCell::new(LayoutNode::Area(0., 0., 1., 1., 0)));
        let mut area = Vec::new();
        area.push(root.clone());
        let element_type = vec![ElementType::Unattributed];
        let area_identifer = HashMap::new();
        Self {
            root,
            area,
            element_type,
            area_identifer,
        }
    }

    /// Vertically split an area in two.
    ///
    /// # Arguments
    ///
    /// * `area_idx` the idenfier of the area beein split.
    ///
    /// * `top_proportion`, the proportion of the initial area attributed to the top area
    ///
    /// # Return value
    ///
    /// A pair `(t, b)` where `t` is the identifier of the top area and `b` the identifier of the
    /// bottom area
    pub fn vsplit(&mut self, area_idx: usize, top_proportion: f64) -> (usize, usize) {
        let bottom_idx = self.area.len();
        let (top, bottom) = {
            let mut area = self.area[area_idx].borrow_mut();
            area.vsplit(top_proportion, bottom_idx)
        };
        self.area[area_idx] = top;
        self.area.push(bottom);
        self.element_type.push(ElementType::Unattributed);
        (area_idx, bottom_idx)
    }

    /// Horizontally split an area in two.
    ///
    /// # Arguments
    ///
    /// * `area_idx` the idenfier of the area beein split.
    ///
    /// * `left_proportion`, the proportion of the initial area attributed to the left area
    ///
    /// # Return value
    ///
    /// A pair `(l, r)` where `l` is the identifier of the left area and `r` the identifier of the
    /// right area
    #[allow(dead_code)]
    pub fn hsplit(&mut self, area_idx: usize, left_proportion: f64) -> (usize, usize) {
        let right_idx = self.area.len();
        let (left, right) = {
            let mut area = self.area[area_idx].borrow_mut();
            area.hsplit(left_proportion, right_idx)
        };
        self.area[area_idx] = left;
        self.area.push(right);
        self.element_type.push(ElementType::Unattributed);
        (area_idx, right_idx)
    }

    /// Get the Element owning the pixel `(x, y)`
    pub fn get_area_pixel(&self, x: f64, y: f64) -> ElementType {
        let identifier = self.root.borrow().get_area_pixel(x, y);
        self.element_type[identifier]
    }

    /// Return the boundaries of the area attributed to an element
    pub fn get_area(&self, element: ElementType) -> (f64, f64, f64, f64) {
        let area_id = *self
            .area_identifer
            .get(&element)
            .expect("Try to get the area of an element that was not given one");
        match *self.area[area_id].borrow() {
            LayoutNode::Area(left, top, right, bottom, _) => (left, top, right, bottom),
            _ => panic!("got split_node"),
        }
    }

    /// Attribute an element_type to an area.
    pub fn attribute_element(&mut self, area: usize, element_type: ElementType) {
        self.element_type[area] = element_type;
        self.area_identifer.insert(element_type, area);
    }
}

impl LayoutNode {
    /// Vertically split an area in two.
    ///
    /// # Arguments
    ///
    /// * `top_proportion`, the proportion of the initial area attributed to the top area
    ///
    /// * `bottom_idx`, the identifier of the bottom area. The top area is given the identifier of
    /// its parent.
    ///
    /// # Return value
    ///
    /// A pair `(t, b)` where `t` is a pointer to the top area and `b` is a pointer to the bottom
    /// area
    pub fn vsplit(
        &mut self,
        top_proportion: f64,
        bottom_idx: usize,
    ) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(top_proportion >= 0. && top_proportion <= 1.);
        match self {
            LayoutNode::Area(left, top, right, bottom, idx) => {
                let separation = top_proportion * (*top + *bottom);
                let top_area = Rc::new(RefCell::new(LayoutNode::Area(
                    *left, *top, *right, separation, *idx,
                )));
                let bottom_area = Rc::new(RefCell::new(LayoutNode::Area(
                    *left, separation, *right, *bottom, bottom_idx,
                )));
                *self = LayoutNode::VSplit(separation, top_area.clone(), bottom_area.clone());
                (top_area, bottom_area)
            }
            _ => {
                panic!("splitting a node");
            }
        }
    }

    /// Horizontally split an area in two.
    ///
    /// # Arguments
    ///
    /// * `left_proportion`, the proportion of the initial area attributed to the left area
    ///
    /// * `right_idx`, the identifier of the right area. The left area is given the identifier of
    /// its parent.
    ///
    /// # Return value
    ///
    /// A pair `(l, r)` where `l` is a pointer to the left area and `r` is a pointer to the right
    /// area
    #[allow(dead_code)]
    pub fn hsplit(
        &mut self,
        left_proportion: f64,
        right_idx: usize,
    ) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(left_proportion >= 0. && left_proportion <= 1.);
        match self {
            LayoutNode::Area(left, top, right, bottom, idx) => {
                let separation = left_proportion * (*left + *right);
                let left_area = Rc::new(RefCell::new(LayoutNode::Area(
                    *left, *top, separation, *bottom, *idx,
                )));
                let right_area = Rc::new(RefCell::new(LayoutNode::Area(
                    separation, *top, *right, *bottom, right_idx,
                )));
                *self = LayoutNode::HSplit(separation, left_area.clone(), right_area.clone());
                (left_area, right_area)
            }
            _ => {
                panic!("splitting a node");
            }
        }
    }

    /// Return the identifier of the leaf owning pixel `(x, y)`
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
