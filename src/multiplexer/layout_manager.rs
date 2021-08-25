/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::PhysicalPosition;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::ElementType;

const RESIZE_REGION_WIDTH: f64 = 0.001;

/// A node of a `LayoutTree`
#[derive(Clone)]
enum LayoutNode {
    /// A leaf of a `LayoutTree`. Represents an area that can be drawn on.
    /// The first 4 attributes represents the boundaries of the area, expressed between 0. and 1.,
    /// the last attribute is the identifier of the area.
    Area {
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        identifier: usize,
    },

    /// A Node representing a vertical splitting of an area.
    VSplit {
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        left_proportion: f64,
        l_child: Rc<RefCell<LayoutNode>>,
        r_child: Rc<RefCell<LayoutNode>>,
        resizable: Option<usize>,
    },

    /// A Node representing a horizontal splitting of an area.
    HSplit {
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        top_proportion: f64,
        t_child: Rc<RefCell<LayoutNode>>,
        b_child: Rc<RefCell<LayoutNode>>,
        resizable: Option<usize>,
    },
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
        let root = Rc::new(RefCell::new(LayoutNode::Area {
            left: 0.,
            top: 0.,
            right: 1.,
            bottom: 1.,
            identifier: 0,
        }));
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
    /// * `left_proportion`, the proportion of the initial area attributed to the left child
    ///
    /// # Return value
    ///
    /// A pair `(l, r)` where `l` is the identifier of the top area and `r` the identifier of the
    /// bottom area
    pub fn vsplit(
        &mut self,
        parent_idx: usize,
        left_proportion: f64,
        resizable: bool,
    ) -> (usize, usize) {
        let left_idx = self.area.len();
        let right_idx = self.area.len() + 1;
        let (left, right) = {
            let mut area = self.area[parent_idx].borrow_mut();
            area.vsplit(left_proportion, left_idx, right_idx, resizable)
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

    /// Horizontally split an area in two.
    ///
    /// # Arguments
    ///
    /// * `parent_idx` the idenfier of the area beein split.
    ///
    /// * `top_proportion`, the proportion of the initial area attributed to the top child
    ///
    /// # Return value
    ///
    /// A pair `(t, b)` where `t` is the identifier of the left area and `b` the identifier of the
    /// right area
    #[allow(dead_code)]
    pub fn hsplit(
        &mut self,
        parent_idx: usize,
        top_proportion: f64,
        resizable: bool,
    ) -> (usize, usize) {
        let top_idx = self.area.len();
        let bottom_idx = self.area.len() + 1;
        let (top, bottom) = {
            let mut area = self.area[parent_idx].borrow_mut();
            area.hsplit(top_proportion, top_idx, bottom_idx, resizable)
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

    /// Undo a split by deleting the leaf with type `old_leaf` and its sibiling and giving their parent the type
    /// `new_leaf`.
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
    pub(super) fn get_area_pixel(&self, x: f64, y: f64) -> PixelRegion {
        let identifier = self.root.borrow().get_area_pixel(x, y);
        if let PixelRegion::Area(identifier) = identifier {
            PixelRegion::Element(self.element_type[identifier])
        } else {
            identifier
        }
    }

    /// Return the boundaries of the area attributed to an element
    pub fn get_area(&self, element: ElementType) -> Option<(f64, f64, f64, f64)> {
        let area_id = *self.area_identifer.get(&element)?;
        match *self.area[area_id].borrow() {
            LayoutNode::Area {
                left,
                top,
                right,
                bottom,
                ..
            } => Some((left, top, right, bottom)),
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

    pub fn resize(&mut self, node_id: usize, new_prop: f64) {
        self.area[node_id].borrow_mut().resize(new_prop)
    }

    pub fn resize_click(
        &mut self,
        node_id: usize,
        position: &PhysicalPosition<f64>,
        clicked_position: &PhysicalPosition<f64>,
        old_proportion: f64,
    ) {
        self.area[node_id]
            .borrow_mut()
            .resize_click(position, clicked_position, old_proportion)
    }

    pub fn get_proportion(&self, region: usize) -> Option<f64> {
        self.area.get(region).and_then(|a| a.borrow().proportion())
    }
}

impl LayoutNode {
    /// Horizontally split an area in two.
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
    pub fn hsplit(
        &mut self,
        top_proportion: f64,
        top_idx: usize,
        bottom_idx: usize,
        resizable: bool,
    ) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(top_proportion >= 0. && top_proportion <= 1.);
        match self {
            LayoutNode::Area {
                left,
                top,
                right,
                bottom,
                identifier,
                ..
            } => {
                let separation = top_proportion * (*top + *bottom);
                let top_area = Rc::new(RefCell::new(LayoutNode::Area {
                    left: *left,
                    top: *top,
                    right: *right,
                    bottom: separation,
                    identifier: top_idx,
                }));
                let bottom_area = Rc::new(RefCell::new(LayoutNode::Area {
                    left: *left,
                    top: separation,
                    right: *right,
                    bottom: *bottom,
                    identifier: bottom_idx,
                }));
                *self = LayoutNode::HSplit {
                    top: *top,
                    bottom: *bottom,
                    left: *left,
                    right: *right,
                    top_proportion,
                    t_child: top_area.clone(),
                    b_child: bottom_area.clone(),
                    resizable: Some(*identifier).filter(|_| resizable),
                };
                (top_area, bottom_area)
            }
            _ => {
                panic!("splitting a node");
            }
        }
    }

    /// Vertically split an area in two.
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
    pub fn vsplit(
        &mut self,
        left_proportion: f64,
        left_idx: usize,
        right_idx: usize,
        resizable: bool,
    ) -> (LayoutNodePtr, LayoutNodePtr) {
        assert!(left_proportion >= 0. && left_proportion <= 1.);
        match self {
            LayoutNode::Area {
                left,
                top,
                right,
                bottom,
                identifier,
                ..
            } => {
                let separation = left_proportion * (*left + *right);
                let left_area = Rc::new(RefCell::new(LayoutNode::Area {
                    left: *left,
                    top: *top,
                    right: separation,
                    bottom: *bottom,
                    identifier: left_idx,
                }));
                let right_area = Rc::new(RefCell::new(LayoutNode::Area {
                    left: separation,
                    top: *top,
                    right: *right,
                    bottom: *bottom,
                    identifier: right_idx,
                }));
                *self = LayoutNode::VSplit {
                    left: *left,
                    top: *top,
                    right: *right,
                    bottom: *bottom,
                    left_proportion,
                    l_child: left_area.clone(),
                    r_child: right_area.clone(),
                    resizable: Some(*identifier).filter(|_| resizable),
                };
                (left_area, right_area)
            }
            _ => {
                panic!("splitting a node");
            }
        }
    }

    /// Merge the two children of self and return their identifiers.
    /// These children must be leaves
    pub fn merge(&mut self, idx: usize) -> (usize, usize) {
        let ret;
        let new_self = match self {
            LayoutNode::VSplit {
                l_child, r_child, ..
            } => match (l_child.borrow().clone(), r_child.borrow().clone()) {
                (
                    LayoutNode::Area {
                        left,
                        top,
                        bottom,
                        identifier: c1,
                        ..
                    },
                    LayoutNode::Area {
                        right,
                        identifier: c2,
                        ..
                    },
                ) => {
                    ret = (c1, c2);
                    LayoutNode::Area {
                        left,
                        top,
                        right,
                        bottom,
                        identifier: idx,
                    }
                }
                _ => panic!("merge"),
            },
            LayoutNode::HSplit {
                t_child, b_child, ..
            } => match (t_child.borrow().clone(), b_child.borrow().clone()) {
                (
                    LayoutNode::Area {
                        left,
                        top,
                        right,
                        identifier: c1,
                        ..
                    },
                    LayoutNode::Area {
                        bottom,
                        identifier: c2,
                        ..
                    },
                ) => {
                    ret = (c1, c2);
                    LayoutNode::Area {
                        left,
                        top,
                        right,
                        bottom,
                        identifier: idx,
                    }
                }
                _ => panic!("merge"),
            },
            _ => panic!("merging a leaf"),
        };
        *self = new_self;
        ret
    }

    /// Return the identifier of the leaf owning pixel `(x, y)`
    pub fn get_area_pixel(&self, x: f64, y: f64) -> PixelRegion {
        match self {
            LayoutNode::Area { identifier, .. } => PixelRegion::Area(*identifier),
            LayoutNode::VSplit {
                left,
                right,
                left_proportion,
                l_child,
                r_child,
                resizable,
                ..
            } => {
                let separation = *left + *left_proportion * (*right - *left);
                if let Some(id) = resizable.filter(|_| {
                    x >= separation - RESIZE_REGION_WIDTH && x <= separation + RESIZE_REGION_WIDTH
                }) {
                    PixelRegion::Resize(id)
                } else {
                    if x <= separation {
                        l_child.borrow().get_area_pixel(x, y)
                    } else {
                        r_child.borrow().get_area_pixel(x, y)
                    }
                }
            }
            LayoutNode::HSplit {
                top,
                bottom,
                top_proportion,
                t_child,
                b_child,
                resizable,
                ..
            } => {
                let separation = *top + *top_proportion * (*bottom - *top);
                if let Some(id) =
                    resizable.filter(|_| y >= separation - 0.02 && y <= separation + 0.02)
                {
                    PixelRegion::Resize(id)
                } else {
                    if y <= separation {
                        t_child.borrow().get_area_pixel(x, y)
                    } else {
                        b_child.borrow().get_area_pixel(x, y)
                    }
                }
            }
        }
    }

    pub fn resize_click(
        &mut self,
        position: &PhysicalPosition<f64>,
        clicked_position: &PhysicalPosition<f64>,
        old_proportion: f64,
    ) {
        match self {
            LayoutNode::VSplit { left, right, .. } => {
                let delta = position.x - clicked_position.x;
                let delta_prop = delta / (*right - *left);
                let new_prop = (old_proportion + delta_prop).min(0.95).max(0.05);
                self.resize(new_prop);
            }
            LayoutNode::HSplit { top, bottom, .. } => {
                let delta = position.y - clicked_position.y;
                let delta_prop = delta / (*bottom - *top);
                let new_prop = (old_proportion + delta_prop).min(0.95).max(0.05);
                self.resize(new_prop);
            }
            LayoutNode::Area { .. } => {
                println!("WARNING, RESIZING AREA, THIS IS A BUG");
            }
        }
    }

    /// Resize a split, if self is a VSplit, new_size, is the proportion of the left portion. If
    /// self is a HSplit, new_size is the proportion of the right portion
    pub fn resize(&mut self, new_size: f64) {
        match self {
            LayoutNode::VSplit {
                left,
                top,
                right,
                bottom,
                left_proportion,
                l_child,
                r_child,
                ..
            } => {
                let separation = *left + new_size * (*right - *left);
                l_child
                    .borrow_mut()
                    .propagate_resize(*left, *top, separation, *bottom);
                r_child
                    .borrow_mut()
                    .propagate_resize(separation, *top, *right, *bottom);
                *left_proportion = new_size;
            }
            LayoutNode::HSplit {
                left,
                top,
                right,
                bottom,
                top_proportion,
                t_child,
                b_child,
                ..
            } => {
                let separation = *top + new_size * (*bottom - *top);
                t_child
                    .borrow_mut()
                    .propagate_resize(*left, *top, *right, separation);
                b_child
                    .borrow_mut()
                    .propagate_resize(*left, separation, *right, *bottom);
                *top_proportion = new_size;
            }
            LayoutNode::Area { .. } => println!("WARNING RESIZING LEAF, THIS IS A BUG!!!"),
        }
    }

    /// Propagate a resize through the tree.
    fn propagate_resize(&mut self, new_left: f64, new_top: f64, new_right: f64, new_bottom: f64) {
        match self {
            LayoutNode::HSplit {
                left,
                top,
                right,
                bottom,
                top_proportion,
                t_child: c1,
                b_child: c2,
                ..
            } => {
                let separation = new_top + *top_proportion * (new_bottom - new_top);
                *left = new_left;
                *top = new_top;
                *right = new_right;
                *bottom = new_bottom;
                c1.borrow_mut()
                    .propagate_resize(new_left, new_top, new_right, separation);
                c2.borrow_mut()
                    .propagate_resize(new_left, separation, new_right, new_bottom);
            }
            LayoutNode::VSplit {
                left,
                top,
                right,
                bottom,
                left_proportion,
                l_child: c1,
                r_child: c2,
                ..
            } => {
                let separation = new_left + *left_proportion * (new_right - new_left);
                *left = new_left;
                *top = new_top;
                *right = new_right;
                *bottom = new_bottom;
                c1.borrow_mut()
                    .propagate_resize(new_left, new_top, separation, new_bottom);
                c2.borrow_mut()
                    .propagate_resize(separation, new_top, new_right, new_bottom);
            }
            LayoutNode::Area {
                left,
                top,
                right,
                bottom,
                ..
            } => {
                *left = new_left;
                *top = new_top;
                *right = new_right;
                *bottom = new_bottom;
            }
        }
    }

    pub fn proportion(&self) -> Option<f64> {
        match self {
            LayoutNode::VSplit {
                left_proportion, ..
            } => Some(left_proportion.clone()),
            LayoutNode::HSplit { top_proportion, .. } => Some(top_proportion.clone()),
            LayoutNode::Area { .. } => None,
        }
    }
}

/// The region in which a pixel lies
#[derive(Debug)]
pub(super) enum PixelRegion {
    /// The pixel is on a region attributed to a certain element
    Element(ElementType),
    /// The pixel is on a region where clicking must resize a pannel
    Resize(usize),
    /// The pixel is on a given area
    Area(usize),
}
