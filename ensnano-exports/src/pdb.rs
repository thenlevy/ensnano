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

use crate::BasisMapper;

use super::ultraviolet;
use super::PathBuf;
use crate::oxdna::{OxDnaHelix, OXDNA_LEN_FACTOR};
use ahash::AHashMap;
use ensnano_design::{Design, Domain, HelixCollection, Nucl};
use std::borrow::Cow;
use ultraviolet::{Rotor3, Vec3};

const MAX_ATOM_SERIAL_NUMBER: usize = 99_999;

#[derive(Debug, Clone)]
pub struct PdbNucleotide {
    residue_idx: usize,
    base_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    phosphate_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    sugar_atoms: AHashMap<Cow<'static, str>, PdbAtom>,
    name: Cow<'static, str>,
}

#[derive(Default, Clone, Debug)]
pub struct ReferenceNucleotides(AHashMap<Cow<'static, str>, ReferenceNucleotide>);

impl ReferenceNucleotides {
    fn present_candidate(&mut self, nucl: PdbNucleotide) -> Result<(), PdbError> {
        let mut candidate = ReferenceNucleotide::from_nucl(nucl)?;
        let rotation = candidate.frame.reversed();

        candidate.nucl = candidate
            .nucl
            .with_center_of_mass(Vec3::zero())?
            .rotated_by(rotation)?;

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
        let a3 = a1.cross(a2).normalized();

        let frame = ultraviolet::Mat3::new(a1, a2, a3).into_rotor3();
        Ok(Self { nucl, score, frame })
    }
}

const CANONICAL_BASE_NAMES: &[&str] = &["A", "T", "G", "C", "U"];

const LONG_BASE_NAMES: &[&str] = &["ADE", "CYT", "GUA", "THY"];

impl PdbNucleotide {
    fn new(name: Cow<'static, str>, residue_idx: usize) -> Self {
        let name: Cow<'static, str> = if CANONICAL_BASE_NAMES.contains(&name.as_ref()) {
            name
        } else if LONG_BASE_NAMES.contains(&name.as_ref()) {
            name[..1].to_string().into()
        } else {
            name[1..].to_string().into()
        };

        Self {
            residue_idx,
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
        } else if name_chars.contains(&'\'') || name_chars.contains(&'*') {
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
        let pairs = if self.name.find(['C', 'T', 'U']).is_some() {
            &[["N3", "C6"], ["C2", "N1"], ["C4", "C5"]]
        } else {
            &[["N1", "C4"], ["C2", "N3"], ["C6", "C5"]]
        };

        let mut ret = Vec3::zero();

        for pair in pairs {
            let p = self
                .base_atoms
                .get(pair[0])
                .ok_or_else(|| PdbError::MissingAtom(pair[0].to_string()))?;
            let q = self
                .base_atoms
                .get(pair[1])
                .ok_or_else(|| PdbError::MissingAtom(pair[1].to_string()))?;
            ret += p.position - q.position;
        }

        Ok(ret.normalized())
    }

    fn compute_a3(&self) -> Result<Vec3, PdbError> {
        let base_com = self.get_base_center_of_mass()?;

        let oxygen4 = self
            .sugar_atoms
            .get("O4'")
            .or_else(|| self.sugar_atoms.get("O4*"))
            .ok_or_else(|| PdbError::MissingAtom(String::from("O4'")))?;
        let parralel_to = oxygen4.position - base_com;

        let mut ret = Vec3::zero();

        let get_base_atom = |name: &str| {
            self.base_atoms
                .get(name)
                .ok_or_else(|| PdbError::MissingAtom(name.to_string()))
        };
        let ring_atom_names = ["C2", "C4", "C5", "C6", "N1", "N3"];

        use itertools::Itertools;
        for perm in ring_atom_names.iter().permutations(3) {
            let p = get_base_atom(perm[0])?;
            let q = get_base_atom(perm[1])?;
            let r = get_base_atom(perm[2])?;

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

    fn pdb_repr(
        &self,
        residue_type: ResidueType,
        nb_atom: &mut usize,
        chain_id: char,
    ) -> Result<String, PdbError> {
        let additional_hydrogen: Option<PdbAtom> = self.additional_hydrogen(residue_type)?;

        let mut lines = Vec::new();

        for a in self
            .phosphate_atoms
            .values()
            .chain(self.sugar_atoms.values())
            .chain(self.base_atoms.values())
            .chain(additional_hydrogen.iter())
        {
            let serial_number = (*nb_atom % MAX_ATOM_SERIAL_NUMBER) + 1;
            lines.push(
                a.format_with_paramters(
                    AtomFormatParamter {
                        serial_number,
                        chain_id,
                        residue_idx: self.residue_idx,
                    },
                    residue_type,
                )
                .map_err(PdbError::Formating)?,
            );
            *nb_atom += 1;
        }

        Ok(lines.join("\n"))
    }

    fn additional_hydrogen(&self, residue_type: ResidueType) -> Result<Option<PdbAtom>, PdbError> {
        let get_phosphate_atom = |name: &str| {
            self.phosphate_atoms
                .get(name)
                .ok_or_else(|| PdbError::MissingAtom(name.to_string()))
        };
        let get_sugar_atom = |name: &str| {
            self.sugar_atoms
                .get(name)
                .ok_or_else(|| PdbError::MissingAtom(name.to_string()))
        };
        match residue_type {
            ResidueType::Prime5 => {
                let phosphorus = get_phosphate_atom("P")?;
                let oxygen_5prime = get_sugar_atom("O5'").or_else(|_| get_sugar_atom("O5*"))?;

                let mut ret = phosphorus.clone();
                ret.name = "HO5'".into();

                let p_o_normalized = (phosphorus.position - oxygen_5prime.position).normalized();
                ret.position = oxygen_5prime.position + p_o_normalized;
                Ok(Some(ret))
            }
            ResidueType::Prime3 => {
                let oxygen_3prime = get_sugar_atom("O3'").or_else(|_| get_sugar_atom("O3*"))?;

                let mut ret = oxygen_3prime.clone();
                ret.name = "HO3'".into();

                let a1 = self.compute_a1()?;
                let a3 = self.compute_a3()?;
                let mut a2 = a3.cross(a1);
                a2.normalize();
                let a3 = a1.cross(a2);
                let oh = (0.2 * a2 - 0.2 * a1 + a3).normalized();

                ret.position = oxygen_3prime.position + oh;
                Ok(Some(ret))
            }
            ResidueType::Middle => Ok(None),
        }
    }

    fn with_center_of_mass(mut self, new_com: Vec3) -> Result<Self, PdbError> {
        let old_com = self.get_center_of_mass()?;

        for a in self
            .phosphate_atoms
            .values_mut()
            .chain(self.sugar_atoms.values_mut())
            .chain(self.base_atoms.values_mut())
        {
            a.position += new_com - old_com;
        }

        Ok(self)
    }

    fn translated_by(mut self, translation: Vec3) -> Self {
        for a in self
            .phosphate_atoms
            .values_mut()
            .chain(self.sugar_atoms.values_mut())
            .chain(self.base_atoms.values_mut())
        {
            a.position += translation;
        }

        self
    }

    fn rotated_by(mut self, rotation: Rotor3) -> Result<Self, PdbError> {
        let com = self.get_center_of_mass()?;

        for a in self
            .phosphate_atoms
            .values_mut()
            .chain(self.sugar_atoms.values_mut())
            .chain(self.base_atoms.values_mut())
        {
            a.position -= com;
            a.position.rotate_by(rotation);
            a.position += com;
        }

        Ok(self)
    }

    fn with_residue_idx(self, residue_idx: usize) -> Self {
        Self {
            residue_idx,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ResidueType {
    Prime5,
    Prime3,
    Middle,
}

impl ResidueType {
    fn suffix(&self) -> String {
        match self {
            Self::Prime5 => "5",
            Self::Prime3 => "3",
            Self::Middle => "",
        }
        .into()
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

const DNA_MIN_LINE_LENGTH: usize = 77;
pub fn make_reference_nucleotides() -> Result<ReferenceNucleotides, PdbError> {
    let pdb_content = include_str!("../dd12_na.pdb");
    read_pdb_string(pdb_content, DNA_MIN_LINE_LENGTH)
}

const RNA_MIN_LINE_LENGTH: usize = 66;
pub fn make_reference_nucleotides_rna() -> Result<ReferenceNucleotides, PdbError> {
    let pdb_content = include_str!("../ds_rna_Helix.pdb");
    read_pdb_string(pdb_content, RNA_MIN_LINE_LENGTH)
}

fn read_pdb_string(
    pdb_content: &str,
    min_line_length: usize,
) -> Result<ReferenceNucleotides, PdbError> {
    // Method taken from https://github.com/lorenzo-rovigatti/tacoxDNA
    let mut ret = ReferenceNucleotides::default();
    let mut current_residue: Cow<'static, str> = "".into();
    let mut current_nucl: Option<PdbNucleotide> = None;
    for (line_number, line) in pdb_content.lines().enumerate() {
        if line.len() >= min_line_length {
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
    println!("{:#?}", ret);
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
    Formating(std::fmt::Error),
    IOError(std::io::Error),
}

const OCCUPENCY: f32 = 1.0;
const TEMPERATURE_FACTOR: f32 = 1.0;

struct AtomFormatParamter {
    serial_number: usize,
    residue_idx: usize,
    chain_id: char,
}

impl PdbAtom {
    fn format_with_paramters(
        &self,
        parameters: AtomFormatParamter,
        residue_type: ResidueType,
    ) -> Result<String, std::fmt::Error> {
        let copy = Self {
            serial_number: parameters.serial_number,
            residue_idx: parameters.residue_idx,
            chain_id: parameters.chain_id,
            ..self.clone()
        };
        copy.pdb_repr(residue_type)
    }

    fn pdb_repr(&self, residue_type: ResidueType) -> Result<String, std::fmt::Error> {
        // https://www.cgl.ucsf.edu/chimera/docs/UsersGuide/tutorials/framepdbintro.html
        use std::fmt::Write;
        let mut ret = String::with_capacity(80);
        write!(&mut ret, "ATOM")?; // 1-4
        ret.push_str("  "); // 5-6
        write!(&mut ret, "{:>5}", self.serial_number)?; // 7-11
        ret.push(' '); //12
        if self.name.len() < 4 {
            // we assume that all atoms that we manipulate have a one letter symbol which is
            // conveniently the case for all atoms of nucleic acids
            write!(&mut ret, " {:<3}", self.name)?; //13-16
        } else {
            write!(&mut ret, "{:<4}", self.name)?; //13-16
        }
        ret.push(' '); // 17
        write!(
            &mut ret,
            "{:>3}",
            self.residue_name.to_string() + &residue_type.suffix()
        )?; // 18-20
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

use std::fs::File;
use std::mem::ManuallyDrop;
pub struct PdbFormatter {
    out_file: File,
    current_strand_id: usize,
    nb_atom: usize,
    reference: ReferenceNucleotides,
}

pub struct PdbStrand<'a> {
    pdb_formater: ManuallyDrop<&'a mut PdbFormatter>,
    nucleotides: Vec<PdbNucleotide>,
    cyclic: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum NucleicAcidKind {
    Dna,
    Rna,
}

impl NucleicAcidKind {
    pub fn compl_to_a(&self) -> char {
        match self {
            Self::Dna => 'T',
            Self::Rna => 'U',
        }
    }
}

use std::path::Path;
impl PdbFormatter {
    pub fn new<P: AsRef<Path>>(path: P, nu_kind: NucleicAcidKind) -> Result<Self, PdbError> {
        let out_file = std::fs::File::create(path).map_err(PdbError::IOError)?;

        let reference = match nu_kind {
            NucleicAcidKind::Dna => make_reference_nucleotides()?,
            NucleicAcidKind::Rna => make_reference_nucleotides_rna()?,
        };

        Ok(Self {
            out_file,
            current_strand_id: 0,
            nb_atom: 0,
            reference,
        })
    }

    /// Create a new strand. The returned value must be droped with `PdbStrand::write`.
    #[allow(clippy::needless_lifetimes)]
    pub fn start_strand<'a>(&'a mut self, cyclic: bool) -> PdbStrand<'a> {
        PdbStrand {
            pdb_formater: ManuallyDrop::new(self),
            nucleotides: Vec::new(),
            cyclic,
        }
    }
}

impl PdbStrand<'_> {
    pub fn add_nucl(
        &mut self,
        base: char,
        position: Vec3,
        orientation: Rotor3,
    ) -> Result<(), PdbError> {
        let nucl = self
            .pdb_formater
            .reference
            .get_nucl(&base.to_string())
            .or_else(|| self.pdb_formater.reference.get_nucl("A"))
            .ok_or_else(|| PdbError::MissingAtom("A".to_string()))?
            .clone()
            .with_residue_idx(self.nucleotides.len() + 1)
            .translated_by(position)
            .rotated_by(orientation)?;
        self.nucleotides.push(nucl);
        Ok(())
    }

    pub fn write(self) -> Result<(), PdbError> {
        let mut nucls_strs = Vec::with_capacity(self.nucleotides.len());

        let mut pdb_formatter = ManuallyDrop::into_inner(self.pdb_formater);

        let chain_id = ((pdb_formatter.current_strand_id % 26) as u8 + b'A') as char;

        let nb_nucl = self.nucleotides.len();
        for (i, n) in self.nucleotides.into_iter().enumerate() {
            let residue_type = if self.cyclic {
                ResidueType::Middle
            } else if i == 0 {
                ResidueType::Prime5
            } else if i == nb_nucl - 1 {
                ResidueType::Prime3
            } else {
                ResidueType::Middle
            };
            nucls_strs.push(n.pdb_repr(residue_type, &mut pdb_formatter.nb_atom, chain_id)?);
        }

        // TODO should we put this when the strand is cyclic ?
        if !self.cyclic {
            nucls_strs.push(String::from("TER"));
        }

        let to_write = nucls_strs.join("\n");

        use std::io::Write;
        writeln!(&mut pdb_formatter.out_file, "{to_write}").map_err(PdbError::IOError)?;

        pdb_formatter.current_strand_id += 1;
        Ok(())
    }
}

pub(super) fn pdb_export(
    design: &Design,
    mut basis_map: BasisMapper,
    out_path: &PathBuf,
) -> Result<(), PdbError> {
    let parameters = design.parameters.unwrap_or_default();
    let na_kind = if parameters.name().name.contains("RNA") {
        NucleicAcidKind::Rna
    } else {
        NucleicAcidKind::Dna
    };
    let mut exporter = PdbFormatter::new(out_path, na_kind)?;
    let mut previous_position = None;

    for s in design.strands.values() {
        let mut pdb_strand = exporter.start_strand(s.cyclic);

        for d in s.domains.iter() {
            if let Domain::HelixDomain(dom) = d {
                for position in dom.iter() {
                    let ox_nucl = design.helices.get(&dom.helix).unwrap().ox_dna_nucl(
                        position,
                        dom.forward,
                        &parameters,
                    );
                    let nucl = Nucl {
                        position,
                        helix: dom.helix,
                        forward: dom.forward,
                    };
                    previous_position = Some(ox_nucl.position);
                    let symbol = basis_map.get_basis(&nucl, na_kind.compl_to_a());
                    let base = super::rand_base_from_symbol(symbol, na_kind.compl_to_a());
                    pdb_strand.add_nucl(
                        base,
                        ox_nucl.position * 10. / OXDNA_LEN_FACTOR,
                        ox_nucl.get_basis(),
                    )?;
                }
            } else if let Domain::Insertion {
                instanciation: Some(instanciation),
                ..
            } = d
            {
                for (insertion_idx, position) in instanciation.pos().iter().enumerate() {
                    let ox_nucl = crate::oxdna::free_oxdna_nucl(
                        *position,
                        previous_position,
                        insertion_idx,
                        &parameters,
                    );
                    previous_position = Some(*position);
                    pdb_strand.add_nucl(
                        na_kind.compl_to_a(),
                        ox_nucl.position * 10.,
                        ox_nucl.get_basis(),
                    )?;
                }
            }
        }
        pdb_strand.write()?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pdb_repr() {
        let expected_prime5 =
            "ATOM      1  N9  DG5 A   1      55.550  70.279 208.461  1.00  1.00              ";
        let expected_prime3 =
            "ATOM      1  N9  DG3 A   1      55.550  70.279 208.461  1.00  1.00              ";
        let expected_middle =
            "ATOM      1  N9   DG A   1      55.550  70.279 208.461  1.00  1.00              ";

        let atom = PdbAtom {
            serial_number: 1,
            name: "N9".into(),
            residue_name: "DG".into(),
            chain_id: 'A',
            position: Vec3::new(55.550, 70.279, 208.461),
            residue_idx: 1,
        };
        assert_eq!(atom.pdb_repr(ResidueType::Prime5).unwrap(), expected_prime5);
        assert_eq!(atom.pdb_repr(ResidueType::Prime3).unwrap(), expected_prime3);
        assert_eq!(atom.pdb_repr(ResidueType::Middle).unwrap(), expected_middle);
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
