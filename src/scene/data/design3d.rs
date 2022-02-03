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
use super::super::maths_3d::{Basis3D, UnalignedBoundaries};
use super::super::view::{
    ConeInstance, Ellipsoid, Instanciable, RawDnaInstance, SphereInstance, TubeInstance,
};
use super::super::GridInstance;
use super::{LetterInstance, SceneElement};
use crate::consts::*;
use crate::utils::instance::Instance;
use ensnano_design::grid::{GridObject, GridPosition};
use ensnano_design::{grid::HelixGridPosition, Nucl};
use ensnano_design::{CubicBezierConstructor, CurveDescriptor};
use ensnano_interactor::{
    phantom_helix_encoder_bound, phantom_helix_encoder_nucl, BezierControlPoint, ObjectType,
    PhantomElement, Referential, PHANTOM_RANGE,
};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3};

/// An object that handles the 3d graphcial representation of a `Design`
pub struct Design3D<R: DesignReader> {
    design: R,
    id: u32,
    symbol_map: HashMap<char, usize>,
    pub thick_helices: bool,
}

impl<R: DesignReader> Design3D<R> {
    pub fn new(design: R, id: u32) -> Self {
        let mut symbol_map = HashMap::new();
        for (s_id, s) in BASIS_SYMBOLS.iter().enumerate() {
            symbol_map.insert(*s, s_id);
        }
        Self {
            design,
            id,
            symbol_map,
            thick_helices: true,
        }
    }

    /*
    /// Convert a list of ids into a list of instances
    pub fn id_to_instances(&self, ids: Vec<u32>) -> Vec<Instance> {
        let mut ret = Vec::new();
        for id in ids.iter() {
            if let Some(instance) = self.make_instance(*id) {
                ret.push(instance)
            }
        }
        ret
    }*/

    /// Convert a list of ids into a list of instances
    pub fn id_to_raw_instances(&self, ids: Vec<u32>) -> Vec<RawDnaInstance> {
        let mut ret = Vec::new();
        for id in ids.iter() {
            if let Some(instance) = self.make_raw_instance(*id) {
                ret.push(instance)
            }
        }
        ret
    }

    /*
    /// Return the list of sphere instances to be displayed to represent the design
    pub fn get_spheres(&self) -> Rc<Vec<Instance>> {
        let ids = self.design.lock().unwrap().get_all_nucl_ids();
        Rc::new(self.id_to_instances(ids))
    }
    */

    /// Return the list of raw sphere instances to be displayed to represent the design
    pub fn get_spheres_raw(&self) -> Rc<Vec<RawDnaInstance>> {
        let ids = self.design.get_all_visible_nucl_ids();
        Rc::new(self.id_to_raw_instances(ids))
    }

    pub fn get_pasted_strand(&self) -> (Vec<RawDnaInstance>, Vec<RawDnaInstance>) {
        let mut spheres = Vec::new();
        let mut tubes = Vec::new();
        let positions = self.design.get_pasted_position();
        for (positions, pastable) in positions {
            let mut previous_postion = None;
            let color = if pastable {
                CANDIDATE_COLOR
            } else {
                SELECTED_COLOR
            };
            let color_vec4 = Instance::color_from_au32(color);
            for position in positions.iter() {
                let sphere = SphereInstance {
                    position: *position,
                    color: color_vec4,
                    id: 0,
                    radius: 1.,
                }
                .to_raw_instance();
                spheres.push(sphere);
                if let Some(prev) = previous_postion {
                    let tube = create_dna_bound(prev, *position, color, 0, true);
                    tubes.push(tube.to_raw_instance());
                }
                previous_postion = Some(*position);
            }
        }
        (spheres, tubes)
    }

    pub fn get_letter_instances(&self) -> Vec<Vec<LetterInstance>> {
        let ids = self.design.get_all_nucl_ids();
        let mut vecs = vec![Vec::new(); NB_BASIS_SYMBOLS];
        for id in ids {
            let pos = self.design.get_symbol_position(id);
            let symbol = self.design.get_symbol(id);
            if let Some((pos, symbol)) = pos.zip(symbol) {
                if let Some(id) = self.symbol_map.get(&symbol) {
                    let instance = LetterInstance {
                        position: pos,
                        color: ultraviolet::Vec4::new(0., 0., 0., 1.),
                        design_id: self.id,
                        scale: 1.,
                        shift: Vec3::zero(),
                    };
                    vecs[*id].push(instance);
                }
            }
        }
        vecs
    }

    /*
    /// Return the list of tube instances to be displayed to represent the design
    pub fn get_tubes(&self) -> Rc<Vec<Instance>> {
        let ids = self.design.get_all_bound_ids();
        Rc::new(self.id_to_instances(ids))
    }
    */

    /// Return the list of tube instances to be displayed to represent the design
    pub fn get_tubes_raw(&self) -> Rc<Vec<RawDnaInstance>> {
        let ids = self.design.get_all_visible_bound_ids();
        Rc::new(self.id_to_raw_instances(ids))
    }

