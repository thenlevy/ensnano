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

#[derive(Debug, Clone)]
pub struct PdbNucleotide {
    chain_idx: usize,
    base_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    phosphate_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    sugar_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    name: Cow<'static, str>,
}

#[derive(Default, Clone, Debug)]
pub struct ReferenceNucleotides(AHashMap<Cow<'static, str>, ReferenceNucleotide>);

impl ReferenceNucleotides {
    fn present_candidate(&mut self, nucl: PdbNucleotide) -> Result<(), PdbError> {
        let candidate = ReferenceNucleotide::from_nucl(nucl)?;

        if let Some(current) = self.0.get_mut(&candidate.nucl.name) {
            if current.score < candidate.score {
                *current = candidate;
            }
        } else {
            self.0.insert(candidate.nucl.name.clone(), candidate);
        }

        Ok(())
    }

    pub fn get_nucl(&self, name: &str) -> Option<&PdbNucleotide> {
        self.0.get(&name[..1]).map(|n| &n.nucl)
    }
}

#[derive(Clone, Debug)]
struct ReferenceNucleotide {
    nucl: PdbNucleotide,
    score: f32,
    frame: Rotor3,
}

impl ReferenceNucleotide {
    fn from_nucl(nucl: PdbNucleotide) -> Result<Self, PdbError> {
        let a1 = nucl.compute_a1()?;
        let a3 = nucl.compute_a3()?;
        let mut a2 = a3.cross(a1);
        let score = a2.mag();
        a2.normalize();
        let a3 = a1.cross(a2);

        let frame = ultraviolet::Mat3::new(a1, a2, a3).into_rotor3();
        Ok(Self { nucl, score, frame })
    }
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

    fn get_center_of_mass(&self) -> Result<Vec3, PdbError> {
        let mut ret = Vec3::zero();

        let nb_atoms = self.sugar_atoms.len() + self.base_atoms.len() + self.phosphate_atoms.len();
        if nb_atoms == 0 {
            return Err(PdbError::EmptyNucleotide);
        }

        for a in self.sugar_atoms.values() {
            ret += a.position
        }
        for a in self.phosphate_atoms.values() {
            ret += a.position
        }
        for a in self.base_atoms.values() {
            ret += a.position
        }

        Ok(ret / (nb_atoms as f32))
    }

    fn get_base_center_of_mass(&self) -> Result<Vec3, PdbError> {
        let mut ret = Vec3::zero();

        let nb_atoms = self.base_atoms.len();
        if nb_atoms == 0 {
            return Err(PdbError::EmptyBase);
        }

        for a in self.base_atoms.values() {
            ret += a.position
        }

        Ok(ret / (nb_atoms as f32))
    }

    fn compute_a1(&self) -> Result<Vec3, PdbError> {
        let pairs = if self.name.find(&['C', 'T']).is_some() {
            &[["N3", "C6"], ["C2", "N1"], ["C4", "C5"]]
        } else {
            &[["N1", "C4"], ["C2", "N3"], ["C6", "C5"]]
        };

        let mut ret = Vec3::zero();

        for pair in pairs {
            let p = self
                .base_atoms
                .get(pair[0])
                .ok_or(PdbError::MissingAtom(pair[0].to_string()))?;
            let q = self
                .base_atoms
                .get(pair[1])
                .ok_or(PdbError::MissingAtom(pair[1].to_string()))?;
            ret += p.position - q.position;
        }

        Ok(ret.normalized())
    }

    fn compute_a3(&self) -> Result<Vec3, PdbError> {
        let base_com = self.get_base_center_of_mass()?;

        let oxygen4 = self
            .sugar_atoms
            .get("O4'")
            .ok_or(PdbError::MissingAtom(String::from("O4'")))?;
        let parralel_to = oxygen4.position - base_com;

        let mut ret = Vec3::zero();

        let get_base_atom = |name: &str| {
            self.base_atoms
                .get(name)
                .ok_or(PdbError::MissingAtom(name.to_string()))
        };
        let ring_atom_names = ["C2", "C4", "C5", "C6", "N1", "N3"];

        use itertools::Itertools;
        for perm in ring_atom_names.iter().permutations(3) {
            let p = get_base_atom(&perm[0])?;
            let q = get_base_atom(&perm[1])?;
            let r = get_base_atom(&perm[2])?;

            let v1 = (p.position - q.position).normalized();
            let v2 = (p.position - r.position).normalized();

            if v1.dot(v2).abs() > 0.01 {
                let mut a3 = v1.cross(v2).normalized();
                a3 *= a3.dot(parralel_to).signum();
                ret += a3;
            }
        }

        Ok(ret.normalized())
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

pub fn make_reference_nucleotides() -> Result<ReferenceNucleotides, PdbError> {
    // Method taken from https://github.com/lorenzo-rovigatti/tacoxDNA
    let pdb_string = include_str!("../dd12_na.pdb");
    let mut ret = ReferenceNucleotides::default();
    let mut current_residue: Cow<'static, str> = "".into();
    let mut current_nucl: Option<PdbNucleotide> = None;
    for (line_number, line) in pdb_string.lines().enumerate() {
        if line.len() >= 77 {
            let atom = PdbAtom::parse_line(&line)
                .map_err(|error| PdbError::ParsingError { line_number, error })?;

            if atom.residue_name != current_residue {
                if let Some(nucl) = current_nucl.take() {
                    ret.present_candidate(nucl)?;
                }
                current_residue = atom.residue_name.clone();
            }

            current_nucl
                .get_or_insert_with(|| {
                    PdbNucleotide::new(atom.residue_name.clone(), atom.residue_idx)
                })
                .add_atom(atom);
        }
    }
    if let Some(nucl) = current_nucl.take() {
        ret.present_candidate(nucl)?;
    }
    Ok(ret)
}

#[derive(Debug)]
pub enum PdbError {
    ParsingError {
        line_number: usize,
        error: PdbAtomParseError,
    },
    EmptyNucleotide,
    EmptyBase,
    MissingAtom(String),
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
        let references = make_reference_nucleotides().unwrap();

        let names = [
            "A5", "T5", "G5", "C5", "A", "T", "G", "C", "A3", "T3", "C3", "G3",
        ];

        for name in names {
            let _ = references.get_nucl(name).expect(name);
        }
    }
}
