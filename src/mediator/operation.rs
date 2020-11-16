use super::{AppNotification, DesignRotation};
use crate::design::IsometryTarget;
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
    fn parameters(&self) -> Vec<Parameter>;
    fn values(&self) -> Vec<String>;
    fn reverse(&self) -> Arc<dyn Operation>;
    fn effect(&self) -> AppNotification;
    fn description(&self) -> String;
    fn target(&self) -> usize;
    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>>;
    fn descr(&self) -> OperationDescriptor;
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

    fn effect(&self) -> AppNotification {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        AppNotification::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Helix(self.helix_id as u32),
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

    fn effect(&self) -> AppNotification {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        AppNotification::Rotation(DesignRotation {
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

    fn effect(&self) -> AppNotification {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        AppNotification::Translation(translation)
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

#[derive(Debug)]
pub enum OperationDescriptor {
    DesignTranslation(usize),
    DesignRotation(usize, Bivec3),
    HelixRotation(usize, usize, Bivec3),
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
            _ => false,
        }
    }

    fn ne(&self, rhs: &Self) -> bool {
        !self.eq(rhs)
    }
}