    pub fn get_model_matrix(&self) -> Mat4 {
        self.design.get_model_matrix()
    }

    /// Convert return an instance representing the object with identifier `id` and custom
    /// color and radius.
    pub fn make_instance(&self, id: u32, color: u32, mut radius: f32) -> Option<RawDnaInstance> {
        let kind = self.get_object_type(id)?;
        let instanciable = match kind {
            ObjectType::Bound(id1, id2) => {
                let pos1 =
                    self.get_graphic_element_position(&SceneElement::DesignElement(self.id, id1))?;
                let pos2 =
                    self.get_graphic_element_position(&SceneElement::DesignElement(self.id, id2))?;
                let id = id | self.id << 24;
                create_dna_bound(pos1, pos2, color, id, true)
                    .with_radius(radius)
                    .to_raw_instance()
            }
            ObjectType::Nucleotide(id) => {
                let position =
                    self.get_graphic_element_position(&SceneElement::DesignElement(self.id, id))?;
                let id = id | self.id << 24;
                let color = Instance::color_from_au32(color);
                let small = self.design.has_small_spheres_nucl_id(id);
                if radius > 1.01 && small {
                    radius *= 2.5;
                }
                radius = if small { radius / 3.5 } else { radius };
                SphereInstance {
                    position,
                    radius,
                    color,
                    id,
                }
                .to_raw_instance()
            }
        };
        Some(instanciable)
    }

    fn make_checked_xover_instance(&self, id: u32, checked: bool) -> Option<RawDnaInstance> {
        let referential = Referential::Model;
        if let Some(ObjectType::Bound(n1, n2)) = self.get_object_type(id) {
            let pos1 = self.get_design_element_position(n1, referential)?;
            let pos2 = self.get_design_element_position(n2, referential)?;
            Some(create_check_bound(pos1, pos2, checked))
        } else {
            None
        }
    }

    pub fn get_all_checked_xover_instance(&self, checked: bool) -> Vec<RawDnaInstance> {
        self.design
            .get_checked_xovers_ids(checked)
            .into_iter()
            .filter_map(|id| self.make_checked_xover_instance(id, checked))
            .collect()
    }

    /// Return (h bonds instances, ellipoids instances)
    pub fn get_all_hbond(&self) -> (Vec<RawDnaInstance>, Vec<RawDnaInstance>) {
        let mut hbonds = Vec::new();
        let mut ellipsoids = Vec::new();
        for hbond in self.design.get_all_h_bonds() {
            let forward_bond = create_dna_bound(
                hbond.forward.backbone,
                hbond.forward.center_of_mass,
                hbond.forward.backbone_color,
                0,
                false,
            );
            let backward_bond = create_dna_bound(
                hbond.backward.backbone,
                hbond.backward.center_of_mass,
                hbond.backward.backbone_color,
                0,
                false,
            );
            let forward_ellipsoid = Ellipsoid {
                orientation: forward_bond.rotor,
                scale: BASIS_SCALE,
                sphere: SphereInstance {
                    position: hbond.forward.center_of_mass,
                    color: Instance::color_from_u32(basis_color(hbond.forward.base.unwrap_or('?'))),
                    radius: 1.,
                    id: 0,
                },
            };
            let backward_ellipsoid = Ellipsoid {
                orientation: backward_bond.rotor,
                scale: BASIS_SCALE,
                sphere: SphereInstance {
                    position: hbond.backward.center_of_mass,
                    color: Instance::color_from_u32(basis_color(
                        hbond.backward.base.unwrap_or('?'),
                    )),
                    radius: 1.,
                    id: 0,
                },
            };
            hbonds.push(forward_bond.to_raw_instance());
            hbonds.push(backward_bond.to_raw_instance());
            ellipsoids.push(backward_ellipsoid.to_raw_instance());
            ellipsoids.push(forward_ellipsoid.to_raw_instance());
        }
        (hbonds, ellipsoids)
    }

    /// Convert return an instance representing the object with identifier `id`
    pub fn make_raw_instance(&self, id: u32) -> Option<RawDnaInstance> {
        let kind = self.get_object_type(id)?;
        let raw_instance = match kind {
            ObjectType::Bound(id1, id2) => {
                let pos1 =
                    self.get_graphic_element_position(&SceneElement::DesignElement(self.id, id1))?;
                let pos2 =
                    self.get_graphic_element_position(&SceneElement::DesignElement(self.id, id2))?;
                let color = self.get_color(id).unwrap_or(0);
                let id = id | self.id << 24;
                let tube = create_dna_bound(pos1, pos2, color, id, false);
                tube.to_raw_instance()
            }
            ObjectType::Nucleotide(id) => {
                let position =
                    self.get_graphic_element_position(&SceneElement::DesignElement(self.id, id))?;
                let color = self.get_color(id)?;
                let color = Instance::color_from_u32(color);
                let id = id | self.id << 24;
                let small = self.design.has_small_spheres_nucl_id(id);
                let radius = if small {
                    BOUND_RADIUS / SPHERE_RADIUS
                } else {
                    1.
                };
                let sphere = SphereInstance {
                    position,
                    color,
                    id,
                    radius,
                };
                sphere.to_raw_instance()
            }
        };
        Some(raw_instance)
    }

