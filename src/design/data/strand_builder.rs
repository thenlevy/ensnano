use super::Data;

pub struct StrandBuilder {
    initial_position: isize,
    forward: bool,
    helix: usize,
    identifier: DomainIdentifier,
    fixed_end: Option<isize>,
    current_position: isize,
    /// The enventual other strand being modified by the current modification
    neighbour_strand: Option<NeighbourDescriptor>,
    /// The initial position of the end of neighbour_strand that can be moved by the curent
    /// modification
    neighbour_initial_position: Option<isize>,
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
    pub fn init_empty(identifier: DomainIdentifier, helix: usize, initial_position: isize, forward: bool, neighbour: Option<NeighbourDescriptor>) -> Self {
        let mut neighbour_strand = None;
        let mut neighbour_direction = None;
        let mut neighbour_initial_position = None;
        let mut min_pos = None;
        let mut max_pos = None;
        if let Some(desc) = neighbour {
            neighbour_strand = Some(desc);
            neighbour_initial_position = Some(desc.initial_moving_end);
            neighbour_direction = if desc.initial_moving_end < initial_position {
                min_pos = Some(desc.fixed_end);
                Some(EditDirection::Negative)
            } else {
                max_pos = Some(desc.fixed_end);
                Some(EditDirection::Positive)
            };
        }

        Self {
            initial_position,
            helix,
            forward,
            identifier,
            fixed_end: None,
            current_position: initial_position,
            neighbour_strand,
            neighbour_initial_position,
            neighbour_direction,
            min_pos,
            max_pos,
            detached_neighbour: None,
        }
    }

    pub fn init_existing(identifier: DomainIdentifier, helix: usize, initial_position: isize, forward: bool, other_end: isize, neighbour: Option<NeighbourDescriptor>) -> Self {
        let mut min_pos = None;
        let mut max_pos = None;
        if initial_position < other_end {
            max_pos = Some(other_end);
        } else {
            min_pos = Some(other_end);
        }
        let neighbour_strand;
        let neighbour_initial_position;
        let neighbour_direction;
        if let Some(desc) = neighbour {
            neighbour_strand = Some(desc);
            neighbour_initial_position = Some(desc.initial_moving_end);
            neighbour_direction = Some(EditDirection::Both);
            if desc.initial_moving_end > initial_position {
                max_pos = Some(desc.fixed_end)
            } else {
                min_pos = Some(desc.fixed_end)
            }
        } else {
            neighbour_strand = None;
            neighbour_initial_position = None;
            neighbour_direction = None;
        }
        Self {
            helix,
            initial_position,
            forward,
            identifier,
            fixed_end: Some(other_end),
            current_position: initial_position,
            neighbour_strand,
            neighbour_initial_position,
            neighbour_direction,
            max_pos,
            min_pos,
            detached_neighbour: None,
        }
    }

    fn detach_neighbour(&mut self) {
        self.neighbour_direction = None;
        self.neighbour_initial_position = None;
        self.neighbour_strand = None;
    }

    fn attach_neighbour(&mut self, descriptor: &NeighbourDescriptor) {
        self.neighbour_direction = if descriptor.fixed_end > descriptor.initial_moving_end {
            Some(EditDirection::Positive)
        } else {
            Some(EditDirection::Negative)
        };
        self.neighbour_initial_position = Some(descriptor.initial_moving_end);
        self.neighbour_strand = Some(*descriptor);
    }

    fn incr_position(&mut self, data: &Data) {
        // Eventually detach from neighbour
        if self.neighbour_initial_position == Some(self.current_position - 1) && self.neighbour_direction == Some(EditDirection::Negative) {
            self.detach_neighbour();
        }
        self.current_position += 1;
        if let Some(ref desc) = data.get_neighbour_nucl(self.helix, self.current_position + 1, self.forward) {
            self.attach_neighbour(desc)
        }
    }

    fn decr_position(&mut self, data: &Data) {
        // Eventually detach from neighbour
        if self.neighbour_initial_position == Some(self.current_position + 1) && self.neighbour_direction == Some(EditDirection::Positive) {
            self.detach_neighbour();
        }
        self.current_position -= 1;
        if let Some(ref desc) = data.get_neighbour_nucl(self.helix, self.current_position - 1, self.forward) {
            self.attach_neighbour(desc)
        }
    }

    fn move_to(&mut self, objective: isize, data: &mut Data) {
        let mut need_update = false;
        if objective > self.current_position {
            while self.current_position < objective.min(self.max_pos.unwrap_or(objective)) {
                self.incr_position(data);
                need_update = true;
            }
        } else if objective < self.current_position {
            while self.current_position > objective.max(self.min_pos.unwrap_or(objective)) {
                self.decr_position(data);
                need_update = true;
            }
        }
        if need_update {
            self.update(data)
        }
    }

    fn update(&self, data: &mut Data) {
        data.update_strand(self.identifier, self.current_position, self.fixed_end.unwrap_or(self.initial_position));
        if let Some(desc) = self.neighbour_strand {
            data.update_strand(desc.identifier, desc.moving_end, desc.fixed_end)
        }
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
