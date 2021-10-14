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
use crate::scene::GridInstance;
use ensnano_design::{grid::GridPosition, Nucl};
use ensnano_interactor::{ObjectType, Referential};
use std::collections::HashSet;
use ultraviolet::{Mat4, Rotor3, Vec3};

use crate::scene::DesignReader as Reader3D;

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

    fn get_grid_basis(&self, g_id: usize) -> Option<Rotor3> {
        self.presenter
            .current_design
            .grids
            .get(g_id)
            .map(|g| g.orientation)
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

    fn get_grid_position(&self, g_id: usize) -> Option<Vec3> {
        self.presenter
            .current_design
            .grids
            .get(g_id)
            .map(|g| g.position)
    }

    fn get_grid_instances(&self) -> Vec<GridInstance> {
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
        self.presenter.content.identifier_nucl.get(nucl).cloned()
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

    fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
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

    fn get_helix_grid_position(&self, h_id: u32) -> Option<GridPosition> {
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

    fn get_grid_latice_position(&self, g_id: usize, x: isize, y: isize) -> Option<Vec3> {
        self.presenter.content.get_grid_latice_position(g_id, x, y)
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

    fn get_helices_grid_key_coord(&self, g_id: usize) -> Option<Vec<((isize, isize), usize)>> {
        Some(self.presenter.content.get_helices_grid_key_coord(g_id))
    }

    fn get_helix_id_at_grid_coord(&self, g_id: usize, x: isize, y: isize) -> Option<u32> {
        self.presenter
            .content
            .get_helix_id_at_grid_coord(g_id, x, y)
            .map(|h_id| h_id as u32)
    }

    fn get_id_of_strand_containing(&self, e_id: u32) -> Option<usize> {
        self.presenter.content.strand_map.get(&e_id).cloned()
    }

    fn get_used_coordinates_on_grid(&self, g_id: usize) -> Option<Vec<(isize, isize)>> {
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
}

impl Presenter {}

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
