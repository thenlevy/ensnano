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

use super::*;
use ensnano_design::{codenano, scadnano};

impl Data {
    /// Create a new data by reading a file. At the moment, the supported format are
    /// * codenano
    /// * icednano
    pub fn new_with_path(json_path: &PathBuf) -> Result<Self, ParseDesignError> {
        let mut xover_ids: IdGenerator<(Nucl, Nucl)> = Default::default();
        let mut design = read_file(json_path)?;
        design.update_version();
        design.remove_empty_domains();
        for s in design.strands.values_mut() {
            s.read_junctions(&mut xover_ids, true);
        }
        for s in design.strands.values_mut() {
            s.read_junctions(&mut xover_ids, false);
        }
        let mut grid_manager = GridManager::new_from_design(&design);
        let mut grids = grid_manager.grids2d();
        for g in grids.iter_mut() {
            g.write().unwrap().update(&design);
        }
        grid_manager.update(&mut design);
        let color_idx = design.strands.keys().len();
        let groups = design.groups.clone();
        let anchors = design.anchors.clone();
        let file_name = real_name(json_path);

        let mut ret = Self {
            design,
            file_name,
            last_backup_time: None,
            object_type: HashMap::default(),
            space_position: HashMap::default(),
            identifier_nucl: HashMap::default(),
            identifier_bound: HashMap::default(),
            nucleotides_involved: HashMap::default(),
            nucleotide: HashMap::default(),
            strand_map: HashMap::default(),
            helix_map: HashMap::default(),
            color: HashMap::default(),
            update_status: false,
            // false because we call make_hash_maps here
            hash_maps_update: false,
            basis_map: Default::default(),
            grid_manager,
            grids,
            color_idx,
            view_need_reset: false,
            groups: Arc::new(RwLock::new(groups)),
            red_cubes: HashMap::default(),
            blue_cubes: HashMap::default(),
            blue_nucl: vec![],
            roller_ptrs: None,
            hyperboloid_helices: vec![],
            hyperboloid_draft: None,
            template_manager: Default::default(),
            xover_copy_manager: Default::default(),
            rigid_body_ptr: None,
            helix_simulation_ptr: None,
            rigid_helix_simulator: None,
            anchors,
            elements_update: None,
            visible: Default::default(),
            visibility_sieve: None,
            xover_ids,
            prime3_set: Default::default(),
        };
        ret.make_hash_maps();
        ret.terminate_movement();
        Ok(ret)
    }
}

/// Create a design by parsing a file
use super::cadnano::FromCadnano;
fn read_file<P: AsRef<Path> + std::fmt::Debug>(path: P) -> Result<Design, ParseDesignError> {
    let json_str =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("File not found {:?}", path));

    let design: Result<Design, _> = serde_json::from_str(&json_str);
    // First try to read icednano format
    if let Ok(design) = design {
        println!("ok icednano");
        Ok(design)
    } else {
        // If the file is not in icednano format, try the other supported format
        let cdn_design: Result<codenano::Design<(), ()>, _> = serde_json::from_str(&json_str);

        let scadnano_design: Result<scadnano::ScadnanoDesign, _> = serde_json::from_str(&json_str);

        // Try codenano format
        if let Ok(scadnano) = scadnano_design {
            Design::from_scadnano(&scadnano).map_err(|e| ParseDesignError::ScadnanoError(e))
        } else if let Ok(design) = cdn_design {
            println!("{:?}", scadnano_design.err());
            println!("ok codenano");
            Ok(Design::from_codenano(&design))
        } else if let Ok(cadnano) = Cadnano::from_file(path) {
            println!("ok cadnano");
            Ok(Design::from_cadnano(cadnano))
        } else {
            // The file is not in any supported format
            //message("Unrecognized file format".into(), rfd::MessageLevel::Error);
            Err(ParseDesignError::UnrecognizedFileFormat)
        }
    }
}

use scadnano::ScadnanoImportError;
pub enum ParseDesignError {
    UnrecognizedFileFormat,
    ScadnanoError(ScadnanoImportError),
}

impl std::convert::From<ScadnanoImportError> for ParseDesignError {
    fn from(error: ScadnanoImportError) -> Self {
        Self::ScadnanoError(error)
    }
}
