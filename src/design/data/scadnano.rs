use std::collections::HashMap;

use super::grid::{GridDescriptor, GridTypeDescr};
use ultraviolet::{Rotor3, Vec3};

#[derive(Serialize, Deserialize)]
pub struct ScadnanoDesign {
    pub version: String,
    #[serde(default = "default_grid")]
    pub grid: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<HashMap<String, ScadnanoGroup>>,
    pub helices: Vec<ScadnanoHelix>,
    pub strands: Vec<ScadnanoStrand>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modifications_in_design: Option<HashMap<String, ScadnanoModification>>,
}

fn default_grid() -> String {
    String::from("square")
}

impl ScadnanoDesign {
    pub fn default_grid_descriptor(&self) -> Option<GridDescriptor> {
        let grid_type = match self.grid.as_str() {
            "square" => Some(GridTypeDescr::Square),
            "honeycomb" => Some(GridTypeDescr::Honeycomb),
            grid_type => {
                println!("Unsported grid type: {}", grid_type);
                None
            }
        }?;
        Some(GridDescriptor {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            grid_type,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct ScadnanoGroup {
    position: Vec3,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pitch: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    yaw: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    roll: Option<f32>,
    grid: String,
}

impl ScadnanoGroup {
    pub fn to_grid_desc(&self) -> Option<GridDescriptor> {
        let grid_type = match self.grid.as_str() {
            "square" => Some(GridTypeDescr::Square),
            "honeycomb" => Some(GridTypeDescr::Honeycomb),
            grid_type => {
                println!("Unsported grid type: {}", grid_type);
                None
            }
        }?;
        let orientation = Rotor3::from_euler_angles(
            self.roll.unwrap_or_default().to_radians(),
            self.pitch.unwrap_or_default().to_radians(),
            self.yaw.unwrap_or_default().to_radians(),
        );
        Some(GridDescriptor {
            grid_type,
            orientation,
            position: self.position,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct ScadnanoHelix {
    #[serde(default)]
    pub max_offset: usize,
    pub grid_position: Vec<isize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ScadnanoStrand {
    #[serde(default)]
    pub is_scaffold: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sequence: Option<String>,
    pub color: String,
    pub domains: Vec<ScadnanoDomain>,
    #[serde(
        rename = "5prime_modification",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub prime5_modification: Option<String>,
    #[serde(
        rename = "3prime_modification",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub prime3_modification: Option<String>,
}

impl ScadnanoStrand {
    pub fn color(&self) -> Option<u32> {
        let ret = u32::from_str_radix(&self.color[1..], 16).ok();
        if ret.is_none() {
            println!("invalid color {}", self.color);
        }
        ret
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScadnanoDomain {
    Loopout {
        loopout: usize,
    },
    HelixDomain {
        helix: usize,
        start: isize,
        end: isize,
        forward: bool,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        insertions: Option<Vec<Vec<isize>>>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        deletions: Option<Vec<isize>>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct ScadnanoModification {
    pub display_text: String,
    pub idt_text: String,
    pub location: String,
}
