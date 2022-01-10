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

use super::curves::*;
use super::{
    codenano,
    grid::{Grid, GridPosition},
    scadnano::*,
    utils::*,
    Nucl, Parameters,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use ultraviolet::{DVec3, Isometry2, Mat4, Rotor3, Vec3};

/// A structure maping helices identifier to `Helix` objects
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Helices(pub(super) Arc<BTreeMap<usize, Arc<Helix>>>);

impl Helices {
    // Collection methods
    // ===========================================================================
    pub fn get(&self, id: &usize) -> Option<&Helix> {
        self.0.get(id).map(|arc| arc.as_ref())
    }

    pub fn get_mut(&mut self, id: &usize) -> Option<&mut Helix> {
        // Ensure that a new map is created so that the modified map is stored at a different
        // address.
        // Calling Arc::make_mut directly does not work because we want a new pointer even if
        // the arc count is 1
        let new_map = BTreeMap::clone(self.0.as_ref());
        *self = Helices(Arc::new(new_map));

        Arc::make_mut(&mut self.0).get_mut(id).map(|arc| {
            // For the same reasons as above, ensure that a new helix is created so that the
            // modified helix is stored at a different address.
            let new_helix = Helix::clone(arc.as_ref());
            *arc = Arc::new(new_helix);

            Arc::make_mut(arc)
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&usize, &Helix)> {
        self.0.iter().map(|(id, arc)| (id, arc.as_ref()))
    }

    pub fn values(&self) -> impl Iterator<Item = &Helix> {
        self.0.values().map(|arc| arc.as_ref())
    }

    // ===========================================================================
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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<Arc<CurveDescriptor>>,

    #[serde(default, skip)]
    pub(super) instanciated_curve: Option<InstanciatedCurve>,

    #[serde(default, skip_serializing_if = "f32_is_zero")]
    delta_bbpt: f32,

    #[serde(default, skip_serializing_if = "isize_is_zero")]
    pub initial_nt_index: isize,

    /// An optional helix whose roll is copied from and to which self transfer forces applying
    /// to its roll
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_helix: Option<usize>,
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
            curve: None,
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
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
            curve: None,
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
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
            curve: None,
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
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
            curve: None,
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
        }
    }

    pub fn new_sphere_like_spiral(radius: f64, theta_0: f64) -> Self {
        let constructor = SphereLikeSpiral { radius, theta_0 };
        Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            isometry2d: None,
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: Some(Arc::new(CurveDescriptor::SphereLikeSpiral(constructor))),
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
        }
    }

    pub fn new_bezier_two_points(
        start: Vec3,
        mut start_axis: Vec3,
        end: Vec3,
        mut end_axis: Vec3,
    ) -> Self {
        start_axis.normalize();
        end_axis.normalize();
        let middle = (end - start) / 2.;
        let proj_start = start + middle.dot(start_axis) * start_axis;
        let proj_end = end - middle.dot(end_axis) * end_axis;
        let constructor = CubicBezierConstructor {
            start,
            end,
            control1: proj_start,
            control2: proj_end,
        };
        Self {
            position: start,
            orientation: Rotor3::identity(),
            isometry2d: None,
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: Some(Arc::new(CurveDescriptor::Bezier(constructor))),
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
        }
    }

    pub fn nb_bezier_nucls(&self) -> usize {
        self.instanciated_curve
            .as_ref()
            .map(|c| c.curve.as_ref().nb_points())
            .unwrap_or(0)
    }

    pub fn roll_at_pos(&self, n: isize, cst: &Parameters) -> f32 {
        use std::f32::consts::PI;
        let bbpt = cst.bases_per_turn + self.delta_bbpt;
        let beta = 2. * PI / bbpt;
        self.roll - n as f32 * beta // Beta is positive but helix turn clockwise when n increases
    }

    /// Angle of base number `n` around this helix.
    pub fn theta(&self, n: isize, forward: bool, cst: &Parameters) -> f32 {
        use std::f32::consts::PI;
        // The groove_angle goes from the backward strand to the forward strand
        let shift = if forward { cst.groove_angle } else { 0. };
        let bbpt = cst.bases_per_turn + self.delta_bbpt;
        let beta = 2. * PI / bbpt;
        self.roll
            -n as f32 * beta  // Beta is positive but helix turn clockwise when n increases
            + shift
            + std::f32::consts::FRAC_PI_2 // Add PI/2 so that when the roll is 0,
                                          // the backward strand is at vertical position on nucl 0
    }

    /// 3D position of a nucleotide on this helix. `n` is the position along the axis, and `forward` is true iff the 5' to 3' direction of the strand containing that nucleotide runs in the same direction as the axis of the helix.
    pub fn space_pos(&self, p: &Parameters, n: isize, forward: bool) -> Vec3 {
        let n = n + self.initial_nt_index;
        let theta = self.theta(n, forward, p);
        self.theta_n_to_space_pos(p, n, theta)
    }

    fn theta_n_to_space_pos(&self, p: &Parameters, n: isize, theta: f32) -> Vec3 {
        if let Some(curve) = self.instanciated_curve.as_ref() {
            if n >= 0 {
                if let Some(point) = curve.as_ref().nucl_pos(n as usize, theta as f64, p) {
                    return dvec_to_vec(point);
                }
            }
        }
        let mut ret = Vec3::new(
            n as f32 * p.z_step,
            theta.sin() * p.helix_radius,
            theta.cos() * p.helix_radius,
        );

        ret = self.rotate_point(ret);
        ret += self.position;
        ret
    }

    pub fn shifted_space_pos(&self, p: &Parameters, n: isize, forward: bool, shift: f32) -> Vec3 {
        let n = self.initial_nt_index + n;
        let theta = self.theta(n, forward, p) + shift;
        self.theta_n_to_space_pos(p, n, theta)
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
            curve: None,
            instanciated_curve: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
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

    pub fn get_axis<'a>(&'a self, p: &Parameters) -> Axis<'a> {
        if let Some(curve) = self.instanciated_curve.as_ref() {
            let shift = self.initial_nt_index;
            let points = curve.as_ref().points();
            Axis::Curve { shift, points }
        } else {
            Axis::Line {
                origin: self.position,
                direction: self.axis_position(p, 1) - self.position,
            }
        }
    }

    pub fn axis_position(&self, p: &Parameters, n: isize) -> Vec3 {
        let n = n + self.initial_nt_index;
        if let Some(curve) = self.instanciated_curve.as_ref().map(|s| &s.curve) {
            if n >= 0 && n <= curve.nb_points() as isize {
                if let Some(point) = curve.axis_pos(n as usize) {
                    return dvec_to_vec(point);
                }
            }
        }
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

    pub fn get_bezier_controls(&self) -> Option<CubicBezierConstructor> {
        self.curve.as_ref().and_then(|c| c.get_bezier_controls())
    }

    pub fn get_curve_range(&self) -> Option<std::ops::RangeInclusive<isize>> {
        if let Some(ref curve) = self.instanciated_curve {
            Some(0..=(curve.curve.nb_points() as isize - 1))
        } else {
            None
        }
    }
}

