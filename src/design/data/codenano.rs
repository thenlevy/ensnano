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
use std::borrow::Cow;
use std::f64::consts::PI;
use std::fmt;
use ultraviolet::DVec3;

/// The main type of this crate, describing a DNA design.
#[derive(Serialize, Deserialize, Clone)]
pub struct Design<StrandLabel, DomainLabel> {
    /// Version of this format.
    pub version: String,
    /// The vector of all helices used in this design. Helices have a
    /// position and an orientation in 3D.
    pub helices: Vec<Helix>,
    /// The vector of strands.
    pub strands: Vec<Strand<StrandLabel, DomainLabel>>,
    /// Parameters of DNA geometry. This can be skipped (in JSON), or
    /// set to `None` in Rust, in which case a default set of
    /// parameters from the literature is used.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parameters: Option<Parameters>,
}

impl<StrandLabel: serde::Serialize, DomainLabel: serde::Serialize>
    Design<StrandLabel, DomainLabel>
{
    /// Initiates a design.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Design {
            version: env!("CARGO_PKG_VERSION").to_string(),
            helices: Vec::new(),
            strands: Vec::new(),
            parameters: Some(Parameters::DEFAULT),
        }
    }
}

/// A DNA strand.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Strand<Label, DomainLabel> {
    /// The (ordered) vector of domains, where each domain is a
    /// directed interval of a helix.
    pub domains: Vec<Domain<DomainLabel>>,
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub color: Option<Color>,
    /// An optional label for the strand. Can be
    /// `serde_json::Value::Null`, and skipped in the serialisation.
    #[serde(skip_serializing_if = "Option::is_none", default = "none")]
    pub label: Option<Label>,
}

fn is_false(x: &bool) -> bool {
    !*x
}

fn none<Label>() -> Option<Label> {
    None
}

