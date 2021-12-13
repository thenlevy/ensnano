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
/// This module defines the ensnano format.
/// All other format supported by ensnano are converted into this format and run-time manipulation
/// of designs are performed on an `ensnano::Design` structure
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::f32::consts::PI;
use std::sync::Arc;

#[macro_use]
extern crate serde_derive;
extern crate serde;
pub use ultraviolet;
use ultraviolet::{Isometry2, Mat4, Rotor3, Vec3};

pub mod codenano;
pub mod grid;
use grid::{Grid, GridDescriptor, GridPosition};
pub mod scadnano;
pub use ensnano_organizer::{GroupId, OrganizerTree};
use scadnano::*;
pub mod elements;
use elements::DnaElementKey;
pub type EnsnTree = OrganizerTree<DnaElementKey>;
pub mod group_attributes;
use group_attributes::GroupAttribute;

mod formating;
#[cfg(test)]
mod tests;

/// The `ensnano` Design structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct Design {
    /// The collection of all helices used in this design. Helices have a
    /// position and an orientation in 3D.
    pub helices: Arc<BTreeMap<usize, Arc<Helix>>>,
    /// The vector of strands.
    pub strands: BTreeMap<usize, Strand>,
    /// Parameters of DNA geometry. This can be skipped (in JSON), or
    /// set to `None` in Rust, in which case a default set of
    /// parameters from the literature is used.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename(serialize = "dna_parameters"),
        alias = "dna_parameters"
    )]
    pub parameters: Option<Parameters>,

    /// The strand that is the scaffold if the design is an origami
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaffold_id: Option<usize>,

    /// The sequence of the scaffold if the design is an origami
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaffold_sequence: Option<String>,

    /// The shifting of the scaffold if the design is an origami. This is used to reduce the number
    /// of anti-patern in the stapples sequences
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaffold_shift: Option<usize>,

    #[serde(default)]
    pub grids: Arc<Vec<GridDescriptor>>,

    /// The cross-over suggestion groups
    #[serde(skip_serializing_if = "groups_is_empty", default)]
    pub groups: Arc<BTreeMap<usize, bool>>,

    /// The set of identifiers of grids whose helices must not always display their phantom
    /// helices.
    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub no_phantoms: HashSet<usize>,

    /// The set of identifiers of grids whose helices are displayed with smaller spheres for the
    /// nucleotides.
    #[serde(
        alias = "small_shperes",
        alias = "no_spheres",
        rename(serialize = "no_spheres"),
        skip_serializing_if = "HashSet::is_empty",
        default
    )]
    pub small_spheres: HashSet<usize>,

    /// The set of nucleotides that must not move during physical simulations
    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub anchors: HashSet<Nucl>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub organizer_tree: Option<Arc<OrganizerTree<DnaElementKey>>>,

    #[serde(default)]
    pub ensnano_version: String,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub group_attributes: HashMap<ensnano_organizer::GroupId, GroupAttribute>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    cameras: BTreeMap<CameraId, Camera>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    favorite_camera: Option<CameraId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    saved_camera: Option<Camera>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CameraId(u64);

/// A saved camera position. This can be use to register intresting point of views of the design.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub name: String,
    pub id: CameraId,
}

fn ensnano_version() -> String {
    std::env!("CARGO_PKG_VERSION").to_owned()
}

fn groups_is_empty<K, V>(groups: &Arc<BTreeMap<K, V>>) -> bool {
    groups.as_ref().is_empty()
}

impl Default for Design {
    fn default() -> Self {
        Self::new()
    }
}

impl Design {
    pub fn from_codenano<Sl, Dl>(codenano_desgin: &codenano::Design<Sl, Dl>) -> Self {
        let mut helices = BTreeMap::new();
        for (i, helix) in codenano_desgin.helices.iter().enumerate() {
            helices.insert(i, Arc::new(Helix::from_codenano(helix)));
        }

        let mut strands = BTreeMap::new();
        for (i, strand) in codenano_desgin.strands.iter().enumerate() {
            strands.insert(i, Strand::from_codenano(strand));
        }

        let parameters = codenano_desgin
            .parameters
            .map(|p| Parameters::from_codenano(&p))
            .unwrap_or_default();

        Self {
            helices: Arc::new(helices),
            strands,
            parameters: Some(parameters),
            grids: Default::default(),
            scaffold_id: None,
            scaffold_sequence: None,
            scaffold_shift: None,
            groups: Default::default(),
            small_spheres: Default::default(),
            no_phantoms: Default::default(),
            anchors: Default::default(),
            organizer_tree: None,
            ensnano_version: ensnano_version(),
            group_attributes: Default::default(),
            cameras: Default::default(),
            ..Default::default()
        }
    }

    pub fn new() -> Self {
        Self {
            helices: Default::default(),
            strands: BTreeMap::new(),
            parameters: Some(Parameters::DEFAULT),
            grids: Default::default(),
            scaffold_id: None,
            scaffold_sequence: None,
            scaffold_shift: None,
            groups: Default::default(),
            small_spheres: Default::default(),
            no_phantoms: Default::default(),
            anchors: Default::default(),
            organizer_tree: None,
            ensnano_version: ensnano_version(),
            group_attributes: Default::default(),
            cameras: Default::default(),
            favorite_camera: None,
            saved_camera: None,
        }
    }

    pub fn get_xovers(&self) -> Vec<(Nucl, Nucl)> {
        let mut ret = vec![];
        for s in self.strands.values() {
            for x in s.xovers() {
                ret.push(x)
            }
        }
        ret
    }

    pub fn get_intervals(&self) -> BTreeMap<usize, (isize, isize)> {
        let mut ret = BTreeMap::new();
        for s in self.strands.values() {
            for d in s.domains.iter() {
                if let Domain::HelixDomain(dom) = d {
                    let left = dom.start;
                    let right = dom.end - 1;
                    let interval = ret.entry(dom.helix).or_insert((left, right));
                    interval.0 = interval.0.min(left);
                    interval.1 = interval.1.max(right);
                }
            }
        }
        ret
    }