    pub fn get_suggested_spheres(&self) -> Vec<RawDnaInstance> {
        let suggestion = self.design.get_suggestions();
        let mut ret = vec![];
        for (n1, n2) in suggestion {
            let nucl_1 = self.design.get_position_of_nucl_on_helix(
                n1,
                Referential::Model,
                !self.thick_helices,
            );
            let nucl_2 = self.design.get_position_of_nucl_on_helix(
                n2,
                Referential::Model,
                !self.thick_helices,
            );
            if let Some(position) = nucl_1 {
                let instance = SphereInstance {
                    color: Instance::color_from_au32(SUGGESTION_COLOR),
                    position,
                    id: 0,
                    radius: SELECT_SCALE_FACTOR,
                }
                .to_raw_instance();
                ret.push(instance);
            }
            if let Some(position) = nucl_2 {
                let instance = SphereInstance {
                    color: Instance::color_from_au32(SUGGESTION_COLOR),
                    position,
                    id: 0,
                    radius: SELECT_SCALE_FACTOR,
                }
                .to_raw_instance();
                ret.push(instance);
            }
        }
        ret
    }

    pub fn get_suggested_tubes(&self) -> Vec<RawDnaInstance> {
        let suggestion = self.design.get_suggestions();
        let mut ret = vec![];
        for (n1, n2) in suggestion {
            let nucl_1 = self.design.get_position_of_nucl_on_helix(
                n1,
                Referential::Model,
                !self.thick_helices,
            );
            let nucl_2 = self.design.get_position_of_nucl_on_helix(
                n2,
                Referential::Model,
                !self.thick_helices,
            );
            if let Some((position1, position2)) = nucl_1.zip(nucl_2) {
                let instance = create_dna_bound(position1, position2, SUGGESTION_COLOR, 0, true)
                    .to_raw_instance();
                ret.push(instance);
            }
        }
        ret
    }

    /// Make a instance with the same postion and orientation as a phantom element.
    pub fn make_instance_phantom(
        &self,
        phantom_element: &PhantomElement,
        color: u32,
        radius: f32,
    ) -> Option<RawDnaInstance> {
        let nucl = Nucl {
            helix: phantom_element.helix_id as usize,
            position: phantom_element.position as isize,
            forward: phantom_element.forward,
        };
        let helix_id = phantom_element.helix_id;
        let i = phantom_element.position;
        let forward = phantom_element.forward;
        if phantom_element.bound {
            let nucl_1 =
                self.design
                    .get_position_of_nucl_on_helix(nucl, Referential::Model, false)?;
            let nucl_2 = self.design.get_position_of_nucl_on_helix(
                nucl.left(),
                Referential::Model,
                false,
            )?;
            let id = phantom_helix_encoder_bound(self.id, helix_id, i, forward);
            Some(create_dna_bound(nucl_1, nucl_2, color, id, true).to_raw_instance())
        } else {
            let nucl_coord =
                self.design
                    .get_position_of_nucl_on_helix(nucl, Referential::Model, false)?;
            let id = phantom_helix_encoder_nucl(self.id, helix_id, i, forward);
            let instance = SphereInstance {
                color: Instance::color_from_au32(color),
                position: nucl_coord,
                id,
                radius,
            }
            .to_raw_instance();
            Some(instance)
        }
    }

    pub fn get_phantom_element_position(
        &self,
        phantom_element: &PhantomElement,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3> {
        let helix_id = phantom_element.helix_id;
        let i = phantom_element.position;
        let forward = phantom_element.forward;
        let nucl = Nucl {
            helix: helix_id as usize,
            position: i as isize,
            forward,
        };
        if phantom_element.bound {
            let nucl_1 = self
                .design
                .get_position_of_nucl_on_helix(nucl, referential, on_axis)?;
            let nucl_2 =
                self.design
                    .get_position_of_nucl_on_helix(nucl.left(), referential, on_axis)?;
            Some((nucl_1 + nucl_2) / 2.)
        } else {
            let nucl_coord = self
                .design
                .get_position_of_nucl_on_helix(nucl, referential, on_axis);
            nucl_coord
        }
    }

    pub fn make_phantom_helix_instances_raw(
        &self,
        helix_ids: &HashMap<u32, bool>,
    ) -> (Rc<Vec<RawDnaInstance>>, Rc<Vec<RawDnaInstance>>) {
        let mut spheres = Vec::new();
        let mut tubes = Vec::new();
        for (helix_id, short) in helix_ids.iter() {
            let range_phantom = if *short {
                PHANTOM_RANGE / 10
            } else {
                PHANTOM_RANGE
            } as isize;
            for forward in [false, true].iter() {
                let mut previous_nucl = None;
                let range = self
                    .design
                    .get_curve_range(*helix_id as usize)
                    .unwrap_or(-range_phantom..=range_phantom);
                for i in range {
                    let i = i as i32;
                    let nucl_coord = self.design.get_position_of_nucl_on_helix(
                        Nucl {
                            helix: *helix_id as usize,
                            position: i as isize,
                            forward: *forward,
                        },
                        Referential::Model,
                        false,
                    );
                    let color = 0xFFD0D0D0;
                    if nucl_coord.is_none() {
                        continue;
                    }
                    let nucl_coord = nucl_coord.unwrap();
                    let id = phantom_helix_encoder_nucl(self.id, *helix_id, i, *forward);
                    spheres.push(
                        SphereInstance {
                            position: nucl_coord,
                            color: Instance::color_from_au32(color),
                            id,
                            radius: 0.6,
                        }
                        .to_raw_instance(),
                    );
                    if let Some(coord) = previous_nucl {
                        let id = phantom_helix_encoder_bound(self.id, *helix_id, i, *forward);
                        tubes.push(
                            create_dna_bound(nucl_coord, coord, color, id, true)
                                .with_radius(0.6)
                                .to_raw_instance(),
                        );
                    }
                    previous_nucl = Some(nucl_coord);
                }
            }
        }
        (Rc::new(spheres), Rc::new(tubes))
    }

    fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.design.get_object_type(id)
    }

