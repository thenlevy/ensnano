use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::ElementType;

/// A node of a `LayoutTree`
#[derive(Clone)]
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
    /// An array mapping area to their parent node
    parent: Vec<usize>,
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
            parent: vec![0],
        }
    }

    /// Vertically split an area in two.
    ///
    /// # Arguments
    ///
    /// * `parent_idx` the idenfier of the area beein split.
    ///
    /// * `top_proportion`, the proportion of the initial area attributed to the top area
    ///
    /// # Return value
    ///
    /// A pair `(t, b)` where `t` is the identifier of the top area and `b` the identifier of the
    /// bottom area
    pub fn vsplit(&mut self, parent_idx: usize, top_proportion: f64) -> (usize, usize) {
        let top_idx = self.area.len();
        let bottom_idx = self.area.len() + 1;
        let (top, bottom) = {
            let mut area = self.area[parent_idx].borrow_mut();
            area.vsplit(top_proportion, top_idx, bottom_idx)
        };
        self.area.push(top);
        self.area.push(bottom);
        self.parent.push(parent_idx);
        self.parent.push(parent_idx);
        self.element_type.push(ElementType::Unattributed);
        self.element_type.push(ElementType::Unattributed);
        let old_element = self.element_type[parent_idx];
        self.area_identifer.remove(&old_element);
        self.element_type[parent_idx] = ElementType::Unattributed;
        (top_idx, bottom_idx)
    }

    /// Horizontally split an area in two.
    ///
    /// # Arguments
    ///
    /// * `parent_idx` the idenfier of the area beein split.
    ///
    /// * `left_proportion`, the proportion of the initial area attributed to the left area
    ///
    /// # Return value
    ///
    /// A pair `(l, r)` where `l` is the identifier of the left area and `r` the identifier of the
    /// right area
    #[allow(dead_code)]
    pub fn hsplit(&mut self, parent_idx: usize, left_proportion: f64) -> (usize, usize) {
        let left_idx = self.area.len();
        let right_idx = self.area.len() + 1;
        let (left, right) = {
            let mut area = self.area[parent_idx].borrow_mut();
            area.hsplit(left_proportion, left_idx, right_idx)
        };
        self.area.push(left);
        self.area.push(right);
        self.parent.push(parent_idx);
        self.parent.push(parent_idx);
        self.element_type.push(ElementType::Unattributed);
        self.element_type.push(ElementType::Unattributed);
        let old_element = self.element_type[parent_idx];
        self.area_identifer.remove(&old_element);
        self.element_type[parent_idx] = ElementType::Unattributed;
        (left_idx, right_idx)
    }

    pub fn merge(&mut self, old_leaf: ElementType, new_leaf: ElementType) {
        let area_id = *self
            .area_identifer
            .get(&old_leaf)
            .expect("Try to get the area of an element that was not given one");
        let parent_id = self.parent[area_id];
        let childs = self.area[parent_id].borrow_mut().merge(parent_id);
        let old_brother = self.element_type[childs.1];
        self.area_identifer.remove(&old_leaf);
        self.area_identifer.remove(&old_brother);
        self.attribute_element(parent_id, new_leaf);
    }

    /// Get the Element owning the pixel `(x, y)`
    pub fn get_area_pixel(&self, x: f64, y: f64) -> ElementType {
        let identifier = self.root.borrow().get_area_pixel(x, y);
        self.element_type[identifier]
    }

    /// Return the boundaries of the area attributed to an element
    pub fn get_area(&self, element: ElementType) -> Option<(f64, f64, f64, f64)> {
        let area_id = *self.area_identifer.get(&element)?;
        match *self.area[area_id].borrow() {
            LayoutNode::Area(left, top, right, bottom, _) => Some((left, top, right, bottom)),
            _ => panic!("got split_node"),
        }
    }

    pub fn get_area_id(&self, element: ElementType) -> Option<usize> {
        self.area_identifer.get(&element).cloned()
    }

    /// Attribute an element_type to an area.
    pub fn attribute_element(&mut self, area: usize, element_type: ElementType) {
        let old_element = self.element_type[area];
        self.area_identifer.remove(&old_element);
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
    /// * `top_idx`, the identifier of the top area.
    ///
    /// * `bottom_idx`, the identifier of the bottom area.
    ///
    /// # Return value
    ///
    /// A pair `(t, b)` where `t` is a pointer to the top area and `b` is a pointer to the bottom
    /// area
    pub fn vsplit(
        &mut self,
        top_proportion: f64,
        top_idx: usize,
        bottom_idx: usize,
    ) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(top_proportion >= 0. && top_proportion <= 1.);
        match self {
            LayoutNode::Area(left, top, right, bottom, _) => {
                let separation = top_proportion * (*top + *bottom);
                let top_area = Rc::new(RefCell::new(LayoutNode::Area(
                    *left, *top, *right, separation, top_idx,
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
    /// * `left_idx`, the identifier to be given to the left area.
    ///
    /// * `right_idx`, the identifier to be given to the right area.
    ///
    /// # Return value
    ///
    /// A pair `(l, r)` where `l` is a pointer to the left area and `r` is a pointer to the right
    /// area
    #[allow(dead_code)]
    pub fn hsplit(
        &mut self,
        left_proportion: f64,
        left_idx: usize,
        right_idx: usize,
    ) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(left_proportion >= 0. && left_proportion <= 1.);
        match self {
            LayoutNode::Area(left, top, right, bottom, _) => {
                let separation = left_proportion * (*left + *right);
                let left_area = Rc::new(RefCell::new(LayoutNode::Area(
                    *left, *top, separation, *bottom, left_idx,
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

    /// Merge the two children of self. These children must be leaves
    pub fn merge(&mut self, idx: usize) -> (usize, usize) {
        let ret;
        let new_self = match self {
            LayoutNode::VSplit(_, top, bottom) => {
                match (top.borrow().clone(), bottom.borrow().clone()) {
                    (
                        LayoutNode::Area(left, top, right, _, c1),
                        LayoutNode::Area(_, _, _, bottom, c2),
                    ) => {
                        ret = (c1, c2);
                        LayoutNode::Area(left, top, right, bottom, idx)
                    }
                    _ => panic!("merge"),
                }
            }
            LayoutNode::HSplit(_, left, right) => {
                match (left.borrow().clone(), right.borrow().clone()) {
                    (
                        LayoutNode::Area(left, top, _, bottom, c1),
                        LayoutNode::Area(_, _, right, _, c2),
                    ) => {
                        ret = (c1, c2);
                        LayoutNode::Area(left, top, right, bottom, idx)
                    }
                    _ => panic!("merge"),
                }
            }
            _ => panic!("merging a leaf"),
        };
        *self = new_self;
        ret
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
