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
use super::*;
use ensnano_design::{InterpolatedCurveDescriptor, InterpolationDescriptor};
use ultraviolet::{DVec3, Similarity3};
#[derive(Debug, Clone)]
pub struct RevolutionSurfaceSystemDescriptor {
    pub scaffold_len_target: usize,
    pub target: RootedRevolutionSurface,
    pub dna_parameters: Parameters,
    pub simulation_parameters: RevolutionSimulationParameters,
}

#[derive(Debug, Clone)]
pub struct RootedRevolutionSurface {
    surface: UnrootedRevolutionSurfaceDescriptor,
    scale: f64,
    nb_spirals: usize,
    pub rooting_parameters: RootingParameters,
    area_radius_0: f64,
    area_per_radius_unit: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnrootedRevolutionSurfaceDescriptor {
    pub curve: CurveDescriptor2D,
    pub revolution_radius: RevolutionSurfaceRadius,
    pub half_turn_count: isize,
    pub curve_plane_position: Vec3,
    pub curve_plane_orientation: Rotor3,
}

#[derive(Debug, Clone)]
pub struct RootingParameters {
    pub nb_helix_per_half_section: usize,
    pub shift_per_turn: isize,
    pub junction_smoothening: f64,
    pub dna_parameters: Parameters,
}

impl UnrootedRevolutionSurfaceDescriptor {
    pub fn rooted(
        mut self,
        rooting_parameters: RootingParameters,
        compute_areas: bool,
    ) -> RootedRevolutionSurface {
        let nb_spirals = rooting_parameters.nb_spirals(self.half_turn_count);
        let (mut area_radius_0, mut area_per_radius_unit) = if compute_areas {
            self.area_affine_function().unwrap_or((1., 1.))
        } else {
            (1., 1.)
        };
        let scale = rooting_parameters.nb_helix_per_half_section as f64
            * 2.
            * Parameters::INTER_CENTER_GAP as f64
            / self.curve.perimeter();
        area_radius_0 *= scale;
        area_per_radius_unit *= scale;

        // In order to keep the same aspect ratio, the revolution radius needs to be rescaled as
        // well as the section.
        self.revolution_radius = self.revolution_radius.scaled(scale);

        RootedRevolutionSurface {
            surface: self,
            rooting_parameters,
            scale,
            nb_spirals,
            area_radius_0,
            area_per_radius_unit,
        }
    }
    pub fn get_frame(&self) -> Isometry3 {
        let Similarity3 {
            translation,
            rotation,
            ..
        } = self.get_frame_when_scaled(1.);
        Isometry3 {
            translation,
            rotation,
        }
    }

    fn get_frame_when_scaled(&self, scale: f64) -> Similarity3 {
        let mut ret = {
            let Isometry3 {
                translation,
                rotation,
            } = CurveDescriptor2D::get_frame_3d();
            let mut s = Similarity3 {
                translation,
                rotation,
                scale: 1.,
            };
            s.append_scaling(scale as f32);
            s
        };

        // Then convert into the plane's frame
        ret.append_rotation(self.curve_plane_orientation);
        ret.append_translation(self.curve_plane_position);

        // To get the rotation axis as it is drawn on the plane, we must scale the revolution
        // radius
        let mut scaled = self.clone();
        scaled.revolution_radius = scaled.revolution_radius.scaled(scale);
        // Center on the rotation axis as drawn on the plane
        let rotation_axis_translation = (Vec3::unit_z()
            * scaled.get_revolution_axis_position() as f32)
            .rotated_by(self.curve_plane_orientation);
        ret.append_translation(rotation_axis_translation);
        ret
    }

    pub fn get_revolution_axis_position(&self) -> f64 {
        self.get_axis_position_when_scaled(1.)
    }

    /// Return the position of the axis of revolution, assuming that the section is scaled.
    ///
    /// Note that the revolution radius is *not* scaled.
    fn get_axis_position_when_scaled(&self, scale: f64) -> f64 {
        use RevolutionSurfaceRadius::*;
        match self.revolution_radius {
            Left(x) => self.curve.min_x() * scale - x,
            Right(x) => x + self.curve.max_x() * scale,
            Inside(x) => x,
        }
    }

