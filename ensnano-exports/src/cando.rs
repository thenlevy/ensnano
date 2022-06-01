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

const FILE_HEADER: &str =
"\"CanDo (.cndo) file format version 1.0, Keyao Pan, Laboratory for Computational Biology and Biophysics, Massachusetts Institute of Technology, November 2015\"";

const DNATOP_HEADER: &str = "dnaTop,id,up,down,across,seq";

const DNODE_HEADER: &str = "dNode,\"e0(1)\",\"e0(2)\",\"e0(3)\"";

const TRIAD_HEADER: &str =
    r#"triad,"e1(1)","e1(2)","e1(3)","e2(1)","e2(2)","e2(3)","e3(1)","e3(2)","e3(3)"#;
const BP_LIST_HEADER: &str = "id_nt,id1,id2";

use super::ultraviolet::{Mat3, Vec3};
use ahash::AHashMap;
use ensnano_design::Nucl;
use std::path::Path;

struct DnaTopEntry {
    serial_number: usize,
    id: usize,
    prime5_id: Option<usize>,
    prime3_id: Option<usize>,
    paired_id: Option<usize>,
    seq: char,
}

impl DnaTopEntry {
    fn format(&self) -> String {
        vec![
            self.serial_number.to_string(),
            self.id.to_string(),
            self.prime5_id
                .map(|n| n.to_string())
                .unwrap_or("-1".to_string()),
            self.prime3_id
                .map(|n| n.to_string())
                .unwrap_or("-1".to_string()),
            self.paired_id
                .map(|n| n.to_string())
                .unwrap_or("-1".to_string()),
            self.seq.to_string(),
        ]
        .join(",")
    }
}

struct NodeEntry {
    id: usize,
    position: Vec3,
}

impl NodeEntry {
    fn format(&self) -> String {
        vec![
            self.id.to_string(),
            self.position.x.to_string(),
            self.position.y.to_string(),
            self.position.z.to_string(),
        ]
        .join(",")
    }
}

struct TriadEntry {
    id: usize,
    // e2 = base_pair, e3 = axis of the helix
    orientation: Mat3,
}

impl TriadEntry {
    fn format(&self) -> String {
        vec![
            self.id.to_string(),
            self.orientation[0].x.to_string(),
            self.orientation[0].y.to_string(),
            self.orientation[0].z.to_string(),
            self.orientation[1].x.to_string(),
            self.orientation[1].y.to_string(),
            self.orientation[1].z.to_string(),
            self.orientation[2].x.to_string(),
            self.orientation[2].y.to_string(),
            self.orientation[2].z.to_string(),
        ]
        .join(",")
    }
}

struct BpEntry {
    node_id: usize,
    nt_1: usize,
    nt_2: usize,
}

impl BpEntry {
    fn format(&self) -> String {
        vec![
            self.node_id.to_string(),
            self.nt_1.to_string(),
            self.nt_2.to_string(),
        ]
        .join(",")
    }
}

pub struct CanDoStrand<'a> {
    previous_nucl: Option<Nucl>,
    first_nucl: Option<Nucl>,
    formatter: &'a mut CanDoFormater,
}

#[derive(Debug, Clone)]
struct CanDoNucl {
    nucl: Nucl,
    position: Vec3,
    id: usize,
    normal: Vec3,
    prime5_id: Option<usize>,
    prime3_id: Option<usize>,
    paired_id: Option<usize>,
    basis: Option<char>,
}

impl CanDoNucl {
    fn make_pair_with(&self, paired: &CanDoNucl) -> Result<(Vec3, Mat3), CanDoError> {
        if self.nucl.compl() != paired.nucl {
            return Err(CanDoError::NotPaired(paired.nucl, self.nucl));
        }

        let position = (self.position + paired.position) / 2.;

        let mut e2 = (self.position - paired.position).normalized();

        if !self.nucl.forward {
            e2 *= -1.;
        }

        let e1 = (e2.cross(self.normal) + e2.cross(paired.normal)).normalized();

        let e3 = e1.cross(e2).normalized();

        let orientation = Mat3::new(e1, e2, e3);

        Ok((position, orientation))
    }

    fn default_basis(&self) -> char {
        let num = self.nucl.position + if self.nucl.forward { 2 } else { 0 };
        match num % 4 {
            0 => 'A',
            1 => 'C',
            2 => 'T',
            _ => 'G',
        }
    }
}

#[derive(Default)]
pub struct CanDoFormater {
    known_nucls: AHashMap<Nucl, CanDoNucl>,
    node_entries: Vec<NodeEntry>,
    triad_entries: Vec<TriadEntry>,
    bp_entries: Vec<BpEntry>,
}

