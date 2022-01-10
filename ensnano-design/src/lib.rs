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
//! This module defines the ensnano format.
//! All other format supported by ensnano are converted into this format and run-time manipulation
//! of designs are performed on an `ensnano::Design` structure
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f32::consts::PI;
use std::sync::Arc;

#[macro_use]
extern crate serde_derive;
extern crate serde;
pub use ultraviolet;
use ultraviolet::{Rotor3, Vec3};

pub mod codenano;
pub mod grid;
use grid::{GridData, GridDescriptor};
pub mod scadnano;
pub use ensnano_organizer::{GroupId, OrganizerTree};
use scadnano::*;
pub mod elements;
use elements::DnaElementKey;
pub type EnsnTree = OrganizerTree<DnaElementKey>;
pub mod group_attributes;
use group_attributes::GroupAttribute;

mod strands;
pub use strands::*;
mod helices;
pub use helices::*;

mod curves;
pub use curves::{CubicBezierConstructor, CurveCache, CurveDescriptor};
pub mod design_operations;
pub mod utils;

#[cfg(test)]
mod tests;

/// The `ensnano` Design structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct Design {
    /// The collection of all helices used in this design. Helices have a
    /// position and an orientation in 3D.
    pub helices: Helices,
    /// The vector of strands.
    pub strands: Strands,
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

    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub checked_xovers: HashSet<usize>,

    /// True if the colors of the scaffold's nucleotides should make a rainbow
    #[serde(default)]
    pub rainbow_scaffold: bool,

    #[serde(skip)]
    instanciated_grid_data: Option<GridData>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot_position: Option<Vec3>,
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
            helices: Helices(Arc::new(helices)),
            strands: Strands(strands),
            parameters: Some(parameters),
            ..Default::default()
        }
    }

    pub fn new() -> Self {
        Self {
            helices: Default::default(),
            strands: Default::default(),
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
            checked_xovers: Default::default(),
            rainbow_scaffold: false,
            instanciated_grid_data: None,
        }
    }

    pub fn update_version(&mut self) {
        if self.ensnano_version == ensnano_version() {
            return;
        } else if self.ensnano_version.is_empty() {
            // Version < 0.2.0 had no version identifier, and the DNA parameters where different.
            // The groove_angle was negative, and the roll was going in the opposite direction
            if let Some(parameters) = self.parameters.as_mut() {
                parameters.groove_angle *= -1.;
            } else {
                self.parameters = Some(Default::default())
            }
            mutate_all_helices(self, |h| h.roll *= -1.);
            self.ensnano_version = ensnano_version();
        }
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

    pub fn add_camera(
        &mut self,
        position: Vec3,
        orientation: Rotor3,
        pivot_position: Option<Vec3>,
    ) {
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
            pivot_position,
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

    pub fn update_curves(&mut self, cached_curve: &mut CurveCache) {
        let mut need_update = false;
        for h in self.helices.values() {
            if h.need_curve_update() {
                need_update = true;
                break;
            }
        }
        if need_update {
            let parameters = self.parameters.unwrap_or(Parameters::DEFAULT);
            let mut new_helices_map = BTreeMap::clone(self.helices.0.as_ref());
            for h in new_helices_map.values_mut() {
                mutate_in_arc(h, |h| h.update_curve(&parameters, cached_curve))
            }
            self.helices = Helices(Arc::new(new_helices_map));
        }
    }

    pub fn update_support_helices(&mut self) {
        let parameters = self.parameters.unwrap_or_default();
        let old_helices = self.helices.clone();
        mutate_all_helices(self, |h| {
            if let Some(mother_id) = h.support_helix {
                if let Some(mother) = old_helices.get(&mother_id) {
                    h.roll = mother.roll;
                }
            }
        })
    }

    pub fn get_nucl_position(&self, nucl: Nucl) -> Option<Vec3> {
        let helix = self.helices.get(&nucl.helix)?;
        Some(helix.space_pos(
            &self.parameters.unwrap_or_default(),
            nucl.position,
            nucl.forward,
        ))
    }

    fn get_updated_grid_data(&mut self) -> &GridData {
        let need_update = if let Some(data) = self.instanciated_grid_data.as_ref() {
            !data.is_up_to_date(&self)
        } else {
            true
        };
        if need_update {
            let updated_data = GridData::new_by_updating_design(self);
            self.instanciated_grid_data = Some(updated_data);
        }
        // unwrap ok: If need_update is true, then instanciated_grid_data has just been given a
        // value, otherwise it was already some up-to-date data
        self.instanciated_grid_data.as_ref().unwrap()
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
            helices: Helices(Arc::new(helices)),
            strands: Strands(strands),
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
    let mut new_helices_map = BTreeMap::clone(design.helices.0.as_ref());
    for h in new_helices_map.values_mut() {
        mutate_in_arc(h, mutation.clone())
    }
    design.helices = Helices(Arc::new(new_helices_map));
}

pub fn mutate_one_helix<F>(design: &mut Design, h_id: usize, mutation: F) -> Option<()>
where
    F: FnMut(&mut Helix) + Clone,
{
    let mut new_helices_map = BTreeMap::clone(design.helices.0.as_ref());
    new_helices_map
        .get_mut(&h_id)
        .map(|h| mutate_in_arc(h, mutation))?;
    design.helices = Helices(Arc::new(new_helices_map));
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

#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct Nucl {
    pub helix: usize,
    pub position: isize,
    pub forward: bool,
}

impl std::cmp::PartialOrd for Nucl {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Nucl {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.helix != other.helix {
            self.helix.cmp(&other.helix)
        } else if self.forward != other.forward {
            self.forward.cmp(&other.forward)
        } else if self.forward {
            self.position.cmp(&other.position)
        } else {
            self.position.cmp(&other.position).reverse()
        }
    }
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