    pub fn set_axis_position(&mut self, position: f64) {
        self.set_axis_position_when_scaled(position, 1.)
    }

    /// Set the position of the axis of revolution, assuming that the section is scaled.
    ///
    /// Note that the revolution radius is *not* scaled.
    fn set_axis_position_when_scaled(&mut self, position: f64, scale: f64) {
        let min_x = self.curve.min_x() * scale;
        let max_x = self.curve.max_x() * scale;
        let new_radius = if position <= min_x {
            RevolutionSurfaceRadius::Left(min_x - position)
        } else if position >= max_x {
            RevolutionSurfaceRadius::Right(position - max_x)
        } else {
            RevolutionSurfaceRadius::Inside(position)
        };
        self.revolution_radius = new_radius;
    }

    /// Assuming that the area of the surface is of the form a*radius + b, return (b, a).
    pub fn area_affine_function(&self) -> Option<(f64, f64)> {
        let mut with_radius_0 = self.clone();
        let mut with_radius_1 = self.clone();
        match self.revolution_radius {
            RevolutionSurfaceRadius::Inside(_) => return None,
            RevolutionSurfaceRadius::Left(_) => {
                with_radius_0.revolution_radius = RevolutionSurfaceRadius::Left(0.);
                with_radius_1.revolution_radius = RevolutionSurfaceRadius::Left(1.);
            }
            RevolutionSurfaceRadius::Right(_) => {
                with_radius_0.revolution_radius = RevolutionSurfaceRadius::Right(0.);
                with_radius_1.revolution_radius = RevolutionSurfaceRadius::Right(1.);
            }
        };

        let area_0 = with_radius_0.approx_surface_area(1000, 1000)?;
        let area_1 = with_radius_1.approx_surface_area(1000, 1000)?;

        Some((area_0, area_1 - area_0))
    }

    /// Approximate the area of the surface by slicing it into strips of triangles.
    ///
    /// The surface is split into `nb_strip` strips of 2 * `nb_section_per_strip` triangles
    pub fn approx_surface_area(&self, nb_strip: usize, nb_section_per_strip: usize) -> Option<f64> {
        use rayon::prelude::*;

        if matches!(self.revolution_radius, RevolutionSurfaceRadius::Inside(_)) {
            return None;
        }

        let ret = (0..nb_strip)
            .into_par_iter()
            .map(|strip_idx| {
                // Parameters along the section for the top and bottom of the strip
                let s_high = strip_idx as f64 / nb_strip as f64;
                let s_low = s_high + 1. / nb_strip as f64;

                let vertices = (0..(nb_section_per_strip + 1)).flat_map(|section_idx| {
                    [s_high, s_low].into_iter().map(move |section_parameter| {
                        use std::f64::consts::TAU;
                        let revolution_fract = section_idx as f64 / nb_section_per_strip as f64;

                        let revolution_angle = TAU * revolution_fract;

                        self.position(section_parameter, revolution_angle)
                    })
                });

                area_strip(vertices, nb_section_per_strip)
            })
            .sum::<f64>();
        Some(ret)
    }

    fn position(&self, section_parameter: f64, revolution_angle: f64) -> DVec3 {
        self.position_when_scaled(section_parameter, revolution_angle, 1.)
    }

    fn position_when_scaled(
        &self,
        section_parameter: f64,
        revolution_angle: f64,
        scale: f64,
    ) -> DVec3 {
        use ensnano_design::PointOnSurface;
        let surface_point = PointOnSurface {
            revolution_angle,
            section_parameter,
            revolution_axis_position: self.get_axis_position_when_scaled(scale),
            section_half_turn_per_revolution: self.half_turn_count,
            curve_scale_factor: scale,
        };
        self.curve.point_on_surface(&surface_point)
    }