    pub fn get_strand_nucl(&self, nucl: &Nucl) -> Option<usize> {
        for (s_id, s) in self.strands.iter() {
            if s.has_nucl(nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    pub fn remove_empty_domains(&mut self) {
        for s in self.strands.values_mut() {
            s.remove_empty_domains()
        }
    }

    pub fn update_version(&mut self) {
        if self.ensnano_version == ensnano_version() {
            return;
        } else if self.ensnano_version.is_empty() {
            // Version < 0.2.0 had no version identifier, and there DNA parameters where different.
            // The groove_angle was negative, and the roll was going in the opposite direction
            if let Some(parameters) = self.parameters.as_mut() {
                parameters.groove_angle *= -1.;
            } else {
                self.parameters = Some(Default::default())
            }
            mutate_all_helices(self, |h| h.roll *= -1.);
            /*
            for h in self.helices.values_mut() {
                h.roll *= -1.;
            }*/
            self.ensnano_version = ensnano_version();
        }
    }

    pub fn has_at_least_on_strand_with_insertions(&self) -> bool {
        self.strands.values().any(|s| s.has_insertions())
    }

    /// Return the strand end status of nucl
    pub fn is_strand_end(&self, nucl: &Nucl) -> Extremity {
        for s in self.strands.values() {
            if !s.cyclic && s.get_5prime() == Some(*nucl) {
                return Extremity::Prime5;
            } else if !s.cyclic && s.get_3prime() == Some(*nucl) {
                return Extremity::Prime3;
            }
        }
        return Extremity::No;
    }

    /// Return a list of tuples (n1, n2, M) where n1 and n2 are nuclotides that are not on the same
    /// helix and whose distance is at most `epsilon` and M is the middle of the segment between
    /// the two positions of n1 and n2.
    pub fn get_pairs_of_close_nucleotides(&self, epsilon: f32) -> Vec<(Nucl, Nucl, Vec3)> {
        let mut ret = Vec::new();
        let mut nucls = Vec::new();
        let parameters = self.parameters.unwrap_or_default();
        for s in self.strands.values() {
            for d in s.domains.iter() {
                if let Domain::HelixDomain(interval) = d {
                    for i in interval.iter() {
                        let nucl = Nucl {
                            helix: interval.helix,
                            forward: interval.forward,
                            position: i,
                        };
                        if let Some(h) = self.helices.get(&interval.helix) {
                            let space_position =
                                h.space_pos(&parameters, nucl.position, nucl.forward);
                            nucls.push((nucl, space_position));
                        }
                    }
                }
            }
        }
        for (n_id, n1) in nucls.iter().enumerate() {
            for n2 in nucls.iter().skip(n_id + 1) {
                if n1.0.helix != n2.0.helix {
                    if (n1.1 - n2.1).mag() < epsilon {
                        ret.push((n1.0, n2.0, ((n1.1 + n2.1) / 2.)));
                    }
                }
            }
        }
        ret
    }

    pub fn is_domain_end(&self, nucl: &Nucl) -> Extremity {
        for strand in self.strands.values() {
            let mut prev_helix = None;
            for domain in strand.domains.iter() {
                if domain.prime5_end() == Some(*nucl) && prev_helix != domain.half_helix() {
                    return Extremity::Prime5;
                } else if domain.prime3_end() == Some(*nucl) {
                    return Extremity::Prime3;
                } else if let Some(_) = domain.has_nucl(nucl) {
                    return Extremity::No;
                }
                prev_helix = domain.half_helix();
            }
        }
        Extremity::No
    }

    pub fn is_true_xover_end(&self, nucl: &Nucl) -> bool {
        self.is_domain_end(nucl).to_opt().is_some() && self.is_strand_end(nucl).to_opt().is_none()
    }

    /// Return true if at least one strand goes through helix h_id
    pub fn uses_helix(&self, h_id: usize) -> bool {
        for s in self.strands.values() {
            for d in s.domains.iter() {
                if let Domain::HelixDomain(interval) = d {
                    if interval.helix == h_id {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn add_camera(&mut self, position: Vec3, orientation: Rotor3) {
        let cam_id = self
            .cameras
            .keys()
            .max()
            .map(|id| CameraId(id.0 + 1))
            .unwrap_or(CameraId(1));
        let new_camera = Camera {
            position,
            orientation,
            name: format!("Camera {}", cam_id.0),
            id: cam_id,
        };
        self.cameras.insert(cam_id, new_camera);
    }

    pub fn rm_camera(&mut self, cam_id: CameraId) -> Result<(), ()> {
        if self.cameras.remove(&cam_id).is_some() {
            if self.favorite_camera == Some(cam_id) {
                self.favorite_camera = self.cameras.keys().min().cloned();
            }
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn get_camera_mut(&mut self, cam_id: CameraId) -> Option<&mut Camera> {
        self.cameras.get_mut(&cam_id)
    }

    pub fn get_camera(&self, cam_id: CameraId) -> Option<&Camera> {
        self.cameras.get(&cam_id)
    }

    pub fn get_favourite_camera(&self) -> Option<&Camera> {
        self.favorite_camera
            .as_ref()
            .and_then(|id| self.cameras.get(id))
            .or(self.saved_camera.as_ref())
    }

    pub fn get_favourite_camera_id(&self) -> Option<CameraId> {
        self.favorite_camera.clone()
    }

    pub fn set_favourite_camera(&mut self, cam_id: CameraId) -> Result<(), ()> {
        if self.cameras.contains_key(&cam_id) {
            if self.favorite_camera != Some(cam_id) {
                self.favorite_camera = Some(cam_id);
            } else {
                self.favorite_camera = None;
            }
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn get_cameras(&self) -> impl Iterator<Item = (&CameraId, &Camera)> {
        self.cameras.iter()
    }

    pub fn prepare_for_save(&mut self, saving_information: SavingInformation) {
        self.saved_camera = saving_information.camera;
    }

    pub fn get_nucl_position(&self, nucl: Nucl) -> Option<Vec3> {
        let helix = self.helices.get(&nucl.helix)?;
        Some(helix.space_pos(
            &self.parameters.unwrap_or_default(),
            nucl.position,
            nucl.forward,
        ))
    }
}

pub struct SavingInformation {
    pub camera: Option<Camera>,
}

impl Design {
    pub fn from_scadnano(scad: &ScadnanoDesign) -> Result<Self, ScadnanoImportError> {
        let mut grids = Vec::new();
        let mut group_map = BTreeMap::new();
        let default_grid = scad.default_grid_descriptor()?;
        let mut deletions = BTreeMap::new();
        group_map.insert(String::from("default_group"), 0usize);
        grids.push(default_grid);
        let mut helices_per_group = vec![0];
        let mut groups: Vec<ScadnanoGroup> = vec![Default::default()];
        if let Some(ref scad_groups) = scad.groups {
            for (name, g) in scad_groups.iter() {
                let group = g.to_grid_desc()?;
                groups.push(g.clone());
                group_map.insert(name.clone(), grids.len());
                grids.push(group);
                helices_per_group.push(0);
            }
        }
        for s in scad.strands.iter() {
            s.read_deletions(&mut deletions);
        }
        let mut helices = BTreeMap::new();
        for (i, h) in scad.helices.iter().enumerate() {
            let helix = Helix::from_scadnano(h, &group_map, &groups, &mut helices_per_group)?;
            helices.insert(i, Arc::new(helix));
        }
        let mut strands = BTreeMap::new();
        for (i, s) in scad.strands.iter().enumerate() {
            let strand = Strand::from_scadnano(s, &deletions)?;
            strands.insert(i, strand);
        }
        println!("grids {:?}", grids);
        println!("helices {:?}", helices);
        Ok(Self {
            grids: Arc::new(grids),
            helices: Arc::new(helices),
            strands,
            small_spheres: Default::default(),
            scaffold_id: None, //TODO determine this value
            scaffold_sequence: None,
            scaffold_shift: None,
            groups: Default::default(),
            no_phantoms: Default::default(),
            parameters: Some(Parameters::DEFAULT),
            anchors: Default::default(),
            organizer_tree: None,
            ensnano_version: ensnano_version(),
            group_attributes: Default::default(),
            cameras: Default::default(),
            ..Default::default()
        })
    }
}

/// A link between a 5' and a 3' domain.
///
/// For any non cyclic strand, the last domain juction must be DomainJunction::Prime3. For a cyclic
/// strand it must be the link that would be appropriate between the first and the last domain.
///
/// An Insertion is considered to be adjacent to its 5' neighbour. The link between an Insertion
/// and it's 3' neighbour is the link that would exist between it's 5' and 3' neighbour if there
/// were no insertion.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum DomainJunction {
    /// A cross-over that has not yet been given an identifier. These should exist only in
    /// transitory states.
    UnindentifiedXover,
    /// A cross-over with an identifier.
    IdentifiedXover(usize),
    /// A link between two neighbouring domains
    Adjacent,
    /// Indicate that the previous domain is the end of the strand.
    Prime3,
}

/// A DNA strand. Strands are represented as sequences of `Domains`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Strand {
    /// The (ordered) vector of domains, where each domain is a
    /// directed interval of a helix.
    pub domains: Vec<Domain>,
    /// The junctions between the consecutive domains of the strand.
    /// This field is optional and will be filled automatically when absent.
    #[serde(default)]
    pub junctions: Vec<DomainJunction>,
    /// The sequence of this strand, if any. If the sequence is longer
    /// than specified by the domains, a prefix is assumed. Can be
    /// skipped in the serialisation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sequence: Option<Cow<'static, str>>,
    /// Is this sequence cyclic? Can be skipped (and defaults to
    /// `false`) in the serialization.
    #[serde(skip_serializing_if = "is_false", default)]
    pub cyclic: bool,
    /// Colour of this strand. If skipped, a default colour will be
    /// chosen automatically.
    #[serde(default)]
    pub color: u32,
    /// A name of the strand, used for strand export. If the name is `None`, the exported strand
    /// will be given a name corresponding to the position of its 5' nucleotide
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<Cow<'static, str>>,
}

/// Return a list of domains that validate the following condition:
/// [SaneDomains]: There must always be a Domain::HelixDomain between two Domain::Insertion. If the
/// strand is cyclic, this include the first and the last domain.
pub fn sanitize_domains(domains: &[Domain], cyclic: bool) -> Vec<Domain> {
    let mut ret = Vec::with_capacity(domains.len());
    let mut current_insertion: Option<usize> = None;
    for d in domains {
        match d {
            Domain::HelixDomain(_) => {
                if let Some(n) = current_insertion.take() {
                    ret.push(Domain::Insertion(n));
                }
                ret.push(d.clone());
            }
            Domain::Insertion(m) => {
                if let Some(n) = current_insertion {
                    current_insertion = Some(n + m);
                } else {
                    current_insertion = Some(*m);
                }
            }
        }
    }

    if let Some(mut n) = current_insertion {
        if cyclic {
            if let Domain::Insertion(k) = ret[0].clone() {
                ret.remove(0);
                n += k;
            }
            ret.push(Domain::Insertion(n));
        } else {
            ret.push(Domain::Insertion(n));
        }
    } else if cyclic {
        if let Domain::Insertion(k) = ret[0].clone() {
            ret.remove(0);
            ret.push(Domain::Insertion(k));
        }
    }
    ret
}

impl Strand {
    pub fn from_codenano<Sl, Dl>(codenano_strand: &codenano::Strand<Sl, Dl>) -> Self {
        let domains: Vec<Domain> = codenano_strand
            .domains
            .iter()
            .map(|d| Domain::from_codenano(d))
            .collect();
        let sane_domains = sanitize_domains(&domains, codenano_strand.cyclic);
        let juctions = read_junctions(&sane_domains, codenano_strand.cyclic);
        Self {
            domains: sane_domains,
            sequence: codenano_strand.sequence.clone(),
            cyclic: codenano_strand.cyclic,
            junctions: juctions,
            color: codenano_strand
                .color
                .clone()
                .unwrap_or_else(|| codenano_strand.default_color())
                .as_int(),
            ..Default::default()
        }
    }

    pub fn from_scadnano(
        scad: &ScadnanoStrand,
        deletions: &BTreeMap<usize, BTreeSet<isize>>,
    ) -> Result<Self, ScadnanoImportError> {
        let color = scad.color()?;
        let domains: Vec<Domain> = scad
            .domains
            .iter()
            .map(|s| Domain::from_scadnano(s, deletions))
            .flatten()
            .collect();
        let sequence = if let Some(ref seq) = scad.sequence {
            Some(Cow::Owned(seq.clone()))
        } else {
            None
        };
        let cyclic = scad.circular;
        let sane_domains = sanitize_domains(&domains, cyclic);
        let junctions = read_junctions(&sane_domains, cyclic);
        Ok(Self {
            domains: sane_domains,
            color,
            cyclic,
            junctions,
            sequence,
            ..Default::default()
        })
    }

    pub fn init(helix: usize, position: isize, forward: bool, color: u32) -> Self {
        let domains = vec![Domain::HelixDomain(HelixInterval {
            sequence: None,
            start: position,
            end: position + 1,
            helix,
            forward,
        })];
        let sane_domains = sanitize_domains(&domains, false);
        let junctions = read_junctions(&sane_domains, false);
        Self {
            domains: sane_domains,
            sequence: None,
            cyclic: false,
            junctions,
            color,
            ..Default::default()
        }
    }

    pub fn get_5prime(&self) -> Option<Nucl> {
        for domain in self.domains.iter() {
            match domain {
                Domain::Insertion(_) => (),
                Domain::HelixDomain(h) => {
                    let position = if h.forward { h.start } else { h.end - 1 };
                    return Some(Nucl {
                        helix: h.helix,
                        position,
                        forward: h.forward,
                    });
                }
            }
        }
        None
    }

    pub fn get_3prime(&self) -> Option<Nucl> {
        for domain in self.domains.iter().rev() {
            match domain {
                Domain::Insertion(_) => (),
                Domain::HelixDomain(h) => {
                    let position = if h.forward { h.end - 1 } else { h.start };
                    return Some(Nucl {
                        helix: h.helix,
                        position,
                        forward: h.forward,
                    });
                }
            }
        }
        None
    }

    pub fn length(&self) -> usize {
        self.domains.iter().map(|d| d.length()).sum()
    }

    /// Merge all consecutive domains that are on the same helix
    pub fn merge_consecutive_domains(&mut self) {
        let mut to_merge = vec![];
        for n in 0..self.domains.len() - 1 {
            let dom1 = &self.domains[n];
            let dom2 = &self.domains[n + 1];
            if dom1.can_merge(dom2) {
                to_merge.push(n)
            }
        }
        while let Some(n) = to_merge.pop() {
            let dom2 = self.domains[n + 1].clone();
            self.domains.get_mut(n).unwrap().merge(&dom2);
            self.domains.remove(n + 1);
        }
    }

    pub fn xovers(&self) -> Vec<(Nucl, Nucl)> {
        let mut ret = vec![];
        for n in 0..self.domains.len() - 1 {
            let dom1 = &self.domains[n];
            let dom2 = &self.domains[n + 1];
            match (dom1, dom2) {
                (Domain::HelixDomain(int1), Domain::HelixDomain(int2))
                    if int1.helix != int2.helix =>
                {
                    ret.push((dom1.prime3_end().unwrap(), dom2.prime5_end().unwrap()));
                }
                _ => (),
            }
        }
        if self.cyclic && self.domains.len() > 1 {
            let dom1 = &self.domains[self.domains.len() - 1];
            let dom2 = &self.domains[0];
            match (dom1, dom2) {
                (Domain::HelixDomain(int1), Domain::HelixDomain(int2))
                    if int1.helix != int2.helix =>
                {
                    ret.push((dom1.prime3_end().unwrap(), dom2.prime5_end().unwrap()));
                }
                _ => (),
            }
        }
        ret
    }

    pub fn intersect_domains(&self, domains: &[Domain]) -> bool {
        for d in self.domains.iter() {
            for other in domains.iter() {
                if d.intersect(other) {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_nucl(&self, nucl: &Nucl) -> bool {
        for d in self.domains.iter() {
            if d.has_nucl(nucl).is_some() {
                return true;
            }
        }
        false
    }

    pub fn find_nucl(&self, nucl: &Nucl) -> Option<usize> {
        let mut ret = 0;
        for d in self.domains.iter() {
            if let Some(n) = d.has_nucl(nucl) {
                return Some(ret + n);
            }
            ret += d.length()
        }
        None
    }

    pub fn get_insertions(&self) -> Vec<Nucl> {
        let mut last_nucl = None;
        let mut ret = Vec::with_capacity(self.domains.len());
        for d in self.domains.iter() {
            match d {
                Domain::Insertion(n) if *n > 0 => {
                    if let Some(nucl) = last_nucl {
                        ret.push(nucl);
                    }
                }
                Domain::Insertion(_) => (),
                Domain::HelixDomain(_) => {
                    last_nucl = d.prime3_end();
                }
            }
        }
        ret
    }

    fn remove_empty_domains(&mut self) {
        self.domains.retain(|d| {
            if d.length() > 0 {
                true
            } else {
                println!("Warning, removing empty domain {:?}", d);
                false
            }
        })
    }

    pub fn get_nth_nucl(&self, n: usize) -> Option<Nucl> {
        let mut seen = 0;
        for d in self.domains.iter() {
            if seen + d.length() > n {
                if let Domain::HelixDomain(d) = d {
                    let position = d.iter().nth(n - seen);
                    return position.map(|position| Nucl {
                        position,
                        helix: d.helix,
                        forward: d.forward,
                    });
                } else {
                    return None;
                }
            } else {
                seen += d.length()
            }
        }
        None
    }

    pub fn insertion_points(&self) -> Vec<(Option<Nucl>, Option<Nucl>)> {
        let mut ret = Vec::new();
        let mut prev_prime3 = if self.cyclic {
            self.domains.last().and_then(|d| d.prime3_end())
        } else {
            None
        };
        for (d1, d2) in self.domains.iter().zip(self.domains.iter().skip(1)) {
            if let Domain::Insertion(_) = d1 {
                ret.push((prev_prime3, d2.prime5_end()))
            } else {
                prev_prime3 = d1.prime3_end()
            }
        }
        if let Some(Domain::Insertion(_)) = self.domains.last() {
            if self.cyclic {
                ret.push((
                    prev_prime3,
                    self.domains.first().and_then(|d| d.prime5_end()),
                ))
            } else {
                ret.push((prev_prime3, None))
            }
        }
        ret
    }

    pub fn has_insertions(&self) -> bool {
        for d in self.domains.iter() {
            if let Domain::Insertion(_) = d {
                return true;
            }
        }
        false
    }

    pub fn add_insertion_at_nucl(&mut self, nucl: &Nucl, insertion_size: usize) {
        let insertion_point = self.locate_nucl(nucl);
        if let Some((d_id, n)) = insertion_point {
            self.add_insertion_at_dom_position(d_id, n, insertion_size);
        } else {
            println!("Could not add insertion");
            if cfg!(test) {
                panic!("Could not locate nucleotide in strand");
            }
        }
    }

    fn locate_nucl(&self, nucl: &Nucl) -> Option<(usize, usize)> {
        for (d_id, d) in self.domains.iter().enumerate() {
            if let Some(n) = d.has_nucl(nucl) {
                return Some((d_id, n));
            }
        }
        None
    }

    fn add_insertion_at_dom_position(&mut self, d_id: usize, pos: usize, insertion_size: usize) {
        if let Some((prime5, prime3)) = self.domains[d_id].split(pos) {
            self.domains[d_id] = prime3;
            self.domains.insert(d_id, Domain::Insertion(insertion_size));
            self.domains.insert(d_id, prime5);
        } else {
            println!("Could not split");
            if cfg!(test) {
                panic!("Could not split domain");
            }
        }
    }

    pub fn set_name<S: Into<Cow<'static, str>>>(&mut self, name: S) {
        self.name = Some(name.into())
    }

    pub fn domain_ends(&self) -> Vec<Nucl> {
        self.domains
            .iter()
            .filter_map(|d| Some([d.prime5_end()?, d.prime3_end()?]))
            .flatten()
            .collect()
    }
}

fn is_false(x: &bool) -> bool {
    !*x
}

/// A domain can be either an interval of nucleotides on an helix, or an "Insertion" that is a set
/// of nucleotides that are not on an helix and form an independent loop.
#[derive(Clone, Serialize, Deserialize)]
pub enum Domain {
    /// An interval of nucleotides on an helix
    HelixDomain(HelixInterval),
    /// A set of nucleotides not on an helix.
    Insertion(usize),
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct HelixInterval {
    /// Index of the helix in the array of helices. Indices start at
    /// 0.
    pub helix: usize,
    /// Position of the leftmost base of this domain along the helix
    /// (this might be the first or last base of the domain, depending
    /// on the `orientation` parameter below).
    pub start: isize,
    /// Position of the first base after the forwardmost base of the
    /// domain, along the helix. Domains must always be such that
    /// `domain.start < domain.end`.
    pub end: isize,
    /// If true, the "5' to 3'" direction of this domain runs in the
    /// same direction as the helix, i.e. "to the forward" along the
    /// axis of the helix. Else, the 5' to 3' runs to the left along
    /// the axis.
    pub forward: bool,
    /// In addition to the strand-level sequence, individual domains
    /// may have sequences too. The precedence has to be defined by
    /// the user of this library.
    pub sequence: Option<Cow<'static, str>>,
}

impl HelixInterval {
    pub fn prime5(&self) -> Nucl {
        if self.forward {
            Nucl {
                helix: self.helix,
                position: self.start,
                forward: true,
            }
        } else {
            Nucl {
                helix: self.helix,
                position: self.end - 1,
                forward: false,
            }
        }
    }

    pub fn prime3(&self) -> Nucl {
        if self.forward {
            Nucl {
                helix: self.helix,
                position: self.end - 1,
                forward: true,
            }
        } else {
            Nucl {
                helix: self.helix,
                position: self.start,
                forward: false,
            }
        }
    }
}

impl Domain {
    pub fn from_codenano<Dl>(codenano_domain: &codenano::Domain<Dl>) -> Self {
        let interval = HelixInterval {
            helix: codenano_domain.helix as usize,
            start: codenano_domain.start,
            end: codenano_domain.end,
            forward: codenano_domain.forward,
            sequence: codenano_domain.sequence.clone(),
        };
        Self::HelixDomain(interval)
    }

    pub fn from_scadnano(
        scad: &ScadnanoDomain,
        deletions: &BTreeMap<usize, BTreeSet<isize>>,
    ) -> Vec<Self> {
        match scad {
            ScadnanoDomain::HelixDomain {
                helix,
                start,
                end,
                forward,
                insertions,
                ..// TODO read insertion and deletion
            } => {
                let adjust = |n| n - deletions.get(helix).map(|s| count_leq(s, n)).unwrap_or(0);

                if let Some(insertions) = insertions {
                    let mut ret = Vec::new();
                    if *forward {
                        let mut ends = insertions.iter();
                        let mut left = *start;
                        let mut right;
                        while let Some(insertion) = ends.next() {
                            right = insertion[0] + 1;
                            let nb_insertion = insertion[1];
                            ret.push(Self::HelixDomain(HelixInterval {
                                helix: *helix,
                                start: adjust(left),
                                end: adjust(right),
                                forward: *forward,
                                sequence: None,
                            }));
                            ret.push(Self::Insertion(nb_insertion as usize));
                            left = right;
                        }
                        ret.push(Self::HelixDomain(HelixInterval {
                            helix: *helix,
                            start: adjust(left),
                            end: adjust(*end),
                            forward: *forward,
                            sequence: None,
                        }));
                    } else {
                        let mut ends = insertions.iter().rev();
                        let mut right = *end;
                        let mut left;
                        while let Some(insertion) = ends.next() {
                            left = insertion[0];
                            let nb_insertion = insertion[1];
                            ret.push(Self::HelixDomain(HelixInterval {
                                helix: *helix,
                                start: adjust(left),
                                end: adjust(right),
                                forward: *forward,
                                sequence: None,
                            }));
                            ret.push(Self::Insertion(nb_insertion as usize));
                            right = left;
                        }
                        ret.push(Self::HelixDomain(HelixInterval {
                            helix: *helix,
                            start: adjust(*start),
                            end: adjust(right),
                            forward: *forward,
                            sequence: None,
                        }));
                    }
                    ret
                } else {
                    let start = adjust(*start);
                    let end = adjust(*end);

                    vec![Self::HelixDomain(HelixInterval {
                        helix: *helix,
                        start,
                        end,
                        forward: *forward,
                        sequence: None,
                    })]
                }
            }
            ScadnanoDomain::Loopout{ loopout: n } => vec![Self::Insertion(*n)]
        }
    }

    pub fn length(&self) -> usize {
        match self {
            Self::Insertion(n) => *n,
            Self::HelixDomain(interval) => (interval.end - interval.start).max(0) as usize,
        }
    }

    pub fn other_end(&self, nucl: Nucl) -> Option<isize> {
        match self {
            Self::Insertion(_) => None,
            Self::HelixDomain(interval) => {
                if interval.helix == nucl.helix && nucl.forward == interval.forward {
                    if interval.start == nucl.position {
                        Some(interval.end - 1)
                    } else if interval.end - 1 == nucl.position {
                        Some(interval.start)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    pub fn prime5_end(&self) -> Option<Nucl> {
        match self {
            Self::Insertion(_) => None,
            Self::HelixDomain(interval) => {
                let position = if interval.forward {
                    interval.start
                } else {
                    interval.end - 1
                };
                Some(Nucl {
                    helix: interval.helix,
                    position,
                    forward: interval.forward,
                })
            }
        }
    }

    pub fn prime3_end(&self) -> Option<Nucl> {
        match self {
            Self::Insertion(_) => None,
            Self::HelixDomain(interval) => {
                let position = if interval.forward {
                    interval.end - 1
                } else {
                    interval.start
                };
                Some(Nucl {
                    helix: interval.helix,
                    position,
                    forward: interval.forward,
                })
            }
        }
    }

    pub fn has_nucl(&self, nucl: &Nucl) -> Option<usize> {
        match self {
            Self::Insertion(_) => None,
            Self::HelixDomain(HelixInterval {
                forward,
                start,
                end,
                helix,
                ..
            }) => {
                if *helix == nucl.helix && *forward == nucl.forward {
                    if nucl.position >= *start && nucl.position <= *end - 1 {
                        if *forward {
                            Some((nucl.position - *start) as usize)
                        } else {
                            Some((*end - 1 - nucl.position) as usize)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Split self at position `n`, putting `n` on the 5' prime half of the split
    pub fn split(&self, n: usize) -> Option<(Self, Self)> {
        match self {
            Self::Insertion(_) => None,
            Self::HelixDomain(HelixInterval {
                forward,
                start,
                end,
                helix,
                sequence,
            }) => {
                if (*end - 1 - *start) as usize >= n {
                    let seq_prim5;
                    let seq_prim3;
                    if let Some(seq) = sequence {
                        let seq = seq.clone().into_owned();
                        let chars = seq.chars();
                        seq_prim5 = Some(Cow::Owned(chars.clone().take(n).collect()));
                        seq_prim3 = Some(Cow::Owned(chars.clone().skip(n).collect()));
                    } else {
                        seq_prim3 = None;
                        seq_prim5 = None;
                    }
                    let dom_left;
                    let dom_right;
                    if *forward {
                        dom_left = Self::HelixDomain(HelixInterval {
                            forward: *forward,
                            start: *start,
                            end: *start + n as isize + 1,
                            helix: *helix,
                            sequence: seq_prim5,
                        });
                        dom_right = Self::HelixDomain(HelixInterval {
                            forward: *forward,
                            start: *start + n as isize + 1,
                            end: *end,
                            helix: *helix,
                            sequence: seq_prim3,
                        });
                    } else {
                        dom_right = Self::HelixDomain(HelixInterval {
                            forward: *forward,
                            start: *end - 1 - n as isize,
                            end: *end,
                            helix: *helix,
                            sequence: seq_prim3,
                        });
                        dom_left = Self::HelixDomain(HelixInterval {
                            forward: *forward,
                            start: *start,
                            end: *end - 1 - n as isize,
                            helix: *helix,
                            sequence: seq_prim5,
                        });
                    }
                    if *forward {
                        Some((dom_left, dom_right))
                    } else {
                        Some((dom_right, dom_left))
                    }
                } else {
                    None
                }
            }
        }
    }

    pub fn helix(&self) -> Option<usize> {
        match self {
            Domain::HelixDomain(domain) => Some(domain.helix),
            Domain::Insertion(_) => None,
        }
    }

    pub fn half_helix(&self) -> Option<(usize, bool)> {
        match self {
            Domain::HelixDomain(domain) => Some((domain.helix, domain.forward)),
            Domain::Insertion(_) => None,
        }
    }

    pub fn merge(&mut self, other: &Domain) {
        let old_self = self.clone();
        match (self, other) {
            (Domain::HelixDomain(dom1), Domain::HelixDomain(dom2)) if dom1.helix == dom2.helix => {
                let start = dom1.start.min(dom2.start);
                let end = dom1.end.max(dom2.end);
                dom1.start = start;
                dom1.end = end;
            }
            _ => println!(
                "Warning attempt to merge unmergeable domains {:?}, {:?}",
                old_self, other
            ),
        }
    }

    pub fn can_merge(&self, other: &Domain) -> bool {
        match (self, other) {
            (Domain::HelixDomain(dom1), Domain::HelixDomain(dom2)) => {
                dom1.helix == dom2.helix
                    && (dom1.end == dom2.start || dom1.start == dom2.end)
                    && dom1.forward == dom2.forward
            }
            _ => false,
        }
    }

    pub fn intersect(&self, other: &Domain) -> bool {
        match (self, other) {
            (Domain::HelixDomain(dom1), Domain::HelixDomain(dom2)) => {
                dom1.helix == dom2.helix
                    && dom1.start < dom2.end
                    && dom2.start < dom1.end
                    && dom1.forward == dom2.forward
            }
            _ => false,
        }
    }
}

impl HelixInterval {
    pub fn iter(&self) -> DomainIter {
        DomainIter {
            start: self.start,
            end: self.end,
            forward: self.forward,
        }
    }
}

/// An iterator over all positions of a domain.
pub struct DomainIter {
    start: isize,
    end: isize,
    forward: bool,
}

impl Iterator for DomainIter {
    type Item = isize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else if self.forward {
            let s = self.start;
            self.start += 1;
            Some(s)
        } else {
            let s = self.end;
            self.end -= 1;
            Some(s - 1)
        }
    }
}

/// DNA geometric parameters.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Parameters {
    /// Distance between two consecutive bases along the axis of a
    /// helix, in nanometers.
    pub z_step: f32,
    /// Radius of a helix, in nanometers.
    pub helix_radius: f32,
    /// Number of bases per turn in nanometers.
    pub bases_per_turn: f32,
    /// Minor groove angle. DNA helices have a "minor groove" and a
    /// "major groove", meaning that two paired nucleotides are not at
    /// opposite positions around a double helix (i.e. at an angle of
    /// 180°), but instead have a different angle.
    ///
    /// Strands are directed. The "normal" direction is called "5' to
    /// 3'" (named after parts of the nucleotides). This parameter is
    /// the small angle, which is clockwise from the normal strand to
    /// the reverse strand.
    pub groove_angle: f32,

    /// Gap between two neighbouring helices.
    pub inter_helix_gap: f32,
}

impl Parameters {
    /// Default values for the parameters of DNA, taken from the litterature (Wikipedia, Cargo
    /// sorting paper, Woo 2011).
    pub const DEFAULT: Parameters = Parameters {
        // z-step and helix radius from: Wikipedia
        z_step: 0.332,
        helix_radius: 1.,
        // bases per turn from Woo Rothemund (Nature Chemistry).
        bases_per_turn: 10.44,
        // minor groove 12 Å, major groove 22 Å total 34 Å
        groove_angle: 2. * PI * 12. / 34.,
        // From Paul's paper.
        inter_helix_gap: 0.65,
    };

    pub fn from_codenano(codenano_param: &codenano::Parameters) -> Self {
        Self {
            z_step: codenano_param.z_step as f32,
            helix_radius: codenano_param.helix_radius as f32,
            bases_per_turn: codenano_param.bases_per_turn as f32,
            groove_angle: codenano_param.groove_angle as f32,
            inter_helix_gap: codenano_param.inter_helix_gap as f32,
        }
    }

    pub fn formated_string(&self) -> String {
        use std::fmt::Write;
        let mut ret = String::new();
        writeln!(&mut ret, "  Z step: {:.3} nm", self.z_step).unwrap_or_default();
        writeln!(&mut ret, "  Helix radius: {:.2} nm", self.helix_radius).unwrap_or_default();
        writeln!(&mut ret, "  #Bases per turn: {:.2}", self.bases_per_turn).unwrap_or_default();
        writeln!(
            &mut ret,
            "  Minor groove angle: {:.1}°",
            self.groove_angle.to_degrees()
        )
        .unwrap_or_default();
        writeln!(
            &mut ret,
            "  Inter helix gap: {:.2} nm",
            self.inter_helix_gap
        )
        .unwrap_or_default();
        ret
    }
}

impl std::default::Default for Parameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A DNA helix. All bases of all strands must be on a helix.
///
/// The three angles are illustrated in the following image, from [the NASA website](https://www.grc.nasa.gov/www/k-12/airplane/rotations.html):
/// Angles are applied in the order yaw -> pitch -> roll
/// ![Aircraft angles](https://www.grc.nasa.gov/www/k-12/airplane/Images/rotations.gif)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Helix {
    /// Position of the origin of the helix axis.
    pub position: Vec3,

    /// Orientation of the helix
    pub orientation: Rotor3,

    /// Indicate wether the helix should be displayed in the 3D view.
    #[serde(default = "default_visibility", skip_serializing_if = "bool::clone")]
    pub visible: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    /// Indicate that the helix cannot move during rigid body simulations.
    pub locked_for_simulations: bool,

    /// The position of the helix on a grid. If this is None, it means that helix is not bound to
    /// any grid.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub grid_position: Option<GridPosition>,

    /// Representation of the helix in 2d
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub isometry2d: Option<Isometry2>,

    /// Roll of the helix. A roll equal to 0 means that the nucleotide 0 of the forward strand is
    /// at point (0., 1., 0.) in the helix's coordinate.
    #[serde(default)]
    pub roll: f32,
}

fn default_visibility() -> bool {
    true
}

impl Helix {
    pub fn from_codenano(codenano_helix: &codenano::Helix) -> Self {
        let position = Vec3::new(
            codenano_helix.position.x as f32,
            codenano_helix.position.y as f32,
            codenano_helix.position.z as f32,
        );
        /*
        let mut roll = codenano_helix.roll.rem_euclid(2. * std::f64::consts::PI);
        if roll > std::f64::consts::PI {
        roll -= 2. * std::f64::consts::PI;
        }
        let mut pitch = codenano_helix.pitch.rem_euclid(2. * std::f64::consts::PI);
        if pitch > std::f64::consts::PI {
        pitch -= 2. * std::f64::consts::PI;
        }
        let mut yaw = codenano_helix.yaw.rem_euclid(2. * std::f64::consts::PI);
        if yaw > std::f64::consts::PI {
        yaw -= 2. * std::f64::consts::PI;
        }
        */
        let orientation = Rotor3::from_rotation_xz(-codenano_helix.yaw as f32)
            * Rotor3::from_rotation_xy(codenano_helix.pitch as f32)
            * Rotor3::from_rotation_yz(codenano_helix.roll as f32);

        Self {
            position,
            orientation,
            grid_position: None,
            isometry2d: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
        }
    }

    pub fn from_scadnano(
        scad: &ScadnanoHelix,
        group_map: &BTreeMap<String, usize>,
        groups: &Vec<ScadnanoGroup>,
        helix_per_group: &mut Vec<usize>,
    ) -> Result<Self, ScadnanoImportError> {
        let group_id = scad.group.clone().unwrap_or(String::from("default_group"));
        let grid_id = if let Some(id) = group_map.get(&group_id) {
            id
        } else {
            return Err(ScadnanoImportError::MissingField(format!(
                "group {}",
                group_id
            )));
        };
        let x = if let Some(x) = scad.grid_position.get(0).cloned() {
            x
        } else {
            return Err(ScadnanoImportError::MissingField(format!("x")));
        };
        let y = if let Some(y) = scad.grid_position.get(1).cloned() {
            y
        } else {
            return Err(ScadnanoImportError::MissingField(format!("y")));
        };
        let group = if let Some(group) = groups.get(*grid_id) {
            group
        } else {
            return Err(ScadnanoImportError::MissingField(format!(
                "group {}",
                grid_id
            )));
        };

        println!("helices per group {:?}", group_map);
        println!("helices per group {:?}", helix_per_group);
        let nb_helices = if let Some(nb_helices) = helix_per_group.get_mut(*grid_id) {
            nb_helices
        } else {
            return Err(ScadnanoImportError::MissingField(format!(
                "helix_per_group {}",
                grid_id
            )));
        };
        let rotation =
            ultraviolet::Rotor2::from_angle(group.pitch.unwrap_or_default().to_radians());
        let isometry2d = Isometry2 {
            translation: (5. * *nb_helices as f32 - 1.)
                * ultraviolet::Vec2::unit_y().rotated_by(rotation)
                + 5. * ultraviolet::Vec2::new(group.position.x, group.position.y),
            rotation,
        };
        *nb_helices += 1;

        Ok(Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            grid_position: Some(GridPosition {
                grid: *grid_id,
                x,
                y,
                axis_pos: 0,
                roll: 0f32,
            }),
            visible: true,
            roll: 0f32,
            isometry2d: Some(isometry2d),
            locked_for_simulations: false,
        })
    }
}

impl Helix {
    pub fn new(origin: Vec3, orientation: Rotor3) -> Self {
        Self {
            position: origin,
            orientation,
            isometry2d: None,
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
        }
    }

    pub fn new_on_grid(grid: &Grid, x: isize, y: isize, g_id: usize) -> Self {
        let position = grid.position_helix(x, y);
        Self {
            position,
            orientation: grid.orientation,
            isometry2d: None,
            grid_position: Some(GridPosition {
                grid: g_id,
                x,
                y,
                axis_pos: 0,
                roll: 0f32,
            }),
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
        }
    }

    /// Angle of base number `n` around this helix.
    pub fn theta(&self, n: isize, forward: bool, cst: &Parameters) -> f32 {
        // The groove_angle goes from the backward strand to the forward strand
        let shift = if forward { cst.groove_angle } else { 0. };
        let beta = 2. * PI / cst.bases_per_turn;
        self.roll
            -n as f32 * beta  // Beta is positive but helix turn clockwise when n increases
            + shift
            + std::f32::consts::FRAC_PI_2 // Add PI/2 so that when the roll is 0,
                                          // the backward strand is at vertical position on nucl 0
    }

    /// 3D position of a nucleotide on this helix. `n` is the position along the axis, and `forward` is true iff the 5' to 3' direction of the strand containing that nucleotide runs in the same direction as the axis of the helix.
    pub fn space_pos(&self, p: &Parameters, n: isize, forward: bool) -> Vec3 {
        let theta = self.theta(n, forward, p);
        let mut ret = Vec3::new(
            n as f32 * p.z_step,
            theta.sin() * p.helix_radius,
            theta.cos() * p.helix_radius,
        );

        ret = self.rotate_point(ret);
        ret += self.position;
        ret
    }

    ///Return an helix that makes an ideal cross-over with self at postion n
    pub fn ideal_neighbour(&self, n: isize, forward: bool, p: &Parameters) -> Helix {
        let other_helix_pos = self.position_ideal_neighbour(n, forward, p);
        let mut new_helix = self.detatched_copy_at(other_helix_pos);
        self.adjust_theta_neighbour(n, forward, &mut new_helix, p);
        new_helix
    }

    fn detatched_copy_at(&self, position: Vec3) -> Helix {
        Helix {
            position,
            orientation: self.orientation,
            grid_position: None,
            roll: 0.,
            visible: true,
            isometry2d: None,
            locked_for_simulations: false,
        }
    }

    fn position_ideal_neighbour(&self, n: isize, forward: bool, p: &Parameters) -> Vec3 {
        let axis_pos = self.axis_position(p, n);
        let my_nucl_pos = self.space_pos(p, n, forward);
        let direction = (my_nucl_pos - axis_pos).normalized();
        let other_helix_pos = (2. * p.helix_radius + p.inter_helix_gap) * direction + axis_pos;
        other_helix_pos
    }

    fn adjust_theta_neighbour(
        &self,
        n: isize,
        forward: bool,
        new_helix: &mut Helix,
        p: &Parameters,
    ) {
        let theta_current = new_helix.theta(0, forward, p);
        let theta_obj = self.theta(n, forward, p) + std::f32::consts::PI;
        new_helix.roll = theta_obj - theta_current;
    }

    pub fn get_axis(&self, p: &Parameters) -> Axis {
        Axis {
            origin: self.position,
            direction: self.axis_position(p, 1) - self.position,
        }
    }

    pub fn axis_position(&self, p: &Parameters, n: isize) -> Vec3 {
        let mut ret = Vec3::new(n as f32 * p.z_step, 0., 0.);

        ret = self.rotate_point(ret);
        ret += self.position;
        ret
    }

    pub fn rotate_point(&self, ret: Vec3) -> Vec3 {
        ret.rotated_by(self.orientation)
    }

    fn append_translation(&mut self, translation: Vec3) {
        self.position += translation;
    }

    fn append_rotation(&mut self, rotation: Rotor3) {
        self.orientation = rotation * self.orientation;
        self.position = rotation * self.position;
    }

    pub fn rotate_arround(&mut self, rotation: Rotor3, origin: Vec3) {
        self.append_translation(-origin);
        self.append_rotation(rotation);
        self.append_translation(origin);
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.append_translation(translation);
    }

    #[allow(dead_code)]
    pub fn roll(&mut self, roll: f32) {
        self.roll += roll
    }

    pub fn set_roll(&mut self, roll: f32) {
        self.roll = roll
    }
}

/// Apply a mutating function to the value wrapped in an Arc<Helix>. This will make `helix_ptr`
/// point to a new helix on which the update has been applied.
pub fn mutate_in_arc<F, Obj: Clone>(obj_ptr: &mut Arc<Obj>, mut mutation: F)
where
    F: FnMut(&mut Obj),
{
    let mut new_obj = Obj::clone(&obj_ptr);
    mutation(&mut new_obj);
    *obj_ptr = Arc::new(new_obj)
}

/// Apply a mutating fucntion to all the helices of a design.
pub fn mutate_all_helices<F>(design: &mut Design, mutation: F)
where
    F: FnMut(&mut Helix) + Clone,
{
    let mut new_helices_map = BTreeMap::clone(&design.helices);
    for h in new_helices_map.values_mut() {
        mutate_in_arc(h, mutation.clone())
    }
    design.helices = Arc::new(new_helices_map);
}

pub fn mutate_one_helix<F>(design: &mut Design, h_id: usize, mutation: F) -> Option<()>
where
    F: FnMut(&mut Helix) + Clone,
{
    let mut new_helices_map = BTreeMap::clone(&design.helices);
    new_helices_map
        .get_mut(&h_id)
        .map(|h| mutate_in_arc(h, mutation))?;
    design.helices = Arc::new(new_helices_map);
    Some(())
}

pub fn mutate_one_grid<F>(design: &mut Design, g_id: usize, mut mutation: F) -> Option<()>
where
    F: FnMut(&mut GridDescriptor) + Clone,
{
    let mut new_grids_map = Vec::clone(&design.grids);
    new_grids_map.get_mut(g_id).map(|g| mutation(g))?;
    design.grids = Arc::new(new_grids_map);
    Some(())
}

#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub struct Nucl {
    pub helix: usize,
    pub position: isize,
    pub forward: bool,
}

impl Nucl {
    pub fn new(helix: usize, position: isize, forward: bool) -> Self {
        Self {
            helix,
            position,
            forward,
        }
    }

    pub fn left(&self) -> Self {
        Self {
            position: self.position - 1,
            ..*self
        }
    }

    pub fn right(&self) -> Self {
        Self {
            position: self.position + 1,
            ..*self
        }
    }

    pub fn prime3(&self) -> Self {
        Self {
            position: if self.forward {
                self.position + 1
            } else {
                self.position - 1
            },
            ..*self
        }
    }

    pub fn prime5(&self) -> Self {
        Self {
            position: if self.forward {
                self.position - 1
            } else {
                self.position + 1
            },
            ..*self
        }
    }

    pub fn compl(&self) -> Self {
        Self {
            forward: !self.forward,
            ..*self
        }
    }

    pub fn is_neighbour(&self, other: &Nucl) -> bool {
        self.helix == other.helix
            && self.forward == other.forward
            && (self.position - other.position).abs() == 1
    }
}

impl std::fmt::Display for Nucl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.helix, self.position, self.forward)
    }
}

/// Represents the axis of an helix. At the moment it is a line. In the future it might also be a
/// bezier curve
#[derive(Debug, Clone)]
pub struct Axis {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Axis {
    pub fn transformed(&self, model_matrix: &Mat4) -> Self {
        let origin = model_matrix.transform_point3(self.origin);
        let direction = model_matrix.transform_vec3(self.direction);
        Self { origin, direction }
    }
}

fn count_leq(set: &BTreeSet<isize>, x: isize) -> isize {
    let mut ret = 0;
    for _ in set.iter().take_while(|y| **y <= x) {
        ret += 1
    }
    ret
}

/// Add the correct juction between current and next to junctions.
/// Assumes and preseve the following invariant
/// Invariant [read_junctions::PrevDomain]: One of the following is true
/// * the strand is not cyclic
/// * the strand is cyclic and its first domain is NOT and insertion.
/// * previous domain points to some Domain::HelixDomain.
///
/// Moreover at the end of each iteration of the loop, previous_domain points to some
/// Domain::HelixDomain. The loop is responsible for preserving the invariant. The invariant is
/// true at initilasation if [SaneDomains] is true.
fn add_juction<'b, 'a: 'b>(
    junctions: &'b mut Vec<DomainJunction>,
    current: &'a Domain,
    next: &'a Domain,
    previous_domain: &'b mut &'a Domain,
    cyclic: bool,
    i: usize,
) {
    match next {
        Domain::Insertion(_) => {
            junctions.push(DomainJunction::Adjacent);
            if let Domain::HelixDomain(_) = current {
                *previous_domain = current;
            } else {
                panic!("Invariant violated: SaneDomains");
            }
        }
        Domain::HelixDomain(prime3) => {
            match current {
                Domain::Insertion(_) => {
                    if i == 0 && !cyclic {
                        // The first domain IS an insertion
                        junctions.push(DomainJunction::Adjacent);
                    } else {
                        // previous domain MUST point to some Domain::HelixDomain.
                        if let Domain::HelixDomain(prime5) = *previous_domain {
                            junctions.push(junction(prime5, prime3))
                        } else {
                            if i == 0 {
                                panic!("Invariant violated: SaneDomains");
                            } else {
                                panic!("Invariant violated: read_junctions::PrevDomain");
                            }
                        }
                    }
                }
                Domain::HelixDomain(prime5) => {
                    junctions.push(junction(prime5, prime3));
                    *previous_domain = current;
                }
            }
        }
    }
}

/// Infer juctions from a succession of domains.
pub fn read_junctions(domains: &[Domain], cyclic: bool) -> Vec<DomainJunction> {
    if domains.len() == 0 {
        return vec![];
    }

    let mut ret = Vec::with_capacity(domains.len());
    let mut previous_domain = &domains[domains.len() - 1];

    for i in 0..(domains.len() - 1) {
        let current = &domains[i];
        let next = &domains[i + 1];
        add_juction(&mut ret, current, next, &mut previous_domain, cyclic, i);
    }

    if cyclic {
        let last = &domains[domains.len() - 1];
        let first = &domains[0];
        add_juction(
            &mut ret,
            last,
            first,
            &mut previous_domain,
            cyclic,
            domains.len() - 1,
        );
    } else {
        ret.push(DomainJunction::Prime3)
    }

    ret
}

/// Return the appropriate junction between two HelixInterval
fn junction(prime5: &HelixInterval, prime3: &HelixInterval) -> DomainJunction {
    let prime5_nucl = prime5.prime3();
    let prime3_nucl = prime3.prime5();

    if prime3_nucl == prime5_nucl.prime3() {
        DomainJunction::Adjacent
    } else {
        DomainJunction::UnindentifiedXover
    }
}

/// The return type for methods that ask if a nucleotide is the end of a domain/strand/xover
#[derive(Debug, Clone, Copy)]
pub enum Extremity {
    No,
    Prime3,
    Prime5,
}

impl Extremity {
    pub fn is_3prime(&self) -> bool {
        match self {
            Extremity::Prime3 => true,
            _ => false,
        }
    }

    pub fn is_5prime(&self) -> bool {
        match self {
            Extremity::Prime5 => true,
            _ => false,
        }
    }

    pub fn is_end(&self) -> bool {
        match self {
            Extremity::No => false,
            _ => true,
        }
    }

    pub fn to_opt(&self) -> Option<bool> {
        match self {
            Extremity::No => None,
            Extremity::Prime3 => Some(true),
            Extremity::Prime5 => Some(false),
        }
    }
}
