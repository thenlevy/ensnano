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
#![allow(clippy::unusual_byte_groupings)]
use ultraviolet::Vec4;
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

pub const CANDIDATE_SCALE_FACTOR: f32 = 1.3;
pub const SELECT_SCALE_FACTOR: f32 = 1. + 2. * (CANDIDATE_SCALE_FACTOR - 1.);
pub const PIVOT_SCALE_FACTOR: f32 = 1.2 * SELECT_SCALE_FACTOR;
pub const FREE_XOVER_SCALE_FACTOR: f32 = 1.25 * SELECT_SCALE_FACTOR;

pub const RIGHT_HANDLE_ID: u32 = 0;
pub const UP_HANDLE_ID: u32 = 1;
pub const DIR_HANDLE_ID: u32 = 2;
pub const RIGHT_CIRCLE_ID: u32 = 3;
pub const UP_CIRCLE_ID: u32 = 4;
pub const FRONT_CIRCLE_ID: u32 = 5;
pub const SPHERE_WIDGET_ID: u32 = 6;
pub const BEZIER_START_WIDGET_ID: u32 = 7;
#[allow(dead_code)]
pub const BEZIER_CONTROL1_WIDGET_ID: u32 = 8;
#[allow(dead_code)]
pub const BEZIER_CONTROL2_WIDGET_ID: u32 = 9;
pub const BEZIER_END_WIDGET_ID: u32 = 10;

pub fn bezier_widget_id(helix_id: u32, control_point: BezierControlPoint) -> u32 {
    let bezier_id = bezier_control_id(control_point);
    (helix_id << 8) | bezier_id
}

use crate::{BezierControlPoint, RevolutionSimulationParameters};
pub fn widget_id_to_bezier(id: u32) -> Option<(usize, BezierControlPoint)> {
    let control = match id & 0xFF {
        n if n > BEZIER_END_WIDGET_ID => Some(BezierControlPoint::PiecewiseBezier(
            (n - 1 - BEZIER_END_WIDGET_ID) as usize,
        )),
        n => {
            let control = ((n - BEZIER_START_WIDGET_ID) as usize).try_into().ok();
            control.map(BezierControlPoint::CubicBezier)
        }
    };
    Some((id >> 8) as usize).zip(control)
}

pub const fn bezier_control_color(control_point: BezierControlPoint) -> u32 {
    use ensnano_design::CubicBezierControlPoint::*;
    match control_point {
        BezierControlPoint::CubicBezier(Start) => BEZIER_START_COLOR,
        BezierControlPoint::CubicBezier(Control1) => BEZIER_CONTROL1_COLOR,
        BezierControlPoint::CubicBezier(Control2) => BEZIER_CONTROL2_COLOR,
        BezierControlPoint::CubicBezier(End) => BEZIER_END_COLOR,
        BezierControlPoint::PiecewiseBezier(_) => PIECEWISE_BEZIER_COLOR,
    }
}

pub fn bezier_control_id(control_point: BezierControlPoint) -> u32 {
    match control_point {
        BezierControlPoint::CubicBezier(c) => {
            let control_id: usize = c.into();
            BEZIER_START_WIDGET_ID + control_id as u32
        }
        BezierControlPoint::PiecewiseBezier(n) => n as u32 + BEZIER_END_WIDGET_ID + 1,
    }
}

pub const BASE_SCROLL_SENSITIVITY: f32 = 0.12;

pub fn scroll_sensitivity_convertion(sensitivity: f32) -> f32 {
    10f32.powf(sensitivity / 10.) * BASE_SCROLL_SENSITIVITY
}

pub const SAMPLE_COUNT: u32 = 4;

pub const HELIX_BORDER_COLOR: u32 = 0xFF_101010;

