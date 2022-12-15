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
use ensnano_design::{
    grid::{GridId, GridObject, GridPosition, HelixGridPosition},
    BezierPlaneDescriptor, BezierPlaneId, BezierVertexId, Collection, CurveDescriptor, Nucl,
};
use ensnano_interactor::{
    graphics::{LoopoutBond, LoopoutNucl},
    BezierControlPoint, ObjectType, Referential,
};
use std::collections::HashSet;
use ultraviolet::{Mat4, Rotor3, Vec2, Vec3};

use crate::scene::{DesignReader as Reader3D, GridInstance, SurfaceInfo};

impl Reader3D for DesignReader {
    fn get_color(&self, e_id: u32) -> Option<u32> {
        self.presenter.content.color.get(&e_id).cloned()
    }

    fn get_basis(&self) -> Rotor3 {
        self.presenter.model_matrix.extract_rotation()
    }

    fn get_symbol(&self, e_id: u32) -> Option<char> {
        self.presenter
            .content
            .nucleotide
            .get(&e_id)
            .and_then(|nucl| self.presenter.content.basis_map.get(nucl))
            .cloned()
    }

    fn get_grid_basis(&self, g_id: GridId) -> Option<Rotor3> {
        match g_id {
            GridId::FreeGrid(_) => self
                .presenter
                .current_design
                .free_grids
                .get_from_g_id(&g_id)
                .map(|g| g.orientation),
            GridId::BezierPathGrid(vertex_id) => {
                let design_data = self.presenter.current_design.try_get_up_to_date()?;
                design_data.paths_data.orientation_vertex(vertex_id)
            }
        }
    }

    fn get_suggestions(&self) -> Vec<(Nucl, Nucl)> {
        self.presenter.content.suggestions.clone()
    }

    fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.presenter.content.object_type.get(&id).cloned()
    }

    fn get_helix_basis(&self, h_id: u32) -> Option<Rotor3> {
        self.presenter
            .current_design
            .helices
            .get(&(h_id as usize))
            .map(|h| h.orientation)
    }

    fn get_all_nucl_ids(&self) -> Vec<u32> {
        self.presenter.content.nucleotide.keys().cloned().collect()
    }

    fn get_model_matrix(&self) -> Mat4 {
        // Mat4 is Copy
        *self.presenter.model_matrix
    }

    fn get_nucl_with_id(&self, e_id: u32) -> Option<Nucl> {
        self.presenter.content.nucleotide.get(&e_id).cloned()
    }

    fn get_all_bound_ids(&self) -> Vec<u32> {
        self.presenter
            .content
            .nucleotides_involved
            .keys()
            .cloned()
            .collect()
    }

    fn get_grid_position(&self, g_id: GridId) -> Option<Vec3> {
        self.presenter
            .current_design
            .free_grids
            .get_from_g_id(&g_id)
            .map(|g| g.position)
    }

    fn get_grid_instances(&self) -> BTreeMap<GridId, GridInstance> {
        self.presenter.content.get_grid_instances()
    }

    fn get_pasted_position(&self) -> Vec<(Vec<Vec3>, bool)> {
        self.controller.get_pasted_position()
    }

    fn get_symbol_position(&self, e_id: u32) -> Option<Vec3> {
        let nucl = self.get_nucl_with_id(e_id)?;
        self.get_position_of_nucl_on_helix(nucl, Referential::World, false)
    }

    fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32> {
        self.presenter
            .content
            .nucl_collection
            .get_identifier(nucl)
            .cloned()
    }

    fn get_position_of_nucl_on_helix(
        &self,
        nucl: Nucl,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3> {
        let helix = self.presenter.current_design.helices.get(&nucl.helix)?;
        let parameters = self.presenter.current_design.parameters.unwrap_or_default();
        let position = if on_axis {
            helix.axis_position(&parameters, nucl.position)
        } else {
            helix.space_pos(&parameters, nucl.position, nucl.forward)
        };
        Some(self.presenter.in_referential(position, referential))
    }

    fn get_helices_on_grid(&self, g_id: GridId) -> Option<HashSet<usize>> {
        self.presenter.content.get_helices_on_grid(g_id)
    }

    fn get_all_prime3_nucl(&self) -> Vec<(Vec3, Vec3, u32)> {
        let locate_nucl = |nucl| {
            let pos_start_opt = self
                .get_identifier_nucl(&nucl)
                .and_then(|nucl_id| self.get_element_position(nucl_id, Referential::World));
            pos_start_opt.or(self.get_position_of_nucl_on_helix(nucl, Referential::World, false))
        };

        self.presenter
            .content
            .prime3_set
            .iter()
            .filter(|prime3| !self.presenter.invisible_nucls.contains(&prime3.nucl))
            .filter_map(|prime3| {
                let start = locate_nucl(prime3.nucl)?;
                let end = locate_nucl(prime3.nucl.prime3())?;
                Some((start, end, prime3.color))
            })
            .collect()
    }

    fn get_element_position(&self, e_id: u32, referential: Referential) -> Option<Vec3> {
        let position = self.presenter.content.get_element_position(e_id)?;
        Some(self.presenter.in_referential(position, referential))
    }

    fn get_identifier_bound(&self, n1: Nucl, n2: Nucl) -> Option<u32> {
        self.presenter
            .content
            .identifier_bound
            .get(&(n1, n2))
            .cloned()
    }

    fn get_helix_grid_position(&self, h_id: u32) -> Option<HelixGridPosition> {
        self.presenter
            .content
            .get_helix_grid_position(h_id as usize)
    }

    fn get_all_visible_nucl_ids(&self) -> Vec<u32> {
        self.presenter.content.get_all_visible_nucl_ids(
            &self.presenter.current_design,
            &self.presenter.invisible_nucls,
        )
    }

    fn get_grid_latice_position(&self, position: GridPosition) -> Option<Vec3> {
        self.presenter.content.get_grid_latice_position(position)
    }

    fn get_nucl_with_id_relaxed(&self, e_id: u32) -> Option<Nucl> {
        self.get_nucl_with_id(e_id).or(self
            .presenter
            .content
            .nucleotides_involved
            .get(&e_id)
            .map(|t| t.0))
    }

    fn get_all_visible_bound_ids(&self) -> Vec<u32> {
        self.presenter.content.get_all_visible_bounds(
            &self.presenter.current_design,
            &self.presenter.invisible_nucls,
        )
    }

    fn get_element_axis_position(&self, e_id: u32, referential: Referential) -> Option<Vec3> {
        if let Some(nucl) = self.get_nucl_with_id(e_id) {
            self.get_position_of_nucl_on_helix(nucl, referential, true)
        } else if let Some((n1, n2)) = self.presenter.content.nucleotides_involved.get(&e_id) {
            let a = self.get_position_of_nucl_on_helix(*n1, referential, true);
            let b = self.get_position_of_nucl_on_helix(*n2, referential, true);
            a.zip(b).map(|(a, b)| (a + b) / 2.)
        } else {
            None
        }
    }

    fn get_id_of_helix_containing(&self, e_id: u32) -> Option<usize> {
        if let Some(nucl) = self.get_nucl_with_id(e_id) {
            Some(nucl.helix)
        } else if let Some((n1, n2)) = self.presenter.content.nucleotides_involved.get(&e_id) {
            if n1.helix == n2.helix {
                Some(n1.helix)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_helices_grid_key_coord(&self, g_id: GridId) -> Option<Vec<((isize, isize), usize)>> {
        Some(self.presenter.content.get_helices_grid_key_coord(g_id))
    }

    fn get_helix_id_at_grid_coord(&self, position: GridPosition) -> Option<u32> {
        self.presenter
            .content
            .get_helix_id_at_grid_coord(position)
            .map(|h_id| h_id as u32)
    }

    fn get_id_of_strand_containing(&self, e_id: u32) -> Option<usize> {
        self.presenter.content.strand_map.get(&e_id).cloned()
    }

    fn get_used_coordinates_on_grid(&self, g_id: GridId) -> Option<Vec<(isize, isize)>> {
        Some(self.presenter.content.get_used_coordinates_on_grid(g_id))
    }

    fn get_persistent_phantom_helices_id(&self) -> HashSet<u32> {
        self.presenter.content.get_persistent_phantom_helices_id()
    }

    fn get_ids_of_elements_belonging_to_helix(&self, h_id: usize) -> Vec<u32> {
        let nucls = self
            .presenter
            .content
            .nucleotide
            .iter()
            .filter(|(_k, n)| n.helix == h_id)
            .map(|t| t.0);
        let bounds = self
            .presenter
            .content
            .nucleotides_involved
            .iter()
            .filter(|(_k, (n1, n2))| n1.helix == h_id && n2.helix == h_id)
            .map(|t| t.0);
        nucls.chain(bounds).cloned().collect()
    }

    fn get_ids_of_elements_belonging_to_strand(&self, s_id: usize) -> Vec<u32> {
        let belong_to_strand = |k: &&u32| self.presenter.content.strand_map.get(*k) == Some(&s_id);
        let nucls = self
            .presenter
            .content
            .nucleotide
            .keys()
            .filter(belong_to_strand);
        let bounds = self
            .presenter
            .content
            .nucleotides_involved
            .keys()
            .filter(belong_to_strand);
        nucls.chain(bounds).cloned().collect()
    }

    fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        self.prime5_of_which_strand(nucl)
    }

    fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        self.prime3_of_which_strand(nucl)
    }

    fn can_start_builder_at(&self, nucl: &Nucl) -> bool {
        self.presenter.can_start_builder_at(*nucl)
    }

    fn has_small_spheres_nucl_id(&self, e_id: u32) -> bool {
        if let Some(nucl) = self.get_nucl_with_id(e_id) {
            if let Some(grid_pos) = self.get_helix_grid_position(nucl.helix as u32) {
                self.presenter.content.grid_has_small_spheres(grid_pos.grid)
            } else {
                false
            }
        } else {
            false
        }
    }

    fn get_all_loopout_nucl(&self) -> &[LoopoutNucl] {
        &self.presenter.content.loopout_nucls
    }

    fn get_all_loopout_bonds(&self) -> &[LoopoutBond] {
        &self.presenter.content.loopout_bonds
    }

    fn get_insertion_length(&self, bond_id: u32) -> usize {
        // If the bond is not is the keys of insertion_length it means that it does not represent
        // an insertion
        self.presenter
            .content
            .insertion_length
            .get(&bond_id)
            .cloned()
            .unwrap_or(0)
    }

    fn get_expected_bond_length(&self) -> f32 {
        self.presenter
            .current_design
            .parameters
            .unwrap_or_default()
            .dist_ac()
    }

    fn get_all_h_bonds(&self) -> &[HBond] {
        self.presenter.bonds.as_ref()
    }

    fn get_position_of_bezier_control(
        &self,
        helix: usize,
        control: BezierControlPoint,
    ) -> Option<Vec3> {
        let helix = self.presenter.current_design.helices.get(&helix)?;
        if let BezierControlPoint::PiecewiseBezier(n) = control {
            let points = helix.piecewise_bezier_points()?;
            points.get(n).cloned()
        } else {
            let points = helix.cubic_bezier_points()?;
            match control {
                BezierControlPoint::CubicBezier(point) => points.get(usize::from(point)),
                BezierControlPoint::PiecewiseBezier { .. } => None,
            }
            .cloned()
        }
    }

    fn get_curve_range(&self, h_id: usize) -> Option<std::ops::RangeInclusive<isize>> {
        self.presenter
            .current_design
            .helices
            .get(&h_id)
            .and_then(|h| h.get_curve_range())
    }

    fn get_checked_xovers_ids(&self, checked: bool) -> Vec<u32> {
        if checked {
            self.presenter.get_checked_xovers_ids()
        } else {
            self.presenter.get_unchecked_xovers_ids()
        }
    }

    fn get_id_of_xover_involving_nucl(&self, nucl: Nucl) -> Option<usize> {
        self.presenter.get_id_of_xover_involving_nucl(nucl)
    }

    fn get_grid_object(&self, position: GridPosition) -> Option<GridObject> {
        self.presenter.content.get_grid_object(position)
    }

    fn get_cubic_bezier_controls(
        &self,
        helix: usize,
    ) -> Option<ensnano_design::CubicBezierConstructor> {
        let helix = self.presenter.current_design.helices.get(&helix)?;
        if let Some(CurveDescriptor::Bezier(constructor)) = helix.curve.as_ref().map(Arc::as_ref) {
            Some(constructor.clone())
        } else {
            None
        }
    }

    fn get_piecewise_bezier_controls(&self, helix: usize) -> Option<Vec<Vec3>> {
        let helix = self.presenter.current_design.helices.get(&helix)?;
        helix.piecewise_bezier_points()
    }

    fn get_curve_descriptor(&self, helix: usize) -> Option<&CurveDescriptor> {
        let helix = self.presenter.current_design.helices.get(&helix)?;
        helix.curve.as_ref().map(Arc::as_ref)
    }

    fn get_bezier_planes(
        &self,
    ) -> &dyn Collection<Key = BezierPlaneId, Item = BezierPlaneDescriptor> {
        &self.presenter.current_design.bezier_planes
    }

    fn get_bezier_paths(
        &self,
    ) -> Option<&BTreeMap<ensnano_design::BezierPathId, Arc<ensnano_design::InstanciatedPath>>>
    {
        self.presenter
            .current_design
            .try_get_up_to_date()
            .map(|data| data.paths_data.instanciated_paths.as_ref())
    }

    fn get_parameters(&self) -> Parameters {
        self.presenter.current_design.parameters.unwrap_or_default()
    }

    fn get_bezier_vertex(
        &self,
        path_id: ensnano_design::BezierPathId,
        vertex_id: usize,
    ) -> Option<ensnano_design::BezierVertex> {
        self.presenter
            .current_design
            .bezier_paths
            .get(&path_id)
            .and_then(|p| p.vertices().get(vertex_id))
            .cloned()
    }

    fn get_corners_of_plane(&self, plane_id: BezierPlaneId) -> [Vec2; 4] {
        let mut top = f32::INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        let mut left = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;

        for path in self.presenter.current_design.bezier_paths.values() {
            for v in path.vertices() {
                if v.plane_id == plane_id {
                    top = top.min(v.position.y);
                    bottom = bottom.max(v.position.y);
                    left = left.min(v.position.x);
                    right = right.max(v.position.x);
                }
            }
        }
        [
            Vec2::new(left, top),
            Vec2::new(right, top),
            Vec2::new(left, bottom),
            Vec2::new(right, bottom),
        ]
    }

    fn get_optimal_xover_arround(&self, source: Nucl, target: Nucl) -> Option<(Nucl, Nucl)> {
        let source_id = self.get_id_of_strand_containing_nucl(&source)?;
        let target_id = self.get_id_of_strand_containing_nucl(&target)?;
        let mut opt_pair = (source, target);
        let helix_source = self.presenter.current_design.helices.get(&source.helix)?;
        let helix_target = self.presenter.current_design.helices.get(&target.helix)?;
        let parameters = self.presenter.current_design.parameters.unwrap_or_default();
        let mut opt_dist = std::f32::INFINITY;
        for i in -2..2 {
            let source_candidate = Nucl {
                position: source.position + i,
                ..source
            };
            if self.get_id_of_strand_containing_nucl(&source_candidate) == Some(source_id) {
                for j in -2..2 {
                    let target_candidate = Nucl {
                        position: target.position + j,
                        ..target
                    };
                    if self.get_id_of_strand_containing_nucl(&target_candidate) == Some(target_id) {
                        let source_pos = helix_source.space_pos(
                            &parameters,
                            source_candidate.position,
                            source_candidate.forward,
                        );
                        let target_pos = helix_target.space_pos(
                            &parameters,
                            target_candidate.position,
                            target_candidate.forward,
                        );
                        let dist = (source_pos - target_pos).mag();
                        if dist < opt_dist {
                            opt_dist = dist;
                            opt_pair = (source_candidate, target_candidate);
                        }
                    }
                }
            }
        }
        Some(opt_pair)
    }

    fn get_bezier_grid_used_by_helix(&self, h_id: usize) -> Vec<GridId> {
        let helix = self.presenter.current_design.helices.get(&h_id);
        if let Some(CurveDescriptor::TranslatedPath { path_id, .. }) =
            helix.and_then(|h| h.curve.as_ref().map(Arc::as_ref))
        {
            if let Some(path) = self.presenter.current_design.bezier_paths.get(path_id) {
                (0..(path.vertices().len()))
                    .map(|i| {
                        GridId::BezierPathGrid(BezierVertexId {
                            path_id: *path_id,
                            vertex_id: i,
                        })
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    fn get_external_objects(&self) -> &ensnano_design::External3DObjects {
        &self.presenter.current_design.external_3d_objects
    }

    fn get_surface_info_nucl(&self, nucl: Nucl) -> Option<SurfaceInfo> {
        let helix = self.presenter.current_design.helices.get(&nucl.helix)?;
        helix.get_surface_info_nucl(nucl)
    }

    fn get_surface_info(&self, point: ensnano_design::SurfacePoint) -> Option<SurfaceInfo> {
        let helix = self.presenter.current_design.helices.get(&point.helix_id)?;
        helix.get_surface_info(point)
    }

    fn get_additional_structure(&self) -> Option<&dyn ensnano_design::AdditionalStructure> {
        self.presenter
            .current_design
            .additional_structure
            .as_ref()
            .map(Arc::as_ref)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    #[ignore]
    fn correct_suggestions() {
        // TODO: write test, and implement function
        assert!(false)
    }

    #[test]
    #[ignore]
    fn correct_pasted_position() {
        assert!(false)
    }

    #[test]
    #[ignore]
    fn nucls_are_filtered_by_visibility() {
        assert!(false)
    }
}