    pub fn get_bound(&self, id: u32) -> Option<(Nucl, Nucl)> {
        if let Some(ObjectType::Bound(n1, n2)) = self.get_object_type(id) {
            self.get_nucl(n1).zip(self.get_nucl(n2))
        } else {
            None
        }
    }

    pub fn get_element_position(
        &self,
        element: &SceneElement,
        referential: Referential,
    ) -> Option<Vec3> {
        match element {
            SceneElement::DesignElement(_, e_id) => {
                self.get_design_element_position(*e_id, referential)
            }
            SceneElement::PhantomElement(phantom) => {
                self.get_phantom_element_position(phantom, referential, false)
            }
            SceneElement::Grid(_, g_id) => self.design.get_grid_position(*g_id),
            SceneElement::GridCircle(_, position) => {
                self.design.get_grid_latice_position(*position)
            }
            _ => None,
        }
    }

    fn get_graphic_element_position(&self, element: &SceneElement) -> Option<Vec3> {
        if self.thick_helices {
            self.get_element_position(element, Referential::World)
        } else {
            self.get_element_axis_position(element, Referential::World)
        }
    }

    pub fn get_element_axis_position(
        &self,
        element: &SceneElement,
        referential: Referential,
    ) -> Option<Vec3> {
        match element {
            SceneElement::DesignElement(_, e_id) => {
                self.get_design_element_axis_position(*e_id, referential)
            }
            SceneElement::PhantomElement(phantom) => {
                self.get_phantom_element_position(phantom, referential, true)
            }
            SceneElement::WidgetElement(_)
            | SceneElement::Grid(_, _)
            | SceneElement::BezierControl { .. }
            | SceneElement::GridCircle(_, _) => None,
        }
    }

    pub fn get_design_element_position(&self, id: u32, referential: Referential) -> Option<Vec3> {
        self.design.get_element_position(id, referential)
    }

    pub fn get_design_element_axis_position(
        &self,
        id: u32,
        referential: Referential,
    ) -> Option<Vec3> {
        self.design.get_element_axis_position(id, referential)
    }

    fn get_color(&self, id: u32) -> Option<u32> {
        self.design.get_color(id)
    }

    /// Return the middle point of `self` in the world coordinates
    pub fn middle_point(&self) -> Vec3 {
        let boundaries = self.boundaries();
        let middle = Vec3::new(
            (boundaries[0] + boundaries[1]) as f32 / 2.,
            (boundaries[2] + boundaries[3]) as f32 / 2.,
            (boundaries[4] + boundaries[5]) as f32 / 2.,
        );
        self.design.get_model_matrix().transform_vec3(middle)
    }

    fn boundaries(&self) -> [f32; 6] {
        let mut min_x = std::f32::INFINITY;
        let mut min_y = std::f32::INFINITY;
        let mut min_z = std::f32::INFINITY;
        let mut max_x = std::f32::NEG_INFINITY;
        let mut max_y = std::f32::NEG_INFINITY;
        let mut max_z = std::f32::NEG_INFINITY;

        let ids = self.design.get_all_nucl_ids();
        for id in ids {
            let coord: [f32; 3] = self
                .design
                .get_element_position(id, Referential::World)
                .unwrap()
                .into();
            if coord[0] < min_x {
                min_x = coord[0];
            }
            if coord[0] > max_x {
                max_x = coord[0];
            }
            if coord[1] < min_y {
                min_y = coord[1];
            }
            if coord[1] > max_y {
                max_y = coord[1];
            }
            if coord[2] < min_z {
                min_z = coord[2];
            }
            if coord[2] > max_z {
                max_z = coord[2];
            }
        }
        for grid in self.get_grid().iter() {
            let coords: [[f32; 3]; 2] = [
                grid.grid
                    .position_helix(grid.min_x as isize, grid.min_y as isize)
                    .into(),
                grid.grid
                    .position_helix(grid.max_x as isize, grid.max_y as isize)
                    .into(),
            ];
            for coord in coords.iter() {
                if coord[0] < min_x {
                    min_x = coord[0];
                }
                if coord[0] > max_x {
                    max_x = coord[0];
                }
                if coord[1] < min_y {
                    min_y = coord[1];
                }
                if coord[1] > max_y {
                    max_y = coord[1];
                }
                if coord[2] < min_z {
                    min_z = coord[2];
                }
                if coord[2] > max_z {
                    max_z = coord[2];
                }
            }
        }
        [min_x, max_x, min_y, max_y, min_z, max_z]
    }

