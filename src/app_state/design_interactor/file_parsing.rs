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
use crate::controller::LoadDesignError;
use crate::utils::id_generator::IdGenerator;
use ensnano_design::{codenano, scadnano, Nucl};
use std::path::{Path, PathBuf};

mod cadnano;
mod junctions;
pub(super) use junctions::StrandJunction;

impl DesignInteractor {
    /// Create a new data by reading a file. At the moment, the supported format are
    /// * codenano
    /// * icednano
    pub fn new_with_path(json_path: &PathBuf) -> Result<Self, LoadDesignError> {
        let mut xover_ids: IdGenerator<(Nucl, Nucl)> = Default::default();
        let mut design = read_file(json_path)?;
        design.strands.remove_empty_domains();
        for s in design.strands.values_mut() {
            s.read_junctions(&mut xover_ids, true);
        }
        for s in design.strands.values_mut() {
            s.read_junctions(&mut xover_ids, false);
        }
        //let file_name = real_name(json_path);
        let suggestion_parameters = SuggestionParameters::default();
        let (presenter, design_ptr) =
            Presenter::from_new_design(design, &xover_ids, suggestion_parameters);
        let ret = Self {
            design: design_ptr,
            presenter: AddressPointer::new(presenter),
            ..Default::default()
        };
        Ok(ret)
    }
}

/// Create a design by parsing a file
use cadnano::{Cadnano, FromCadnano};
fn read_file<P: AsRef<Path> + std::fmt::Debug>(path: P) -> Result<Design, LoadDesignError> {
    let json_str =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("File not found {:?}", path));

    let design: Result<Design, _> = serde_json::from_str(&json_str);
    // First try to read icednano format
    match design {
        Ok(mut design) => {
            design.update_version();
            use version_compare::Cmp;
            log::info!("ok icednano");
            let required_version = design.ensnano_version.clone();
            let current_version = ensnano_design::ensnano_version();
            match version_compare::compare(&required_version, &current_version) {
                Ok(Cmp::Lt) | Ok(Cmp::Eq) => Ok(design),
                _ => Err(LoadDesignError::IncompatibleVersion {
                    current: current_version,
                    required: required_version,
                }),
            }
        }
        Err(e) => {
            // If the file is not in icednano format, try the other supported format
            let cdn_design: Result<codenano::Design<(), ()>, _> = serde_json::from_str(&json_str);

            let scadnano_design: Result<scadnano::ScadnanoDesign, _> =
                serde_json::from_str(&json_str);

            // Try codenano format
            if let Ok(scadnano) = scadnano_design {
                Design::from_scadnano(&scadnano)
                    .map_err(|e| LoadDesignError::ScadnanoImportError(e))
            } else if let Ok(design) = cdn_design {
                log::error!("{:?}", scadnano_design.err());
                log::info!("ok codenano");
                Ok(Design::from_codenano(&design))
            } else if let Ok(cadnano) = Cadnano::from_file(path) {
                log::info!("ok cadnano");
                Ok(Design::from_cadnano(cadnano))
            } else {
                log::error!("{:?}", e);
                // The file is not in any supported format
                //message("Unrecognized file format".into(), rfd::MessageLevel::Error);
                Err(LoadDesignError::JsonError(e))
            }
        }
    }
}

use scadnano::ScadnanoImportError;

impl std::convert::From<ScadnanoImportError> for LoadDesignError {
    fn from(error: ScadnanoImportError) -> Self {
        Self::ScadnanoImportError(error)
    }
}

#[cfg(test)]
mod tests {
    use ensnano_design::HelixCollection;

    fn one_helix_path() -> PathBuf {
        let mut ret = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
        ret.push("tests");
        ret.push("one_helix.json");
        ret
    }

    use super::*;

    #[test]
    fn parse_one_helix() {
        let path = one_helix_path();
        let interactor = DesignInteractor::new_with_path(&path).ok().unwrap();
        let design = interactor.design.as_ref();
        assert_eq!(design.helices.len(), 1);
    }
}
