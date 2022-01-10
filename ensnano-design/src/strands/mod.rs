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

use super::scadnano::*;
use super::{codenano, Nucl};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
mod formating;

/// A collection of strands, that maps strand identifier to strands.
///
/// It contains all the information about the "topology of the design".  Information about
/// cross-over or helix interval are obtained via this structure
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Strands(pub(super) BTreeMap<usize, Strand>);

impl Strands {
    pub fn get_xovers(&self) -> Vec<(Nucl, Nucl)> {
        let mut ret = vec![];
        for s in self.0.values() {
            for x in s.xovers() {
                ret.push(x)
            }
        }
        ret
    }

    pub fn get_intervals(&self) -> BTreeMap<usize, (isize, isize)> {
        let mut ret = BTreeMap::new();
        for s in self.0.values() {
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
        for (s_id, s) in self.0.iter() {
            if s.has_nucl(nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    pub fn remove_empty_domains(&mut self) {
        for s in self.0.values_mut() {
            s.remove_empty_domains()
        }
    }

    pub fn has_at_least_on_strand_with_insertions(&self) -> bool {
        self.0.values().any(|s| s.has_insertions())
    }

    /// Return the strand end status of nucl
    pub fn is_strand_end(&self, nucl: &Nucl) -> Extremity {
        for s in self.0.values() {
            if !s.cyclic && s.get_5prime() == Some(*nucl) {
                return Extremity::Prime5;
            } else if !s.cyclic && s.get_3prime() == Some(*nucl) {
                return Extremity::Prime3;
            }
        }
        return Extremity::No;
    }

    pub fn is_domain_end(&self, nucl: &Nucl) -> Extremity {
        for strand in self.0.values() {
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
        for s in self.0.values() {
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

    // Collection methods
    //============================================================================================
    pub fn get(&self, id: &usize) -> Option<&Strand> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &usize) -> Option<&mut Strand> {
        self.0.get_mut(id)
    }

    pub fn insert(&mut self, key: usize, strand: Strand) -> Option<Strand> {
        self.0.insert(key, strand)
    }

    pub fn remove(&mut self, key: &usize) -> Option<Strand> {
        self.0.remove(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &usize> {
        self.0.keys()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut Strand)> {
        self.0.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&usize, &Strand)> {
        self.0.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &Strand> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Strand> {
        self.0.values_mut()
    }
    //============================================================================================
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

// used to serialize `Strand.cyclic`
fn is_false(x: &bool) -> bool {
    !*x
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

    pub(super) fn remove_empty_domains(&mut self) {
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

// used in Domain::from_scadnano
fn count_leq(set: &BTreeSet<isize>, x: isize) -> isize {
    let mut ret = 0;
    for _ in set.iter().take_while(|y| **y <= x) {
        ret += 1
    }
    ret
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