/// The virtual position of a nucleotide.
///
/// Two nucleotides on different helices with the same support helix will be mapped
/// to the same `VirtualNucl` if they are at the same position on that support helix
#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub struct VirtualNucl(Nucl);

impl VirtualNucl {
    pub fn compl(&self) -> Self {
        Self(self.0.compl())
    }
}

impl Nucl {
    pub fn map_to_virtual_nucl<P: AsRef<Helix>, H: AsRef<BTreeMap<usize, P>>>(
        nucl: Nucl,
        helices: H,
    ) -> Option<VirtualNucl> {
        let h = helices.as_ref().get(&nucl.helix)?;
        let support_helix_id = h
            .as_ref()
            .support_helix
            .or(Some(nucl.helix))
            .filter(|h_id| helices.as_ref().contains_key(h_id))?;
        Some(VirtualNucl(Nucl {
            helix: support_helix_id,
            position: nucl.position + h.as_ref().initial_nt_index,
            forward: nucl.forward,
        }))
    }
}

/// Represents the axis of an helix. At the moment it is a line. In the future it might also be a
/// bezier curve
#[derive(Debug, Clone)]
pub enum Axis<'a> {
    Line { origin: Vec3, direction: Vec3 },
    Curve { shift: isize, points: &'a [DVec3] },
}

#[derive(Debug, Clone)]
pub enum OwnedAxis {
    Line { origin: Vec3, direction: Vec3 },
    Curve { shift: isize, points: Vec<DVec3> },
}

impl<'a> Axis<'a> {
    pub fn to_owned(self) -> OwnedAxis {
        match self {
            Self::Line { origin, direction } => OwnedAxis::Line { origin, direction },
            Self::Curve { shift, points } => OwnedAxis::Curve {
                shift,
                points: points.to_vec(),
            },
        }
    }
}

impl OwnedAxis {
    pub fn borrow<'a>(&'a self) -> Axis<'a> {
        match self {
            Self::Line { origin, direction } => Axis::Line {
                origin: *origin,
                direction: *direction,
            },
            Self::Curve { shift, points } => Axis::Curve {
                shift: *shift,
                points: &points[..],
            },
        }
    }
}

impl<'a> Axis<'a> {
    pub fn transformed(&self, model_matrix: &Mat4) -> Self {
        match self {
            Self::Line {
                origin: old_origin,
                direction: old_direction,
            } => {
                let origin = model_matrix.transform_point3(*old_origin);
                let direction = model_matrix.transform_vec3(*old_direction);
                Self::Line { origin, direction }
            }
            _ => self.clone(),
        }
    }

    pub fn direction(&self) -> Option<Vec3> {
        if let Axis::Line { direction, .. } = self {
            Some(*direction)
        } else {
            None
        }
    }
}
