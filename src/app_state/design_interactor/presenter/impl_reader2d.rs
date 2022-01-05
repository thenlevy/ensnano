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

use crate::flatscene::DesignReader as Reader2D;
use ahash::RandomState;
use ensnano_design::{Domain, Extremity, Helix, Strand};
use ensnano_interactor::{torsion::Torsion, Referential};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use ultraviolet::{Isometry2, Vec3};

impl Reader2D for DesignReader {
    fn get_isometry(&self, h_id: usize) -> Option<Isometry2> {
        self.presenter
            .current_design
            .helices
            .get(&h_id)
            .and_then(|h| h.isometry2d)
    }

    fn get_strand_points(&self, s_id: usize) -> Option<Vec<Nucl>> {
        let strand = self.presenter.current_design.strands.get(&s_id)?;
        let mut ret = Vec::new();
        for domain in strand.domains.iter() {
            if let Domain::HelixDomain(domain) = domain {
                if domain.forward {
                    ret.push(Nucl::new(domain.helix, domain.start, domain.forward));
                    ret.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                } else {
                    ret.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                    ret.push(Nucl::new(domain.helix, domain.start, domain.forward));
                }
            }
        }
        if strand.cyclic {
            ret.push(ret[0])
        }
        Some(ret)
    }

    fn get_all_strand_ids(&self) -> Vec<usize> {
        self.presenter
            .current_design
            .strands
            .keys()
            .cloned()
            .collect()
    }

    fn get_strand_color(&self, s_id: usize) -> Option<u32> {
        self.presenter
            .current_design
            .strands
            .get(&s_id)
            .map(|s| s.color)
    }

    fn get_torsions(&self) -> HashMap<(Nucl, Nucl), Torsion> {
        HashMap::new()
    }

    fn get_raw_helix(&self, h_id: usize) -> Option<Arc<Helix>> {
        self.presenter.current_design.helices.get(&h_id).cloned()
    }

    fn get_basis_map(&self) -> Arc<HashMap<Nucl, char, RandomState>> {
        self.presenter.content.basis_map.clone()
    }

    fn get_group_map(&self) -> Arc<BTreeMap<usize, bool>> {
        self.presenter.current_design.groups.clone()
    }

    fn get_insertions(&self, s_id: usize) -> Option<Vec<Nucl>> {
        self.presenter
            .current_design
            .strands
            .get(&s_id)
            .map(|s| s.get_insertions())
    }

    fn get_raw_strand(&self, s_id: usize) -> Option<Strand> {
        self.presenter.current_design.strands.get(&s_id).cloned()
    }

    fn get_copy_points(&self) -> Vec<Vec<Nucl>> {
        self.controller.get_copy_points()
    }

    fn get_suggestions(&self) -> Vec<(Nucl, Nucl)> {
        self.presenter.content.suggestions.clone()
    }

    fn get_xover_with_id(&self, xover_id: usize) -> Option<(Nucl, Nucl)> {
        self.presenter.junctions_ids.get_element(xover_id)
    }

    fn get_identifier_nucl(&self, nucl: &Nucl) -> Option<u32> {
        self.presenter.content.identifier_nucl.get(nucl).cloned()
    }

    fn get_helices_on_grid(&self, g_id: usize) -> Option<HashSet<usize>> {
        self.presenter.content.get_helices_on_grid(g_id)
    }

    fn get_visibility_helix(&self, h_id: usize) -> Option<bool> {
        self.presenter
            .current_design
            .helices
            .get(&h_id)
            .map(|h| h.visible)
    }

    fn get_xovers_list_with_id(&self) -> Vec<(usize, (Nucl, Nucl))> {
        self.presenter.junctions_ids.get_all_elements()
    }

    fn get_position_of_nucl_on_helix(
        &self,
        nucl: Nucl,
        referential: Referential,
        on_axis: bool,
    ) -> Option<Vec3> {
        self.get_position_of_nucl_on_helix(nucl, referential, on_axis)
    }

    fn get_id_of_strand_containing_elt(&self, e_id: u32) -> Option<usize> {
        self.presenter.content.strand_map.get(&e_id).cloned()
    }

    fn get_id_of_strand_containing_nucl(&self, nucl: &Nucl) -> Option<usize> {
        self.get_id_of_strand_containing_nucl(nucl)
    }

    fn get_id_of_of_helix_containing_elt(&self, e_id: u32) -> Option<usize> {
        self.presenter.content.helix_map.get(&e_id).cloned()
    }

    fn has_helix(&self, h_id: usize) -> bool {
        self.presenter.current_design.helices.contains_key(&h_id)
    }

    fn can_start_builder_at(&self, nucl: Nucl) -> bool {
        self.presenter.can_start_builder_at(nucl)
    }

    fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        self.prime3_of_which_strand(nucl)
    }

    fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        self.prime5_of_which_strand(nucl)
    }

    fn helix_is_empty(&self, h_id: usize) -> Option<bool> {
        self.helix_is_empty(h_id)
    }

    fn is_xover_end(&self, nucl: &Nucl) -> Extremity {
        self.is_xover_end(nucl)
    }

    fn get_helices_map(&self) -> Arc<BTreeMap<usize, Arc<Helix>>> {
        self.presenter.current_design.helices.clone()
    }

    fn get_strand_ends(&self) -> Vec<Nucl> {
        self.presenter
            .current_design
            .strands
            .values()
            .flat_map(|s| Some([s.get_5prime()?, s.get_3prime()?]))
            .flatten()
            .collect()
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
    fn get_torsions_implemented() {
        assert!(false)
    }

    #[test]
    #[ignore]
    fn get_correct_visibility_helix() {
        assert!(false)
    }
}
