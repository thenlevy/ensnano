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
pub const BEZIER_START_WIDGET_ID: u32 = 7;
pub const BEZIER_CONTROL1_WIDGET_ID: u32 = 8;
pub const BEZIER_CONTROL2_WIDGET_ID: u32 = 9;
pub const BEZIER_END_WIDGET_ID: u32 = 10;

pub fn bezier_widget_id(helix_id: u32, control_point: BezierControlPoint) -> u32 {
    let bezier_id = bezier_control_id(control_point);
    (helix_id << 8) | bezier_id
}

use ensnano_interactor::BezierControlPoint;
pub fn widget_id_to_bezier(id: u32) -> Option<(usize, BezierControlPoint)> {
    let control = match id & 0xFF {
        BEZIER_START_WIDGET_ID => Some(BezierControlPoint::Start),
        BEZIER_END_WIDGET_ID => Some(BezierControlPoint::End),
        BEZIER_CONTROL1_WIDGET_ID => Some(BezierControlPoint::Control1),
        BEZIER_CONTROL2_WIDGET_ID => Some(BezierControlPoint::Control2),
        _ => None,
    };
    Some((id >> 8) as usize).zip(control)
}

pub const fn bezier_control_color(control_point: BezierControlPoint) -> u32 {
    match control_point {
        BezierControlPoint::Start => BEZIER_START_COLOR,
        BezierControlPoint::Control1 => BEZIER_CONTROL1_COLOR,
        BezierControlPoint::Control2 => BEZIER_CONTROL2_COLOR,
        BezierControlPoint::End => BEZIER_END_COLOR,
    }
}

pub const fn bezier_control_id(control_point: BezierControlPoint) -> u32 {
    match control_point {
        BezierControlPoint::Start => BEZIER_START_WIDGET_ID,
        BezierControlPoint::Control1 => BEZIER_CONTROL1_WIDGET_ID,
        BezierControlPoint::Control2 => BEZIER_CONTROL2_WIDGET_ID,
        BezierControlPoint::End => BEZIER_END_WIDGET_ID,
    }
}

pub const BASIS_SYMBOLS: &[char] = &['A', 'T', 'G', 'C', '*'];
pub const NB_BASIS_SYMBOLS: usize = BASIS_SYMBOLS.len();

pub const BASE_SCROLL_SENSITIVITY: f32 = 0.12;

pub const SAMPLE_COUNT: u32 = 4;

pub const HELIX_BORDER_COLOR: u32 = 0xFF_101010;

pub const CANDIDATE_COLOR: u32 = 0xBF_00_FF_00;
pub const SELECTED_COLOR: u32 = 0xBF_FF_00_00;
pub const SUGGESTION_COLOR: u32 = 0xBF_FF_00_FF;
pub const PIVOT_SPHERE_COLOR: u32 = 0xBF_FF_FF_00;
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

use iced::Color;
pub const fn innactive_color() -> Color {
    Color::from_rgb(0.6, 0.6, 0.6)
}

pub const CTRL: &'static str = if cfg!(target_os = "macos") {
    "\u{2318}"
} else {
    "ctrl"
};

pub const ALT: &'static str = if cfg!(target_os = "macos") {
    "\u{2325}"
} else {
    "alt"
};

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

pub const ENS_EXTENSION: &'static str = "ens";
pub const ENS_BACKUP_EXTENSION: &'static str = "ensbackup";
pub const ENS_UNAMED_FILE_NAME: &'static str = "Unamed_design";
pub const CANNOT_OPEN_DEFAULT_DIR: &'static str = "Unable to open document or home directory.
No backup will be saved for this unamed design";

pub const NO_DESIGN_TITLE: &'static str = "New file";

pub const BEZIER_CONTROL_RADIUS: f32 = 2.5;
pub const BEZIER_SQUELETON_RADIUS: f32 = 0.5;
pub const BEZIER_START_COLOR: u32 = 0xFF_B0_21_21;
pub const BEZIER_END_COLOR: u32 = 0xFF_F0_CA_22;
pub const BEZIER_CONTROL1_COLOR: u32 = 0xFF_37_85_30;
pub const BEZIER_CONTROL2_COLOR: u32 = 0xFF_1A_15_70;
pub const SEC_BETWEEN_BACKUPS: u64 = 60;
pub const SEC_PER_YEAR: u64 = 31_536_000;