pub const CANDIDATE_COLOR: u32 = 0xBF_00_FF_00;
pub const SELECTED_COLOR: u32 = 0xBF_FF_00_00;
pub const SUGGESTION_COLOR: u32 = 0xBF_FF_00_FF;
pub const PIVOT_SPHERE_COLOR: u32 = 0xBF_FF_FF_00;
pub const SURFACE_PIVOT_SPHERE_COLOR: u32 = 0xBF_FF_14_B9; // pinkish
pub const FREE_XOVER_COLOR: u32 = 0xBF_00_00_FF;
pub const CHECKED_XOVER_COLOR: u32 = 0xBF_3C_B3_71; //Medium sea green
pub const UNCHECKED_XOVER_COLOR: u32 = 0xCF_FF_14_93; // Deep pink
pub const STEREOGRAPHIC_SPHERE_COLOR: u32 = 0xDD_2F_4F_4F; // Slate grey
pub const STEREOGRAPHIC_SPHERE_RADIUS: f32 = 2.;

pub const MAX_ZOOM_2D: f32 = 50.0;

pub const CIRCLE2D_GREY: u32 = 0xFF_4D4D4D;
pub const CIRCLE2D_BLUE: u32 = 0xFF_036992;
pub const CIRCLE2D_RED: u32 = 0xFF_920303;
pub const CIRCLE2D_GREEN: u32 = 0xFF_0C9203;

pub const SCAFFOLD_COLOR: u32 = 0xFF_3498DB;

pub const SELECTED_HELIX2D_COLOR: u32 = 0xFF_BF_1E_28;

pub const ICON_PHYSICAL_ENGINE: char = '\u{e917}';
pub const ICON_ATGC: char = '\u{e90d}';
pub const ICON_SQUARE_GRID: char = '\u{e90e}';
pub const ICON_HONEYCOMB_GRID: char = '\u{e907}';
pub const ICON_NANOTUBE: char = '\u{e914}';

pub const CTRL: &str = if cfg!(target_os = "macos") {
    "\u{2318}"
} else {
    "ctrl"
};

pub const ALT: &str = if cfg!(target_os = "macos") {
    "\u{2325}"
} else {
    "alt"
};

pub const KEY_RIGHT: char = '\u{2192}';
pub const KEY_LEFT: char = '\u{2190}';
pub const KEY_UP: char = '\u{2191}';
pub const KEY_DOWN: char = '\u{2193}';

pub const BACKSPACECHAR: char = '\u{232b}';
pub const SUPPRCHAR: char = '\u{2326}';
pub const SELECTCHAR: char = '\u{e90c}';
pub const HELIXCHAR: char = '\u{e913}';
pub const STRANDCHAR: char = '\u{e901}';
pub const NUCLCHAR: char = '\u{e900}';

pub const SHIFT: char = '\u{21e7}';
pub const MOVECHAR: char = '\u{e904}';
pub const ROTCHAR: char = '\u{e915}';
pub const LCLICK: char = '\u{e918}';
pub const MCLICK: char = '\u{e91b}';
pub const RCLICK: char = '\u{e91a}';

pub const WELCOME_MSG: &str = "
==============================================================================
==============================================================================
                               WELCOME TO ENSNANO\n
During runtime, the console may print error messages that are useful to the
programer to investigate bugs.\n
==============================================================================
==============================================================================
";

pub const RGB_HANDLE_COLORS: [u32; 3] = [0xFF0000, 0xFF00, 0xFF];
pub const CYM_HANDLE_COLORS: [u32; 3] = [0x00FFFF, 0xFF00FF, 0xFFFF00];

pub const ORIGAMI_EXTENSION: &str = "origami";
pub const ENS_EXTENSION: &str = "ens";
pub const ENS_BACKUP_EXTENSION: &str = "ensbackup";
pub const ENS_UNNAMED_FILE_NAME: &str = "Unnamed_design";
pub const CANNOT_OPEN_DEFAULT_DIR: &str = "Unable to open document or home directory.
No backup will be saved for this unnamed design";

pub const NO_DESIGN_TITLE: &str = "New file";