    /// Return the list of corners of grid with no helices on them
    fn get_all_naked_grids_corners(&self) -> Vec<Vec3> {
        let mut ret = Vec::new();
        for grid in self.get_grid().iter() {
            if self
                .design
                .get_helices_on_grid(grid.id)
                .map(|s| s.is_empty())
                .unwrap_or(false)
            {
                ret.push(
                    grid.grid
                        .position_helix(grid.min_x as isize, grid.min_y as isize),
                );
                ret.push(
                    grid.grid
                        .position_helix(grid.min_x as isize, grid.max_y as isize),
                );
                ret.push(
                    grid.grid
                        .position_helix(grid.max_x as isize, grid.min_y as isize),
                );
                ret.push(
                    grid.grid
                        .position_helix(grid.max_x as isize, grid.max_y as isize),
                );
            }
        }
        ret
    }

    fn get_all_points(&self) -> Vec<Vec3> {
        let ids = self.design.get_all_nucl_ids();
        let mut ret: Vec<Vec3> = ids
            .iter()
            .filter_map(|id| self.design.get_element_position(*id, Referential::World))
            .collect();
        ret.extend(self.get_all_naked_grids_corners().into_iter());
        ret
    }

    fn boundaries_unaligned(&self, basis: Basis3D) -> UnalignedBoundaries {
        let mut ret = UnalignedBoundaries::from_basis(basis);
        for point in self.get_all_points().into_iter() {
            ret.add_point(point)
        }
        ret
    }

    pub fn get_fitting_camera_position(
        &self,
        basis: Basis3D,
        fovy: f32,
        ratio: f32,
    ) -> Option<Vec3> {
        let boundaries = self.boundaries_unaligned(basis);
        boundaries.fit_point(fovy, ratio)
    }

    pub fn get_all_elements(&self) -> HashSet<u32> {
        let mut ret = HashSet::new();
        for x in self.design.get_all_nucl_ids().iter() {
            ret.insert(*x);
        }
        for x in self.design.get_all_bound_ids().iter() {
            ret.insert(*x);
        }
        ret
    }

    pub fn get_strand(&self, element_id: u32) -> Option<usize> {
        self.design.get_id_of_strand_containing(element_id)
    }

    pub fn get_helix(&self, element_id: u32) -> Option<usize> {
        self.design.get_id_of_helix_containing(element_id)
    }

    pub fn get_strand_elements(&self, strand_id: u32) -> HashSet<u32> {
        self.design
            .get_ids_of_elements_belonging_to_strand(strand_id as usize)
            .into_iter()
            .collect()
    }

    pub fn get_element_type(&self, e_id: u32) -> Option<ObjectType> {
        self.design.get_object_type(e_id)
    }

    pub fn get_helix_elements(&self, helix_id: u32) -> HashSet<u32> {
        self.design
            .get_ids_of_elements_belonging_to_helix(helix_id as usize)
            .into_iter()
            .collect()
    }

    pub fn get_helix_basis(&self, h_id: u32) -> Option<Rotor3> {
        self.design.get_helix_basis(h_id)
    }

    pub fn get_basis(&self) -> Rotor3 {
        self.design.get_basis()
    }

    pub fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32> {
        self.design.get_identifier_nucl(nucl)
    }

    pub fn get_identifier_bound(&self, n1: Nucl, n2: Nucl) -> Option<u32> {
        self.design.get_identifier_bound(n1, n2)
    }

    pub fn get_element_identifier_from_xover_id(&self, xover_id: usize) -> Option<u32> {
        self.design
            .get_xover_with_id(xover_id)
            .and_then(|(n1, n2)| self.design.get_identifier_bound(n1, n2))
    }

    pub fn get_xover_id(&self, xover: &(Nucl, Nucl)) -> Option<usize> {
        self.design.get_xover_id(xover)
    }

    pub fn get_xover_with_id(&self, xover_id: usize) -> Option<(Nucl, Nucl)> {
        self.design.get_xover_with_id(xover_id)
    }

