use super::{AppNotification, DesignRotation, DesignTranslation};
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
    fn compose(&self, other: &dyn Operation) -> Option<Arc<dyn Operation>>;
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

    fn effect(&self) -> AppNotification {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        AppNotification::Rotation(DesignRotation {
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

    fn effect(&self) -> AppNotification {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        AppNotification::Translation(DesignTranslation {
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

    fn effect(&self) -> AppNotification {
        let translation = self.x * self.right + self.y * self.top + self.z * self.dir;
        AppNotification::Translation(DesignTranslation {
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

#[derive(Debug)]
pub enum OperationDescriptor {
    DesignTranslation(usize),
    DesignRotation(usize, Bivec3),
    HelixRotation(usize, usize, Bivec3),
    GridRotation(usize, usize, Bivec3),
    GridTranslation(usize, usize),
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
            (GridTranslation(d1, g1), GridTranslation(d2, g2)) => d1 == d2 && g1 == g2,
            (GridRotation(d1, g1, bv1), GridRotation(d2, g2, bv2)) => {
                d1 == d2 && g1 == g2 && (*bv1 - *bv2).mag() < 1e-3
            }
            _ => false,
        }
    }

    fn ne(&self, rhs: &Self) -> bool {
        !self.eq(rhs)
    }
}
