pub const VIEWER_BINDING_ID: u32 = 0;
pub const INSTANCES_BINDING_ID: u32 = 1;
pub const LIGHT_BINDING_ID: u32 = 2;

pub const VERTEX_POSITION_ADRESS: u32 = 0;
pub const VERTEX_NORMAL_ADRESS: u32 = 1;

pub const BOUND_RADIUS: f32 = 0.03;
pub const BOUND_LENGTH: f32 = 0.1;
pub const DIAG_BOUND_LENGTH: f32 = std::f32::consts::SQRT_2 * BOUND_LENGTH;
pub const NB_RAY_TUBE: usize = 12;

pub const SPHERE_RADIUS: f32 = 0.1;
pub const NB_STACK_SPHERE: u16 = 12;
pub const NB_SECTOR_SPHERE: u16 = 12;