    pub fn can_start_builder(&self, element: &SceneElement) -> Option<Nucl> {
        match element {
            SceneElement::DesignElement(_, e_id) => self.can_start_builder_on_element(*e_id),
            SceneElement::PhantomElement(phantom_element) => {
                self.can_start_builder_on_phantom(phantom_element)
            }
            _ => None,
        }
    }

    fn can_start_builder_on_element(&self, e_id: u32) -> Option<Nucl> {
        let nucl = self.design.get_nucl_with_id(e_id);
        nucl.filter(|n| self.design.can_start_builder_at(n))
    }

    fn can_start_builder_on_phantom(&self, phantom_element: &PhantomElement) -> Option<Nucl> {
        let nucl = Nucl {
            helix: phantom_element.helix_id as usize,
            position: phantom_element.position as isize,
            forward: phantom_element.forward,
        };
        if self.design.can_start_builder_at(&nucl) {
            Some(nucl)
        } else {
            None
        }
    }

    pub fn get_grid(&self) -> Vec<GridInstance> {
        self.design.get_grid_instances()
    }

    pub fn get_helices_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
        self.design.get_helices_on_grid(g_id)
    }

    pub fn get_helices_grid_coord(&self, g_id: usize) -> Vec<(isize, isize)> {
        self.design
            .get_used_coordinates_on_grid(g_id)
            .unwrap_or(Vec::new())
    }

    pub fn get_helices_grid_key_coord(&self, g_id: usize) -> Vec<((isize, isize), usize)> {
        self.design
            .get_helices_grid_key_coord(g_id)
            .unwrap_or(Vec::new())
    }

    pub fn get_helix_grid(&self, position: GridPosition) -> Option<u32> {
        self.design.get_helix_id_at_grid_coord(position)
    }

    pub fn get_grid_object(&self, position: GridPosition) -> Option<GridObject> {
        self.design.get_grid_object(position)
    }

    pub fn get_persistent_phantom_helices(&self) -> HashSet<u32> {
        self.design.get_persistent_phantom_helices_id()
    }

    pub fn get_grid_basis(&self, g_id: usize) -> Option<Rotor3> {
        self.design.get_grid_basis(g_id)
    }

    pub fn get_nucl(&self, e_id: u32) -> Option<Nucl> {
        self.design.get_nucl_with_id(e_id)
    }

    pub fn get_nucl_relax(&self, e_id: u32) -> Option<Nucl> {
        self.design.get_nucl_with_id_relaxed(e_id)
    }

    pub fn get_helix_grid_position(&self, h_id: u32) -> Option<HelixGridPosition> {
        self.design.get_helix_grid_position(h_id)
    }

    pub fn get_nucl_position(&self, nucl: Nucl) -> Option<Vec3> {
        self.design
            .get_position_of_nucl_on_helix(nucl, Referential::World, false)
    }

    pub fn pivot_sphere(position: Vec3) -> RawDnaInstance {
        SphereInstance {
            position,
            id: 0,
            radius: 1.2 * SELECT_SCALE_FACTOR,
            color: Instance::color_from_au32(PIVOT_SPHERE_COLOR),
        }
        .to_raw_instance()
    }

    pub fn free_xover_sphere(position: Vec3) -> RawDnaInstance {
        SphereInstance {
            position,
            id: 0,
            radius: 1.1 * SELECT_SCALE_FACTOR,
            color: Instance::color_from_au32(FREE_XOVER_COLOR),
        }
        .to_raw_instance()
    }

    pub fn free_xover_tube(pos1: Vec3, pos2: Vec3) -> RawDnaInstance {
        create_dna_bound(pos1, pos2, FREE_XOVER_COLOR, 0, true).to_raw_instance()
    }

    pub fn has_nucl(&self, nucl: &Nucl) -> bool {
        self.design.get_identifier_nucl(nucl).is_some()
    }

    pub fn both_prime3(&self, nucl1: Nucl, nucl2: Nucl) -> bool {
        let prime3_1 = self.design.prime3_of_which_strand(nucl1);
        let prime3_2 = self.design.prime3_of_which_strand(nucl2);
        prime3_1.and(prime3_2).is_some()
    }

    pub fn both_prime5(&self, nucl1: Nucl, nucl2: Nucl) -> bool {
        let prime5_1 = self.design.prime5_of_which_strand(nucl1);
        let prime5_2 = self.design.prime5_of_which_strand(nucl2);
        prime5_1.and(prime5_2).is_some()
    }

    pub fn get_all_prime3_cone(&self) -> Vec<RawDnaInstance> {
        if !self.thick_helices {
            return vec![];
        }
        let cones = self.design.get_all_prime3_nucl();
        let mut ret = Vec::with_capacity(cones.len());
        for c in cones {
            ret.push(create_prime3_cone(c.0, c.1, c.2));
        }
        ret
    }

    pub fn get_bezier_elements(&self, h_id: usize) -> (Vec<RawDnaInstance>, Vec<RawDnaInstance>) {
        let mut spheres = Vec::new();
        let mut tubes = Vec::new();
        if let Some(constructor) = self.design.get_cubic_bezier_controls(h_id) {
            log::info!("got control");
            for (control_point, position) in constructor.iter() {
                spheres.push(make_bezier_controll(
                    *position,
                    h_id as u32,
                    BezierControlPoint::CubicBezier(control_point),
                ));
            }
            tubes.push(make_bezier_squelton(
                constructor.start,
                constructor.control1,
            ));
            tubes.push(make_bezier_squelton(
                constructor.control1,
                constructor.control2,
            ));
            tubes.push(make_bezier_squelton(constructor.control2, constructor.end));
            (spheres, tubes)
        } else if let Some(controls) = self.design.get_piecewise_bezier_controls(h_id) {
            let mut iter = controls.into_iter().enumerate();
            while let Some(((n1, c1), (n2, c2))) = iter.next().zip(iter.next()) {
                spheres.push(make_bezier_controll(
                    c1,
                    h_id as u32,
                    BezierControlPoint::PiecewiseBezier(n1),
                ));
                spheres.push(make_bezier_controll(
                    c2,
                    h_id as u32,
                    BezierControlPoint::PiecewiseBezier(n2),
                ));
                tubes.push(make_bezier_squelton(c1, c2));
            }
            (spheres, tubes)
        } else {
            (vec![], vec![])
        }
    }

    pub fn get_control_point(&self, helix_id: usize, control: BezierControlPoint) -> Option<Vec3> {
        self.design
            .get_position_of_bezier_control(helix_id, control)
    }

    pub fn get_bezier_control_basis(
        &self,
        h_id: usize,
        bezier_control: BezierControlPoint,
    ) -> Option<Rotor3> {
        log::info!(
            "Getting bezier basis {:?} of helix {}",
            bezier_control,
            h_id
        );
        match bezier_control {
            BezierControlPoint::CubicBezier(_) => None,
            BezierControlPoint::PiecewiseBezier(n) => {
                let descriptor = self.design.get_curve_descriptor(h_id)?;
                if let CurveDescriptor::PiecewiseBezier {
                    points, tengents, ..
                } = descriptor
                {
                    // There are two control points per bezier grid position
                    let g_id = points.get(n / 2).map(|pos| pos.grid)?;
                    let grid_orientation = self.design.get_grid_basis(g_id)?;
                    tengents.get(n / 2).map(|t| {
                        let world_tengent = t.normalized().rotated_by(grid_orientation);
                        if world_tengent.dot(Vec3::unit_x()) > -0.999 {
                            Rotor3::from_rotation_between(Vec3::unit_x(), world_tengent)
                        } else {
                            Rotor3::identity()
                        }
                    })
                } else {
                    None
                }
            }
        }
    }
}

