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

#[cfg(test)]
pub use self::design_content::Staple;

use super::*;
use ensnano_design::{Extremity, HelixCollection, Nucl, VirtualNucl};
use ensnano_interactor::{
    application::Camera3D, NeighbourDescriptor, NeighbourDescriptorGiver, ScaffoldInfo, Selection,
    SuggestionParameters,
};
use ultraviolet::Mat4;

use crate::utils::id_generator::IdGenerator;
type JunctionsIds = IdGenerator<(Nucl, Nucl)>;
mod design_content;
mod impl_main_reader;
mod impl_reader2d;
mod impl_reader3d;
mod impl_readergui;
mod oxdna;
use crate::scene::{HBond, HalfHBond};
use ahash::AHashMap;
use design_content::DesignContent;
use std::collections::{BTreeMap, HashSet};

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
    current_suggestion_paramters: SuggestionParameters,
    model_matrix: AddressPointer<Mat4>,
    content: AddressPointer<DesignContent>,
    pub junctions_ids: AddressPointer<JunctionsIds>,
    visibility_sive: Option<VisibilitySieve>,
    invisible_nucls: HashSet<Nucl>,
    bonds: AddressPointer<Vec<HBond>>,
}

impl Default for Presenter {
    fn default() -> Self {
        Self {
            current_design: Default::default(),
            current_suggestion_paramters: Default::default(),
            model_matrix: AddressPointer::new(Mat4::identity()),
            content: Default::default(),
            junctions_ids: Default::default(),
            visibility_sive: None,
            invisible_nucls: Default::default(),
            bonds: Default::default(),
        }
    }
}

impl Presenter {
    #[cfg(test)]
    pub(super) fn get_staples(&self) -> Vec<Staple> {
        self.content.get_staples(&self.current_design, self)
    }

