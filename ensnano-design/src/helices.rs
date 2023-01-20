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

use crate::design_operations::ErrOperation;
use crate::grid::*;

use super::curves::*;
use super::{
    codenano,
    grid::{Grid, GridData, HelixGridPosition},
    scadnano::*,
    utils::*,
    BezierPathId, Nucl, Parameters,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use ultraviolet::{DRotor3, DVec3, Isometry2, Mat4, Rotor3, Vec2, Vec3};

/// A structure maping helices identifier to `Helix` objects
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Helices(pub(super) Arc<BTreeMap<usize, Arc<Helix>>>);

impl Helices {
    #[allow(clippy::needless_lifetimes)]
    pub fn make_mut<'a>(&'a mut self) -> HelicesMut<'a> {
        let new_map = BTreeMap::clone(self.0.as_ref());
        HelicesMut {
            source: self,
            new_map,
        }
    }
}

pub trait HelixCollection {
    fn get(&self, id: &usize) -> Option<&Helix>;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a usize, &'a Helix)> + 'a>;
    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Helix> + 'a>;
    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a usize> + 'a>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn contains_key(&self, id: &usize) -> bool;
}

pub trait HasHelixCollection {
    fn get_collection(&self) -> &BTreeMap<usize, Arc<Helix>>;
}

impl HasHelixCollection for Helices {
    fn get_collection(&self) -> &BTreeMap<usize, Arc<Helix>> {
        &self.0
    }
}

impl<'a> HasHelixCollection for HelicesMut<'a> {
    fn get_collection(&self) -> &BTreeMap<usize, Arc<Helix>> {
        &self.new_map
    }
}

impl<T> HelixCollection for T
where
    T: HasHelixCollection,
{
    fn get(&self, id: &usize) -> Option<&Helix> {
        self.get_collection().get(id).map(|arc| arc.as_ref())
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a usize, &'a Helix)> + 'a> {
        Box::new(
            self.get_collection()
                .iter()
                .map(|(id, arc)| (id, arc.as_ref())),
        )
    }

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a usize> + 'a> {
        Box::new(self.get_collection().keys())
    }

    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Helix> + 'a> {
        Box::new(self.get_collection().values().map(|arc| arc.as_ref()))
    }

    fn len(&self) -> usize {
        self.get_collection().len()
    }

    fn contains_key(&self, id: &usize) -> bool {
        self.get_collection().contains_key(id)
    }
}

pub struct HelicesMut<'a> {
    source: &'a mut Helices,
    new_map: BTreeMap<usize, Arc<Helix>>,
}

impl<'a> HelicesMut<'a> {
    pub fn get_mut(&mut self, id: &usize) -> Option<&mut Helix> {
        self.new_map.get_mut(id).map(|arc| {
            // For the same reasons as above, ensure that a new helix is created so that the
            // modified helix is stored at a different address.
            // Calling Arc::make_mut directly does not work because we want a new pointer even if
            // the arc count is 1
            let new_helix = Helix::clone(arc.as_ref());
            *arc = Arc::new(new_helix);

            Arc::make_mut(arc)
        })
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Helix> {
        self.new_map.values_mut().map(|arc| {
            let new_helix = Helix::clone(arc.as_ref());
            *arc = Arc::new(new_helix);
            Arc::make_mut(arc)
        })
    }

    pub fn insert(&mut self, id: usize, helix: Helix) {
        self.new_map.insert(id, Arc::new(helix));
    }

    pub fn remove(&mut self, id: &usize) {
        self.new_map.remove(id);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut Helix)> {
        self.new_map.iter_mut().map(|(id, arc)| {
            let new_helix = Helix::clone(arc.as_ref());
            *arc = Arc::new(new_helix);
            (id, Arc::make_mut(arc))
        })
    }

    /// Add an helix to the collection and return the identifier of the added helix in the
    /// collection.
    pub fn push_helix(&mut self, helix: Helix) -> usize {
        let helix_id = self.get_collection().keys().last().unwrap_or(&0) + 1;
        self.insert(helix_id, helix);
        helix_id
    }
}

impl<'a> AsRef<Helices> for HelicesMut<'a> {
    fn as_ref(&self) -> &Helices {
        self.source
    }
}

impl<'a> Drop for HelicesMut<'a> {
    fn drop(&mut self) {
        *self.source = Helices(Arc::new(std::mem::take(&mut self.new_map)))
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
    pub grid_position: Option<HelixGridPosition>,

    /// Representation of the helix in 2d
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub isometry2d: Option<Isometry2>,

    /// Additional segments for representing the helix in 2d
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub additonal_isometries: Vec<AdditionalHelix2D>,

    #[serde(default = "Vec2::one")]
    /// Symmetry applied inside the representation of the helix in 2d
    pub symmetry: Vec2,

    /// Roll of the helix. A roll equal to 0 means that the nucleotide 0 of the forward strand is
    /// at point (0., 1., 0.) in the helix's coordinate.
    #[serde(default)]
    pub roll: f32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<Arc<CurveDescriptor>>,

    #[serde(default, skip)]
    pub(super) instanciated_descriptor: Option<Arc<InstanciatedCurveDescriptor>>,

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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) path_id: Option<BezierPathId>,
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
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: None,
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    pub fn from_scadnano(
        scad: &ScadnanoHelix,
        group_map: &BTreeMap<String, usize>,
        groups: &[ScadnanoGroup],
        helix_per_group: &mut Vec<usize>,
    ) -> Result<Self, ScadnanoImportError> {
        let group_id = scad
            .group
            .clone()
            .unwrap_or_else(|| String::from("default_group"));
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
            return Err(ScadnanoImportError::MissingField(String::from("x")));
        };
        let y = if let Some(y) = scad.grid_position.get(1).cloned() {
            y
        } else {
            return Err(ScadnanoImportError::MissingField(String::from("y")));
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
            grid_position: Some(HelixGridPosition {
                grid: GridId::FreeGrid(*grid_id),
                x,
                y,
                axis_pos: 0,
                roll: 0f32,
            }),
            visible: true,
            roll: 0f32,
            isometry2d: Some(isometry2d),
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            locked_for_simulations: false,
            curve: None,
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        })
    }

    pub fn translated_by(&self, edge: crate::grid::Edge, grid_data: &GridData) -> Option<Self> {
        log::debug!("attempt to translate helix");
        let grid_position = self
            .grid_position
            .as_ref()
            .and_then(|gp| grid_data.translate_by_edge(gp, &edge));
        let new_curve_descriptor = self
            .curve
            .as_ref()
            .and_then(|c| c.translate(edge, grid_data));

        if self.curve.is_some() != new_curve_descriptor.is_some() {
            None
        } else {
            Some(Self {
                instanciated_curve: None,
                instanciated_descriptor: None,
                grid_position,
                isometry2d: None,
                curve: new_curve_descriptor.map(Arc::new),
                ..self.clone()
            })
        }
    }
}

