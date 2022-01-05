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

use ensnano_design::{Axis, Domain, Nucl};

use std::cmp::Ordering;
use ultraviolet::Mat4;

#[derive(Clone, Debug)]
pub struct StrandBuilder {
    /// The nucleotide that can move
    pub moving_end: Nucl,
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
    /// A timestamp used to distinguish between strand building operation initiated at different
    /// moment
    timestamp: std::time::SystemTime,
    de_novo: bool,
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
        de_novo: bool,
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
            timestamp: std::time::SystemTime::now(),
            de_novo,
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
        other_end: Option<isize>,
        neighbour: Option<NeighbourDescriptor>,
        stick: bool,
    ) -> Self {
        let mut min_pos = None;
        let mut max_pos = None;
        let initial_position = nucl.position;
        if let Some(other_end) = other_end {
            if initial_position < other_end {
                max_pos = Some(other_end);
            } else {
                min_pos = Some(other_end);
            }
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
        let ret = Self {
            moving_end: nucl,
            initial_position,
            axis,
            identifier,
            fixed_end: other_end,
            neighbour_strand,
            neighbour_direction,
            max_pos,
            min_pos,
            detached_neighbour: None,
            design_id: 0,
            timestamp: std::time::SystemTime::now(),
            de_novo: false,
        };
        log::info!("builder {:?}", ret);
        ret
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
        if self.identifier.is_same_domain_than(&descriptor.identifier)
            || self.neighbour_strand.is_some()
            || descriptor.moving_end > self.max_pos.unwrap_or(descriptor.moving_end)
            || descriptor.moving_end < self.min_pos.unwrap_or(descriptor.moving_end)
        {
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
    fn incr_position(&mut self, design: &Design, ignored_domains: &[DomainIdentifier]) {
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
        let desc = design
            .get_neighbour_nucl(self.moving_end.right())
            .filter(|neighbour| !ignored_domains.contains(&neighbour.identifier));
        if let Some(ref desc) = desc {
            if self.attach_neighbour(desc) {
                self.max_pos = self.max_pos.or(Some(desc.fixed_end - 1));
            }
        }
    }

    /// Decrease the postion of the moving end by one, and update the neighbour in consequences.
    fn decr_position(&mut self, design: &Design, ignored_domains: &[DomainIdentifier]) {
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
        let desc = design
            .get_neighbour_nucl(self.moving_end.left())
            .filter(|neighbour| !ignored_domains.contains(&neighbour.identifier));
        if let Some(ref desc) = desc {
            if self.attach_neighbour(desc) {
                self.min_pos = self.min_pos.or(Some(desc.fixed_end + 1));
            }
        }
    }

    /// Move the moving end to an objective position. If this position cannot be reached by the
    /// moving end, it will go as far as possible.
    pub fn move_to(
        &mut self,
        objective: isize,
        design: &mut Design,
        ignored_domains: &[DomainIdentifier],
    ) {
        log::info!("self {:?}", self);
        log::info!("move to {}", objective);
        let mut need_update = true;
        match objective.cmp(&self.moving_end.position) {
            Ordering::Greater => {
                while self.moving_end.position < objective.min(self.max_pos.unwrap_or(objective)) {
                    self.incr_position(design, ignored_domains);
                    need_update = true;
                }
            }
            Ordering::Less => {
                while self.moving_end.position > objective.max(self.min_pos.unwrap_or(objective)) {
                    self.decr_position(design, ignored_domains);
                    need_update = true;
                }
            }
            _ => (),
        }
        if need_update {
            self.update(design)
        }
    }

    pub fn try_incr(&mut self, design: &Design, ignored_domains: &[DomainIdentifier]) -> bool {
        if self.moving_end.position < self.max_pos.unwrap_or(isize::MAX) {
            self.incr_position(design, ignored_domains);
            true
        } else {
            false
        }
    }

    pub fn try_decr(&mut self, design: &Design, ignored_domains: &[DomainIdentifier]) -> bool {
        if self.moving_end.position > self.min_pos.unwrap_or(isize::MIN) {
            self.decr_position(design, ignored_domains);
            true
        } else {
            false
        }
    }

    /// Apply the modification on the data
    pub fn update(&mut self, design: &mut Design) {
        Self::update_strand(
            design,
            self.identifier,
            self.moving_end.position,
            self.fixed_end.unwrap_or(self.initial_position),
        );
        if let Some(desc) = self.neighbour_strand {
            Self::update_strand(design, desc.identifier, desc.moving_end, desc.fixed_end)
        }
        if let Some(desc) = self.detached_neighbour.take() {
            Self::update_strand(design, desc.identifier, desc.moving_end, desc.fixed_end)
        }
    }

    fn update_strand(
        design: &mut Design,
        identifier: DomainIdentifier,
        position: isize,
        fixed_position: isize,
    ) {
        log::info!(
            "updating {:?}, position {}, fixed_position {}",
            identifier,
            position,
            fixed_position
        );
        let domain =
            &mut design.strands.get_mut(&identifier.strand).unwrap().domains[identifier.domain];
        if let Domain::HelixDomain(domain) = domain {
            match identifier.start {
                None => {
                    let start = position.min(fixed_position);
                    let end = position.max(fixed_position) + 1;
                    domain.start = start;
                    domain.end = end;
                }
                Some(false) => {
                    domain.end = position + 1;
                }
                Some(true) => {
                    domain.start = position;
                }
            }
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

    /// Return the identifier of the design being eddited
    pub fn get_design_id(&self) -> u32 {
        self.design_id
    }

    pub fn get_strand_id(&self) -> usize {
        self.identifier.strand
    }

    /*
    pub fn reset(&mut self, design: &Design) {
        self.move_to(self.initial_position);
    }*/

    /// Return false if self is modifying an existing strand and true otherwise
    pub fn created_de_novo(&self) -> bool {
        self.de_novo
    }

    pub fn get_moving_end_position(&self) -> isize {
        self.moving_end.position
    }

    pub fn get_moving_end_nucl(&self) -> Nucl {
        self.moving_end
    }

    pub fn get_initial_nucl(&self) -> Nucl {
        Nucl {
            position: self.initial_position,
            ..self.moving_end
        }
    }

    pub fn get_domain_identifier(&self) -> DomainIdentifier {
        self.identifier
    }

    pub fn get_timestamp(&self) -> std::time::SystemTime {
        self.timestamp
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
    pub start: Option<bool>,
}

impl DomainIdentifier {
    pub fn other_end(&self) -> Option<Self> {
        if let Some(end) = self.start {
            Some(Self {
                strand: self.strand,
                domain: self.domain,
                start: Some(!end),
            })
        } else {
            None
        }
    }

    pub fn is_same_domain_than(&self, other: &Self) -> bool {
        self.strand == other.strand && self.domain == other.domain
    }
}

use ensnano_design::Design;
pub trait NeighbourDescriptorGiver {
    fn get_neighbour_nucl(&self, nucl: Nucl) -> Option<NeighbourDescriptor>;
}

impl NeighbourDescriptorGiver for Design {
    fn get_neighbour_nucl(&self, nucl: Nucl) -> Option<NeighbourDescriptor> {
        for (s_id, s) in self.strands.iter() {
            for (d_id, d) in s.domains.iter().enumerate() {
                if let Some(other) = d.other_end(nucl) {
                    let start = if let Domain::HelixDomain(i) = d {
                        // if the domain has length one, we are not at a specific end
                        (d.length() > 1).then(|| i.start)
                    } else {
                        None
                    };
                    return Some(NeighbourDescriptor {
                        identifier: DomainIdentifier {
                            strand: *s_id,
                            domain: d_id,
                            start: start.map(|s| s == nucl.position),
                        },
                        fixed_end: other,
                        initial_moving_end: nucl.position,
                        moving_end: nucl.position,
                    });
                }
            }
        }
        None
    }
}
