use super::{DesignRotation, AppNotification};
use crate::design::IsometryTarget;
use ultraviolet::{Bivec3, Rotor3, Vec3};
use std::sync::Arc;

pub enum ParameterField {
    Choice(Vec<String>),
    Value,
}

pub struct Parameter {
    pub field: ParameterField,
    pub name: String
}

pub trait Operation: std::fmt::Debug + Sync + Send {
    fn parameters(&self) -> Vec<Parameter>;
    fn values(&self) -> Vec<String>;
    fn reverse(&self) -> Arc<dyn Operation>;
    fn effect(&self) -> AppNotification;
    fn description(&self) -> String;
    fn target(&self) -> usize;
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
    fn parameters(&self) -> Vec<Parameter> {
        vec![Parameter {
        field: ParameterField::Value,
        name: String::from("angle"),
        }]
    }

    fn values(&self) -> Vec<String> {
        vec![self.angle.to_string()]
    }

    fn reverse(&self) -> Arc<dyn Operation> {
        Arc::new(Self {
            plane: -self.plane,
            ..*self
        })
    }

    fn effect(&self) -> AppNotification {
        let rotor = Rotor3::from_angle_plane(self.angle, self.plane);
        AppNotification::Rotation(DesignRotation {
            rotation: rotor,
            origin: self.origin,
            target: IsometryTarget::Helix(self.helix_id as u32)
        })
    }

    fn description(&self) -> String {
        format!("Rotate helix {} of design {}", self.helix_id, self.design_id)
    }

    fn target(&self) -> usize {
        self.design_id
    }

}

