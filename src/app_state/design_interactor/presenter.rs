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

use crate::design::ObjectType;

use super::*;
use ensnano_design::Nucl;
use ensnano_interactor::Extremity;
use ultraviolet::Mat4;

use crate::utils::id_generator::IdGenerator;
type JunctionsIds = IdGenerator<(Nucl, Nucl)>;
mod design_content;
mod impl_reader2d;
mod impl_reader3d;
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
    current_design: AddressPointer<Design>,
    model_matrix: AddressPointer<Mat4>,
    id_generator: AddressPointer<IdGenerator<(Nucl, Nucl)>>,
    content: AddressPointer<DesignContent>,
    junctions_ids: AddressPointer<JunctionsIds>,
    helices_groups: AddressPointer<BTreeMap<usize, bool>>,
}

impl Default for Presenter {
    fn default() -> Self {
        Self {
            current_design: Default::default(),
            model_matrix: AddressPointer::new(Mat4::identity()),
            id_generator: Default::default(),
            content: Default::default(),
            junctions_ids: Default::default(),
            helices_groups: Default::default(),
        }
    }
}

impl Presenter {
    pub fn update(mut self, design: AddressPointer<Design>) -> Self {
        if self.current_design != design {
            self.read_design(design);
            self.read_scaffold_seq();
            self.update_visibility();
        }
        self
    }

    fn read_design(&mut self, design: AddressPointer<Design>) {
        let (content, design, junctions_ids) = DesignContent::make_hash_maps(
            design.clone_inner(),
            self.helices_groups.as_ref(),
            self.junctions_ids.clone(),
        );
        self.current_design = AddressPointer::new(design);
        self.content = AddressPointer::new(content);
        if let Some(junctions_ids) = junctions_ids {
            self.junctions_ids = AddressPointer::new(junctions_ids);
        }
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
}

pub(super) fn update_presenter(
    presenter: &AddressPointer<Presenter>,
    design: AddressPointer<Design>,
) -> AddressPointer<Presenter> {
    if presenter.current_design != design {
        let mut new_presenter = presenter.clone_inner();
        new_presenter.read_design(design);
        AddressPointer::new(new_presenter)
    } else {
        presenter.clone()
    }
}

use ultraviolet::Vec3;
use ensnano_interactor::Referential;
impl DesignReader {
    pub (super) fn get_position_of_nucl_on_helix(
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

    pub (super) fn prime5_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        for (s_id, s) in self.presenter.current_design.strands.iter() {
            if !s.cyclic && s.get_5prime() == Some(nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    pub (super) fn prime3_of_which_strand(&self, nucl: Nucl) -> Option<usize> {
        for (s_id, s) in self.presenter.current_design.strands.iter() {
            if !s.cyclic && s.get_5prime() == Some(nucl) {
                return Some(*s_id);
            }
        }
        None
    }

    pub (super) fn helix_is_empty(&self, h_id: usize) -> Option<bool> {
        if !self.presenter.current_design.helices.contains_key(&h_id) {
            None
        } else {
            for h in self.presenter.content.helix_map.values() {
                if *h == h_id {
                    return Some(true)
                }
            }
            Some(false)
        }
    }

    pub (super) fn get_id_of_strand_containing_nucl(&self, nucl: &Nucl) -> Option<usize> {
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
            return Extremity::No
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

}
