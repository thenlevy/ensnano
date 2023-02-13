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
use std::sync::Arc;

#[macro_use]
extern crate serde_derive;
extern crate serde;
pub use bezier_plane::*;
pub use ultraviolet;
use ultraviolet::{Rotor3, Vec3};

pub mod codenano;
pub mod grid;
use grid::{FreeGrids, GridData, GridDescriptor, GridId};
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
pub use curves::*;
mod collection;
pub mod design_operations;
pub mod utils;
pub use collection::{Collection, HasMap};

mod parameters;
pub use parameters::*;

/// Re-export ultraviolet for linear algebra
pub use ultraviolet::*;

mod bezier_plane;
mod external_3d_objects;
mod insertions;
#[cfg(test)]
mod tests;
pub use external_3d_objects::*;

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
    pub free_grids: FreeGrids,

    #[serde(default, skip_serializing, alias = "grids")]
    old_grids: Vec<GridDescriptor>,

    /// The cross-over suggestion groups
    #[serde(skip_serializing_if = "groups_is_empty", default)]
    pub groups: Arc<BTreeMap<usize, bool>>,

    /// The set of identifiers of grids whose helices must not always display their phantom
    /// helices.
    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub no_phantoms: Arc<HashSet<GridId>>,

    /// The set of identifiers of grids whose helices are displayed with smaller spheres for the
    /// nucleotides.
    #[serde(
        alias = "small_shperes",
        alias = "no_spheres",
        rename(serialize = "no_spheres"),
        skip_serializing_if = "HashSet::is_empty",
        default
    )]
    pub small_spheres: Arc<HashSet<GridId>>,

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

    #[serde(skip, default)]
    cached_curve: Arc<CurveCache>,

    #[serde(default)]
    pub bezier_planes: BezierPlanes,

    #[serde(default)]
    pub bezier_paths: BezierPaths,

    #[serde(skip)]
    instanciated_paths: Option<BezierPathData>,

    #[serde(default)]
    pub external_3d_objects: External3DObjects,

    #[serde(skip)]
    pub additional_structure: Option<Arc<dyn AdditionalStructure>>,
}

pub trait AdditionalStructure: Send + Sync {
    fn frame(&self) -> Similarity3;
    fn position(&self) -> Vec<Vec3>;
    fn right(&self) -> Vec<(usize, usize)>;
    fn next(&self) -> Vec<(usize, usize)>;
    fn nt_path(&self) -> Option<Vec<Vec3>>;
    fn current_length(&self) -> Option<usize>;
}

/// An immuatable reference to a design whose helices pahts and grid data are guaranteed to be up-to
/// date.
pub struct UpToDateDesign<'a> {
    pub design: &'a Design,
    pub grid_data: &'a GridData,
    pub paths_data: &'a BezierPathData,
}

