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
//! Exports utilities from ENSnano to other file formats used in DNA nanotechnologies

use ahash::AHashMap;
use strum::Display;

pub mod cadnano;
pub mod cando;
pub mod oxdna;
pub mod pdb;
use cadnano::CadnanoError;
use cando::CanDoError;
use ensnano_design::{ultraviolet, Design, Nucl};
use pdb::PdbError;
use std::collections::HashMap;
use std::path::PathBuf;

/// The file formats to which an export is implemented
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum ExportType {
    Cadnano,
    Cando,
    Pdb,
    Oxdna,
}

/// A value returned by the export functions when exports was successfull.
///
/// This means that both the format conversion and the write to the output file were successful.
pub enum ExportSuccess {
    Cadnano(PathBuf),
    Cando(PathBuf),
    Pdb(PathBuf),
    Oxdna {
        topology: PathBuf,
        configuration: PathBuf,
    },
}

const SUCCESSFUL_EXPORT_MSG_PREFIX: &str = "Succussfully exported to";

impl ExportSuccess {
    /// A message telling that the export operation was successfull and giving the path to which
    /// the export was made
    pub fn message(&self) -> String {
        match self {
            Self::Cadnano(p) => format!("{SUCCESSFUL_EXPORT_MSG_PREFIX}\n{}", p.to_string_lossy()),
            Self::Cando(p) => format!("{SUCCESSFUL_EXPORT_MSG_PREFIX}\n{}", p.to_string_lossy()),
            Self::Pdb(p) => format!("{SUCCESSFUL_EXPORT_MSG_PREFIX}\n{}", p.to_string_lossy()),
            Self::Oxdna {
                topology,
                configuration,
            } => format!(
                "{SUCCESSFUL_EXPORT_MSG_PREFIX}\n{}\n{}",
                configuration.to_string_lossy(),
                topology.to_string_lossy()
            ),
        }
    }
}

#[derive(Debug)]
pub enum ExportError {
    CadnanoConversion(CadnanoError),
    CandoConversion(CanDoError),
    PdbConversion(PdbError),
    IOError(std::io::Error),
}

impl From<CadnanoError> for ExportError {
    fn from(e: CadnanoError) -> Self {
        Self::CadnanoConversion(e)
    }
}
impl From<CanDoError> for ExportError {
    fn from(e: CanDoError) -> Self {
        Self::CandoConversion(e)
    }
}
impl From<PdbError> for ExportError {
    fn from(e: PdbError) -> Self {
        Self::PdbConversion(e)
    }
}
impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

/// A collection mapping nucleotide location to their basis
pub trait BasisMap {
    fn get(&self, nucl: &Nucl) -> Option<&char>;
}

impl BasisMap for HashMap<Nucl, char, ahash::RandomState> {
    fn get(&self, nucl: &Nucl) -> Option<&char> {
        self.get(nucl)
    }
}

struct BasisMapper<'a> {
    map: Option<&'a dyn BasisMap>,
    alternative: HashMap<Nucl, char>,
}

impl<'a> BasisMapper<'a> {
    fn get_basis(&mut self, nucl: &Nucl) -> char {
        if let Some(c) = self.map.and_then(|m| m.get(nucl)) {
            *c
        } else if let Some(c) = self.map.and_then(|m| m.get(&nucl.compl())) {
            compl(*c)
        } else {
            let base = rand_base();
            self.alternative.insert(nucl.clone(), base);
            self.alternative.insert(nucl.compl(), compl(base));
            base
        }
    }

    fn new(map: Option<&'a dyn BasisMap>) -> Self {
        Self {
            map,
            alternative: HashMap::new(),
        }
    }
}

fn compl(c: char) -> char {
    match c {
        'A' => 'T',
        'G' => 'C',
        'T' => 'A',
        _ => 'G',
    }
}

fn rand_base() -> char {
    match rand::random::<u8>() % 4 {
        0 => 'A',
        1 => 'T',
        2 => 'G',
        _ => 'C',
    }
}

pub fn export(
    design: &Design,
    export_type: ExportType,
    basis_map: Option<&dyn BasisMap>,
    export_path: &PathBuf,
) -> Result<ExportSuccess, ExportError> {
    let basis_mapper = BasisMapper::new(basis_map);
    match export_type {
        ExportType::Oxdna => {
            let configuration = export_path.clone();
            let mut topology = export_path.clone();
            topology.set_extension("top");
            let (config, topo) = oxdna::to_oxdna(design, basis_mapper);
            config.write(&configuration)?;
            topo.write(&topology)?;
            Ok(ExportSuccess::Oxdna {
                topology,
                configuration,
            })
        }
        _ => todo!(),
    }
}

pub type ExportResult = Result<ExportSuccess, ExportError>;
