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

use super::{DesignOperation, DesignRotation, DesignTranslation, GroupId, IsometryTarget};
use ensnano_design::{
    grid::{GridDescriptor, GridTypeDescr},
    Nucl,
};
use ultraviolet::{Bivec3, Rotor3, Vec3};

pub enum ParameterField {
    Choice(Vec<String>),
    Value,
}

pub struct Parameter {
    pub field: ParameterField,
    pub name: String,
}

use std::sync::Arc;

pub trait Operation: std::fmt::Debug + Sync + Send {
    /// The effect of self that must be sent as a notifications to the targeted designs
    fn effect(&self) -> DesignOperation;
    /// A description of self of display in the GUI
    fn description(&self) -> String;

    /// Produce an new opperation by setting the value of the `n`-th parameter to `val`.
    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }

    /// The set of parameters that can be modified via a GUI component
    fn parameters(&self) -> Vec<Parameter> {
        vec![]
    }
    /// The values associated to the parameters.
    fn values(&self) -> Vec<String> {
        vec![]
    }

    /// If true, this new operation is applied to the last initial state instead
    fn replace_previous(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub struct GridRotation {
    pub origin: Vec3,
    pub design_id: usize,
    pub grid_ids: Vec<usize>,
    pub angle: f32,
    pub plane: Bivec3,
    pub group_id: Option<GroupId>,
    pub replace: bool,
}

impl Operation for GridRotation {
    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Value,
            name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_degrees().to_string()]
    }

    fn effect(&self) -> DesignOperation {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        DesignOperation::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Grids(self.grid_ids.clone()),
            group_id: self.group_id,
        })
    }

    fn description(&self) -> String {
        format!(
            "Rotate grids {:?} of design {}",
            self.grid_ids, self.design_id
        )
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        if n == 0 {
            let degrees: f32 = val.parse().ok()?;
            Some(Arc::new(Self {
                angle: degrees.to_radians(),
                replace: true,
                ..self.clone()
            }))
        } else {
            None
        }
    }

    fn replace_previous(&self) -> bool {
        self.replace
    }
}

#[derive(Clone, Debug)]
pub struct HelixRotation {
    pub origin: Vec3,
    pub design_id: usize,
    pub helices: Vec<usize>,
    pub angle: f32,
    pub plane: Bivec3,
    pub group_id: Option<GroupId>,
    pub replace: bool,
}

impl Operation for HelixRotation {
    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Value,
            name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_degrees().to_string()]
    }

    fn effect(&self) -> DesignOperation {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        DesignOperation::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Helices(self.helices.clone(), false),
            group_id: self.group_id,
        })
    }

    fn description(&self) -> String {
        format!(
            "Rotate helices {:?} of design {}",
            self.helices, self.design_id
        )
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        if n == 0 {
            let degrees: f32 = val.parse().ok()?;
            Some(Arc::new(Self {
                angle: degrees.to_radians(),
                replace: true,
                ..self.clone()
            }))
        } else {
            None
        }
    }

    fn replace_previous(&self) -> bool {
        self.replace
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
    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Value,
            name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_degrees().to_string()]
    }

    fn effect(&self) -> DesignOperation {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        DesignOperation::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Design,
            group_id: None,
        })
    }

    fn description(&self) -> String {
        format!("Rotate view of design {}", self.design_id)
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

    fn effect(&self) -> DesignOperation {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        DesignOperation::Translation(DesignTranslation {
            translation,
            target: IsometryTarget::Design,
            group_id: None,
        })
    }

    fn description(&self) -> String {
        format!("Translate design {}", self.design_id)
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
    pub helices: Vec<usize>,
    pub right: Vec3,
    pub top: Vec3,
    pub dir: Vec3,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub snap: bool,
    pub group_id: Option<GroupId>,
    pub replace: bool,
}

impl Operation for HelixTranslation {
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

    fn effect(&self) -> DesignOperation {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        DesignOperation::Translation(DesignTranslation {
            translation,
            target: IsometryTarget::Helices(self.helices.clone(), self.snap),
            group_id: self.group_id,
        })
    }

    fn description(&self) -> String {
        format!(
            "Translate helices {:?} of design {}",
            self.helices, self.design_id
        )
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    x: new_x,
                    replace: true,
                    ..self.clone()
                }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    y: new_y,
                    replace: true,
                    ..self.clone()
                }))
            }
            2 => {
                let new_z: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    z: new_z,
                    replace: true,
                    ..self.clone()
                }))
            }
            _ => None,
        }
    }

    fn replace_previous(&self) -> bool {
        self.replace
    }
}

#[derive(Debug, Clone)]
pub struct GridTranslation {
    pub design_id: usize,
    pub grid_ids: Vec<usize>,
    pub right: Vec3,
    pub top: Vec3,
    pub dir: Vec3,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub group_id: Option<GroupId>,
    pub replace: bool,
}

impl Operation for GridTranslation {
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

