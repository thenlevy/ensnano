//! The modules defines the `StrandBuilder` struct. A `StrandBuilder` is responsible for edditing a
//! strand. It is initialized with a domain that might be an already existing domain or a new
//! strand beign created.
//!
//! The role of the `StrandBuilder` is to move one extremity of the domain being eddited.
//! If the domain is a new one (the `StrandBuilder` was created with `StrandBuilder::init_empty`) then the
//! the the moving end can go in both direction and the fixed end is the nucleotide on whihch the
//! domain was initiated.
//! If the domain is an existing one (the `StrandBuilder` was created with
//! `StrandBuilder::init_existing`), then the moving end in the nucleotide that was selected at the
//! moment of the builder's creation and the fixed end is the other end of the domain. In that case
//! the moving end can never go "on the other side" of the fixed end.
//!
//! The `StrandBuilder` can also modify a second domain, the "neighbour", a neighbour can be a
//! domain that needs to be shortenend to elongate the main domain. Or it can be an existing
//! neighbour of the moving_end at the moment of the builder creation.
//!
//! If the neighbour was already next to the domain at the creation of the builder, it follows the
//! moving end, meaning that the neighbour domain can become larger or smaller. If the neighbour
//! was not next to the domain at the creation of the builder, it can only become smaller than it
//! initially was.
use super::{Axis, Data, Nucl};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use ultraviolet::Mat4;

#[derive(Clone)]
pub struct StrandBuilder {
    /// The data to modify when applying updates
    data: Option<Arc<Mutex<Data>>>,
    /// The nucleotide that can move
    moving_end: Nucl,
    /// The initial position of the moving end
    pub initial_position: isize,
    /// Axis of the support helix on which the domain lies
    pub axis: Axis,
    /// The identifier of the domain being eddited
    identifier: DomainIdentifier,
    /// The fixed_end of the domain being eddited, `None` if the domain is new and can go in both
    /// direction
    fixed_end: Option<isize>,
    /// The enventual other strand being modified by the current modification
    neighbour_strand: Option<NeighbourDescriptor>,
    /// The direction in which the end of neighbour_strand can go, starting from its inital
    /// position
    neighbour_direction: Option<EditDirection>,
    /// The minimum position to which the eddited domain can go. It corresponds to the eventual
    /// minimum position of the neighbour_strand or to the other end of the domain being eddited
    min_pos: Option<isize>,
    /// The maximum position to which the eddited domain can go. It corresponds to the eventual
    /// maximum position of the neighbour_strand, or to the other end of the domain being eddited
    max_pos: Option<isize>,
    /// A envtual neighbour that was detached during the movement
    detached_neighbour: Option<NeighbourDescriptor>,
    /// The id of the design being eddited
    design_id: u32,
}

impl StrandBuilder {
    /// Create a strand that will build a new strand. This means that the inital position
    /// correspons to a phantom nucleotide
    /// # Argument
    ///
    /// * identifier: The identifier of the domain that will be created
    ///
    /// * nucl: The fixed end of the domain that will be created
    ///
    /// * axis: The axis of the helix on which the domain will be created
    ///
    /// * neighbour: An evental existing neighbour of the strand being created
    pub fn init_empty(
        identifier: DomainIdentifier,
        nucl: Nucl,
        axis: Axis,
        neighbour: Option<NeighbourDescriptor>,
    ) -> Self {
        let mut neighbour_strand = None;
        let mut neighbour_direction = None;
        let mut min_pos = None;
        let mut max_pos = None;
        if let Some(desc) = neighbour {
            neighbour_strand = Some(desc);
            neighbour_direction = if desc.initial_moving_end < nucl.position {
                min_pos = Some(desc.fixed_end + 1);
                Some(EditDirection::Negative)
            } else {
                max_pos = Some(desc.fixed_end - 1);
                Some(EditDirection::Positive)
            };
        }

        Self {
            data: None,
            initial_position: nucl.position,
            moving_end: nucl,
            identifier,
            axis,
            fixed_end: None,
            neighbour_strand,
            neighbour_direction,
            min_pos,
            max_pos,
            detached_neighbour: None,
            design_id: 0,
        }
    }

    /// Create a strand that will eddit an existing domain. This means that the initial position
    /// corresponds to an end of an existing domain
    /// # Argument
    ///
    /// * identifier: The identifier of the domain that will be created
    ///
    /// * nucl: The moving end of the domain that will be created
    ///
    /// * axis: The axis of the helix on which the domain will be created
    ///
    /// * other_end: The position of the fixed end of the domain that will be eddited
    ///
    /// * neighbour: An evental existing neighbour of the strand being created
    pub fn init_existing(
        identifier: DomainIdentifier,
        nucl: Nucl,
        axis: Axis,
        other_end: isize,
        neighbour: Option<NeighbourDescriptor>,
        stick: bool,
    ) -> Self {
        let mut min_pos = None;
        let mut max_pos = None;
        let initial_position = nucl.position;
        if initial_position < other_end {
            max_pos = Some(other_end);
        } else {
            min_pos = Some(other_end);
        }
        let neighbour_strand;
        let neighbour_direction;
        if let Some(desc) = neighbour {
            neighbour_strand = Some(desc);
            neighbour_direction = if stick {
                Some(EditDirection::Both) 
            } else if desc.moving_end > initial_position {
                Some(EditDirection::Positive)
            } else {
                Some(EditDirection::Negative)
            };
            if desc.initial_moving_end > initial_position {
                max_pos = max_pos.or(Some(desc.fixed_end - 1))
            } else {
                min_pos = min_pos.or(Some(desc.fixed_end + 1))
            }
        } else {
            neighbour_strand = None;
            neighbour_direction = None;
        }
        Self {
            data: None,
            moving_end: nucl,
            initial_position,
            axis,
            identifier,
            fixed_end: Some(other_end),
            neighbour_strand,
            neighbour_direction,
            max_pos,
            min_pos,
            detached_neighbour: None,
            design_id: 0,
        }
    }

