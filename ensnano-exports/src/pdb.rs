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

//! Export to pdb file format. The method used here is an adpatation from the one used in
//! [tacOxDNA](https://github.com/lorenzo-rovigatti/tacoxDNA)

use super::ultraviolet;
use ahash::AHashMap;
use std::borrow::Cow;
use ultraviolet::{Rotor3, Vec3};

pub struct PdbNucleotide {
    chain_idx: usize,
    base_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    phosphate_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    sugar_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    name: Cow<'static, str>,
}

const CANONICAL_BASE_NAMES: &[&str] = &["A", "T", "G", "C"];

const LONG_BASE_NAMES: &[&str] = &["ADE", "CYT", "GUA", "THY"];

impl PdbNucleotide {
    fn new(name: Cow<'static, str>, chain_idx: usize) -> Self {
        let name: Cow<'static, str> = if CANONICAL_BASE_NAMES.contains(&name.as_ref()) {
            name
        } else if LONG_BASE_NAMES.contains(&name.as_ref()) {
            name[..1].to_string().into()
        } else {
            name[1..].to_string().into()
        };

        Self {
            chain_idx,
            base_atoms: Default::default(),
            phosphate_atoms: Default::default(),
            sugar_atoms: Default::default(),
            name,
        }
    }

    fn add_atom(&mut self, atom: PdbAtom) {
        let name_chars: Vec<char> = atom.name.chars().collect();
        if name_chars.contains(&'P') || atom.name == "HO5'" {
            self.phosphate_atoms.insert(atom.name.clone(), atom);
        } else if name_chars.contains(&'\'') {
            self.sugar_atoms.insert(atom.name.clone(), atom);
        } else {
            self.base_atoms.insert(atom.name.clone(), atom);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PdbAtom {
    serial_number: usize,
    name: Cow<'static, str>,
    residue_name: Cow<'static, str>,
    chain_id: char,
    residue_idx: usize,
    position: Vec3,
}

pub fn make_reference_nucleotides() -> Result<AHashMap<Cow<'static, str>, PdbNucleotide>, PdbError>
{
    /*
    with open(os.path.join(os.path.dirname(__file__), DD12_PDB_PATH)) as f:
        nucleotides = []
        old_residue = ""
        for line in f.readlines():
            if len(line) > 77:
                na = Atom(line)
                if na.residue_idx != old_residue:
                    nn = Nucleotide(na.residue, na.residue_idx)
                    nucleotides.append(nn)
                    old_residue = na.residue_idx
                nn.add_atom(na)

    bases = {}
    for n in nucleotides:
        n.compute_as()
        if n.base in bases:
            if n.check < bases[n.base].check: bases[n.base] = copy.deepcopy(n)
        else:
            bases[n.base] = n

    for n in nucleotides:
        n.a1, n.a2, n.a3 = utils.get_orthonormalized_base(n.a1, n.a2, n.a3)
    */

    let pdb_string = include_str!("../dd12_na.pdb");
    let mut ret = AHashMap::new();
    let mut current_residue: Cow<'static, str> = "".into();
    let mut current_nucl: Option<PdbNucleotide> = None;
    for (line_number, line) in pdb_string.lines().enumerate() {
        if line.len() > 77 {
            let atom = PdbAtom::parse_line(&line)
                .map_err(|error| PdbError::ParsingError { line_number, error })?;

            if atom.residue_name != current_residue {
                if let Some(nucl) = current_nucl.take() {
                    ret.insert(current_residue.clone(), nucl);
                }
                current_residue = atom.residue_name.clone();
                //TODO get the atom with the best orthogonal basis
            }

            current_nucl
                .get_or_insert_with(|| {
                    PdbNucleotide::new(atom.residue_name.clone(), atom.residue_idx)
                })
                .add_atom(atom);
        }
    }
    Ok(ret)
}

pub enum PdbError {
    ParsingError {
        line_number: usize,
        error: PdbAtomParseError,
    },
}

const OCCUPENCY: f32 = 1.0;
const TEMPERATURE_FACTOR: f32 = 1.0;

impl PdbAtom {
    fn pdb_repr(&self) -> Result<String, std::fmt::Error> {
        // https://www.cgl.ucsf.edu/chimera/docs/UsersGuide/tutorials/framepdbintro.html
        use std::fmt::Write;
        let mut ret = String::with_capacity(80);
        write!(&mut ret, "ATOM")?; // 1-4
        ret.push_str("  "); // 5-6
        write!(&mut ret, "{:>5}", self.serial_number)?; // 7-11
        ret.push_str(" "); //12
        if self.name.len() < 4 {
            // we assume that all atoms that we manipulate have a one letter symbol which is
            // conveniently the case for all atoms of nucleic acids
            write!(&mut ret, " {:<3}", self.name)?; //13-16
        } else {
            write!(&mut ret, "{:<4}", self.name)?; //13-16
        }
        ret.push_str(" "); // 17
        write!(&mut ret, "{:>3}", self.residue_name)?; // 18-20
        write!(&mut ret, " {}", self.chain_id)?; //21-22
        write!(&mut ret, "{:>4}", self.residue_idx)?; // 23-26
        ret.push_str(&vec![" "; 4].join("")); // 27-30
        write!(&mut ret, "{:>8.3}", self.position.x)?; // 31-38
        write!(&mut ret, "{:>8.3}", self.position.y)?; // 39-46
        write!(&mut ret, "{:>8.3}", self.position.z)?; // 47-54
        write!(&mut ret, "{:>6.2}", OCCUPENCY)?; // 55-60
        write!(&mut ret, "{:>6.2}", TEMPERATURE_FACTOR)?; // 61-66
        ret.push_str(&vec![" "; 14].join("")); // 67-80
        Ok(ret)
    }

    fn parse_line<S: AsRef<str>>(input: &S) -> Result<Self, PdbAtomParseError> {
        let input: &str = input.as_ref();
        if !input.is_ascii() {
            return Err(PdbAtomParseError::InputIsNotAscii);
        }

        if input.len() < 66 {
            return Err(PdbAtomParseError::InputTooShort);
        }

        if &input[0..4] != "ATOM" {
            return Err(PdbAtomParseError::NotAnAtom);
        }

        let serial_number = input[6..11]
            .trim()
            .parse::<usize>()
            .map_err(|_| PdbAtomParseError::InvalidSerialNumber)?;
        let name = input[12..16].trim().to_string();
        let residue_name = input[17..20].trim().to_string();
        let chain_id: char = input
            .chars()
            .nth(21)
            .ok_or(PdbAtomParseError::InputTooShort)?;
        let residue_idx = input[22..26]
            .trim()
            .parse::<usize>()
            .map_err(|_| PdbAtomParseError::InvalidResidueSequenceNumber)?;

        let position_x = input[30..38]
            .trim()
            .parse::<f32>()
            .map_err(|_| PdbAtomParseError::InvalidCoordinateX)?;
        let position_y = input[38..46]
            .trim()
            .parse::<f32>()
            .map_err(|_| PdbAtomParseError::InvalidCoordinateY)?;
        let position_z = input[46..54]
            .trim()
            .parse::<f32>()
            .map_err(|_| PdbAtomParseError::InvalidCoordinateZ)?;

        Ok(Self {
            serial_number,
            name: name.into(),
            residue_idx,
            chain_id,
            residue_name: residue_name.into(),
            position: Vec3::new(position_x, position_y, position_z),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PdbAtomParseError {
    InputIsNotAscii,
    InputTooShort,
    NotAnAtom,
    InvalidSerialNumber,
    InvalidResidueSequenceNumber,
    InvalidCoordinateX,
    InvalidCoordinateY,
    InvalidCoordinateZ,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pdb_repr() {
        let expected =
            "ATOM      1  N9  DG5 A   1      55.550  70.279 208.461  1.00  1.00              ";

        let atom = PdbAtom {
            serial_number: 1,
            name: "N9".into(),
            residue_name: "DG5".into(),
            chain_id: 'A',
            position: Vec3::new(55.550, 70.279, 208.461),
            residue_idx: 1,
        };
        assert_eq!(atom.pdb_repr().unwrap(), expected);
    }

    #[test]
    fn parse_atom() {
        let atom = PdbAtom {
            serial_number: 1,
            name: "N9".into(),
            residue_name: "DG5".into(),
            chain_id: 'A',
            position: Vec3::new(55.550, 70.279, 208.461),
            residue_idx: 1,
        };
        let input =
            "ATOM      1  N9  DG5 A   1      55.550  70.279 208.461  1.00  1.00              ";

        let parsed_atom = PdbAtom::parse_line(&input).unwrap();
        assert_eq!(parsed_atom, atom);
    }

    #[test]
    fn can_make_reference_collection() {
        assert!(make_reference_nucleotides().is_ok())
    }
}