fn make_bezier_controll(
    position: Vec3,
    helix_id: u32,
    bezier_control: BezierControlPoint,
) -> RawDnaInstance {
    let id = bezier_widget_id(helix_id, bezier_control);
    let color = bezier_control_color(bezier_control);
    SphereInstance {
        position,
        id,
        color: Instance::color_from_au32(color),
        radius: BEZIER_CONTROL_RADIUS,
    }
    .to_raw_instance()
}

fn make_bezier_squelton(source: Vec3, dest: Vec3) -> RawDnaInstance {
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());
    let position = (dest + source) / 2.;
    let length = (dest - source).mag();

    TubeInstance {
        position,
        color: Instance::color_from_u32(0),
        id: 0,
        rotor,
        radius: BEZIER_SQUELETON_RADIUS,
        length,
    }
    .to_raw_instance()
}

fn create_dna_bound(
    source: Vec3,
    dest: Vec3,
    color: u32,
    id: u32,
    use_alpha: bool,
) -> TubeInstance {
    let color = if use_alpha {
        Instance::color_from_au32(color)
    } else {
        Instance::color_from_u32(color)
    };
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());
    let position = (dest + source) / 2.;
    let length = (dest - source).mag();

    TubeInstance {
        position,
        color,
        rotor,
        id,
        radius: 1.,
        length,
    }
}

fn create_check_bound(source: Vec3, dest: Vec3, checked: bool) -> RawDnaInstance {
    let radius = (source - dest).mag() / 2. / SPHERE_RADIUS;
    let position = (source + dest) / 2.;
    let color = if checked {
        Instance::color_from_au32(CHECKED_XOVER_COLOR)
    } else {
        Instance::color_from_au32(UNCHECKED_XOVER_COLOR)
    };
    SphereInstance {
        position,
        radius,
        color,
        id: 0,
    }
    .to_raw_instance()
}

fn create_prime3_cone(source: Vec3, dest: Vec3, color: u32) -> RawDnaInstance {
    let color = Instance::color_from_u32(color);
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());
    let position = source;
    let length = (2. / 3. * (dest - source).mag()).min(2. / 3. * 0.7);
    ConeInstance {
        position,
        length,
        rotor,
        color,
        id: 0,
        radius: 1.5 * SPHERE_RADIUS,
    }
    .to_raw_instance()
}