    pub fn can_start_builder_at(&self, nucl: Nucl) -> bool {
        let left = self.current_design.get_neighbour_nucl(nucl.left());
        let right = self.current_design.get_neighbour_nucl(nucl.right());
        if self.content.nucl_collection.contains_nucl(&nucl) {
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

    pub fn update(
        mut self,
        design: AddressPointer<Design>,
        suggestion_parameters: &SuggestionParameters,
    ) -> Self {
        if self.current_design != design
            || &self.current_suggestion_paramters != suggestion_parameters
        {
            self.read_design(design, suggestion_parameters);
            self.read_scaffold_seq();
            self.collect_h_bonds();
            self.update_visibility();
        }
        self
    }

    /// Return a fresh presenter presenting an imported `Design` with a given set of junctions, as
    /// well as a pointer to the design held by this fresh presenter.
    pub fn from_new_design(
        design: Design,
        old_junctions_ids: &JunctionsIds,
        suggestion_parameters: SuggestionParameters,
    ) -> (Self, AddressPointer<Design>) {
        log::info!("new design presenter");
        let model_matrix = Mat4::identity();
        let (content, design, junctions_ids) =
            DesignContent::make_hash_maps(design, old_junctions_ids, &suggestion_parameters);
        let design = AddressPointer::new(design);
        let mut ret = Self {
            current_design: design.clone(),
            current_suggestion_paramters: suggestion_parameters,
            content: AddressPointer::new(content),
            model_matrix: AddressPointer::new(model_matrix),
            junctions_ids: AddressPointer::new(junctions_ids),
            visibility_sive: None,
            invisible_nucls: Default::default(),
            bonds: Default::default(),
        };
        ret.read_scaffold_seq();
        ret.collect_h_bonds();
        (ret, design)
    }

    fn apply_simulation_update(&mut self, update: impl AsRef<dyn SimulationUpdate>) {
        let mut new_content = self.content.clone_inner();
        update.as_ref().update_positions(
            new_content.nucl_collection.as_ref(),
            &mut new_content.space_position,
        );
        self.content = AddressPointer::new(new_content);
    }

    fn read_design(
        &mut self,
        design: AddressPointer<Design>,
        suggestion_parameters: &SuggestionParameters,
    ) {
        let (content, new_design, new_junctions_ids) = DesignContent::make_hash_maps(
            design.clone_inner(),
            self.junctions_ids.as_ref(),
            suggestion_parameters,
        );
        self.current_design = AddressPointer::new(new_design);
        log::trace!("Presenter design <- {:p}", self.current_design);
        self.content = AddressPointer::new(content);
        self.junctions_ids = AddressPointer::new(new_junctions_ids);
        self.current_suggestion_paramters = suggestion_parameters.clone();
    }

    pub(super) fn has_different_model_matrix_than(&self, other: &Self) -> bool {
        self.model_matrix != other.model_matrix
    }

    fn read_scaffold_seq(&mut self) {
        let sequence = self.current_design.scaffold_sequence.as_ref();
        if sequence.is_none() {
            return;
        }
        let sequence: String = sequence
            .unwrap()
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect();
        let nb_skip = {
            let shift = self.current_design.scaffold_shift.unwrap_or(0);
            sequence.len() - (shift % sequence.len())
        };
        if let Some(mut sequence) = self.current_design.scaffold_sequence.as_ref().map(|s| {
            s.chars()
                .filter(|c| c.is_alphabetic())
                .cycle()
                .skip(nb_skip)
        }) {
            let mut basis_map = HashMap::clone(self.content.basis_map.as_ref());
            if let Some(strand) = self
                .current_design
                .scaffold_id
                .as_ref()
                .and_then(|s_id| self.current_design.strands.get(s_id))
            {
                for domain in &strand.domains {
                    if let ensnano_design::Domain::HelixDomain(dom) = domain {
                        for nucl_position in dom.iter() {
                            let nucl = Nucl {
                                helix: dom.helix,
                                position: nucl_position,
                                forward: dom.forward,
                            };
                            let basis = sequence.next();
                            let basis_compl = compl(basis);
                            log::debug!("basis {:?}, basis_compl {:?}", basis, basis_compl);
                            if let Some((basis, basis_compl)) = basis.zip(basis_compl) {
                                basis_map.insert(nucl, basis);
                                if let Some(virtual_compl) = Nucl::map_to_virtual_nucl(
                                    nucl.compl(),
                                    &self.current_design.helices,
                                ) {
                                    if let Some(real_compl) =
                                        self.content.nucl_collection.virtual_to_real(&virtual_compl)
                                    {
                                        basis_map.insert(*real_compl, basis_compl);
                                    }
                                }
                            } else {
                                log::error!("Could not get virtual mapping of {:?}", nucl.compl())
                            }
                        }
                    } else if let ensnano_design::Domain::Insertion(n) = domain {
                        for _ in 0..*n {
                            sequence.next();
                        }
                    }
                }
            }
            let mut new_content = self.content.clone_inner();
            new_content.basis_map = Arc::new(basis_map);
            self.content = AddressPointer::new(new_content);
        }
    }

    fn collect_h_bonds(&mut self) {
        let nucl_collection = self.content.nucl_collection.as_ref();
        let mut bonds = Vec::with_capacity(nucl_collection.nb_nucls());
        for (forward_nucl, virtual_nucl_forward, forward_id) in nucl_collection
            .iter_nucls_ids()
            .filter(|(n, _)| n.forward)
            .filter_map(|(n, id)| {
                Nucl::map_to_virtual_nucl(*n, &self.current_design.helices)
                    .map(move |v| (*n, v, *id))
            })
        {
            let virtual_nucl_backward = virtual_nucl_forward.compl();
            if let Some(backward_nucl) = nucl_collection.virtual_to_real(&virtual_nucl_backward) {
                if let Some(backward_id) = nucl_collection.get_identifier(backward_nucl) {
                    if let Some(bond) =
                        self.h_bond(forward_id, *backward_id, forward_nucl, *backward_nucl)
                    {
                        bonds.push(bond);
                    }
                }
            }
        }
        self.bonds = AddressPointer::new(bonds);
    }

    fn h_bond(
        &self,
        forward_id: u32,
        backward_id: u32,
        forward_nucl: Nucl,
        backward_nucl: Nucl,
    ) -> Option<HBond> {
        let pos_forward: Vec3 = self
            .content
            .space_position
            .get(&forward_id)
            .cloned()?
            .into();
        let pos_backward: Vec3 = self
            .content
            .space_position
            .get(&backward_id)
            .cloned()?
            .into();
        let a1 = (pos_backward - pos_forward).normalized();
        let forward_half = HalfHBond {
            backbone: pos_forward,
            center_of_mass: pos_forward + 2. * a1 * oxdna::BACKBONE_TO_CM,
            base: self.content.basis_map.get(&forward_nucl).cloned(),
            backbone_color: self.content.color.get(&forward_id).cloned()?,
        };

        let backward_half = HalfHBond {
            backbone: pos_backward,
            center_of_mass: pos_backward - 2. * a1 * oxdna::BACKBONE_TO_CM,
            base: self.content.basis_map.get(&backward_nucl).cloned(),
            backbone_color: self.content.color.get(&backward_id).cloned()?,
        };
        Some(HBond {
            forward: forward_half,
            backward: backward_half,
        })
    }

    fn update_visibility(&mut self) {
        let mut new_invisible_nucls = HashSet::new();
        if let Some(VisibilitySieve {
            selection,
            compl,
            visible,
        }) = self.visibility_sive.as_ref()
        {
            for nucl in self.content.nucleotide.values() {
                if self.selection_contains_nucl(selection, *nucl) != *compl {
                    if !visible {
                        new_invisible_nucls.insert(nucl.clone());
                    }
                } else if self.invisible_nucls.contains(nucl) {
                    new_invisible_nucls.insert(nucl.clone());
                }
            }
        }
        self.invisible_nucls = new_invisible_nucls;
    }

    fn in_referential(&self, position: Vec3, referential: Referential) -> Vec3 {
        match referential {
            Referential::World => self.model_matrix.transform_point3(position),
            Referential::Model => position,
        }
    }

    fn selection_contains_nucl(&self, selection: &[Selection], nucl: Nucl) -> bool {
        let identifier_nucl = if let Some(id) = self.content.nucl_collection.get_identifier(&nucl) {
            id
        } else {
            return false;
        };
        let mut ret = false;
        for s in selection.iter() {
            ret = ret
                || match s {
                    Selection::Design(_) => true,
                    Selection::Strand(_, s_id) => {
                        self.content.strand_map.get(identifier_nucl).cloned()
                            == Some(*s_id as usize)
                    }
                    Selection::Grid(_, _) => false,
                    Selection::Nucleotide(_, n) => nucl == *n,
                    Selection::Helix(_, h_id) => nucl.helix == *h_id as usize,
                    Selection::Nothing => false,
                    Selection::Xover(_, xover_id) => {
                        if let Some((n1, n2)) = self.junctions_ids.get_element(*xover_id) {
                            n1 == nucl || n2 == nucl
                        } else {
                            false
                        }
                    }
                    Selection::Bound(_, n1, n2) => *n1 == nucl || *n2 == nucl,
                    Selection::Phantom(e) => e.to_nucl() == nucl,
                    Selection::BezierControlPoint { .. } => false,
                };
        }
        ret
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

    fn get_name_of_group_having_strand(&self, s_id: usize) -> Vec<String> {
        let tree = &self.current_design.organizer_tree.as_ref();
        tree.map(|t| {
            t.get_names_of_groups_having(&ensnano_design::elements::DnaElementKey::Strand(s_id))
        })
        .unwrap_or_default()
    }

    pub fn get_strand_domain(&self, s_id: usize, d_id: usize) -> Option<&ensnano_design::Domain> {
        self.current_design
            .strands
            .get(&s_id)
            .and_then(|s| s.domains.get(d_id))
    }

    pub(super) fn get_owned_nucl_collection(&self) -> Arc<impl NuclCollection> {
        self.content.nucl_collection.clone()
    }

    fn whole_selection_is_visible(&self, selection: &[Selection], compl: bool) -> bool {
        for nucl in self.content.nucleotide.values() {
            if self.selection_contains_nucl(selection, *nucl) != compl {
                if self.invisible_nucls.contains(nucl) {
                    return false;
                }
            }
        }
        true
    }

    pub fn set_visibility_sieve(&mut self, selection: Vec<Selection>, compl: bool) {
        if selection.is_empty() {
            self.visibility_sive = None;
        } else {
            let visible = !self.whole_selection_is_visible(&selection, compl);
            self.visibility_sive = Some(VisibilitySieve {
                selection,
                compl,
                visible,
            });
        }
        self.update_visibility();
    }

    pub fn get_checked_xovers_ids(&self) -> Vec<u32> {
        self.current_design
            .checked_xovers
            .iter()
            .filter_map(|xover_id| {
                self.junctions_ids
                    .get_element(*xover_id)
                    .as_ref()
                    .and_then(|bound_id| self.content.identifier_bound.get(bound_id))
            })
            .cloned()
            .collect()
    }

    pub fn get_unchecked_xovers_ids(&self) -> Vec<u32> {
        let mut checked_nucl = HashSet::new();
        let mut unchecked_pairs = Vec::new();
        for (xover_id, (n1, n2)) in self.junctions_ids.get_all_elements() {
            if self.current_design.checked_xovers.contains(&xover_id) {
                checked_nucl.insert(n1);
                checked_nucl.insert(n2);
            } else {
                unchecked_pairs.push((n1, n2));
            }
        }
        let mut ret = Vec::new();
        for (n1, n2) in unchecked_pairs {
            if !checked_nucl.contains(&n1.prime3()) && !checked_nucl.contains(&n1.prime5()) {
                if let Some(id) = self.content.identifier_bound.get(&(n1, n2)) {
                    ret.push(*id)
                }
            }
        }
        ret
    }

    pub fn get_xover_len(&self, xover_id: usize) -> Option<f32> {
        let (n1, n2) = self.junctions_ids.get_element(xover_id)?;
        let pos1 = self
            .content
            .nucl_collection
            .get_identifier(&n1)
            .and_then(|id| self.content.space_position.get(id))?;
        let pos2 = self
            .content
            .nucl_collection
            .get_identifier(&n2)
            .and_then(|id| self.content.space_position.get(id))?;
        Some((Vec3::from(pos1) - Vec3::from(pos2)).mag())
    }

    pub fn get_id_of_xover_involving_nucl(&self, nucl: Nucl) -> Option<usize> {
        self.junctions_ids
            .get_all_elements()
            .into_iter()
            .find(|(_, pair)| pair.0 == nucl || pair.1 == nucl)
            .map(|t| t.0)
    }
}

pub(super) fn design_need_update(
    presenter: &AddressPointer<Presenter>,
    design: &AddressPointer<Design>,
    suggestion_parameters: &SuggestionParameters,
) -> bool {
    if log::log_enabled!(log::Level::Trace) || cfg!(test) {
        println!("presenter current design");
        presenter.current_design.show_address();
        println!("design address");
        design.show_address();
    }
    presenter.current_design != *design
        || &presenter.current_suggestion_paramters != suggestion_parameters
}

pub(super) fn update_presenter(
    presenter: &AddressPointer<Presenter>,
    design: AddressPointer<Design>,
    suggestion_parameters: &SuggestionParameters,
) -> (AddressPointer<Presenter>, AddressPointer<Design>) {
    log::trace!("Calling from presenter");
    if design_need_update(presenter, &design, suggestion_parameters) {
        if cfg!(test) {
            println!("updating presenter");
        }
        let new_presenter = presenter
            .clone_inner()
            .update(design, suggestion_parameters);
        let design = new_presenter.current_design.clone();
        (AddressPointer::new(new_presenter), design)
    } else {
        (presenter.clone(), design)
    }
}

pub(super) fn apply_simulation_update(
    presenter: &AddressPointer<Presenter>,
    design: AddressPointer<Design>,
    update: impl AsRef<dyn SimulationUpdate>,
    suggestion_parameters: &SuggestionParameters,
) -> (AddressPointer<Presenter>, AddressPointer<Design>) {
    let mut new_design = design.clone_inner();
    update.as_ref().update_design(&mut new_design);
    log::trace!("calling from apply_simulation_update");
    let (new_presenter, returned_design) = update_presenter(
        presenter,
        AddressPointer::new(new_design),
        suggestion_parameters,
    );
    let mut new_content = new_presenter.content.clone_inner();
    let mut returned_presenter = new_presenter.clone_inner();
    new_content.read_simualtion_update(update.as_ref());
    returned_presenter.content = AddressPointer::new(new_content);
    returned_presenter.apply_simulation_update(update);
    (AddressPointer::new(returned_presenter), returned_design)
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
        let e_id = self
            .presenter
            .content
            .nucl_collection
            .get_identifier(nucl)?;
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

    pub fn get_camera_with_id(&self, cam_id: ensnano_design::CameraId) -> Option<Camera3D> {
        self.presenter
            .current_design
            .get_camera(cam_id)
            .cloned()
            .map(|c| Camera3D {
                position: c.position,
                orientation: c.orientation,
                pivot_position: c.pivot_position,
            })
    }

    pub fn get_nth_camera(&self, n: u32) -> Option<Camera3D> {
        self.presenter
            .current_design
            .get_cameras()
            .nth(n as usize)
            .map(|(_, c)| Camera3D {
                position: c.position,
                orientation: c.orientation,
                pivot_position: c.pivot_position,
            })
    }

    pub fn get_favourite_camera(&self) -> Option<(Vec3, ultraviolet::Rotor3)> {
        self.presenter
            .current_design
            .get_favourite_camera()
            .map(|c| (c.position, c.orientation))
    }
}

impl HelixPresenter for Presenter {
    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)> {
        self.current_design.strands.get_xovers()
    }

    fn get_design(&self) -> &Design {
        self.current_design.as_ref()
    }

    fn get_all_bounds(&self) -> Vec<(Nucl, Nucl)> {
        self.content.identifier_bound.keys().cloned().collect()
    }

    fn get_identifier(&self, nucl: &Nucl) -> Option<u32> {
        self.content.nucl_collection.get_identifier(nucl).cloned()
    }

    fn get_space_position(&self, nucl: &Nucl) -> Option<Vec3> {
        self.get_identifier(nucl)
            .and_then(|id| self.content.space_position.get(&id).map(|v| v.into()))
    }

    fn has_nucl(&self, nucl: &Nucl) -> bool {
        self.content.nucl_collection.contains_nucl(nucl)
    }
}

impl GridPresenter for Presenter {
    fn get_design(&self) -> &Design {
        self.current_design.as_ref()
    }

    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)> {
        self.current_design.strands.get_xovers()
    }

    fn get_helices_attached_to_grid(&self, g_id: usize) -> Option<Vec<usize>> {
        self.content
            .get_helices_on_grid(g_id)
            .map(|set| set.into_iter().collect())
    }

    fn get_grid(&self, g_id: usize) -> Option<&ensnano_design::grid::Grid> {
        self.content.grid_manager.grids.get(g_id)
    }
}

impl RollPresenter for Presenter {
    fn get_design(&self) -> &Design {
        self.current_design.as_ref()
    }