    pub fn shifts_to_get_n_spirals(
        &self,
        half_nb_helix: usize,
        nb_spirals: usize,
    ) -> Option<ShiftGenerator> {
        if nb_spirals == 0 || half_nb_helix == 0 || nb_spirals >= half_nb_helix {
            return None;
        }

        let nb_helix = half_nb_helix * 2;
        if nb_helix % nb_spirals == 0 {
            let additional_shift = if self.half_turn_count % 2 == 1 {
                half_nb_helix
            } else {
                0
            };
            let a = nb_helix / nb_spirals;
            let coprimes = (1..a)
                .filter(|n| gcd(*n as isize, a as isize) == 1)
                .collect();
            Some(ShiftGenerator {
                coprimes_with_a: coprimes,
                nb_spirals,
                additional_shift,
                nb_section: nb_helix,
            })
        } else {
            None
        }
    }
}

fn gcd(a: isize, b: isize) -> usize {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();

    if a < b {
        std::mem::swap(&mut a, &mut b);
    }

    while b > 0 {
        let b_ = b;
        b = a % b;
        a = b_;
    }

    a
}

impl RootingParameters {
    fn nb_spirals(&self, surface_half_turn_count: isize) -> usize {
        /*
         * let q be the total shift and n be the number of segments
         * Spirals seen as set of segments are class of equivalence for the relation ~
         * where a ~ b iff there exists k1, k2 st a = b  + k1 q + k2 n
         *
         * let d = gcd(q, n). If a ~ b then a = b (mod d)
         *
         * Recp. if a = b (mod d) there exists x y st xq + yn = d
         *
         * a = k (xq + yn) + b
         * so a ~ b
         *
         * So ~ is the relation of equivalence modulo d and has d classes.
         */
        let additional_shift = if surface_half_turn_count % 2 == 1 {
            self.nb_helix_per_half_section
        } else {
            0
        };
        let total_shift = self.shift_per_turn + additional_shift as isize;
        gcd(total_shift, self.nb_helix_per_half_section as isize * 2)
    }
}

/// A structure that can generate values of shift so that the resulting number of spirals is fixed.
pub struct ShiftGenerator {
    coprimes_with_a: Vec<usize>,
    nb_spirals: usize,
    additional_shift: usize,
    nb_section: usize,
}

impl ShiftGenerator {
    /// Return the i-th value generated by self, and check that self if still valid.
    pub fn ith_value(
        &self,
        i: isize,
        nb_spirals: usize,
        surface: &UnrootedRevolutionSurfaceDescriptor,
        half_nb_helix: usize,
    ) -> Option<isize> {
        self.still_valid(nb_spirals, surface, half_nb_helix)
            .then(|| {
                /*
                To get a rooting with `d` spirals within `k` segments. We must have
                `gcd(total_shift, k) = d` (see the implementation of `Rooting::Parameters::nb_spirals`)
                this means that
                (1) `d` divides `k`, so `k = a·d` for an integer `a`.
                (2) total_shift = b·d` where `a` and `b` are coprimes.

                So if `a = k / d` and `ℤ_a*` is the set of all numbers <= `a` that are coprime with `a`,
                the set of total_shift that give the desired amount of spirals is
                `Shifts_d = {(n·a + p) * d | n ∈ ℤ, p ∈ ℤ_a* }`
                */
                let nb_coprime = self.coprimes_with_a.len();
                let p = {
                    let idx = i.rem_euclid(nb_coprime as isize) as usize;
                    self.coprimes_with_a[idx] as isize
                };
                let n = i.div_euclid(nb_coprime as isize);
                let a = (self.nb_section / self.nb_spirals) as isize;

                let total_shift = (n * a + p) * nb_spirals as isize;
                total_shift - self.additional_shift as isize
            })
    }

