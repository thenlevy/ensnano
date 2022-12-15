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

use std::path::Path;
pub const NO_FILE_RECIEVED_LOAD: &'static str = "Open canceled";
pub const NO_FILE_RECIEVED_SAVE: &'static str = "Save canceled";
pub const NO_FILE_RECIEVED_OXDNA: &'static str = "OxDNA export canceled";
pub const NO_FILE_RECIEVED_SCAFFOLD: &'static str = "Scaffold setting canceled";
pub const NO_FILE_RECIEVED_STAPPLE: &'static str = "Staple export canceled";

pub fn failed_to_save_msg<D: std::fmt::Debug>(reason: &D) -> String {
    format!("Failed to save {:?}", reason)
}

pub const NO_SCAFFOLD_SET: &'static str = "No scaffold set. \n
                    Chose a strand and set it as the scaffold by checking the scaffold checkbox\
                    in the status bar";

pub const NO_SCAFFOLD_SEQUENCE_SET: &'static str = "No sequence uploaded for scaffold. \n
                Upload a sequence for the scaffold by pressing the \"Load scaffold\" button";

pub const NO_DESIGN_SELECTED: &'static str =
    "No design selected, select a design by selecting one of its elements";

pub fn successfull_staples_export_msg<P: AsRef<Path>>(file: P) -> String {
    format!(
        "Successfully wrote staples in {}",
        file.as_ref().to_string_lossy()
    )
}

pub const OXDNA_EXPORT_FAILED: &'static str = "OxDNA export failed";
pub const SAVE_DESIGN_FAILED: &'static str = "Could not save design";
pub const SAVE_BEFORE_EXIT: &'static str = "Do you want to save your design before exiting?";
pub const SAVE_BEFORE_LOAD: &'static str =
    "Do you want to save your design before loading an other one?";
pub const SAVE_BEFORE_RELOAD: &'static str =
    "Do you want to save your changes in an other file before reloading?";
pub const SAVE_BEFORE_NEW: &'static str =
    "Do you want to save your design before starting a new one?";

pub fn optimize_scaffold_position_msg(default_position: usize) -> String {
    format!("Optimize the scaffold position ?\n
              If you chose \"Yes\", ENSnano will position the scaffold in a way that minimizes the \
              number of anti-patern (G^4, C^4 (A|T)^7) in the stapples sequence. If you chose \"No\", \
              the scaffold sequence will begin at position {}", default_position)
}

pub fn invalid_sequence_file(first_invalid_char_position: usize) -> String {
    format!(
        "This text file does not contain a valid DNA sequence.\n
             First invalid char at position {}",
        first_invalid_char_position
    )
}

use crate::dialog::Filters;
pub const DESIGN_LOAD_FILTER: Filters = &[
    (
        "All supported files",
        &[
            crate::consts::ENS_EXTENSION,
            crate::consts::ENS_BACKUP_EXTENSION,
            "json",
            "sc",
        ],
    ),
    (
        "ENSnano files",
        &[
            crate::consts::ENS_EXTENSION,
            crate::consts::ENS_BACKUP_EXTENSION,
        ],
    ),
    ("json files", &["json"]),
    ("scadnano files", &["sc"]),
];

pub const DESIGN_WRITE_FILTER: Filters = &[("ENSnano files", &[crate::consts::ENS_EXTENSION])];

pub const SEQUENCE_FILTERS: Filters = &[("Text files", &["txt"])];

pub const CHANGING_DNA_PARAMETERS_WARNING: &'static str =
    "Are you sure that you want to change DNA parameters?";

pub const OXDNA_CONFIG_EXTENSTION: &str = "oxdna";

pub const OXDNA_CONFIG_FILTERS: Filters = &[("Oxdna config files", &[OXDNA_CONFIG_EXTENSTION])];

pub const STAPLES_FILTER: Filters = &[("Excel files", &["xlsx"])];

pub const ORIGAMI_FLTER: Filters = &[("Origami files", &[crate::consts::ORIGAMI_EXTENSION])];

pub const PDB_FILTER: Filters = &[("Pdb files", &["pdb"])];
pub const CADNANO_FILTER: Filters = &[("Cadnano files", &["json"])];

pub const OBJECT3D_FILTERS: Filters = &[
    ("All supported files", &["gltf", "stl"]),
    ("Stl files", &["stl"]),
    ("Gltf files", &["gltf"]),
];

pub const SVG_FILTERS: Filters = &[("Svg files", &["svg"])];

pub const SET_DESIGN_DIRECTORY_FIRST: &str =
    "It is not possible to import 3D objects in an unamed design.
Please save your design first to give it a name";
