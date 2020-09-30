/// This module defines the icednano format.
/// All other format supported by icednano are converted into this format and run-time manipulation
/// of designs are performed on an `icednano::Design` structure

use std::borrow::Cow;
use std::f32::consts::PI;

use ultraviolet::Vec3;

/// The `icednano` Design structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct Design {
    /// The vector of all helices used in this design. Helices have a
    /// position and an orientation in 3D.
    pub helices: Vec<Helix>,
    /// The vector of strands.
    pub strands: Vec<Strand>,
    /// Parameters of DNA geometry. This can be skipped (in JSON), or
    /// set to `None` in Rust, in which case a default set of
    /// parameters from the literature is used.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parameters: Option<Parameters>,
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
    #[serde(serialize_with = "hexa_u32", default)]
    pub color: u32,
}

fn is_false(x: &bool) -> bool {
    !*x
}

fn hexa_u32<S>(x: &u32, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    serializer.serialize_str(&format!("{:#08X}", x))
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
    pub helix: isize,
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
}

/// A DNA helix. All bases of all strands must be on a helix.
///
/// The three angles are illustrated in the following image, from [the NASA website](https://www.grc.nasa.gov/www/k-12/airplane/rotations.html):
/// Angles are applied in the order yaw -> pitch -> roll
/// ![Aircraft angles](https://www.grc.nasa.gov/www/k-12/airplane/Images/rotations.gif)
#[derive(Serialize, Deserialize, Clone)]
pub struct Helix {
    /// Position of the origin of the helix axis.
    #[serde(default = "Vec3::zero")]
    pub position: Vec3,

    /// Angle around the axis of the helix.
    #[serde(default = "zero_f32")]
    pub roll: f32,

    /// Horizontal rotation.
    #[serde(default = "zero_f32")]
    pub yaw: f32,

    /// Vertical rotation.
    #[serde(default = "zero_f32")]
    pub pitch: f32,

}

pub fn zero_f32() -> f32 {
    0f32
}


const KELLY: [u32; 19] = [
    0xF3C300,
    0x875692, // 0xF38400, // Orange, too close to others
    0xA1CAF1,
    0xBE0032,
    0xC2B280,
    0x848482,
    0x008856,
    0xE68FAC,
    0x0067A5,
    0xF99379,
    0x604E97,
    0xF6A600,
    0xB3446C,
    0xDCD300,
    0x882D17,
    0x8DB600,
    0x654522,
    0xE25822,
    0x2B3D26,
];

impl Helix {
    /// Angle of base number `n` around this helix.
    pub fn theta(&self, n: isize, forward: bool, cst: &Parameters) -> f32 {
        let shift = if forward { cst.groove_angle } else { 0. };
        n as f32 * 2. * PI / cst.bases_per_turn + shift + self.roll + PI
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
        /*
        ret = Helix::ry(&ret, self.yaw);
        ret = Helix::rz(&ret, self.pitch);
        */
        ret += self.position;
        ret
    }

    pub(crate) fn rotate_point(&self, ret: Vec3) -> Vec3 {
        let forward = [
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.sin() * self.pitch.cos(),
        ];
        let right = [self.yaw.sin(), 0., self.yaw.cos()];
        let up = [
            right[1] * forward[2] - right[2] * forward[1],
            right[2] * forward[0] - right[0] * forward[2],
            right[0] * forward[1] - right[1] * forward[0],
        ];

        Vec3::new(
            ret[0] * forward[0] + ret[1] * up[0] + ret[2] * right[0],
            ret[0] * forward[1] + ret[1] * up[1] + ret[2] * right[1],
            ret[0] * forward[2] + ret[1] * up[2] + ret[2] * right[2],
        )
    }
}