    fn effect(&self) -> DesignOperation {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        DesignOperation::Translation(DesignTranslation {
            translation,
            target: IsometryTarget::Grids(self.grid_ids.clone()),
            group_id: self.group_id,
        })
    }

    fn description(&self) -> String {
        format!(
            "Translate grids {:?} of design {}",
            self.grid_ids, self.design_id
        )
    }

    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>> {
        match n {
            0 => {
                let new_x: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    x: new_x,
                    replace: true,
                    ..self.clone()
                }))
            }
            1 => {
                let new_y: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    y: new_y,
                    replace: true,
                    ..self.clone()
                }))
            }
            2 => {
                let new_z: f32 = val.parse().ok()?;
                Some(Arc::new(Self {
                    z: new_z,
                    replace: true,
                    ..self.clone()
                }))
            }
            _ => None,
        }
    }

    fn replace_previous(&self) -> bool {
        self.replace
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
    fn values(&self) -> Vec<String> {
        vec![self.x.to_string(), self.y.to_string()]
    }

    fn effect(&self) -> DesignOperation {
        DesignOperation::AddGridHelix {
            position: ensnano_design::grid::GridPosition {
                grid: self.grid_id,
                x: self.x,
                y: self.y,
                roll: 0f32,
                axis_pos: 0,
            },
            start: self.position,
            length: self.length,
        }
    }

    fn description(&self) -> String {
        format!(
            "Create helix on grid {} of design {}",
            self.grid_id, self.design_id
        )
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
/// Cut a strand at a given nucleotide.
///
/// If the nucleotide is the 3' end of a cross-over, it will be the 5' end of the 3' half of the
/// split.
/// In all other cases, it will be the 3' end of the 5' end of the split.
pub struct Cut {
    pub nucl: Nucl,
    pub strand_id: usize,
    pub design_id: usize,
}

impl Operation for Cut {
    fn effect(&self) -> DesignOperation {
        DesignOperation::Cut {
            nucl: self.nucl,
            s_id: self.strand_id,
        }
    }

    fn description(&self) -> String {
        format!("Cut on nucleotide {}", self.nucl)
    }

    fn with_new_value(&self, _n: usize, _val: String) -> Option<Arc<dyn Operation>> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Xover {
    pub prime5_id: usize,
    pub prime3_id: usize,
    pub undo: bool,
    pub design_id: usize,
}

impl Operation for Xover {
    fn effect(&self) -> DesignOperation {
        DesignOperation::Xover {
            prime5_id: self.prime5_id,
            prime3_id: self.prime3_id,
        }
    }

    fn description(&self) -> String {
        if self.undo {
            format!("Undo Cut")
        } else {
            format!("Do Cut")
        }
    }
}

/*
/// Delete a strand
#[derive(Clone, Debug)]
pub struct RmStrand {
    pub strand_id: usize,
    pub design_id: usize,
}

impl Operation for RmStrand {
    fn effect(&self) -> DesignOperation {
        DesignOperation::RmStrand {
            strand_id: self.strand_id,
            design_id: self.design_id,
        }
    }

    fn description(&self) -> String {
        format!(
            "Remove strand {} of design {}",
            self.strand_id, self.design_id
        )
    }
}

#[derive(Clone, Debug)]
pub struct RmHelix {
    pub helix_id: usize,
    pub design_id: usize,
}

impl Operation for RmHelix {
    fn effect(&self) -> DesignOperation {
        DesignOperation::RmHelix {
            h_id: self.helix_id,
        }
    }

    fn description(&self) -> String {
        format!("Remove helix {}", self.helix_id)
    }
}
*/

/// Cut the target strand at nucl, and make a cross over from the source strand.
#[derive(Clone, Debug)]
pub struct CrossCut {
    pub source_id: usize,
    pub target_id: usize,
    pub nucl: Nucl,
    /// True if the target strand will be the 3 prime part of the merged strand
    pub target_3prime: bool,
    pub design_id: usize,
}

impl Operation for CrossCut {
    fn effect(&self) -> DesignOperation {
        DesignOperation::CrossCut {
            source_id: self.source_id,
            target_id: self.target_id,
            target_3prime: self.target_3prime,
            nucl: self.nucl,
        }
    }

    fn description(&self) -> String {
        format!(
            "Cross cut from strand {} on nucl {} (strand {})",
            self.source_id, self.nucl, self.target_id
        )
    }
}

#[derive(Clone, Debug)]
pub struct CreateGrid {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub grid_type: GridTypeDescr,
    pub design_id: usize,
}

impl Operation for CreateGrid {
    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
            field: ParameterField::Choice(vec![String::from("Square"), String::from("Honeycomb")]),
            name: String::from("Grid type"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.grid_type.to_string()]
    }

    fn effect(&self) -> DesignOperation {
        DesignOperation::AddGrid(GridDescriptor {
            position: self.position,
            orientation: self.orientation,
            grid_type: self.grid_type,
            invisible: false,
        })
    }

    fn description(&self) -> String {
        String::from("Create Grid")
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