impl Helix {
    pub fn new(origin: Vec3, orientation: Rotor3) -> Self {
        Self {
            position: origin,
            orientation,
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: None,
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    pub fn new_on_grid(grid: &Grid, x: isize, y: isize, g_id: GridId) -> Self {
        let position = grid.position_helix(x, y);
        Self {
            position,
            orientation: grid.orientation,
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: Some(HelixGridPosition {
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
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    pub fn new_sphere_like_spiral(desc: SphereLikeSpiralDescriptor) -> Self {
        Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: Some(Arc::new(CurveDescriptor::SphereLikeSpiral(desc))),
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    pub fn new_tube_spiral(desc: TubeSpiralDescritor) -> Self {
        Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: Some(Arc::new(CurveDescriptor::TubeSpiral(desc))),
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    pub fn new_with_curve(desc: CurveDescriptor) -> Self {
        Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: None,
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: Some(Arc::new(desc)),
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    pub fn piecewise_bezier_points(&self) -> Option<Vec<Vec3>> {
        if let Some(CurveDescriptor::PiecewiseBezier { .. }) = self.curve.as_ref().map(Arc::as_ref)
        {
            Some(self.bezier_points())
        } else {
            None
        }
    }

    pub fn cubic_bezier_points(&self) -> Option<Vec<Vec3>> {
        if let Some(CurveDescriptor::Bezier(_)) = self.curve.as_ref().map(Arc::as_ref) {
            Some(self.bezier_points())
        } else {
            None
        }
    }

    pub fn translate_bezier_point(
        &mut self,
        _bezier_point: BezierControlPoint,
        _translation: GridAwareTranslation,
    ) -> Result<(), ErrOperation> {
        /*
        let point = match bezier_point {
            BezierControlPoint::PiecewiseBezier(n) => {
                if let Some(CurveDescriptor::PiecewiseBezier { tengents, .. }) =
                    self.curve.as_mut().map(Arc::make_mut)
                {
                    tengents.get_mut(n / 2)
                } else {
                    None
                }
            }
            _ => {
                log::error!("Translation of cubic bezier point not implemented");
                None
            }
        }
        .ok_or(ErrOperation::NotEnoughBezierPoints)?;
        *point += translation.0;
        */
        log::error!("Translation of cubic bezier point not implemented");
        Ok(())
    }

    fn bezier_points(&self) -> Vec<Vec3> {
        if let Some(desc) = self.instanciated_descriptor.as_ref() {
            desc.bezier_points()
        } else {
            vec![]
        }
    }

    pub fn new_bezier_two_points(
        grid_manager: &GridData,
        grid_pos_start: HelixGridPosition,
        grid_pos_end: HelixGridPosition,
    ) -> Result<Self, ErrOperation> {
        let position = grid_manager
            .pos_to_space(grid_pos_start.light())
            .ok_or(ErrOperation::GridDoesNotExist(grid_pos_start.grid))?;
        let point_start = BezierEnd {
            position: grid_pos_start.light(),
            inward_coeff: 1.,
            outward_coeff: 1.,
        };
        let point_end = BezierEnd {
            position: grid_pos_end.light(),
            inward_coeff: 1.,
            outward_coeff: 1.,
        };
        let constructor = CurveDescriptor::PiecewiseBezier {
            points: vec![point_start, point_end],
            t_max: None,
            t_min: None,
        };
        let mut ret = Self {
            position,
            orientation: Rotor3::identity(),
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: Some(grid_pos_start),
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve: Some(Arc::new(constructor)),
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        };
        // we can use a fake cache because we don't need it for bezier curves.
        let mut fake_cache = Default::default();
        grid_manager.update_curve(&mut ret, &mut fake_cache);
        Ok(ret)
    }

    pub fn new_on_bezier_path(
        grid_manager: &GridData,
        grid_pos: HelixGridPosition,
        path_id: BezierPathId,
    ) -> Result<Self, ErrOperation> {
        let translation = (|| {
            let grid = grid_manager.grids.get(&grid_pos.grid)?;
            let position = grid.position_helix_in_grid_coordinates(grid_pos.x, grid_pos.y);
            Some(position)
        })();

        let curve = translation
            .map(|translation| CurveDescriptor::TranslatedPath {
                path_id,
                translation,
                legacy: false,
            })
            .map(Arc::new);

        let mut ret = Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            isometry2d: None,
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            grid_position: Some(grid_pos),
            visible: true,
            roll: 0f32,
            locked_for_simulations: false,
            curve,
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: Some(path_id),
        };
        let mut fake_cache = Default::default();
        grid_manager.update_curve(&mut ret, &mut fake_cache);
        Ok(ret)
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
        self.shifted_space_pos(p, n, forward, 0.0)
    }

    pub fn normal_at_pos(&self, n: isize, forward: bool) -> Vec3 {
        self.instanciated_curve
            .as_ref()
            .and_then(|c| {
                let axis = c.curve.axis_at_pos(n, forward)?;
                Some(dvec_to_vec(axis[2]))
            })
            .unwrap_or_else(|| Vec3::unit_x().rotated_by(self.orientation))
    }

    fn theta_n_to_space_pos(&self, p: &Parameters, n: isize, theta: f32, forward: bool) -> Vec3 {
        let mut ret;
        if let Some(curve) = self.instanciated_curve.as_ref() {
            if let Some(point) = curve
                .as_ref()
                .nucl_pos(n, forward, theta as f64, p)
                .map(dvec_to_vec)
            {
                let (position, orientation) = if curve.as_ref().has_its_own_encoded_frame() {
                    (Vec3::zero(), Rotor3::identity())
                } else {
                    (self.position, self.orientation)
                };
                return point.rotated_by(orientation) + position;
            } else {
                let delta_inclination = if forward { 0.0 } else { p.inclination };
                ret = Vec3::new(
                    n as f32 * p.z_step + delta_inclination,
                    theta.sin() * p.helix_radius,
                    theta.cos() * p.helix_radius,
                );
            }
        } else {
            let delta_inclination = if forward { 0.0 } else { p.inclination };
            ret = Vec3::new(
                n as f32 * p.z_step + delta_inclination,
                theta.sin() * p.helix_radius,
                theta.cos() * p.helix_radius,
            );
        }

        ret = self.rotate_point(ret);
        ret += self.position;
        ret
    }

    pub fn shifted_space_pos(&self, p: &Parameters, n: isize, forward: bool, shift: f32) -> Vec3 {
        let n = self.initial_nt_index + n;
        let theta = self.theta(n, forward, p) + shift;
        self.theta_n_to_space_pos(p, n, theta, forward)
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
            additonal_isometries: Vec::new(),
            symmetry: Vec2::one(),
            locked_for_simulations: false,
            curve: None,
            instanciated_curve: None,
            instanciated_descriptor: None,
            delta_bbpt: 0.,
            initial_nt_index: 0,
            support_helix: None,
            path_id: None,
        }
    }

    fn position_ideal_neighbour(&self, n: isize, forward: bool, p: &Parameters) -> Vec3 {
        let axis_pos = self.axis_position(p, n);
        let my_nucl_pos = self.space_pos(p, n, forward);
        let direction = (my_nucl_pos - axis_pos).normalized();

        #[allow(clippy::let_and_return)]
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
            let (position, orientation) = if curve.as_ref().has_its_own_encoded_frame() {
                (DVec3::zero(), DRotor3::identity())
            } else {
                (
                    vec_to_dvec(self.position),
                    rotor_to_drotor(self.orientation),
                )
            };
            Axis::Curve {
                shift,
                points,
                nucl_t0: curve.as_ref().nucl_t0(),
                position,
                orientation,
            }
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
            if let Some(point) = curve.axis_pos(n).map(dvec_to_vec) {
                let (position, orientation) = if curve.as_ref().has_its_own_encoded_frame() {
                    (Vec3::zero(), Rotor3::identity())
                } else {
                    (self.position, self.orientation)
                };
                return point.rotated_by(orientation) + position;
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
        self.instanciated_descriptor
            .as_ref()
            .and_then(|c| c.get_bezier_controls())
    }

    pub fn get_curve_range(&self) -> Option<std::ops::RangeInclusive<isize>> {
        self.instanciated_curve
            .as_ref()
            .map(|curve| curve.curve.range())
    }

    pub fn get_surface_info_nucl(&self, nucl: Nucl) -> Option<SurfaceInfo> {
        let mut surface_info = self.instanciated_curve.as_ref().and_then(|curve| {
            let curve = &curve.curve;
            let t = curve.nucl_time(nucl.position)?;
            curve.geometry.surface_info_time(t, nucl.helix)
        })?;
        surface_info.local_frame.rotate_by(self.orientation);
        surface_info.position.rotate_by(self.orientation);
        surface_info.position += self.position;
        Some(surface_info)
    }

    pub fn get_surface_info(&self, point: SurfacePoint) -> Option<SurfaceInfo> {
        let mut surface_info = self.instanciated_curve.as_ref().and_then(|curve| {
            let curve = &curve.curve;
            curve.geometry.surface_info(point)
        })?;
        surface_info.local_frame.rotate_by(self.orientation);
        surface_info.position.rotate_by(self.orientation);
        surface_info.position += self.position;
        Some(surface_info)
    }
}

/// The virtual position of a nucleotide.
///
/// Two nucleotides on different helices with the same support helix will be mapped
/// to the same `VirtualNucl` if they are at the same position on that support helix
#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub struct VirtualNucl(pub(super) Nucl);

impl VirtualNucl {
    pub fn compl(&self) -> Self {
        Self(self.0.compl())
    }
}

impl Nucl {
    pub fn map_to_virtual_nucl(nucl: Nucl, helices: &Helices) -> Option<VirtualNucl> {
        let h = helices.get(&nucl.helix)?;
        let support_helix_id = h
            .support_helix
            .or(Some(nucl.helix))
            .filter(|h_id| helices.contains_key(h_id))?;
        Some(VirtualNucl(Nucl {
            helix: support_helix_id,
            position: nucl.position + h.initial_nt_index,
            forward: nucl.forward,
        }))
    }
}

/// Represents the axis of an helix. At the moment it is a line. In the future it might also be a
/// bezier curve
#[derive(Debug, Clone)]
pub enum Axis<'a> {
    Line {
        origin: Vec3,
        direction: Vec3,
    },
    Curve {
        shift: isize,
        points: &'a [DVec3],
        nucl_t0: usize,
        position: DVec3,
        orientation: DRotor3,
    },
}

#[derive(Debug, Clone)]
pub enum OwnedAxis {
    Line {
        origin: Vec3,
        direction: Vec3,
    },
    Curve {
        shift: isize,
        points: Vec<DVec3>,
        nucl_t0: usize,
        position: DVec3,
        orientation: DRotor3,
    },
}

impl<'a> Axis<'a> {
    pub fn to_owned(self) -> OwnedAxis {
        match self {
            Self::Line { origin, direction } => OwnedAxis::Line { origin, direction },
            Self::Curve {
                shift,
                points,
                nucl_t0,
                orientation,
                position,
            } => OwnedAxis::Curve {
                shift,
                points: points.to_vec(),
                nucl_t0,
                orientation,
                position,
            },
        }
    }
}

impl OwnedAxis {
    #[allow(clippy::needless_lifetimes)]
    pub fn borrow<'a>(&'a self) -> Axis<'a> {
        match self {
            Self::Line { origin, direction } => Axis::Line {
                origin: *origin,
                direction: *direction,
            },
            Self::Curve {
                shift,
                points,
                nucl_t0,
                orientation,
                position,
            } => Axis::Curve {
                shift: *shift,
                points: &points[..],
                nucl_t0: *nucl_t0,
                orientation: *orientation,
                position: *position,
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

/// An additional 2d helix used to represent an helix in the 2d view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalHelix2D {
    /// The minimum nucleotide index of the helix.
    /// Nucleotides with smalle indices are represented by the previous helix
    pub left: isize,
    /// The Isomettry to be applied after applying the isometry of the main helix 2d representation
    /// to obtain this segment
    pub additional_isometry: Option<Isometry2>,
    pub additional_symmetry: Option<Vec2>,
}
