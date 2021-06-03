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
//! This modules defines the `Operation` trait and several struct that implement it.
//!
//! An structure that implements `Operation` can produce an `UndoableOpperation` that will have an
//! effect on the design.
//!
//! Moreover, these operations are meant to be modifiable via GUI component or user interaction.
use super::{DesignRotation, DesignTranslation, GridDescriptor, GridHelixDescriptor, UndoableOp};
use crate::design::{Helix, IsometryTarget, Nucl, Strand, StrandBuilder, StrandState};
use ensnano_design::grid::{GridTypeDescr, Hyperboloid};
use std::sync::Arc;
use ultraviolet::{Bivec3, Rotor3, Vec3};

pub enum ParameterField {
    Choice(Vec<String>),
    Value,
}

pub struct Parameter {
    pub field: ParameterField,
    pub name: String,
}

pub trait Operation: std::fmt::Debug + Sync + Send {
    /// The set of parameters that can be modified via a GUI component
    fn parameters(&self) -> Vec<Parameter>;
    /// The values associated to the parameters.
    fn values(&self) -> Vec<String>;
    /// Return an opperation whose effect cancels the effect of `self`.
    fn reverse(&self) -> Arc<dyn Operation>;
    /// The effect of self that must be sent as a notifications to the targeted designs
    fn effect(&self) -> UndoableOp;
    /// A description of self of display in the GUI
    fn description(&self) -> String;
    /// The targeted designs of self.
    fn target(&self) -> usize;
    /// Produce an new opperation by setting the value of the `n`-th parameter to `val`.
    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>>;
    fn descr(&self) -> OperationDescriptor;
    /// If `other` is compatible with `self` return the operation whose effect is equivalent to
    /// applying the effects of `other` and then `self`.
    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>>;

    fn must_reverse(&self) -> bool {
        true
    }

    fn drop_undo(&self) -> bool {
        false
    }

