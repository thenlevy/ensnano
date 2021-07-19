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
use ensnano_design::{Extremity, Nucl};
use ensnano_interactor::{NeighbourDescriptor, NeighbourDescriptorGiver, ScaffoldInfo};
use ultraviolet::Mat4;

use crate::utils::id_generator::IdGenerator;
type JunctionsIds = IdGenerator<(Nucl, Nucl)>;
mod design_content;
mod impl_main_reader;
mod impl_reader2d;
mod impl_reader3d;
mod impl_readergui;
mod oxdna;
use ahash::AHashMap;
use design_content::DesignContent;
use std::collections::BTreeMap;

#[derive(Clone)]
/// The structure that handles "read" operations on designs.
///
/// It contains several data structure that are pre-computed to allow quicker response to the read
/// requests. The strategy to ensure that the data structure are updated when the design is
/// modified is the following:
/// When the data structures are updated, a pointer to the design that was used to build them is
/// stored. To obtain a design reader, a pointer to the current design must be given. If the given
/// pointer does not point to the same address as the one that was used to create the data
/// structures, the strucutres are updated before returning the design reader.
pub(super) struct Presenter {
    pub current_design: AddressPointer<Design>,
    model_matrix: AddressPointer<Mat4>,
    content: AddressPointer<DesignContent>,
    pub junctions_ids: AddressPointer<JunctionsIds>,
    helices_groups: AddressPointer<BTreeMap<usize, bool>>,
    old_grid_ptr: Option<usize>,
}

impl Default for Presenter {
    fn default() -> Self {
        Self {
            current_design: Default::default(),
            model_matrix: AddressPointer::new(Mat4::identity()),
            content: Default::default(),
            junctions_ids: Default::default(),
            helices_groups: Default::default(),
            old_grid_ptr: None,
        }
    }
}

impl Presenter {
    pub fn can_start_builder_at(&self, nucl: Nucl) -> bool {
        let left = self.current_design.get_neighbour_nucl(nucl.left());
        let right = self.current_design.get_neighbour_nucl(nucl.right());
        if self.content.identifier_nucl.contains_key(&nucl) {
            if let Some(desc) = self.current_design.get_neighbour_nucl(nucl) {
                let filter = |d: &NeighbourDescriptor| d.identifier != desc.identifier;
                !left.filter(filter).and(right.filter(filter)).is_some()
            } else {
                false
            }
        } else {
            !(left.is_some() && right.is_some())
        }
    }

    pub fn update(mut self, design: AddressPointer<Design>) -> Self {
        if self.current_design != design {
            self.read_design(design);
            self.read_scaffold_seq();
            self.update_visibility();
        }
        self
    }

    /// Return a fresh presenter presenting an imported `Design` with a given set of junctions, as
    /// well as a pointer to the design held by this fresh presenter.
    pub fn from_new_design(
        design: Design,
        old_junctions_ids: &JunctionsIds,
    ) -> (Self, AddressPointer<Design>) {
        let helices_groups = design.groups.clone();
        let model_matrix = Mat4::identity();
        let mut old_grid_ptr = None;
        let (content, design, junctions_ids) = DesignContent::make_hash_maps(
            design,
            &helices_groups,
            old_junctions_ids,
            &mut old_grid_ptr,
        );
        let design = AddressPointer::new(design);
        let ret = Self {
            current_design: design.clone(),
            content: AddressPointer::new(content),
            model_matrix: AddressPointer::new(model_matrix),
            junctions_ids: AddressPointer::new(junctions_ids),
            helices_groups: AddressPointer::from(helices_groups),
            old_grid_ptr,
        };
        (ret, design)
    }

    fn read_design(&mut self, design: AddressPointer<Design>) {
        let (content, new_design, new_junctions_ids) = DesignContent::make_hash_maps(
            design.clone_inner(),
            self.helices_groups.as_ref(),
            self.junctions_ids.as_ref(),
            &mut self.old_grid_ptr,
        );
        self.current_design = AddressPointer::new(new_design);
        self.content = AddressPointer::new(content);
        self.junctions_ids = AddressPointer::new(new_junctions_ids);
    }

    pub(super) fn has_different_model_matrix_than(&self, other: &Self) -> bool {
        self.model_matrix != other.model_matrix
    }

    fn read_scaffold_seq(&mut self) {
        ()
    }

    fn update_visibility(&mut self) {
        ()
    }

    fn in_referential(&self, position: Vec3, referential: Referential) -> Vec3 {
        match referential {
            Referential::World => self.model_matrix.transform_point3(position),
            Referential::Model => position,
        }
    }

