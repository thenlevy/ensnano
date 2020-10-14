use super::{Axis, Data};
use std::sync::{Arc, Mutex};
use ultraviolet::Mat4;

pub struct StrandBuilder {
    data: Option<Arc<Mutex<Data>>>,
    pub initial_position: isize,
    forward: bool,
    helix: usize,
    pub axis: Axis,
    identifier: DomainIdentifier,
    fixed_end: Option<isize>,
    current_position: isize,
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
    detached_neighbour: Option<NeighbourDescriptor>,
}

impl StrandBuilder {
    /// Create a strand that will build a new strand. This means that the inital position
    /// correspons to a phantom nucleotide
    pub fn init_empty(
        identifier: DomainIdentifier,
        helix: usize,
        initial_position: isize,
        forward: bool,
        axis: Axis,
        neighbour: Option<NeighbourDescriptor>,
    ) -> Self {
        let mut neighbour_strand = None;
        let mut neighbour_direction = None;
        let mut min_pos = None;
        let mut max_pos = None;
        if let Some(desc) = neighbour {
            neighbour_strand = Some(desc);
            neighbour_direction = if desc.initial_moving_end < initial_position {
                min_pos = Some(desc.fixed_end);
                Some(EditDirection::Negative)
            } else {
                max_pos = Some(desc.fixed_end);
                Some(EditDirection::Positive)
            };
        }

        Self {
            data: None,
            initial_position,
            helix,
            forward,
            identifier,
            axis,
            fixed_end: None,
            current_position: initial_position,
            neighbour_strand,
            neighbour_direction,
            min_pos,
            max_pos,
            detached_neighbour: None,
        }
    }

    pub fn init_existing(
        identifier: DomainIdentifier,
        helix: usize,
        initial_position: isize,
        forward: bool,
        axis: Axis,
        other_end: isize,
        neighbour: Option<NeighbourDescriptor>,
    ) -> Self {
        let mut min_pos = None;
        let mut max_pos = None;
        if initial_position < other_end {
            max_pos = Some(other_end);
        } else {
            min_pos = Some(other_end);
        }
        let neighbour_strand;
        let neighbour_direction;
        if let Some(desc) = neighbour {
            neighbour_strand = Some(desc);
            neighbour_direction = Some(EditDirection::Both);
            if desc.initial_moving_end > initial_position {
                max_pos = Some(desc.fixed_end)
            } else {
                min_pos = Some(desc.fixed_end)
            }
        } else {
            neighbour_strand = None;
            neighbour_direction = None;
        }
        Self {
            data: None,
            helix,
            initial_position,
            forward,
            axis,
            identifier,
            fixed_end: Some(other_end),
            current_position: initial_position,
            neighbour_strand,
            neighbour_direction,
            max_pos,
            min_pos,
            detached_neighbour: None,
        }
    }

    fn detach_neighbour(&mut self) {
        self.neighbour_direction = None;
        self.detached_neighbour = self.neighbour_strand.take();
    }

    fn attach_neighbour(&mut self, descriptor: &NeighbourDescriptor) -> bool {
        if self.identifier == descriptor.identifier || self.neighbour_strand.is_some() {
            return false;
        }
        self.neighbour_direction = if descriptor.fixed_end > descriptor.initial_moving_end {
            Some(EditDirection::Positive)
        } else {
            Some(EditDirection::Negative)
        };
        self.neighbour_strand = Some(*descriptor);
        true
    }

    fn incr_position(&mut self) {
        // Eventually detach from neighbour
        if let Some(desc) = self.neighbour_strand.as_mut() {
            if desc.initial_moving_end == self.current_position - 1
                && self.neighbour_direction == Some(EditDirection::Negative)
            {
                self.detach_neighbour();
            } else {
                desc.moving_end += 1;
            }
        }
        self.current_position += 1;
        let desc = self
            .data
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .get_neighbour_nucl(self.helix, self.current_position + 1, self.forward);
        if let Some(ref desc) = desc {
            if self.attach_neighbour(desc) {
                self.max_pos = Some(desc.fixed_end - 1);
            }
        }
    }

    fn decr_position(&mut self) {
        // Update neighbour and eventually detach from it
        if let Some(desc) = self.neighbour_strand.as_mut() {
            if desc.initial_moving_end == self.current_position + 1
                && self.neighbour_direction == Some(EditDirection::Positive)
            {
                self.detach_neighbour();
            } else {
                desc.moving_end -= 1;
            }
        }
        self.current_position -= 1;
        let desc = self
            .data
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .get_neighbour_nucl(self.helix, self.current_position - 1, self.forward);
        if let Some(ref desc) = desc {
            if self.attach_neighbour(desc) {
                self.min_pos = Some(desc.fixed_end + 1);
            }
        }
    }

    pub fn move_to(&mut self, objective: isize) {
        let mut need_update = false;
        if objective > self.current_position {
            while self.current_position < objective.min(self.max_pos.unwrap_or(objective)) {
                self.incr_position();
                need_update = true;
            }
        } else if objective < self.current_position {
            while self.current_position > objective.max(self.min_pos.unwrap_or(objective)) {
                self.decr_position();
                need_update = true;
            }
        }
        if need_update {
            self.update()
        }
    }

    fn update(&mut self) {
        let mut data = self.data.as_mut().unwrap().lock().unwrap();
        data.update_strand(
            self.identifier,
            self.current_position,
            self.fixed_end.unwrap_or(self.initial_position),
        );
        if let Some(desc) = self.neighbour_strand {
            data.update_strand(desc.identifier, desc.moving_end, desc.fixed_end)
        }
        if let Some(desc) = self.detached_neighbour.take() {
            data.update_strand(desc.identifier, desc.moving_end, desc.fixed_end)
        }
    }

    pub fn transformed(self, model_matrix: &Mat4) -> Self {
        let new_axis = self.axis.transformed(model_matrix);
        Self {
            axis: new_axis,
            ..self
        }
    }

    pub fn given_data(self, data: Arc<Mutex<Data>>) -> Self {
        let data = Some(data);
        Self { data, ..self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditDirection {
    Both,
    Negative,
    Positive,
}

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
