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
use ultraviolet::Mat4;

use crate::utils::id_generator::IdGenerator;
type JunctionsIds = IdGenerator<(Nucl, Nucl)>;
mod design_content;
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