impl<StrandLabel, DomainLabel> Strand<StrandLabel, DomainLabel> {
    /// Provide a default color to the strand.
    pub fn default_color(&self) -> Color {
        if let Some(domain) = self.domains.get(0) {
            let x1 = if domain.forward {
                domain.end - 1
            } else {
                domain.start
            };
            let h = domain.helix as isize;
            let x = x1 + (x1 % 11) + 5 * h;
            let n = KELLY.len() as isize;
            return KELLY[(((x % n) + n) % n) as usize].clone();
        }
        Color::Int(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
/// Colors
pub enum Color {
    /// Colors encoded as u32.
    Int(u32),
    /// Hexadecimal colors
    Hex(String),
    /// Three distinct fields for red, green and blue
    Rgb {
        /// Red field
        r: u8,
        /// Green field
        g: u8,
        /// Blue field
        b: u8,
    },
}

impl Color {
    /// Returns the u32 encoding this color.
    pub fn as_int(&self) -> u32 {
        match *self {
            Color::Int(n) => n,
            Color::Hex(ref s) => {
                let s = s.trim_start_matches("0x");
                let s = s.trim_start_matches('#');
                u32::from_str_radix(s, 16).unwrap()
            }
            Color::Rgb { r, g, b } => ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
        }
    }

    #[allow(dead_code)]
    /// Kelly color number `n`.
    pub fn kelly(n: usize) -> Self {
        KELLY[n % KELLY.len()].clone()
    }
}

/// A domain, i.e. an interval of a helix.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Domain<Label> {
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
    /// An optional label that can be attached to strands.
    #[serde(skip_serializing_if = "Option::is_none", default = "none")]
    pub label: Option<Label>,
    /// In addition to the strand-level sequence, individual domains
    /// may have sequences too. The precedence has to be defined by
    /// the user of this library.
    pub sequence: Option<Cow<'static, str>>,
}

impl<Label> Domain<Label> {
    /// Iterate through the positions of this domain, in 5' to 3'
    /// order (meaning that the values produced by this iterator might
    /// be increasing or decreasing).
    #[allow(dead_code)]
    pub fn iter(&self) -> DomainIter {
        DomainIter {
            start: self.start,
            end: self.end,
            forward: self.forward,
        }
    }
    #[allow(dead_code)]
    /// Translate this domain. The first parameter is the translation
    /// along the helix, the second one is a translation across
    /// helices (probably most meaningful for a flat design).
    pub fn translate(self, dx: isize, dy: isize) -> Self {
        use std::convert::TryFrom;
        Domain {
            start: self.start + dx,
            end: self.end + dx,
            helix: usize::try_from(self.helix as isize + dy).unwrap() as isize,
            ..self
        }
    }
    #[allow(dead_code)]
    /// Translate this domain along its helix.
    pub fn shift_x(self, dx: isize) -> Self {
        Domain {
            start: self.start + dx,
            end: self.end + dx,
            ..self
        }
    }
    #[allow(dead_code)]
    /// Translate this domain to a different helix (probably most
    /// meaningful for a flat design).
    pub fn shift_y(self, dy: isize) -> Self {
        use std::convert::TryFrom;
        Domain {
            helix: usize::try_from(self.helix as isize + dy).unwrap() as isize,
            ..self
        }
    }

    #[allow(dead_code)]
    /// Number of Nucleotides on the domain
    pub fn length(&self) -> isize {
        self.end - self.start
    }

    #[allow(dead_code)]
    /// Return a domain that has the same bounds as self
    pub fn pseudo_copy(&self) -> Self {
        Domain {
            helix: self.helix,
            start: self.start,
            end: self.end,
            forward: self.forward,
            label: None,
            sequence: None,
        }
    }

    #[allow(dead_code)]
    /// Return true iff `self` contains the nucleotide (h, x, b)
    pub fn contains(&self, h: isize, x: isize, b: bool) -> bool {
        self.helix == h && self.forward == b && self.start <= x && self.end > x
    }

    #[allow(dead_code)]
    /// Return the first nucl of `self` or `None` if `self` is empty
    pub fn first_nucl(&self) -> Option<(isize, isize, bool)> {
        if self.start >= self.end {
            None
        } else if self.forward {
            Some((self.helix, self.start, self.forward))
        } else {
            Some((self.helix, self.end - 1, self.forward))
        }
    }

    #[allow(dead_code)]
    /// Return the last nucl of `self` or `None` if `self` is empty
    pub fn last_nucl(&self) -> Option<(isize, isize, bool)> {
        if self.start >= self.end {
            None
        } else if self.forward {
            Some((self.helix, self.end - 1, self.forward))
        } else {
            Some((self.helix, self.start, self.forward))
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
    pub z_step: f64,
    /// Radius of a helix, in nanometers.
    pub helix_radius: f64,
    /// Number of bases per turn in nanometers.
    pub bases_per_turn: f64,
    /// Minor groove angle. DNA helices have a "minor groove" and a
    /// "major groove", meaning that two paired nucleotides are not at
    /// opposite positions around a double helix (i.e. at an angle of
    /// 180°), but instead have a different angle.
    ///
    /// Strands are directed. The "normal" direction is called "5' to
    /// 3'" (named after parts of the nucleotides). This parameter is
    /// the small angle, which is clockwise from the normal strand to
    /// the reverse strand.
    pub groove_angle: f64,

    /// Gap between two neighbouring helices.
    pub inter_helix_gap: f64,
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

/// Represents 3D coordinates of the point of a finite element system
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct Point {
    /// x coordinate
    pub x: f64,
    /// y coordinate
    pub y: f64,
    /// z coordinate
    pub z: f64,
}

impl Point {
    /// Convert an array of 3 floats into a Point
    #[allow(dead_code)]
    pub fn from_coord(coord: [f64; 3]) -> Self {
        Point {
            x: coord[0],
            y: coord[1],
            z: coord[2],
        }
    }

    #[allow(dead_code)]
    pub fn to_vec3(&self) -> DVec3 {
        DVec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

/// A DNA helix. All bases of all strands must be on a helix.
///
/// The three angles are illustrated in the following image, from [the NASA website](https://www.grc.nasa.gov/www/k-12/airplane/rotations.html):
///
/// ![Aircraft angles](https://www.grc.nasa.gov/www/k-12/airplane/Images/rotations.gif)
#[derive(Serialize, Deserialize, Clone)]
pub struct Helix {
    /// Position of the position of the helix axis.
    #[serde(default = "zero_point")]
    pub position: Point,

    /// Angle around the axis of the helix.
    #[serde(default = "zero_f64")]
    pub roll: f64,

    /// Horizontal rotation.
    #[serde(default = "zero_f64")]
    pub yaw: f64,

    /// Vertical rotation.
    #[serde(default = "zero_f64")]
    pub pitch: f64,

    /// Maximum available position of the helix.
    pub max_offset: Option<isize>,

    /// Bold tickmarks.
    pub major_ticks: Option<Vec<isize>>,
}

fn zero_point() -> Point {
    Point {
        x: 0.,
        y: 0.,
        z: 0.,
    }
}
fn zero_f64() -> f64 {
    0.
}

impl fmt::Debug for Helix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").field(&self.position).finish()
    }
}

impl Helix {
    /// Angle of base number `n` around this helix.
    #[allow(dead_code)]
    pub fn theta(&self, n: isize, forward: bool, cst: &Parameters) -> f64 {
        let shift = if forward { cst.groove_angle } else { 0. };
        n as f64 * 2. * PI / cst.bases_per_turn + shift + self.roll + PI
    }

    /// 3D position of a nucleotide on this helix. `n` is the position along the axis, and `forward` is true iff the 5' to 3' direction of the strand containing that nucleotide runs in the same direction as the axis of the helix.
    #[allow(dead_code)]
    pub fn space_pos(&self, p: &Parameters, n: isize, forward: bool) -> [f64; 3] {
        let theta = self.theta(n, forward, p);
        let mut ret = [
            n as f64 * p.z_step,
            -theta.cos() * p.helix_radius,
            -theta.sin() * p.helix_radius,
        ];

        ret = self.rotate_point(ret);
        /*
        ret = Helix::ry(&ret, self.yaw);
        ret = Helix::rz(&ret, self.pitch);
        */
        ret[0] += self.position.x;
        ret[1] += self.position.y;
        ret[2] += self.position.z;
        ret
    }

    pub(crate) fn rotate_point(&self, ret: [f64; 3]) -> [f64; 3] {
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

        [
            ret[0] * forward[0] + ret[1] * up[0] + ret[2] * right[0],
            ret[0] * forward[1] + ret[1] * up[1] + ret[2] * right[1],
            ret[0] * forward[2] + ret[1] * up[2] + ret[2] * right[2],
        ]
    }

    #[allow(dead_code)]
    /// Return a basis of the Helix PoV
    pub fn basis(&self) -> [[f64; 3]; 3] {
        [
            self.rotate_point([1., 0., 0.]),
            self.rotate_point([0., self.roll.cos(), self.roll.sin()]),
            self.rotate_point([0., -self.roll.sin(), self.roll.cos()]),
        ]
    }

    #[allow(dead_code)]
    /// 3D position of the projection of the nucleotide on its helix.
    /// `n` is the position along the axis.
    pub fn axis_pos(&self, p: &Parameters, n: isize) -> DVec3 {
        let mut ret = [n as f64 * p.z_step, 0., 0.];

        ret = self.rotate_point(ret);

        ret[0] += self.position.x;
        ret[1] += self.position.y;
        ret[2] += self.position.z;
        ret.into()
    }

    #[allow(dead_code)]
    /// Test if two helices overlap.
    pub fn overlap(&self, other: &Helix, p: &Parameters) -> bool {
        let dir_vec = self.axis_pos(p, 1) - self.position.to_vec3();
        let vec1 = other.axis_pos(p, 30) - self.position.to_vec3();
        if vec1.cross(dir_vec).mag() / dir_vec.mag() > p.helix_radius {
            false
        } else {
            let vec2 = other.axis_pos(p, -30) - self.position.to_vec3();
            vec2.cross(dir_vec).mag() / dir_vec.mag() < p.helix_radius
        }
    }

    #[allow(dead_code)]
    /// A clone of `self` translated by one step along the y vector
    pub fn clone_up(&self, p: &Parameters) -> Self {
        let mut new_position = [0., p.helix_radius * 2. + p.inter_helix_gap, 0.];
        new_position = self.rotate_point(new_position);
        new_position[0] += self.position.x;
        new_position[1] += self.position.y;
        new_position[2] += self.position.z;
        Helix {
            position: Point::from_coord(new_position),
            ..self.clone()
        }
    }

    #[allow(dead_code)]
    /// A clone of `self` translated by minus one step along the y vector
    pub fn clone_down(&self, p: &Parameters) -> Self {
        let mut new_position = [0., -p.helix_radius * 2. - p.inter_helix_gap, 0.];
        new_position = self.rotate_point(new_position);
        new_position[0] += self.position.x;
        new_position[1] += self.position.y;
        new_position[2] += self.position.z;
        Helix {
            position: Point::from_coord(new_position),
            ..self.clone()
        }
    }

    #[allow(dead_code)]
    /// A clone of `self` translated by minus one step along the z vector
    pub fn clone_left(&self, p: &Parameters) -> Self {
        let mut new_position = [0., 0., -p.helix_radius * 2. - p.inter_helix_gap];
        new_position = self.rotate_point(new_position);
        new_position[0] += self.position.x;
        new_position[1] += self.position.y;
        new_position[2] += self.position.z;
        Helix {
            position: Point::from_coord(new_position),
            ..self.clone()
        }
    }

    #[allow(dead_code)]
    /// A clone of `self` translated by one step along the z vector
    pub fn clone_forward(&self, p: &Parameters) -> Self {
        let mut new_position = [0., 0., p.helix_radius * 2. + p.inter_helix_gap];
        new_position = self.rotate_point(new_position);
        new_position[0] += self.position.x;
        new_position[1] += self.position.y;
        new_position[2] += self.position.z;
        Helix {
            position: Point::from_coord(new_position),
            ..self.clone()
        }
    }

    #[allow(dead_code)]
    /// Return the position on axis that is the closest to the point given in argument
    pub fn closest_nucl(&self, point: [f64; 3], p: &Parameters) -> isize {
        let point: DVec3 = point.into();
        let mut up = 10000;
        let mut low = -10000;
        while up - low > 1 {
            let point_low = self.axis_pos(p, low);
            let point_up = self.axis_pos(p, up);
            let dist_low = (point_low - point).mag();
            let dist_up = (point_up - point).mag();
            if dist_low > dist_up {
                low = (up + low) / 2;
            } else {
                up = (up + low) / 2;
            }
        }
        let point_low = self.axis_pos(p, low);
        let point_up = self.axis_pos(p, up);
        let dist_low = (point_low - point).mag();
        let dist_up = (point_up - point).mag();
        if dist_low > dist_up {
            up
        } else {
            low
        }
    }
}

const KELLY: [Color; 19] = [
    // 0xF2F3F4, // White
    // 0x222222, // Black,
    Color::Int(0xF3C300),
    Color::Int(0x875692), // 0xF38400, // Orange, too close to others
    Color::Int(0xA1CAF1),
    Color::Int(0xBE0032),
    Color::Int(0xC2B280),
    Color::Int(0x848482),
    Color::Int(0x008856),
    Color::Int(0xE68FAC),
    Color::Int(0x0067A5),
    Color::Int(0xF99379),
    Color::Int(0x604E97),
    Color::Int(0xF6A600),
    Color::Int(0xB3446C),
    Color::Int(0xDCD300),
    Color::Int(0x882D17),
    Color::Int(0x8DB600),
    Color::Int(0x654522),
    Color::Int(0xE25822),
    Color::Int(0x2B3D26),
];