    /// Return a string describing the decomposition of the length of the strand `s_id` into the
    /// sum of the length of its domains
    pub fn decompose_length(&self, s_id: usize) -> String {
        let mut ret = String::new();
        if let Some(strand) = self.current_design.strands.get(&s_id) {
            ret.push_str(&strand.length().to_string());
            let mut first = true;
            for d in strand.domains.iter() {
                let sign = if first { '=' } else { '+' };
                ret.push_str(&format!(" {} {}", sign, d.length()));
                first = false;
            }
        }
        ret
    }

    pub(super) fn get_nucl_map(&self) -> AHashMap<Nucl, u32> {
        self.content.identifier_nucl.clone().into()
    }
}

pub(super) fn update_presenter(
    presenter: &AddressPointer<Presenter>,
    design: AddressPointer<Design>,
) -> (AddressPointer<Presenter>, AddressPointer<Design>) {
    if presenter.current_design != design {
        if cfg!(test) {
            println!("updating presenter");
        }
        let new_presenter = presenter.clone_inner().update(design);
        let design = new_presenter.current_design.clone();
        (AddressPointer::new(new_presenter), design)
    } else {
        (presenter.clone(), design)
    }
}

use ensnano_interactor::Referential;
use ultraviolet::Vec3;
impl DesignReader {
    pub(super) fn get_position_of_nucl_on_helix(
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

    pub(super) fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        for (s_id, s) in self.presenter.current_design.strands.iter() {
            if !s.cyclic && s.get_5prime() == Some(nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    pub(super) fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        for (s_id, s) in self.presenter.current_design.strands.iter() {
            if !s.cyclic && s.get_3prime() == Some(nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    pub(super) fn helix_is_empty(&self, h_id: usize) -> Option<bool> {
        if !self.presenter.current_design.helices.contains_key(&h_id) {
            None
        } else {
            for h in self.presenter.content.helix_map.values() {
                if *h == h_id {
                    return Some(true);
                }
            }
            Some(false)
        }
    }

    pub(super) fn get_id_of_strand_containing_nucl(&self, nucl: &Nucl) -> Option<usize> {
        let e_id = self.presenter.content.identifier_nucl.get(nucl)?;
        self.presenter.content.strand_map.get(e_id).cloned()
    }

    /// Return the xover extremity status of nucl.
    pub fn is_xover_end(&self, nucl: &Nucl) -> Extremity {
        let strand_id = if let Some(id) = self.get_id_of_strand_containing_nucl(nucl) {
            id
        } else {
            return Extremity::No;
        };

        let strand = if let Some(strand) = self.presenter.current_design.strands.get(&strand_id) {
            strand
        } else {
            return Extremity::No;
        };
        let mut prev_helix = None;
        for domain in strand.domains.iter() {
            if domain.prime5_end() == Some(*nucl) && prev_helix != domain.half_helix() {
                return Extremity::Prime5;
            } else if domain.prime3_end() == Some(*nucl) {
                return Extremity::Prime3;
            } else if let Some(_) = domain.has_nucl(nucl) {
                return Extremity::No;
            }
            prev_helix = domain.half_helix();
        }
        return Extremity::No;
    }

    fn get_strand_length(&self, s_id: usize) -> Option<usize> {
        self.presenter
            .current_design
            .strands
            .get(&s_id)
            .map(|s| s.length())
    }

    pub fn get_scaffold_info(&self) -> Option<ScaffoldInfo> {
        let id = self.presenter.current_design.scaffold_id?;
        let length = self.get_strand_length(id)?;
        let shift = self.presenter.current_design.scaffold_shift;
        let starting_nucl = self
            .presenter
            .current_design
            .strands
            .get(&id)
            .and_then(|s| s.get_nth_nucl(shift.unwrap_or(0)));
        Some(ScaffoldInfo {
            id,
            shift,
            length,
            starting_nucl,
        })
    }
}

impl HelixPresenter for Presenter {
    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)> {
        self.current_design.get_xovers()
    }

    fn get_design(&self) -> &Design {
        self.current_design.as_ref()
    }

    fn get_all_bounds(&self) -> Vec<(Nucl, Nucl)> {
        self.content.identifier_bound.keys().cloned().collect()
    }

    fn get_identifier(&self, nucl: &Nucl) -> Option<u32> {
        self.content.identifier_nucl.get(nucl).cloned()
    }

    fn get_space_position(&self, nucl: &Nucl) -> Option<Vec3> {
        self.get_identifier(nucl)
            .and_then(|id| self.content.space_position.get(&id).map(|v| v.into()))
    }

    fn has_nucl(&self, nucl: &Nucl) -> bool {
        self.content.identifier_nucl.contains_key(nucl)
    }
}
