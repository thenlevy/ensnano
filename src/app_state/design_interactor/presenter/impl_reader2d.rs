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

use crate::design::{Extremity, Referential, Torsion};
use crate::flatscene::DesignReader as Reader2D;
use ahash::RandomState;
use ensnano_design::{Helix, Strand};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use ultraviolet::{Isometry2, Vec3};

impl Reader2D for DesignReader {
    fn get_isometry(&self, h_id: usize) -> Option<Isometry2> {
        todo!()
    }

    fn get_strand_points(&self, s_id: usize) -> Option<Vec<Nucl>> {
        todo!()
    }

    fn get_all_strand_ids(&self) -> Vec<usize> {
        todo!()
    }

    fn get_strand_color(&self, s_id: usize) -> Option<u32> {
        todo!()
    }

    fn get_torsions(&self) -> HashMap<(Nucl, Nucl), Torsion> {
        todo!()
    }

    fn get_raw_helix(&self, h_id: usize) -> Option<Helix> {
        todo!()
    }

    fn get_basis_map(&self) -> Arc<HashMap<Nucl, char, RandomState>> {
        todo!()
    }

    fn get_group_map(&self) -> Arc<BTreeMap<usize, bool>> {
        todo!()
    }

    fn get_insertions(&self, s_id: usize) -> Option<Vec<Nucl>> {
        todo!()
    }

    fn get_raw_strand(&self, s_id: usize) -> Option<Strand> {
        todo!()
    }

    fn get_copy_points(&self) -> Vec<Vec<Nucl>> {
        todo!()
    }

    fn get_suggestions(&self) -> Vec<(Nucl, Nucl)> {
        todo!()
    }

    fn get_xover_with_id(&self, xover_id: usize) -> Option<(Nucl, Nucl)> {
        todo!()
    }

    fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32> {
        todo!()
    }

    fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
        todo!()
    }

    fn get_visibility_helix(&self, h_id: usize) -> Option<bool> {
        todo!()
    }

    fn get_xovers_list_with_id(&self) -> Vec<(usize, (Nucl, Nucl))> {
        todo!()
    }

    fn get_position_of_nucl_on_helix(
        &self,
        nucl: Nucl,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3> {
        todo!()
    }

    fn get_id_of_strand_containing_elt(&self, e_id: u32) -> Option<usize> {
        todo!()
    }

    fn get_id_of_strand_containing_nucl(&self, nucl: &Nucl) -> Option<usize> {
        todo!()
    }

    fn get_id_of_of_helix_containing_elt(&self, e_id: u32) -> Option<usize> {
        todo!()
    }

    fn has_helix(&self, h_id: usize) -> bool {
        todo!()
    }

    fn can_start_builder_at(&self, nucl: Nucl) -> bool {
        todo!()
    }

    fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        todo!()
    }

    fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        todo!()
    }

    fn helix_is_empty(&self, h_id: usize) -> Option<bool> {
        todo!()
    }

    fn is_xover_end(&self, nucl: &Nucl) -> Extremity {
        todo!()
    }
}