    /// Detach the neighbour, this function must be called when the moving end goes to a position
    /// where the moving end of the neighbour cannot follow it.
    fn detach_neighbour(&mut self) {
        self.neighbour_direction = None;
        self.detached_neighbour = self.neighbour_strand.take();
    }

    /// Attach a new neighbour. This function must be called when the moving end goes to a position
    /// where it is next to an existing domain
    fn attach_neighbour(&mut self, descriptor: &NeighbourDescriptor) -> bool {
        // To prevent attaching to self or attaching to the same neighbour or attaching to a
        // neighbour in the wrong direction
        if self.identifier == descriptor.identifier || self.neighbour_strand.is_some() || descriptor.moving_end > self.max_pos.unwrap_or(descriptor.moving_end) || descriptor.moving_end < self.min_pos.unwrap_or(descriptor.moving_end) {
            return false;
        }
        self.neighbour_direction = if self.moving_end.position < descriptor.initial_moving_end {
            Some(EditDirection::Positive)
        } else {
            Some(EditDirection::Negative)
        };
        self.neighbour_strand = Some(*descriptor);
        true
    }

    /// Increase the postion of the moving end by one, and update the neighbour in consequences.
    fn incr_position(&mut self) {
        // Eventually detach from neighbour
        if let Some(desc) = self.neighbour_strand.as_mut() {
            if desc.initial_moving_end == self.moving_end.position - 1
                && self.neighbour_direction == Some(EditDirection::Negative)
            {
                self.detach_neighbour();
            } else {
                desc.moving_end += 1;
            }
        }
        self.moving_end.position += 1;
        let desc = self
            .data
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .get_neighbour_nucl(self.moving_end.right());
        if let Some(ref desc) = desc {
            if self.attach_neighbour(desc) {
                self.max_pos = self.max_pos.or(Some(desc.fixed_end - 1));
            }
        }
    }

    /// Decrease the postion of the moving end by one, and update the neighbour in consequences.
    fn decr_position(&mut self) {
        // Update neighbour and eventually detach from it
        if let Some(desc) = self.neighbour_strand.as_mut() {
            if desc.initial_moving_end == self.moving_end.position + 1
                && self.neighbour_direction == Some(EditDirection::Positive)
            {
                self.detach_neighbour();
            } else {
                desc.moving_end -= 1;
            }
        }
        self.moving_end.position -= 1;
        let desc = self
            .data
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .get_neighbour_nucl(self.moving_end.left());
        if let Some(ref desc) = desc {
            if self.attach_neighbour(desc) {
                self.min_pos = self.min_pos.or(Some(desc.fixed_end + 1));
            }
        }
    }

    /// Move the moving end to an objective position. If this position cannot be reached by the
    /// moving end, it will go as far as possible.
    pub fn move_to(&mut self, objective: isize) {
        let mut need_update = false;
        match objective.cmp(&self.moving_end.position) {
            Ordering::Greater => {
                while self.moving_end.position < objective.min(self.max_pos.unwrap_or(objective)) {
                    self.incr_position();
                    need_update = true;
                }
            }
            Ordering::Less => {
                while self.moving_end.position > objective.max(self.min_pos.unwrap_or(objective)) {
                    self.decr_position();
                    need_update = true;
                }
            }
            _ => (),
        }
        if need_update {
            self.update()
        }
    }

    /// Apply the modification on the data
    fn update(&mut self) {
        let mut data = self.data.as_mut().unwrap().lock().unwrap();
        data.update_strand(
            self.identifier,
            self.moving_end.position,
            self.fixed_end.unwrap_or(self.initial_position),
        );
        if let Some(desc) = self.neighbour_strand {
            data.update_strand(desc.identifier, desc.moving_end, desc.fixed_end)
        }
        if let Some(desc) = self.detached_neighbour.take() {
            data.update_strand(desc.identifier, desc.moving_end, desc.fixed_end)
        }
    }

    /// Convert the axis in the world's coordinate. This function is used at the creation of the
    /// builder.
    pub fn transformed(self, model_matrix: &Mat4) -> Self {
        let new_axis = self.axis.transformed(model_matrix);
        Self {
            axis: new_axis,
            ..self
        }
    }

    /// Initialise the data pointer. This function is used at the creation of the
    /// builder.
    pub fn given_data(self, data: Arc<Mutex<Data>>, design_id: u32) -> Self {
        let data = Some(data);
        Self {
            data,
            design_id,
            ..self
        }
    }

    /// Return the identifier of the element on the moving end position
    pub fn get_moving_end_identifier(&self) -> Option<u32> {
        self.data
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .get_identifier_nucl(self.moving_end)
    }

    /// Return the identifier of the design being eddited
    pub fn get_design_id(&self) -> u32 {
        self.design_id
    }

    pub fn get_strand_id(&self) -> usize {
        self.identifier.strand
    }

    pub fn reset(&mut self) {
        self.move_to(self.initial_position)
    }
}

/// The direction in which a moving end can go
#[derive(Debug, Clone, Copy, PartialEq)]
enum EditDirection {
    /// In both direction
    Both,
    /// Only on position smaller that the inital position
    Negative,
    /// Only on position bigger that the inital position
    Positive,
}

/// Describes a domain being eddited
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeighbourDescriptor {
    pub identifier: DomainIdentifier,
    pub initial_moving_end: isize,
    pub moving_end: isize,
    pub fixed_end: isize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomainIdentifier {
    pub strand: usize,
    pub domain: usize,
}
