pub const VIEWER_BINDING_ID: u32 = 0;
pub const INSTANCES_BINDING_ID: u32 = 1;
pub const LIGHT_BINDING_ID: u32 = 2;
pub const TEXTURE_BINDING_ID: u32 = 2;
pub const MODEL_BINDING_ID: u32 = 3;

pub const VERTEX_POSITION_ADRESS: u32 = 0;
pub const VERTEX_NORMAL_ADRESS: u32 = 1;

pub const BOUND_RADIUS: f32 = 0.06;
pub const BOUND_LENGTH: f32 = 1.;
pub const NB_RAY_TUBE: usize = 12;

pub const SPHERE_RADIUS: f32 = 0.2;
pub const NB_STACK_SPHERE: u16 = 12;
pub const NB_SECTOR_SPHERE: u16 = 12;

pub const NB_SECTOR_CIRCLE: u16 = 36;

pub const SELECT_SCALE_FACTOR: f32 = 1.3;

pub const RIGHT_HANDLE_ID: u32 = 0;
pub const UP_HANDLE_ID: u32 = 1;
pub const DIR_HANDLE_ID: u32 = 2;
pub const RIGHT_CIRCLE_ID: u32 = 3;
pub const UP_CIRCLE_ID: u32 = 4;
pub const FRONT_CIRCLE_ID: u32 = 5;
pub const SPHERE_WIDGET_ID: u32 = 6;

pub const PHANTOM_RANGE: i32 = 1000;

pub const BASIS_SYMBOLS: &[char] = &['A', 'T', 'G', 'C', '*'];
pub const NB_BASIS_SYMBOLS: usize = BASIS_SYMBOLS.len();

pub const BASE_SCROLL_SENSITIVITY: f32 = 0.12;

pub const SAMPLE_COUNT: u32 = 4;

pub const HELIX_BORDER_COLOR: u32 = 0xFF_101010;

pub const CANDIDATE_COLOR: u32 = 0xBF_00_FF_00;
pub const SELECTED_COLOR: u32 = 0xBF_FF_00_00;
pub const SUGGESTION_COLOR: u32 = 0xBF_FF_00_FF;

pub const MAX_ZOOM_2D: f32 = 50.0;

pub const CIRCLE2D_GREY: u32 = 0xFF_4D4D4D;
pub const CIRCLE2D_BLUE: u32 = 0xFF_036992;
pub const CIRCLE2D_RED: u32 = 0xFF_920303;
pub const CIRCLE2D_GREEN: u32 = 0xFF_0C9203;