pub const BEZIER_CONTROL_RADIUS: f32 = 2.5;
pub const BEZIER_SQUELETON_RADIUS: f32 = 0.5;
pub const BEZIER_START_COLOR: u32 = 0xFF_B0_21_21;
pub const BEZIER_END_COLOR: u32 = 0xFF_F0_CA_22;
pub const BEZIER_CONTROL1_COLOR: u32 = 0xFF_37_85_30;
pub const BEZIER_CONTROL2_COLOR: u32 = 0xFF_1A_15_70;
pub const SEC_BETWEEN_BACKUPS: u64 = 60;
pub const SEC_PER_YEAR: u64 = 31_536_000;

pub const DEFAULT_STEREOGRAPHIC_ZOOM: f32 = 3.0;
pub const STEREOGRAPHIC_ZOOM_STEP: f32 = 1.1;
pub const PIECEWISE_BEZIER_COLOR: u32 = 0xFF_66_CD_AA; // Medium Aquamarine

pub const UPDATE_VISIBILITY_SIEVE_LABEL: &str = "Update visibility sieve";

pub const COLOR_ADENOSINE: u32 = 0x00_CC0000;
pub const COLOR_THYMINE: u32 = 0x00_0000CC;
pub const COLOR_GUANINE: u32 = 0x00_00CC00;
pub const COLOR_CYTOSINE: u32 = 0x00_CC00CC;
pub const UNKONW_BASE_COLOR: u32 = 0x00_77_88_99;
pub const REGULAR_H_BOND_COLOR: u32 = 0x_29_26_26;

pub const fn basis_color(basis: char) -> u32 {
    match basis {
        'A' => COLOR_ADENOSINE,
        'T' => COLOR_THYMINE,
        'G' => COLOR_GUANINE,
        'C' => COLOR_CYTOSINE,
        _ => UNKONW_BASE_COLOR,
    }
}

pub const BASIS_SCALE: ultraviolet::Vec3 = ultraviolet::Vec3 {
    x: 0.33 / SPHERE_RADIUS,
    y: BOUND_RADIUS / SPHERE_RADIUS,
    z: 2. * BOUND_RADIUS / SPHERE_RADIUS,
};

pub const BLACK_VEC4: Vec4 = Vec4 {
    x: 0.,
    y: 0.,
    z: 0.,
    w: 1.,
};
const GREY_UNKNOWN_NUCL: f32 = 0.3;
pub const GREY_UNKNOWN_NUCL_VEC4: Vec4 = Vec4 {
    x: GREY_UNKNOWN_NUCL,
    y: GREY_UNKNOWN_NUCL,
    z: GREY_UNKNOWN_NUCL,
    w: 1.,
};

pub const PRINTABLE_CHARS: &[char] = &[
    'A', 'T', 'G', 'C', 'N', 'K', 'U', 'X', 'S', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    '-', 'n', 't', 'm', '.', '/', ' ', '(', ')', '?',
];
pub const NB_PRINTABLE_CHARS: usize = PRINTABLE_CHARS.len();

/// The factor by which the width of candidate highlighted strands is multiplied
pub const CANDIDATE_STRAND_HIGHLIGHT_FACTOR_2D: f32 = 1.7;
/// The factor by which the width of selected highlighted strands is multiplied
pub const SELECTED_STRAND_HIGHLIGHT_FACTOR_2D: f32 =
    1. + 2. * (CANDIDATE_STRAND_HIGHLIGHT_FACTOR_2D - 1.);

pub const SELECTION_2D_CYCLE_TIME_LIMIT_MS: u64 = 2_000;

// steel blue
pub const BEZIER_SHEET_CORNER_COLOR: u32 = 0x46_82_B4;
pub const BEZIER_SHEET_CORNER_RADIUS: f32 = 15.0;

pub const APP_NAME: &str = "ENSnano";

pub const DEFAULT_REVOLUTION_SIMULATION_PARAMETERS: RevolutionSimulationParameters =
    RevolutionSimulationParameters {
        nb_section_per_segment: 100,
        spring_stiffness: 8.0,
        torsion_stiffness: 30.0,
        fluid_friction: 1.0,
        ball_mass: 10.0,
        time_span: 5.0e-2,
        simulation_step: 1e-3,
        method: crate::EquadiffSolvingMethod::Ralston,
        rescaling: 1.,
    };