    fn still_valid(
        &self,
        nb_spirals: usize,
        surface: &UnrootedRevolutionSurfaceDescriptor,
        half_nb_helix: usize,
    ) -> bool {
        if self.nb_spirals != nb_spirals {
            return false;
        }

        let nb_helix = half_nb_helix * 2;
        if nb_helix != self.nb_section {
            return false;
        }
        let expected_additional_shift = if surface.half_turn_count % 2 == 1 {
            half_nb_helix
        } else {
            0
        };
        self.additional_shift == expected_additional_shift
    }
}

#[derive(Debug, Clone)]
pub struct RevolutionSimulationParameters {
    pub nb_section_per_segment: usize,
    pub spring_stiffness: f64,
    pub torsion_stiffness: f64,
    pub fluid_friction: f64,
    pub ball_mass: f64,
    pub time_span: f64,
    pub simulation_step: f64,
    pub method: EquadiffSolvingMethod,
    pub rescaling: f64,
}

impl Default for RevolutionSimulationParameters {
    fn default() -> Self {
        consts::DEFAULT_REVOLUTION_SIMULATION_PARAMETERS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquadiffSolvingMethod {
    Euler,
    Ralston,
}

impl EquadiffSolvingMethod {
    pub const ALL_METHODS: &'static [Self] = &[Self::Euler, Self::Ralston];
}

impl ToString for EquadiffSolvingMethod {
    fn to_string(&self) -> String {
        match self {
            Self::Euler => "Euler".to_string(),
            Self::Ralston => "Ralston".to_string(),
        }
    }
}

/// The position of the axis of revolution of a revolution surface
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum RevolutionSurfaceRadius {
    /// The axis is on the left of the leftmost point of the section
    Left(f64),
    /// The axis is on the right of the rightmost point of the section
    Right(f64),
    /// The axis is inside the section.
    Inside(f64),
}

impl Default for RevolutionSurfaceRadius {
    fn default() -> Self {
        Self::Left(0.)
    }
}

impl RevolutionSurfaceRadius {
    /// Convert to Self from an f64. The sign indicate the position of the revolution axis relative
    /// to the section.
    ///
    /// Positive value indicate that the axis of revolution is on the right of the section
    /// Negative value indicate that the axis of revolution is on the left of the section
    pub fn from_signed_f64(radius: f64) -> Self {
        if radius.is_sign_positive() {
            Self::Right(radius)
        } else {
            Self::Left(-radius)
        }
    }

    /// Convert self to an f64. The sign indicate the position of the revolution axis relative to
    /// the section.
    ///
    /// Positive value indicate that the axis of revolution is on the right of the section
    /// Negative value indicate that the axis of revolution is on the left of the section
    pub fn to_signed_f64(self) -> Option<f64> {
        match self {
            Self::Left(x) => Some(-x),
            Self::Right(x) => Some(x),
            Self::Inside(_) => None,
        }
    }

    pub fn scaled(self, scale: f64) -> Self {
        match self {
            Self::Left(x) => Self::Left(scale * x),
            Self::Right(x) => Self::Right(scale * x),
            Self::Inside(x) => Self::Inside(scale * x),
        }
    }
}

impl RootedRevolutionSurface {
    pub fn position(&self, revolution_angle: f64, section_t: f64) -> DVec3 {
        self.surface
            .position_when_scaled(section_t, revolution_angle, self.scale)
    }

    pub fn dpos_dtheta(&self, revolution_angle: f64, section_parameter: f64) -> DVec3 {
        use ensnano_design::PointOnSurface;
        let surface_point = PointOnSurface {
            revolution_angle,
            section_parameter,
            revolution_axis_position: self.surface.get_axis_position_when_scaled(self.scale),
            section_half_turn_per_revolution: self.surface.half_turn_count,
            curve_scale_factor: self.scale,
        };

        self.surface
            .curve
            .derivative_position_on_surface_wrp_section_parameter(&surface_point)
            * self.scale
    }

    pub fn d2pos_dtheta2(&self, revolution_angle: f64, section_parameter: f64) -> DVec3 {
        use ensnano_design::PointOnSurface;
        let surface_point = PointOnSurface {
            revolution_angle,
            section_parameter,
            revolution_axis_position: self.surface.get_axis_position_when_scaled(self.scale),
            section_half_turn_per_revolution: self.surface.half_turn_count,
            curve_scale_factor: self.scale,
        };

        self.surface
            .curve
            .second_derivative_position_on_surface_wrp_section_parameter(&surface_point)
            * self.scale
    }

    pub fn axis(&self, revolution_angle: f64) -> DVec3 {
        DVec3 {
            x: -revolution_angle.sin(),
            y: revolution_angle.cos(),
            z: 0.,
        }
    }

    pub fn total_shift(&self) -> isize {
        let additional_shift = if self.surface.half_turn_count % 2 == 1 {
            self.rooting_parameters.nb_helix_per_half_section
        } else {
            0
        };
        self.rooting_parameters.shift_per_turn + additional_shift as isize
    }

    pub fn curve_is_open(&self) -> bool {
        self.surface.curve.is_open()
    }

    pub fn get_revolution_radius(&self) -> RevolutionSurfaceRadius {
        self.surface.revolution_radius.scaled(self.scale)
    }

    pub fn nb_spirals(&self) -> usize {
        self.nb_spirals
    }

    pub fn half_turn_count(&self) -> isize {
        self.surface.half_turn_count
    }

    pub fn curve_descriptor(
        &self,
        interpolations: Vec<InterpolationDescriptor>,
        objective_number_of_nts: Option<usize>,
    ) -> InterpolatedCurveDescriptor {
        InterpolatedCurveDescriptor {
            curve: self.surface.curve.clone(),
            curve_scale_factor: self.scale,
            chevyshev_smoothening: self.rooting_parameters.junction_smoothening,
            interpolation: interpolations,
            half_turns_count: self.surface.half_turn_count,
            revolution_radius: -self.surface.get_axis_position_when_scaled(self.scale),
            nb_turn: None,
            revolution_angle_init: None,
            known_number_of_helices_in_shape: Some(self.nb_spirals()),
            known_helix_id_in_shape: None,
            objective_number_of_nts,
            full_turn_at_nt: None,
        }
    }

    pub fn curve_2d(&self) -> &CurveDescriptor2D {
        &self.surface.curve
    }

    pub fn rescale_section(&mut self, scaling_factor: f64) {
        self.scale *= scaling_factor;
        self.surface.revolution_radius = self.surface.revolution_radius.scaled(1. / scaling_factor);
        self.area_per_radius_unit *= scaling_factor.powi(2);
        self.area_radius_0 *= scaling_factor;
    }

    pub fn rescale_radius(&mut self, objective: usize, actual: usize) {
        let incr = (objective as f64 - actual as f64) / self.area_per_radius_unit;

        match &mut self.surface.revolution_radius {
            RevolutionSurfaceRadius::Left(x) => *x += incr,
            RevolutionSurfaceRadius::Right(x) => *x += incr,
            _ => (),
        }
    }

    pub fn get_frame(&self) -> Similarity3 {
        self.surface.get_frame_when_scaled(1. / self.scale)
    }
}

/// Compute the area of the triangles of the strip using the formula
/// area(ABC) = 1/2 * mag(AB cross AC)
fn area_strip<I: Iterator<Item = DVec3> + Clone>(vertices: I, nb_section_per_strip: usize) -> f64 {
    vertices
        .clone()
        .zip(vertices.clone().skip(1))
        .zip(vertices.skip(2))
        .take(2 * nb_section_per_strip)
        .map(|((a, b), c)| 0.5 * (b - a).cross(c - a).mag())
        .sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(non_snake_case)]
    fn surface_area() {
        let r = 1.0;
        let R = 3.0;
        let surface = UnrootedRevolutionSurfaceDescriptor {
            curve: CurveDescriptor2D::Ellipse {
                semi_minor_axis: r.into(),
                semi_major_axis: r.into(),
            },
            revolution_radius: RevolutionSurfaceRadius::Left(R - r),
            half_turn_count: 0,
            curve_plane_position: Vec3::zero(),
            curve_plane_orientation: Rotor3::identity(),
        };

        let expected = 4. * std::f64::consts::PI * std::f64::consts::PI * r * R;

        let actual = surface.approx_surface_area(1_000, 1_000).unwrap();

        assert!(
            (expected - actual).abs() < 1e-3,
            "exptected {expected},  actual {actual}"
        );
    }
}