impl CanDoFormater {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_strand<'a>(&'a mut self) -> CanDoStrand<'a> {
        CanDoStrand {
            previous_nucl: None,
            first_nucl: None,
            formatter: self,
        }
    }

    fn add_nucl(
        &mut self,
        nucl: Nucl,
        position: Vec3,
        normal: Vec3,
        basis: Option<char>,
    ) -> Result<(), CanDoError> {
        let id = self.known_nucls.len() + 1;

        let paired_id = self.known_nucls.get(&nucl.compl()).map(|n| n.id);

        let cando_nucl = CanDoNucl {
            nucl,
            position,
            id,
            normal,
            paired_id,
            prime3_id: None,
            prime5_id: None,
            basis,
        };

        if let Some(paired) = self.known_nucls.get_mut(&nucl.compl()) {
            let (bp_position, orientation) = paired.make_pair_with(&cando_nucl)?;

            paired.paired_id = Some(id);

            let bp_id = self.bp_entries.len() + 1;
            self.bp_entries.push(BpEntry {
                node_id: bp_id,
                nt_1: paired.id,
                nt_2: id,
            });
            self.triad_entries.push(TriadEntry {
                id: bp_id,
                orientation,
            });
            self.node_entries.push(NodeEntry {
                id: bp_id,
                position: bp_position,
            });
        }

        if self.known_nucls.insert(nucl, cando_nucl).is_some() {
            return Err(CanDoError::DuplicateNucleotide(nucl));
        }

        Ok(())
    }

    fn make_bound(&mut self, prime5_end: Nucl, prime3_end: Nucl) -> Result<(), CanDoError> {
        let prime5_id = self
            .known_nucls
            .get(&prime5_end)
            .map(|n| n.id)
            .ok_or(CanDoError::CannotFindNucl(prime5_end))?;
        let prime3_id = self
            .known_nucls
            .get(&prime3_end)
            .map(|n| n.id)
            .ok_or(CanDoError::CannotFindNucl(prime3_end))?;

        self.known_nucls
            .get_mut(&prime5_end)
            .ok_or(CanDoError::CannotFindNucl(prime5_end))?
            .prime3_id = Some(prime3_id);
        self.known_nucls
            .get_mut(&prime3_end)
            .ok_or(CanDoError::CannotFindNucl(prime3_end))?
            .prime5_id = Some(prime5_id);

        Ok(())
    }

    pub fn write_to<P: AsRef<Path>>(self, path: P) -> Result<(), std::io::Error> {
        let mut out_file = std::fs::File::create(path)?;
        use std::io::Write;

        writeln!(&mut out_file, "{FILE_HEADER}")?;

        writeln!(&mut out_file, "")?;

        writeln!(&mut out_file, "{DNATOP_HEADER}")?;

        let mut known_nucls = self.known_nucls.values().collect::<Vec<_>>();
        known_nucls.sort_by_key(|n| n.id);

        // For each nucl make topology entry and write
        let topologie: Vec<String> = known_nucls
            .into_iter()
            .enumerate()
            .map(|(id, nucl)| {
                DnaTopEntry {
                    seq: nucl.basis.unwrap_or_else(|| nucl.default_basis()),
                    serial_number: id + 1,
                    id: nucl.id,
                    prime5_id: nucl.prime5_id,
                    prime3_id: nucl.prime3_id,
                    paired_id: nucl.paired_id,
                }
                .format()
            })
            .collect();

        writeln!(&mut out_file, "{}\n", topologie.join("\n"))?;

        // TODO write self.node_entries, self.triad_entries and self.bp_entries
        writeln!(&mut out_file, "{DNODE_HEADER}")?;
        let node_str = self
            .node_entries
            .iter()
            .map(NodeEntry::format)
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(&mut out_file, "{node_str}\n")?;

        writeln!(&mut out_file, "{TRIAD_HEADER}")?;
        let triad_str = self
            .triad_entries
            .iter()
            .map(TriadEntry::format)
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(&mut out_file, "{triad_str}\n")?;

        writeln!(&mut out_file, "{BP_LIST_HEADER}")?;
        let bp_str = self
            .bp_entries
            .iter()
            .map(BpEntry::format)
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(&mut out_file, "{bp_str}")?;

        Ok(())
    }
}

impl CanDoStrand<'_> {
    pub fn add_nucl(
        &mut self,
        nucl: Nucl,
        position: Vec3,
        normal: Vec3,
        basis: Option<char>,
    ) -> Result<(), CanDoError> {
        self.formatter.add_nucl(nucl, position, normal, basis)?;

        if let Some(prime5) = self.previous_nucl.take() {
            self.formatter.make_bound(prime5, nucl)?;
        }

        self.previous_nucl = Some(nucl);
        self.first_nucl = self.first_nucl.or(Some(nucl));

        Ok(())
    }

    pub fn end(mut self, cyclic: bool) -> Result<(), CanDoError> {
        if cyclic {
            if let Some((prime5, prime3)) = self
                .previous_nucl
                .take()
                .zip(self.first_nucl.take())
                .filter(|(a, b)| a != b)
            {
                self.formatter.make_bound(prime5, prime3)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum CanDoError {
    DuplicateNucleotide(Nucl),
    NotPaired(Nucl, Nucl),
    CannotFindNuclWithId(usize),
    CannotFindNucl(Nucl),
    IOError(std::io::Error),
}