impl Design {
    /// If self is up-to-date return an `UpToDateDesign` reference to self.
    ///
    /// If this methods returns `None`, one needs to call `Design::get_up_to_date` to get an
    /// `UpToDateDesign` reference to the data.
    /// Having an option to not mutate the design is meant to prevent unecessary run-time cloning
    /// of the design
    #[allow(clippy::needless_lifetimes)]
    pub fn try_get_up_to_date<'a>(&'a self) -> Option<UpToDateDesign<'a>> {
        let paths_data = self
            .instanciated_paths
            .as_ref()
            .filter(|data| !data.need_update(&self.bezier_planes, &self.bezier_paths))?;
        if let Some(data) = self.instanciated_grid_data.as_ref() {
            if data.is_up_to_date(self) {
                Some(UpToDateDesign {
                    design: self,
                    grid_data: data,
                    paths_data,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Update self if necessary and returns an up-to-date reference to self.
    #[allow(clippy::needless_lifetimes)]
    pub fn get_up_to_date<'a>(&'a mut self) -> UpToDateDesign<'a> {
        let parameters = self.parameters.as_ref().unwrap_or(&Parameters::DEFAULT);
        if let Some(paths_data) = self.instanciated_paths.as_ref() {
            if let Some(new_data) = paths_data.updated(
                self.bezier_planes.clone(),
                self.bezier_paths.clone(),
                parameters,
            ) {
                self.instanciated_paths = Some(new_data);
            }
        } else {
            self.instanciated_paths = Some(BezierPathData::new(
                self.bezier_planes.clone(),
                self.bezier_paths.clone(),
                parameters,
            ));
        }
        if self.needs_update() {
            let grid_data = GridData::new_by_updating_design(self);
            self.instanciated_grid_data = Some(grid_data);
        }
        UpToDateDesign {
            design: self,
            grid_data: self.instanciated_grid_data.as_ref().unwrap(),
            paths_data: self.instanciated_paths.as_ref().unwrap(),
        }
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn get_up_to_date_paths<'a>(&'a mut self) -> &'a BezierPathData {
        let parameters = self.parameters.as_ref().unwrap_or(&Parameters::DEFAULT);
        if let Some(paths_data) = self.instanciated_paths.as_ref() {
            if let Some(new_data) = paths_data.updated(
                self.bezier_planes.clone(),
                self.bezier_paths.clone(),
                parameters,
            ) {
                self.instanciated_paths = Some(new_data);
            }
        } else {
            self.instanciated_paths = Some(BezierPathData::new(
                self.bezier_planes.clone(),
                self.bezier_paths.clone(),
                parameters,
            ));
        }
        self.instanciated_paths.as_ref().unwrap()
    }

    fn needs_update(&self) -> bool {
        if let Some(data) = self.instanciated_grid_data.as_ref() {
            !data.is_up_to_date(self)
        } else {
            true
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CameraId(u64);

use serde_with::{serde_as, DefaultOnError};
/// A saved camera position. This can be use to register intresting point of views of the design.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub name: String,
    pub id: CameraId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde_as(deserialize_as = "DefaultOnError")]
    pub pivot_position: Option<Vec3>,
}

pub fn ensnano_version() -> String {
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
            free_grids: Default::default(),
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
            cached_curve: Default::default(),
            bezier_planes: Default::default(),
            bezier_paths: Default::default(),
            old_grids: Vec::new(),
            instanciated_paths: None,
            external_3d_objects: Default::default(),
            additional_structure: None,
        }
    }

    pub fn update_version(&mut self) {
        // The conversion from the old grid data structure to the new one can be made regardless of
        // the version.
        let grids = std::mem::take(&mut self.old_grids);
        let mut grids_mut = self.free_grids.make_mut();
        for g in grids.into_iter() {
            grids_mut.push(g);
        }
        drop(grids_mut);

        if version_compare::compare(&self.ensnano_version, "0.5.0") == Ok(version_compare::Cmp::Lt)
        {
            // For legacy reason, the version of curved design must be set to a value >= 0.5.0
            for h in self.helices.values() {
                if h.curve.is_some() {
                    self.ensnano_version = "0.5.0".to_owned();
                    break;
                }
            }
        }

        if self.ensnano_version.is_empty() {
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
                if n1.0.helix != n2.0.helix && (n1.1 - n2.1).mag() < epsilon {
                    ret.push((n1.0, n2.0, ((n1.1 + n2.1) / 2.)));
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

    pub fn rm_camera(&mut self, cam_id: CameraId) -> bool {
        if self.cameras.remove(&cam_id).is_some() {
            if self.favorite_camera == Some(cam_id) {
                self.favorite_camera = self.cameras.keys().min().cloned();
            }
            true
        } else {
            false
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
        self.favorite_camera
    }

    pub fn set_favourite_camera(&mut self, cam_id: CameraId) -> bool {
        if self.cameras.contains_key(&cam_id) {
            if self.favorite_camera != Some(cam_id) {
                self.favorite_camera = Some(cam_id);
            } else {
                self.favorite_camera = None;
            }
            true
        } else {
            false
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

    pub fn get_updated_grid_data(&mut self) -> &GridData {
        self.update_curve_bounds();
        for _ in 0..3 {
            let need_update = if let Some(data) = self.instanciated_grid_data.as_ref() {
                !data.is_up_to_date(self)
            } else {
                true
            };
            if need_update {
                let updated_data = GridData::new_by_updating_design(self);
                self.instanciated_grid_data = Some(updated_data);
            }
            if !self.update_curve_bounds() {
                // we are done
                break;
            }
        }
        self.get_up_to_date().grid_data
    }

    fn update_curve_bounds(&mut self) -> bool {
        log::debug!("updating curve bounds");
        let mut new_helices = self.helices.clone();
        let mut new_helices_mut = new_helices.make_mut();
        let mut replace = false;
        let parameters = self.parameters.unwrap_or_default();
        for (h_id, h) in self.helices.iter() {
            log::debug!("Helix {}", h_id);
            if let Some((n_min, n_max)) =
                self.strands.get_used_bounds_for_helix(*h_id, &self.helices)
            {
                log::debug!("bounds {} {}", n_min, n_max);
                if let Some(curve) = h.instanciated_curve.as_ref() {
                    if let Some(t_min) = curve.curve.left_extension_to_have_nucl(n_min, &parameters)
                    {
                        log::debug!("t_min {}", t_min);
                        if let Some(h_mut) = new_helices_mut.get_mut(h_id) {
                            replace |= h_mut
                                .curve
                                .as_mut()
                                .map(|c| Arc::make_mut(c).set_t_min(t_min))
                                .unwrap_or(false);
                        }
                    }
                    if let Some(t_max) =
                        curve.curve.right_extension_to_have_nucl(n_max, &parameters)
                    {
                        log::debug!("tmax {}", t_max);
                        if let Some(h_mut) = new_helices_mut.get_mut(h_id) {
                            replace |= h_mut
                                .curve
                                .as_mut()
                                .map(|c| Arc::make_mut(c).set_t_max(t_max))
                                .unwrap_or(false);
                        }
                    }
                }
            }
        }
        drop(new_helices_mut);
        if replace {
            self.helices = new_helices;
            true
        } else {
            false
        }
    }

    pub fn mut_strand_and_data(&mut self) -> MutStrandAndData {
        self.get_updated_grid_data();
        MutStrandAndData {
            strands: &mut self.strands,
            grid_data: self.instanciated_grid_data.as_ref().unwrap(),
            helices: &self.helices,
            parameters: self.parameters.unwrap_or_default(),
        }
    }
}

/// A structure that wraps a mutable reference to the design's strands along with a read only
/// access to the grid and helices.
pub struct MutStrandAndData<'a> {
    pub strands: &'a mut Strands,
    pub grid_data: &'a GridData,
    pub helices: &'a Helices,
    pub parameters: Parameters,
}

pub struct SavingInformation {
    pub camera: Option<Camera>,
}

impl Design {
    pub fn from_scadnano(scad: &ScadnanoDesign) -> Result<Self, ScadnanoImportError> {
        let mut grids = Vec::new();
        let mut group_map = BTreeMap::new();
        let default_grid = scad.default_grid_descriptor()?;
        let mut insertion_deletions = ScadnanoInsertionsDeletions::default();
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
            for d in s.domains.iter() {
                insertion_deletions.read_domain(d)
            }
        }
        let mut helices = BTreeMap::new();
        for (i, h) in scad.helices.iter().enumerate() {
            let helix = Helix::from_scadnano(h, &group_map, &groups, &mut helices_per_group)?;
            helices.insert(i, Arc::new(helix));
        }
        let mut strands = BTreeMap::new();
        for (i, s) in scad.strands.iter().enumerate() {
            let strand = Strand::from_scadnano(s, &insertion_deletions)?;
            strands.insert(i, strand);
        }
        println!("grids {:?}", grids);
        println!("helices {:?}", helices);
        Ok(Self {
            free_grids: FreeGrids::from_vec(grids),
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

    pub fn _set_helices(&mut self, helices: BTreeMap<usize, Arc<Helix>>) {
        self.helices = Helices(Arc::new(helices));
    }
}

/// Apply a mutating function to the value wrapped in an Arc<Helix>. This will make `helix_ptr`
/// point to a new helix on which the update has been applied.
pub fn mutate_in_arc<F, Obj: Clone>(obj_ptr: &mut Arc<Obj>, mut mutation: F)
where
    F: FnMut(&mut Obj),
{
    let mut new_obj = Obj::clone(obj_ptr);
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
