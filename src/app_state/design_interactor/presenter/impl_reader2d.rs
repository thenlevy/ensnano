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
use ensnano_design::{Domain, Extremity, Helix, HelixInterval, Strand};
use ensnano_interactor::{torsion::Torsion, Referential};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use ultraviolet::{Isometry2, Vec3};

impl Reader2D for DesignReader {
    type NuclCollection = super::design_content::NuclCollection;
    fn get_isometry(&self, h_id: usize, segment_idx: usize) -> Option<Isometry2> {
        if segment_idx == 0 {
            self.presenter
                .current_design
                .helices
                .get(&h_id)
                .and_then(|h| h.isometry2d)
        } else {
            self.presenter
                .current_design
                .helices
                .get(&h_id)
                .and_then(|h| h.additonal_isometries.get(segment_idx - 1))
                .and_then(|i| i.additional_isometry)
        }
    }

    fn get_helix_segment_symmetry(
        &self,
        h_id: usize,
        segment_idx: usize,
    ) -> Option<ensnano_design::Vec2> {
        if segment_idx == 0 {
            self.presenter
                .current_design
                .helices
                .get(&h_id)
                .map(|h| h.symmetry)
        } else {
            self.presenter
                .current_design
                .helices
                .get(&h_id)
                .and_then(|h| h.additonal_isometries.get(segment_idx - 1))
                .and_then(|i| i.additional_symmetry)
        }
    }

    fn get_strand_points(&self, s_id: usize) -> Option<Vec<Nucl>> {
        let strand = self.presenter.current_design.strands.get(&s_id)?;
        let helices = &self.presenter.current_design.helices;
        let mut ret = Vec::new();
        for domain in strand.domains.iter() {
            if let Domain::HelixDomain(domain) = domain {
                ret.extend(split_domain_into_helices_segment(domain, helices));
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
        self.presenter
            .current_design
            .helices
            .get(&h_id)
            .cloned()
            .map(|h| Arc::new(h))
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
        self.presenter
            .content
            .nucl_collection
            .get_identifier(nucl)
            .cloned()
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

    fn get_helices_map(&self) -> &ensnano_design::Helices {
        &self.presenter.current_design.helices
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

    fn get_nucl_collection(&self) -> Arc<super::design_content::NuclCollection> {
        self.presenter.content.nucl_collection.clone()
    }

    fn get_abcissa_converter(&self, h_id: usize) -> ensnano_design::AbscissaConverter {
        self.presenter
            .current_design
            .try_get_up_to_date()
            .map(|data| data.grid_data.get_abscissa_converter(h_id))
            .unwrap_or_default()
    }
}

impl crate::flatscene::NuclCollection for super::design_content::NuclCollection {
    fn contains(&self, nucl: &Nucl) -> bool {
        self.contains_nucl(nucl)
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Nucl> + 'a> {
        self.iter_nucls()
    }
}

fn split_domain_into_helices_segment(
    domain: &HelixInterval,
    helices: &ensnano_design::Helices,
) -> Vec<Nucl> {
    let helix = helices.get(&domain.helix);
    let empty = vec![];
    let additional_segments = helix.map(|h| &h.additonal_isometries).unwrap_or(&empty);
    let mut ret = Vec::new();

    let intermediate_positions: Vec<isize> = additional_segments
        .iter()
        .map(|s| [s.left - 1, s.left])
        .flatten()
        .collect();

    let mut iter = intermediate_positions
        .into_iter()
        .skip_while(|pos| *pos <= domain.start);

    ret.push(Nucl {
        helix: domain.helix,
        forward: domain.forward,
        position: domain.start,
    });
    while let Some(position) = iter.next().filter(|pos| *pos < domain.end - 1) {
        ret.push(Nucl {
            helix: domain.helix,
            forward: domain.forward,
            position,
        });
    }
    ret.push(Nucl {
        helix: domain.helix,
        forward: domain.forward,
        position: domain.end - 1,
    });
    if !domain.forward {
        ret.reverse();
    }
    ret
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
