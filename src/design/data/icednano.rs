/// This module defines the icednano format.
/// All other format supported by icednano are converted into this format and run-time manipulation
/// of designs are performed on an `icednano::Design` structure
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::f32::consts::PI;

use ultraviolet::{Isometry2, Mat4, Rotor3, Vec3};

use super::codenano;
use super::grid::{Grid, GridDescriptor, GridPosition};
use super::strand_builder::{DomainIdentifier, NeighbourDescriptor};

/// The `icednano` Design structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct Design {
    /// The collection of all helices used in this design. Helices have a
    /// position and an orientation in 3D.
    pub helices: BTreeMap<usize, Helix>,
    /// The vector of strands.
    pub strands: BTreeMap<usize, Strand>,
    /// Parameters of DNA geometry. This can be skipped (in JSON), or
    /// set to `None` in Rust, in which case a default set of
    /// parameters from the literature is used.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parameters: Option<Parameters>,

    /// The strand that is the scaffold if the design is an origami
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaffold_id: Option<usize>,

    /// The sequence of the scaffold if the design is an origami
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaffold_sequence: Option<String>,

    /// The shifting of the scaffold if the design is an origami. This is used to reduce the number
    /// of anti-patern in the stapples sequences
    pub scaffold_shift: Option<usize>,

    #[serde(default)]
    pub grids: Vec<GridDescriptor>,

    /// The groups in which the helices are.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub groups: BTreeMap<usize, bool>,

    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub no_phantoms: HashSet<usize>,

    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub small_shperes: HashSet<usize>,
}

impl Design {
    pub fn from_codenano<Sl, Dl>(codenano_desgin: &codenano::Design<Sl, Dl>) -> Self {
        let mut helices = BTreeMap::new();
        for (i, helix) in codenano_desgin.helices.iter().enumerate() {
            helices.insert(i, Helix::from_codenano(helix));
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
            helices,
            strands,
            parameters: Some(parameters),
            grids: Vec::new(),
            scaffold_id: None,
            scaffold_sequence: None,
            scaffold_shift: None,
            groups: Default::default(),
            small_shperes: Default::default(),
            no_phantoms: Default::default(),
        }
    }

    pub fn new() -> Self {
        Self {
            helices: BTreeMap::new(),
            strands: BTreeMap::new(),
            parameters: Some(Parameters::DEFAULT),
            grids: Vec::new(),
            scaffold_id: None,
            scaffold_sequence: None,
            scaffold_shift: None,
            groups: Default::default(),
            small_shperes: Default::default(),
            no_phantoms: Default::default(),
        }
    }

    pub fn get_neighbour_nucl(&self, nucl: Nucl) -> Option<NeighbourDescriptor> {
        for (s_id, s) in self.strands.iter() {
            for (d_id, d) in s.domains.iter().enumerate() {
                if let Some(other) = d.other_end(nucl) {
                    return Some(NeighbourDescriptor {
                        identifier: DomainIdentifier {
                            strand: *s_id,
                            domain: d_id,
                        },
                        fixed_end: other,
                        initial_moving_end: nucl.position,
                        moving_end: nucl.position,
                    });
                }
            }
        }
        None
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
}

/// A DNA strand. Strands are represented as sequences of `Domains`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strand {
    /// The (ordered) vector of domains, where each domain is a
    /// directed interval of a helix.
    pub domains: Vec<Domain>,
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
}

impl Strand {
    pub fn from_codenano<Sl, Dl>(codenano_strand: &codenano::Strand<Sl, Dl>) -> Self {
        let domains = codenano_strand
            .domains
            .iter()
            .map(|d| Domain::from_codenano(d))
            .collect();
        Self {
            domains,
            sequence: codenano_strand.sequence.clone(),
            cyclic: codenano_strand.cyclic,
            color: codenano_strand
                .color
                .clone()
                .unwrap_or_else(|| codenano_strand.default_color())
                .as_int(),
        }
    }

    pub fn init(helix: usize, position: isize, forward: bool, color: u32) -> Self {
        let domains = vec![Domain::HelixDomain(HelixInterval {
            sequence: None,
            start: position,
            end: position + 1,
            helix,
            forward,
        })];
        Self {
            domains,
            sequence: None,
            cyclic: false,
            color,
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
        ret
    }
}

fn is_false(x: &bool) -> bool {
    !*x
}

/// A domain can be either an interval of nucleotides on an helix, or an "Insertion" that is a set
/// of nucleotides that are not on an helix and form an independent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Domain {
    /// An interval of nucleotides on an helix
    HelixDomain(HelixInterval),
    /// A set of nucleotides not on an helix.
    Insertion(usize),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

    #[allow(dead_code)]
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
    /// Default values for the parameters of DNA, taken from the litterature.
    pub const DEFAULT: Parameters = Parameters {
        // z-step and helix radius from:
        //
        // Single-molecule portrait of DNA and RNA double helices,
        // J. Ricardo Arias-Gonzalez, Integrative Biology, Royal
        // Society of Chemistry, 2014, vol. 6, p.904
        z_step: 0.332,
        helix_radius: 1.,
        // bases per turn from Woo Rothemund (Nature Chemistry).
        bases_per_turn: 10.44,
        groove_angle: -24. * PI / 34.,
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

    #[serde(default = "default_visibility")]
    pub visible: bool,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub grid_position: Option<GridPosition>,

    #[serde(default, skip_serializing)]
    old_position: Vec3,
    #[serde(default, skip_serializing)]
    old_orientation: Rotor3,

    /// Representation of the helix in 2d
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub isometry2d: Option<Isometry2>,

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
            old_position: position,
            old_orientation: orientation,
            grid_position: None,
            isometry2d: None,
            visible: true,
            roll: 0f32,
        }
    }
}

impl Helix {
    pub fn new_on_grid(grid: &Grid, x: isize, y: isize, g_id: usize) -> Self {
        let position = grid.position_helix(x, y);
        Self {
            position,
            orientation: grid.orientation,
            old_orientation: grid.orientation,
            old_position: position,
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
        }
    }

    /// Angle of base number `n` around this helix.
    pub fn theta(&self, n: isize, forward: bool, cst: &Parameters) -> f32 {
        let shift = if forward { cst.groove_angle } else { 0. };
        n as f32 * 2. * PI / cst.bases_per_turn + shift + PI + self.roll
    }

    /// 3D position of a nucleotide on this helix. `n` is the position along the axis, and `forward` is true iff the 5' to 3' direction of the strand containing that nucleotide runs in the same direction as the axis of the helix.
    pub fn space_pos(&self, p: &Parameters, n: isize, forward: bool) -> Vec3 {
        let theta = self.theta(n, forward, p);
        let mut ret = Vec3::new(
            n as f32 * p.z_step,
            -theta.cos() * p.helix_radius,
            -theta.sin() * p.helix_radius,
        );

        ret = self.rotate_point(ret);
        ret += self.position;
        ret
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

    pub(crate) fn rotate_point(&self, ret: Vec3) -> Vec3 {
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
        self.orientation = self.old_orientation;
        self.position = self.old_position;
        self.append_translation(-origin);
        self.append_rotation(rotation);
        self.append_translation(origin);
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.position = self.old_position;
        self.append_translation(translation);
    }

    pub fn end_movement(&mut self) {
        self.old_position = self.position;
        self.old_orientation = self.orientation;
    }

    #[allow(dead_code)]
    pub fn roll(&mut self, roll: f32) {
        self.roll -= roll
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct Nucl {
    pub position: isize,
    pub helix: usize,
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
