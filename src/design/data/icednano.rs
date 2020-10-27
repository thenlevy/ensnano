/// This module defines the icednano format.
/// All other format supported by icednano are converted into this format and run-time manipulation
/// of designs are performed on an `icednano::Design` structure
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::f32::consts::PI;

use ultraviolet::{Mat4, Rotor3, Vec3};

use super::codenano;
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
        }
    }

    pub fn new() -> Self {
        Self {
            helices: BTreeMap::new(),
            strands: BTreeMap::new(),
            parameters: Some(Parameters::DEFAULT),
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

    pub fn init(helix: usize, position: isize, forward: bool) -> Self {
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
            color: 0xFF_FF_FF,
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
#[derive(Serialize, Deserialize, Clone)]
pub struct Helix {
    /// Position of the origin of the helix axis.
    pub position: Vec3,

    /// Orientation of the helix
    pub orientation: Rotor3,

    #[serde(default, skip_serializing)]
    old_position: Vec3,
    #[serde(default, skip_serializing)]
    old_orientation: Rotor3,
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
        }
    }
}

impl Helix {
    /// Angle of base number `n` around this helix.
    pub fn theta(&self, n: isize, forward: bool, cst: &Parameters) -> f32 {
        let shift = if forward { cst.groove_angle } else { 0. };
        n as f32 * 2. * PI / cst.bases_per_turn + shift + PI
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

    pub fn end_movement(&mut self) {
        self.old_position = self.position;
        self.old_orientation = self.orientation;
    }

    #[allow(dead_code)]
    pub fn roll(&mut self, roll: f32) {
        self.orientation = self.orientation * Rotor3::from_rotation_xy(roll)
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

    pub fn compl(&self) -> Self {
        Self {
            forward: !self.forward,
            ..*self
        }
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