#[derive(Debug, Clone)]
pub struct HalfHBond {
    pub backbone: Vec3,
    pub center_of_mass: Vec3,
    pub base: Option<char>,
    pub backbone_color: u32,
}

#[derive(Debug, Clone)]
pub struct HBond {
    pub forward: HalfHBond,
    pub backward: HalfHBond,
}

pub trait DesignReader: 'static + ensnano_interactor::DesignReader {
    /// Return the identifier of all the visible nucleotides
    fn get_all_visible_nucl_ids(&self) -> Vec<u32>;
    /// Return the identifier of all the visible bounds
    fn get_all_visible_bound_ids(&self) -> Vec<u32>;
    fn get_all_nucl_ids(&self) -> Vec<u32>;
    fn get_all_bound_ids(&self) -> Vec<u32>;
    fn get_pasted_position(&self) -> Vec<(Vec<Vec3>, bool)>;
    /// If e_id is the identifier of a nucleotide, return the position on which the
    /// nucleotide's symbols must be displayed
    fn get_symbol_position(&self, e_id: u32) -> Option<Vec3>;
    /// If e_id is the identifier of a nucleotide, return the symbol associated to the
    /// nucleotide.
    fn get_symbol(&self, e_id: u32) -> Option<char>;
    fn get_model_matrix(&self) -> Mat4;
    /// Return true iff e_id is the identifier of a nucleotide that must be displayed with a
    /// smaller size
    fn has_small_spheres_nucl_id(&self, e_id: u32) -> bool;
    /// Return the list of pairs of nucleotides that can be linked by a cross-over
    fn get_suggestions(&self) -> Vec<(Nucl, Nucl)>;
    fn get_position_of_nucl_on_helix(
        &self,
        nucl: Nucl,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3>;
    fn get_object_type(&self, id: u32) -> Option<ObjectType>;
    fn get_grid_position(&self, g_id: usize) -> Option<Vec3>;
    fn get_grid_latice_position(&self, position: GridPosition) -> Option<Vec3>;
    fn get_element_position(&self, e_id: u32, referential: Referential) -> Option<Vec3>;
    fn get_element_axis_position(&self, id: u32, referential: Referential) -> Option<Vec3>;
    fn get_color(&self, e_id: u32) -> Option<u32>;
    fn get_id_of_strand_containing(&self, e_id: u32) -> Option<usize>;
    fn get_id_of_helix_containing(&self, e_id: u32) -> Option<usize>;
    fn get_ids_of_elements_belonging_to_strand(&self, s_id: usize) -> Vec<u32>;
    fn get_ids_of_elements_belonging_to_helix(&self, h_id: usize) -> Vec<u32>;
    fn get_helix_basis(&self, h_id: u32) -> Option<Rotor3>;
    fn get_basis(&self) -> Rotor3;
    fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32>;
    fn get_identifier_bound(&self, n1: Nucl, n2: Nucl) -> Option<u32>;
    fn get_nucl_with_id(&self, e_id: u32) -> Option<Nucl>;
    /// Return the nucleotide with id e_id or the 5' end of the bound with id e_id
    fn get_nucl_with_id_relaxed(&self, e_id: u32) -> Option<Nucl>;
    fn can_start_builder_at(&self, nucl: &Nucl) -> bool;
    fn get_grid_instances(&self) -> Vec<GridInstance>;
    fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>>;
    fn get_used_coordinates_on_grid(&self, g_id: usize) -> Option<Vec<(isize, isize)>>;
    fn get_helices_grid_key_coord(&self, g_id: usize) -> Option<Vec<((isize, isize), usize)>>;
    fn get_helix_id_at_grid_coord(&self, position: GridPosition) -> Option<u32>;
    fn get_persistent_phantom_helices_id(&self) -> HashSet<u32>;
    fn get_grid_basis(&self, g_id: usize) -> Option<Rotor3>;
    fn get_helix_grid_position(&self, h_id: u32) -> Option<HelixGridPosition>;
    fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize>;
    fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize>;
    fn get_all_prime3_nucl(&self) -> Vec<(Vec3, Vec3, u32)>;
    fn get_curve_range(&self, h_id: usize) -> Option<std::ops::RangeInclusive<isize>>;
    fn get_checked_xovers_ids(&self, checked: bool) -> Vec<u32>;
    fn get_id_of_xover_involving_nucl(&self, nucl: Nucl) -> Option<usize>;
    fn get_grid_object(&self, position: GridPosition) -> Option<GridObject>;
    fn get_position_of_bezier_control(
        &self,
        helix: usize,
        control: BezierControlPoint,
    ) -> Option<Vec3>;
    fn get_cubic_bezier_controls(&self, helix: usize) -> Option<CubicBezierConstructor>;
    fn get_piecewise_bezier_controls(&self, helix: usize) -> Option<Vec<Vec3>>;
    fn get_curve_descriptor(&self, helix: usize) -> Option<&CurveDescriptor>;
    fn get_all_h_bonds(&self) -> &[HBond];
}