    fn redoable(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct GridRotation {
    pub origin: Vec3,
    pub design_id: usize,
    pub grid_id: usize,
    pub angle: f32,
    pub plane: Bivec3,
}

impl Operation for GridRotation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::GridRotation(self.design_id, self.grid_id, self.plane)
    }

    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        if self.descr() == other.descr() {
            let angle = other.values()[0].parse::<f32>().unwrap().to_radians();
            Some(Arc::new(Self {
                angle: self.angle + angle,
                ..*self
            }))
        } else {
            None
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Value,
            name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_degrees().to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            angle: -self.angle,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        UndoableOp::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Grid(self.grid_id as u32),
        })
    }

    fn description(&self) -> String {
        format!("Rotate grid {} of design {}", self.grid_id, self.design_id)
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        if n == 0 {
            let degrees: f32 = val.parse().ok()?;
            Some(Arc::new(Self {
                angle: degrees.to_radians(),
                ..*self
            }))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct HelixRotation {
    pub origin: Vec3,
    pub design_id: usize,
    pub helix_id: usize,
    pub angle: f32,
    pub plane: Bivec3,
}

impl Operation for HelixRotation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::HelixRotation(self.design_id, self.helix_id, self.plane)
    }

    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        if self.descr() == other.descr() {
            let angle = other.values()[0].parse::<f32>().unwrap().to_radians();
            Some(Arc::new(Self {
                angle: self.angle + angle,
                ..*self
            }))
        } else {
            None
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Value,
            name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_degrees().to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            angle: -self.angle,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        UndoableOp::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Helix(self.helix_id as u32, false),
        })
    }

    fn description(&self) -> String {
        format!(
            "Rotate helix {} of design {}",
            self.helix_id, self.design_id
        )
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        if n == 0 {
            let degrees: f32 = val.parse().ok()?;
            Some(Arc::new(Self {
                angle: degrees.to_radians(),
                ..*self
            }))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesignViewRotation {
    pub origin: Vec3,
    pub design_id: usize,
    pub angle: f32,
    pub plane: Bivec3,
}

impl Operation for DesignViewRotation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::DesignRotation(self.design_id, self.plane)
    }

    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        if self.descr() == other.descr() {
            let angle = other.values()[0].parse::<f32>().unwrap().to_radians();
            Some(Arc::new(Self {
                angle: self.angle + angle,
                ..*self
            }))
        } else {
            None
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Value,
            name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_degrees().to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            angle: -self.angle,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        UndoableOp::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Design,
        })
    }

    fn description(&self) -> String {
        format!("Rotate view of design {}", self.design_id)
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        if n == 0 {
            let degrees: f32 = val.parse().ok()?;
            Some(Arc::new(Self {
                angle: degrees.to_radians(),
                ..*self
            }))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesignViewTranslation {
    pub design_id: usize,
    pub right: Vec3,
    pub top: Vec3,
    pub dir: Vec3,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Operation for DesignViewTranslation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::DesignTranslation(self.design_id)
    }

    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        if self.descr() == other.descr() {
            let x = other.values()[0].parse::<f32>().unwrap();
            let y = other.values()[1].parse::<f32>().unwrap();
            let z = other.values()[2].parse::<f32>().unwrap();
            Some(Arc::new(Self {
                x: self.x + x,
                y: self.y + y,
                z: self.z + z,
                ..*self
            }))
        } else {
            None
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                field: ParameterField::Value,
                name: String::from("x"),
            },
            Parameter {
                field: ParameterField::Value,
                name: String::from("y"),
            },
            Parameter {
                field: ParameterField::Value,
                name: String::from("z"),
            },
        ]
    }

    fn values(&self) -> Vec<String> {
        vec![self.x.to_string(), self.y.to_string(), self.z.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        UndoableOp::Translation(DesignTranslation {
            translation,
            target: IsometryTarget::Design,
        })
    }

    fn description(&self) -> String {
        format!("Translate design {}", self.design_id)
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self { x: new_x, ..*self }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self { y: new_y, ..*self }))
            }
            2 => {
                let new_z: f32 = val.parse().ok()?;
                Some(Arc::new(Self { z: new_z, ..*self }))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HelixTranslation {
    pub design_id: usize,
    pub helix_id: usize,
    pub right: Vec3,
    pub top: Vec3,
    pub dir: Vec3,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub snap: bool,
}

impl Operation for HelixTranslation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::HelixTranslation(self.design_id, self.helix_id)
    }

    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        if self.descr() == other.descr() {
            let x = other.values()[0].parse::<f32>().unwrap();
            let y = other.values()[1].parse::<f32>().unwrap();
            let z = other.values()[2].parse::<f32>().unwrap();
            Some(Arc::new(Self {
                x: self.x + x,
                y: self.y + y,
                z: self.z + z,
                ..*self
            }))
        } else {
            None
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                field: ParameterField::Value,
                name: String::from("x"),
            },
            Parameter {
                field: ParameterField::Value,
                name: String::from("y"),
            },
            Parameter {
                field: ParameterField::Value,
                name: String::from("z"),
            },
        ]
    }

    fn values(&self) -> Vec<String> {
        vec![self.x.to_string(), self.y.to_string(), self.z.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        UndoableOp::Translation(DesignTranslation {
            translation,
            target: IsometryTarget::Helix(self.helix_id as u32, self.snap),
        })
    }

    fn description(&self) -> String {
        format!(
            "Translate helix {} of design {}",
            self.helix_id, self.design_id
        )
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self { x: new_x, ..*self }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self { y: new_y, ..*self }))
            }
            2 => {
                let new_z: f32 = val.parse().ok()?;
                Some(Arc::new(Self { z: new_z, ..*self }))
            }
            _ => None,
        }
    }

    fn must_reverse(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct GridTranslation {
    pub design_id: usize,
    pub grid_id: usize,
    pub right: Vec3,
    pub top: Vec3,
    pub dir: Vec3,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Operation for GridTranslation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::GridTranslation(self.design_id, self.grid_id)
    }

    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        if self.descr() == other.descr() {
            let x = other.values()[0].parse::<f32>().unwrap();
            let y = other.values()[1].parse::<f32>().unwrap();
            let z = other.values()[2].parse::<f32>().unwrap();
            Some(Arc::new(Self {
                x: self.x + x,
                y: self.y + y,
                z: self.z + z,
                ..*self
            }))
        } else {
            None
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                field: ParameterField::Value,
                name: String::from("x"),
            },
            Parameter {
                field: ParameterField::Value,
                name: String::from("y"),
            },
            Parameter {
                field: ParameterField::Value,
                name: String::from("z"),
            },
        ]
    }

    fn values(&self) -> Vec<String> {
        vec![self.x.to_string(), self.y.to_string(), self.z.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        UndoableOp::Translation(DesignTranslation {
            translation,
            target: IsometryTarget::Grid(self.grid_id as u32),
        })
    }

    fn description(&self) -> String {
        format!(
            "Translate grid {} of design {}",
            self.grid_id, self.design_id
        )
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self { x: new_x, ..*self }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self { y: new_y, ..*self }))
            }
            2 => {
                let new_z: f32 = val.parse().ok()?;
                Some(Arc::new(Self { z: new_z, ..*self }))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GridHelixCreation {
    pub design_id: usize,
    pub grid_id: usize,
    pub x: isize,
    pub y: isize,
    pub position: isize,
    pub length: usize,
}

impl Operation for GridHelixCreation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::GridHelixCreation(self.design_id, self.grid_id)
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![self.x.to_string(), self.y.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(GridHelixDeletion {
            x: self.x,
            y: self.y,
            design_id: self.design_id,
            grid_id: self.grid_id,
            position: self.position,
            length: self.length,
        })
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::AddGridHelix(
            GridHelixDescriptor {
                grid_id: self.grid_id,
                x: self.x,
                y: self.y,
            },
            self.position,
            self.length,
        )
    }

    fn description(&self) -> String {
        format!(
            "Create helix on grid {} of design {}",
            self.grid_id, self.design_id
        )
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    x: new_x as isize,
                    ..*self
                }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    y: new_y as isize,
                    ..*self
                }))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GridHelixDeletion {
    design_id: usize,
    grid_id: usize,
    x: isize,
    y: isize,
    position: isize,
    length: usize,
}

