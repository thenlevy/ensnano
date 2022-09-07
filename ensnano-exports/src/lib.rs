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
    NotImplemented,
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
    fn get_basis(&mut self, nucl: &Nucl, compl_a: char) -> char {
        if let Some(c) = self.map.and_then(|m| m.get(nucl)) {
            *c
        } else if let Some(c) = self.map.and_then(|m| m.get(&nucl.compl())) {
            compl(*c, compl_a)
        } else if let Some(c) = self.alternative.get(nucl) {
            *c
        } else {
            let base = rand_base();
            self.alternative.insert(nucl.clone(), base);
            self.alternative.insert(nucl.compl(), compl(base, compl_a));
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

fn compl(c: char, compl_a: char) -> char {
    match c {
        'A' => compl_a,
        'G' => 'C',
        'T' => 'A',
        'U' => 'A',
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

fn rand_pick(list: &[char]) -> char {
    let idx = rand::random::<usize>() % list.len();
    list[idx]
}

const CANNONICAL_BASES: &[char] = &['A', 'T', 'G', 'C', 'U'];

/// Perform a symbol conversion based on this [list](http://www.hgmd.cf.ac.uk/docs/nuc_lett.html)
fn rand_base_from_symbol(symbol: char, compl_a: char) -> char {
    match symbol {
        c if CANNONICAL_BASES.contains(&c) => c,
        'R' => rand_pick(&['G', 'A']),
        'Y' => rand_pick(&['C', compl_a]),
        'K' => rand_pick(&['G', compl_a]),
        'M' => rand_pick(&['A', 'C']),
        'S' => rand_pick(&['G', 'C']),
        'W' => rand_pick(&['A', compl_a]),
        'B' => rand_pick(&['G', 'C', compl_a]),
        'D' => rand_pick(&['G', 'A', compl_a]),
        'H' => rand_pick(&['C', 'A', compl_a]),
        'V' => rand_pick(&['G', 'C', 'A']),
        'N' => rand_pick(&['C', 'G', 'A', compl_a]),
        c => {
            println!("WARNING USING UNUSUAL SYMBOL {c}");
            rand_pick(&['C', 'G', 'A', compl_a])
        }
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
        ExportType::Pdb => {
            pdb::pdb_export(design, basis_mapper, export_path)?;
            Ok(ExportSuccess::Pdb(export_path.clone()))
        }
        ExportType::Cadnano => {
            let cadnano_content = cadnano::cadnano_export(design)?;
            let mut out_file = std::fs::File::create(export_path)?;
            use std::io::Write;
            writeln!(&mut out_file, "{cadnano_content}")?;
            Ok(ExportSuccess::Cadnano(export_path.clone()))
        }
        _ => Err(ExportError::NotImplemented),
    }
}

pub type ExportResult = Result<ExportSuccess, ExportError>;