    fn get_xovers_list(&self) -> Vec<(Nucl, Nucl)> {
        self.current_design.strands.get_xovers()
    }

    fn get_helices(&self) -> BTreeMap<usize, ensnano_design::Helix> {
        self.current_design
            .helices
            .iter()
            .map(|(k, h)| (*k, ensnano_design::Helix::clone(h)))
            .collect()
    }
}

impl TwistPresenter for Presenter {}

use std::collections::HashMap;
pub trait SimulationUpdate: Send + Sync {
    fn update_positions(
        &self,
        _identifier_nucl: &dyn NuclCollection,
        _space_position: &mut HashMap<u32, [f32; 3], ahash::RandomState>,
    ) {
    }

    fn update_design(&self, design: &mut Design);
}

pub trait NuclCollection: Send + Sync + 'static {
    fn iter_nucls_ids<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a Nucl, &'a u32)> + 'a>;
    fn contains_nucl(&self, nucl: &Nucl) -> bool;
    fn iter_nucls<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Nucl> + 'a>;
    fn virtual_to_real(&self, virtual_nucl: &VirtualNucl) -> Option<&Nucl>;
}

#[derive(Clone)]
struct VisibilitySieve {
    selection: Vec<Selection>,
    compl: bool,
    visible: bool,
}

fn compl(c: Option<char>) -> Option<char> {
    match c {
        Some('T') => Some('A'),
        Some('A') => Some('T'),
        Some('G') => Some('C'),
        Some('C') => Some('G'),
        _ => None,
    }
}