impl Operation for GridHelixDeletion {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::GridHelixCreation(self.design_id, self.grid_id)
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![self.x.to_string(), self.y.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(GridHelixCreation {
            x: self.x,
            y: self.y,
            design_id: self.design_id,
            grid_id: self.grid_id,
            position: self.position,
            length: self.length,
        })
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::RmGridHelix(
            GridHelixDescriptor {
                grid_id: self.grid_id,
                x: self.x,
                y: self.y,
            },
            self.position,
            self.length,
        )
    }

    fn description(&self) -> String {
        format!(
            "Create helix on grid {} of design {}",
            self.grid_id, self.design_id
        )
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    x: new_x as isize,
                    ..*self
                }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    y: new_y as isize,
                    ..*self
                }))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawHelixCreation {
    pub helix: Helix,
    pub helix_id: usize,
    pub delete: bool,
    pub design_id: usize,
}

impl Operation for RawHelixCreation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::RawHelixCreation
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(RawHelixCreation {
            delete: !self.delete,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::RawHelixCreation {
            helix: self.helix.clone(),
            h_id: self.helix_id,
            delete: self.delete,
        }
    }

    fn description(&self) -> String {
        if self.delete {
            format!("Delete grid")
        } else {
            format!("Create grid")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

#[derive(Clone, Debug)]
/// Cut a strand at a given nucleotide.
///
/// If the nucleotide is the 3' end of a cross-over, it will be the 5' end of the 3' half of the
/// split.
/// In all other cases, it will be the 3' end of the 5' end of the split.
pub struct Cut {
    pub strand: Strand,
    pub nucl: Nucl,
    pub strand_id: usize,
    pub undo: bool,
    pub design_id: usize,
}

impl Operation for Cut {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::Cut
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Cut {
            undo: !self.undo,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        if self.strand.length() < 2 {
            UndoableOp::RmStrand {
                strand: self.strand.clone(),
                strand_id: self.strand_id,
                undo: self.undo,
            }
        } else {
            UndoableOp::Cut {
                nucl: self.nucl,
                strand: self.strand.clone(),
                s_id: self.strand_id,
                undo: self.undo,
            }
        }
    }

    fn description(&self) -> String {
        if self.undo {
            format!("Undo Cut")
        } else {
            format!("Do Cut")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Xover {
    pub strand_5prime: Strand,
    pub strand_3prime: Strand,
    pub prime5_id: usize,
    pub prime3_id: usize,
    pub undo: bool,
    pub design_id: usize,
}

impl Operation for Xover {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::Xover
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Xover {
            undo: !self.undo,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::Xover {
            strand_5prime: self.strand_5prime.clone(),
            strand_3prime: self.strand_3prime.clone(),
            prime5_id: self.prime5_id,
            prime3_id: self.prime3_id,
            undo: self.undo,
        }
    }

    fn description(&self) -> String {
        if self.undo {
            format!("Undo Cut")
        } else {
            format!("Do Cut")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

/// Delete a strand
#[derive(Clone, Debug)]
pub struct RmStrand {
    pub strand: Strand,
    pub strand_id: usize,
    pub undo: bool,
    pub design_id: usize,
}

impl Operation for RmStrand {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::CrossCut
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(RmStrand {
            undo: !self.undo,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::RmStrand {
            strand: self.strand.clone(),
            strand_id: self.strand_id,
            undo: self.undo,
        }
    }

    fn description(&self) -> String {
        if self.undo {
            format!("Undo Cut")
        } else {
            format!("Do Cut")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

/// Cut the target strand at nucl, and make a cross over from the source strand.
#[derive(Clone, Debug)]
pub struct CrossCut {
    pub source_strand: Strand,
    pub target_strand: Strand,
    pub source_id: usize,
    pub target_id: usize,
    pub nucl: Nucl,
    /// True if the target strand will be the 3 prime part of the merged strand
    pub target_3prime: bool,
    pub undo: bool,
    pub design_id: usize,
}

impl Operation for CrossCut {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::CrossCut
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(CrossCut {
            undo: !self.undo,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::CrossCut {
            source_strand: self.source_strand.clone(),
            target_strand: self.target_strand.clone(),
            source_id: self.source_id,
            target_id: self.target_id,
            target_3prime: self.target_3prime,
            nucl: self.nucl,
            undo: self.undo,
        }
    }

    fn description(&self) -> String {
        if self.undo {
            format!("Undo Cut")
        } else {
            format!("Do Cut")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct CreateGrid {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub grid_type: GridTypeDescr,
    pub delete: bool,
    pub design_id: usize,
}

impl Operation for CreateGrid {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::CreateGrid
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Choice(vec![String::from("Square"), String::from("Honeycomb")]),
            name: String::from("Grid type"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.grid_type.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(CreateGrid {
            delete: !self.delete,
            ..*self
        })
    }

    fn effect(&self) -> UndoableOp {
        if self.delete {
            UndoableOp::RmGrid
        } else {
            UndoableOp::AddGrid(GridDescriptor {
                position: self.position,
                orientation: self.orientation,
                grid_type: self.grid_type,
            })
        }
    }

    fn description(&self) -> String {
        if self.delete {
            format!("Delete grid")
        } else {
            format!("Create grid")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => match val.as_str() {
                "Square" => Some(Arc::new(Self {
                    grid_type: GridTypeDescr::Square,
                    ..*self
                })),
                "Honeycomb" => Some(Arc::new(Self {
                    grid_type: GridTypeDescr::Honeycomb,
                    ..*self
                })),
                _ => None,
            },
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct BigStrandModification {
    pub initial_state: StrandState,
    pub final_state: StrandState,
    pub reverse: bool,
    pub design_id: usize,
}

impl std::fmt::Debug for BigStrandModification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BigStrandModification")
            .field("reverse", &self.reverse)
            .finish()
    }
}

impl Operation for BigStrandModification {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::BigStrandModification
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(BigStrandModification {
            reverse: !self.reverse,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        if self.reverse {
            UndoableOp::NewStrandState(self.initial_state.clone())
        } else {
            UndoableOp::NewStrandState(self.final_state.clone())
        }
    }

    fn description(&self) -> String {
        if self.reverse {
            format!("Reverse Big Change")
        } else {
            format!("Redo Big Change")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct NewHyperboloid {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub hyperboloid: Hyperboloid,
    pub delete: bool,
    pub design_id: usize,
}

impl Operation for NewHyperboloid {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::CreateGrid
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(NewHyperboloid {
            delete: !self.delete,
            ..self.clone()
        })
    }

    fn effect(&self) -> UndoableOp {
        if self.delete {
            UndoableOp::ClearHyperboloid
        } else {
            UndoableOp::NewHyperboloid {
                position: self.position,
                orientation: self.orientation,
                hyperboloid: self.hyperboloid.clone(),
            }
        }
    }

    fn description(&self) -> String {
        if self.delete {
            format!("Delete nanotube")
        } else {
            format!("Create nanotube")
        }
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }

    fn drop_undo(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct StrandConstruction {
    pub builder: Box<StrandBuilder>,
    pub redo: Option<u32>,
    pub color: u32,
}

impl Operation for StrandConstruction {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::BuildStrand(self.builder.get_timestamp())
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        let redo = self.redo.xor(Some(self.color));
        Arc::new(StrandConstruction {
            builder: self.builder.clone(),
            redo,
            color: self.color,
        })
    }

    fn effect(&self) -> UndoableOp {
        if let Some(color) = self.redo {
            let remake = if self.builder.created_de_novo() {
                Some((self.builder.get_strand_id(), color))
            } else {
                None
            };
            UndoableOp::MoveBuilder(self.builder.clone(), remake)
        } else {
            UndoableOp::ResetBuilder(self.builder.clone())
        }
    }

    fn description(&self) -> String {
        "Building strand".to_string()
    }

    fn target(&self) -> usize {
        self.builder.get_design_id() as usize
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }

    fn must_reverse(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct RigidGridSimulation {
    pub initial_state: crate::design::GridSystemState,
    pub design_id: usize,
}

impl std::fmt::Debug for RigidGridSimulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RigidGridSimulation").finish()
    }
}

impl Operation for RigidGridSimulation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::BigStrandModification
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(self.clone())
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::UndoGridSimulation(self.initial_state.clone())
    }

    fn description(&self) -> String {
        format!("Undo grid simulation")
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }

    fn redoable(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct RigidHelixSimulation {
    pub initial_state: crate::design::RigidHelixState,
    pub design_id: usize,
}

impl std::fmt::Debug for RigidHelixSimulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RigidHelixSimulation").finish()
    }
}

impl Operation for RigidHelixSimulation {
    fn descr(&self) -> OperationDescriptor {
        OperationDescriptor::BigStrandModification
    }

    fn compose(&self, _other: &dyn Operation) -> Option<Arc<dyn Operation>> {
        None
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }

    fn values(&self) -> Vec<String> {
        vec![]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(self.clone())
    }

    fn effect(&self) -> UndoableOp {
        UndoableOp::UndoHelixSimulation(self.initial_state.clone())
    }

    fn description(&self) -> String {
        format!("Undo helix simulation")
    }

    fn target(&self) -> usize {
        self.design_id
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }

    fn redoable(&self) -> bool {
        false
    }
}

#[derive(Debug)]
/// A description of an operation. Two opperations whose `descr` is equal are considered to be the
/// same operation with different parameters.
pub enum OperationDescriptor {
    DesignTranslation(usize),
    DesignRotation(usize, Bivec3),
    HelixRotation(usize, usize, Bivec3),
    HelixTranslation(usize, usize),
    GridRotation(usize, usize, Bivec3),
    GridTranslation(usize, usize),
    GridHelixCreation(usize, usize),
    GridHelixDeletion(usize, usize),
    RawHelixCreation,
    Cut,
    CrossCut,
    Xover,
    RmStrand,
    BuildStrand(std::time::SystemTime),
    CreateGrid,
    BigStrandModification,
}

impl PartialEq<Self> for OperationDescriptor {
    fn eq(&self, rhs: &Self) -> bool {
        use OperationDescriptor::*;
        match (self, rhs) {
            (DesignTranslation(d1), DesignTranslation(d2)) => d1 == d2,
            (DesignRotation(d1, bv1), DesignRotation(d2, bv2)) => {
                d1 == d2 && (*bv1 - *bv2).mag() < 1e-3
            }
            (HelixRotation(d1, h1, bv1), HelixRotation(d2, h2, bv2)) => {
                d1 == d2 && h1 == h2 && (*bv1 - *bv2).mag() < 1e-3
            }
            (HelixTranslation(d1, h1), HelixTranslation(d2, h2)) => d1 == d2 && h1 == h2,
            (GridTranslation(d1, g1), GridTranslation(d2, g2)) => d1 == d2 && g1 == g2,
            (GridRotation(d1, g1, bv1), GridRotation(d2, g2, bv2)) => {
                d1 == d2 && g1 == g2 && (*bv1 - *bv2).mag() < 1e-3
            }
            (GridHelixCreation(d1, g1), GridHelixCreation(d2, g2)) => d1 == d2 && g1 == g2,
            (GridHelixDeletion(d1, g1), GridHelixDeletion(d2, g2)) => d1 == d2 && g1 == g2,
            (CreateGrid, CreateGrid) => true,
            (BuildStrand(ts1), BuildStrand(ts2)) => ts1 == ts2,
            _ => false,
        }
    }

    fn ne(&self, rhs: &Self) -> bool {
        !self.eq(rhs)
    }
}
