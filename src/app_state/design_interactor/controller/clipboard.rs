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

use super::{
    AddressPointer, Controller, ControllerState, Design, Domain, ErrOperation, HelixGridPosition,
    HelixInterval, Nucl, Strand,
};
use ensnano_design::{
    grid::{Edge, FreeGridId, GridData, GridId, GridPosition},
    Helices, HelixCollection, MutStrandAndData, Parameters, Strands, UpToDateDesign,
};
use ultraviolet::Vec3;

pub(super) enum Clipboard {
    Empty,
    Strands(StrandClipboard),
    Xovers(Vec<(Nucl, Nucl)>),
    Grids(Vec<GridId>),
    Helices(Vec<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum PastePosition {
    Nucl(Nucl),
    GridPosition(GridPosition),
}

impl PastePosition {
    pub fn to_nucl(self) -> Option<Nucl> {
        if let Self::Nucl(n) = self {
            Some(n)
        } else {
            None
        }
    }

    pub fn to_grid_position(self) -> Option<GridPosition> {
        if let Self::GridPosition(gp) = self {
            Some(gp)
        } else {
            None
        }
    }
}

impl Clipboard {
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Strands(strand_clipboard) => strand_clipboard.templates.len(),
            Self::Xovers(xovers) => xovers.len(),
            Self::Grids(grids) => grids.len(),
            Self::Helices(helices) => helices.len(),
        }
    }

    pub fn get_strand_clipboard(&self) -> Result<StrandClipboard, ErrOperation> {
        match self {
            Self::Empty => Err(ErrOperation::EmptyClipboard),
            Self::Strands(strand_clipboard) => Ok(strand_clipboard.clone()),
            Self::Xovers(_) => Err(ErrOperation::WrongClipboard),
            Self::Grids(_) => Err(ErrOperation::WrongClipboard),
            Self::Helices(_) => Err(ErrOperation::WrongClipboard),
        }
    }

    fn get_leading_xover_nucl(&self) -> Option<Nucl> {
        match self {
            Self::Xovers(v) => v.get(0).map(|t| t.0),
            _ => None,
        }
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Clone, Debug)]
pub(super) struct StrandClipboard {
    templates: Vec<StrandTemplate>,
    template_edges: Vec<(Edge, isize)>,
}

#[derive(Debug, Clone)]
pub(super) struct PastedStrand {
    pub domains: Vec<Domain>,
    pub nucl_position: Vec<Vec3>,
    pub pastable: bool,
}

/// A template describing the relation between the domains of a strand. Can be used for copy-paste
/// of strands.
#[derive(Debug, Clone)]
struct StrandTemplate {
    origin: TemplateOrigin,
    domains: Vec<DomainTemplate>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
/// The starting point of a template. Used to determine weither a nucleotide is a correct starting
/// point for a copy of the strand.
struct TemplateOrigin {
    helix: HelixGridPosition,
    start: isize,
    forward: bool,
}

#[derive(Debug, Clone)]
/// A domain of a template.
/// The HelixInterval variant does not have an helix attribute because helices are determined by
/// a path in the grid's graph when instanciating the template.
enum DomainTemplate {
    Insertion(usize),
    HelixInterval {
        start: isize,
        end: isize,
        forward: bool,
    },
}

impl Controller {
    pub fn copy_grids(
        &mut self,
        design: &Design,
        grid_ids: Vec<GridId>,
    ) -> Result<(), ErrOperation> {
        for grid_id in grid_ids.iter() {
            if design.free_grids.get_from_g_id(grid_id).is_none() {
                return Err(ErrOperation::GridDoesNotExist(*grid_id));
            }
        }
        self.clipboard = AddressPointer::new(Clipboard::Grids(grid_ids));
        Ok(())
    }

    pub fn set_templates(
        &mut self,
        data: &UpToDateDesign<'_>,
        strand_ids: Vec<usize>,
    ) -> Result<(), ErrOperation> {
        let mut templates = Vec::with_capacity(strand_ids.len());
        for id in strand_ids.iter() {
            let strand = data
                .design
                .strands
                .get(&id)
                .ok_or(ErrOperation::StrandDoesNotExist(*id))?;
            let template = self.strand_to_template(strand, &data.design.helices, data.grid_data)?;
            templates.push(template);
        }
        let mut edges = vec![];
        if templates.len() == 0 {
            self.clipboard = Default::default();
        } else {
            if let Some(s_id1) = strand_ids.get(0) {
                for s_id2 in strand_ids.iter().skip(1) {
                    edges.push(Self::edge_between_strands(
                        *s_id1,
                        *s_id2,
                        &data.design.helices,
                        &data.design.strands,
                        data.grid_data,
                    ));
                }
            }
            let edges = edges.into_iter().collect::<Option<Vec<(Edge, isize)>>>();
            if let Some(edges) = edges {
                self.clipboard = AddressPointer::new(Clipboard::Strands(StrandClipboard {
                    templates,
                    template_edges: edges,
                }));
            } else {
                return Err(ErrOperation::CouldNotCreateEdges);
            }
        }
        Ok(())
    }

    fn strand_to_template(
        &self,
        strand: &Strand,
        helices: &Helices,
        grid_manager: &GridData,
    ) -> Result<StrandTemplate, ErrOperation> {
        let mut origin: Option<TemplateOrigin> = None;
        let mut domains = Vec::with_capacity(strand.domains.len());
        let mut edges = Vec::with_capacity(strand.domains.len());
        let mut previous_position = None;
        for domain in strand.domains.iter() {
            match domain {
                Domain::Insertion { nb_nucl, .. } => {
                    domains.push(DomainTemplate::Insertion(*nb_nucl))
                }
                Domain::HelixDomain(dom) => {
                    if let Some(ref pos1) = previous_position {
                        let helix = helices
                            .get(&dom.helix)
                            .ok_or(ErrOperation::HelixDoesNotExists(dom.helix))?;
                        let pos2 = helix
                            .grid_position
                            .ok_or(ErrOperation::HelixHasNoGridPosition(dom.helix))?;
                        let edge = grid_manager
                            .get_edge(pos1, &pos2)
                            .ok_or(ErrOperation::CouldNotMakeEdge(*pos1, pos2))?;
                        edges.push(edge);
                        previous_position = Some(pos2);
                        domains.push(DomainTemplate::HelixInterval {
                            start: dom.start,
                            end: dom.end,
                            forward: dom.forward,
                        });
                    } else {
                        let helix = helices
                            .get(&dom.helix)
                            .ok_or(ErrOperation::HelixDoesNotExists(dom.helix))?;
                        let grid_position = helix
                            .grid_position
                            .ok_or(ErrOperation::HelixHasNoGridPosition(dom.helix))?;
                        let start = if dom.forward { dom.start } else { dom.end };
                        origin = Some(TemplateOrigin {
                            helix: grid_position,
                            start,
                            forward: dom.forward,
                        });
                        previous_position = Some(grid_position);
                        domains.push(DomainTemplate::HelixInterval {
                            start: dom.start,
                            end: dom.end,
                            forward: dom.forward,
                        });
                    }
                }
            }
        }
        origin
            .ok_or(ErrOperation::EmptyOrigin)
            .map(|origin| StrandTemplate {
                origin,
                domains,
                edges,
            })
    }

    fn edge_between_strands(
        s_id1: usize,
        s_id2: usize,
        helices: &Helices,
        strands: &Strands,
        grid_manager: &GridData,
    ) -> Option<(Edge, isize)> {
        let strand1 = strands.get(&s_id1)?;
        let strand2 = strands.get(&s_id2)?;
        let nucl1 = strand1.get_5prime()?;
        let nucl2 = strand2.get_5prime()?;
        let pos1 = helices.get(&nucl1.helix).and_then(|h| h.grid_position)?;
        let pos2 = helices.get(&nucl2.helix).and_then(|h| h.grid_position)?;
        grid_manager
            .get_edge(&pos1, &pos2)
            .zip(Some(nucl2.position - nucl1.position))
    }

    fn edge_beteen_nucls(
        helices: &Helices,
        grid_manager: &GridData,
        n1: &Nucl,
        n2: &Nucl,
    ) -> Option<(Edge, isize)> {
        let pos1 = helices.get(&n1.helix).and_then(|h| h.grid_position)?;
        let pos2 = helices.get(&n2.helix).and_then(|h| h.grid_position)?;
        grid_manager
            .get_edge(&pos1, &pos2)
            .zip(Some(n2.position - n1.position))
    }

    pub(super) fn position_copy(
        &mut self,
        mut design: Design,
        position: Option<PastePosition>,
    ) -> Result<Design, ErrOperation> {
        let mut data = design.mut_strand_and_data();
        match self.clipboard.as_ref() {
            Clipboard::Strands(_) => {
                self.position_strand_copies(&mut data, position.and_then(PastePosition::to_nucl))?;
                Ok(design)
            }
            Clipboard::Xovers(_) => {
                self.position_xover_copies(&mut design, position.and_then(PastePosition::to_nucl))?;
                Ok(design)
            }
            Clipboard::Grids(grid_ids) => {
                let grid_ids: Vec<_> = grid_ids
                    .iter()
                    .filter_map(|g_id| FreeGridId::try_from_grid_id(*g_id))
                    .collect();
                design
                    .copy_grids(&grid_ids, Vec3::zero(), ultraviolet::Rotor3::identity())
                    .map_err(|e| ErrOperation::GridCopyError(e))?;
                Ok(design)
            }
            Clipboard::Empty => Err(ErrOperation::EmptyClipboard),
            Clipboard::Helices(helices) => {
                let helices = helices.clone();
                log::info!("positioning helices copy");
                self.position_helices_copy(&mut design, helices, position)?;
                Ok(design)
            }
        }
    }

    pub(super) fn position_strand_copies(
        &mut self,
        data: &mut MutStrandAndData<'_>,
        nucl: Option<Nucl>,
    ) -> Result<(), ErrOperation> {
        let strand_clipboard = if let Clipboard::Strands(clipboard) = self.clipboard.as_ref() {
            Ok(clipboard)
        } else {
            Err(ErrOperation::EmptyClipboard)
        }?;
        if let Some(nucl) = nucl {
            let (pasted_strands, duplication_edge) =
                self.paste_clipboard(&strand_clipboard, nucl, data)?;
            self.state.update_pasting_position(
                Some(PastePosition::Nucl(nucl)),
                pasted_strands,
                duplication_edge,
            )
        } else {
            self.state.update_pasting_position(None, vec![], None)
        }
    }

    fn paste_clipboard(
        &self,
        clipboard: &StrandClipboard,
        nucl: Nucl,
        data: &mut MutStrandAndData<'_>,
    ) -> Result<(Vec<PastedStrand>, Option<(Edge, isize)>), ErrOperation> {
        let mut duplication_edge = None;
        let template_0 = clipboard
            .templates
            .get(0)
            .ok_or(ErrOperation::EmptyClipboard)?;
        let domains_0 = self.template_to_domains(
            template_0,
            nucl,
            &mut duplication_edge,
            data.helices,
            data.grid_data,
        )?;
        let mut domains_vec = vec![domains_0];
        for n in 1..clipboard.templates.len() {
            let t = clipboard.templates.get(n);
            log::info!("updated template {:?}", t);
            let domains = t.as_ref().and_then(|t| {
                clipboard
                    .template_edges
                    .get(n - 1)
                    .and_then(|(e, s)| {
                        self.translate_nucl_by_edge(&nucl, e, *s, data.helices, data.grid_data)
                    })
                    .and_then(|n2| {
                        // If some strands cannot be pasted they are just ignored and no error is
                        // returned.
                        self.template_to_domains(t, n2, &mut None, data.helices, data.grid_data)
                            .ok()
                    })
            });
            if let Some(domains) = domains {
                domains_vec.push(domains);
            }
        }
        let pasted_strands = self.domains_vec_to_pasted_strands(
            domains_vec,
            data.helices,
            data.strands,
            &data.parameters,
        );
        Ok((pasted_strands, duplication_edge))
    }

    fn template_to_domains(
        &self,
        template: &StrandTemplate,
        start_nucl: Nucl,
        duplication_info: &mut Option<(Edge, isize)>,
        helices: &Helices,
        grid_manager: &GridData,
    ) -> Result<Vec<Domain>, ErrOperation> {
        let mut ret = Vec::with_capacity(template.domains.len());
        let mut edge_iter = template.edges.iter();
        let mut previous_position: Option<HelixGridPosition> = None;
        let mut edge_opt = None;
        let shift = if template.origin.forward {
            start_nucl.position - template.origin.start
        } else {
            start_nucl.position - template.origin.start + 1
        };
        for domain in template.domains.iter() {
            match domain {
                DomainTemplate::Insertion(n) => ret.push(Domain::new_insertion(*n)),
                DomainTemplate::HelixInterval {
                    start,
                    end,
                    forward,
                } => {
                    if let Some(ref pos1) = previous_position {
                        let edge = edge_iter.next().ok_or(ErrOperation::CannotPasteHere)?;
                        let pos2 = grid_manager
                            .translate_by_edge(pos1, edge)
                            .ok_or(ErrOperation::CannotPasteHere)?;
                        let helix = grid_manager
                            .pos_to_object(pos2.light())
                            .map(|obj| obj.helix())
                            .ok_or(ErrOperation::CannotPasteHere)?;
                        ret.push(Domain::HelixDomain(HelixInterval {
                            helix,
                            start: start + shift,
                            end: end + shift,
                            forward: *forward,
                            sequence: None,
                        }));
                        previous_position = Some(pos2);
                    } else {
                        let position = template.origin.helix;
                        let pos2 = helices
                            .get(&start_nucl.helix)
                            .and_then(|h| h.grid_position)
                            .ok_or(ErrOperation::CannotPasteHere)?;

                        edge_opt = grid_manager.get_edge(&position, &pos2);
                        if grid_manager.get_edge(&position, &pos2).is_none() {
                            return Err(ErrOperation::CannotPasteHere);
                        }
                        let helix = grid_manager
                            .pos_to_object(pos2.light())
                            .map(|obj| obj.helix())
                            .ok_or(ErrOperation::CannotPasteHere)?;

                        ret.push(Domain::HelixDomain(HelixInterval {
                            helix,
                            start: start + shift,
                            end: end + shift,
                            forward: template.origin.forward,
                            sequence: None,
                        }));
                        previous_position = Some(pos2);
                    }
                }
            }
        }
        *duplication_info = edge_opt.zip(Some(shift));
        Ok(ret)
    }

    fn translate_nucl_by_edge(
        &self,
        nucl1: &Nucl,
        edge: &Edge,
        shift: isize,
        helices: &Helices,
        grid_manager: &GridData,
    ) -> Option<Nucl> {
        let pos1 = helices.get(&nucl1.helix).and_then(|h| h.grid_position)?;
        let h2 = grid_manager
            .translate_by_edge(&pos1, edge)
            .and_then(|pos2| grid_manager.pos_to_object(pos2.light()))
            .map(|obj| obj.helix())?;
        Some(Nucl {
            helix: h2,
            position: nucl1.position + shift,
            forward: nucl1.forward,
        })
    }

    fn domains_vec_to_pasted_strands(
        &self,
        domains_vec: Vec<Vec<Domain>>,
        helices: &Helices,
        strands: &Strands,
        parameters: &Parameters,
    ) -> Vec<PastedStrand> {
        let mut pasted_strands = vec![];
        for domains in domains_vec.into_iter() {
            let mut nucl_position = Vec::with_capacity(domains.len() * 15);
            for dom in domains.iter() {
                if let Domain::HelixDomain(dom) = dom {
                    let helix = helices.get(&dom.helix).unwrap();
                    for position in dom.iter() {
                        nucl_position.push(helix.space_pos(parameters, position, dom.forward));
                    }
                }
            }
            let pastable = Self::can_add_domains(strands, &domains);
            pasted_strands.push(PastedStrand {
                domains,
                nucl_position,
                pastable,
            });
        }
        pasted_strands
    }

    fn can_add_domains(strands: &Strands, domains: &[Domain]) -> bool {
        for s in strands.values() {
            if s.intersect_domains(domains) {
                return false;
            }
        }
        true
    }

    pub(super) fn apply_paste(&mut self, design: Design) -> Result<Design, ErrOperation> {
        match self.state {
            ControllerState::PastingXovers { .. }
            | ControllerState::DoingFirstXoversDuplication { .. } => {
                self.apply_paste_xovers(design)
            }
            ControllerState::PositioningStrandPastingPoint { .. }
            | ControllerState::PositioningStrandDuplicationPoint { .. } => {
                self.apply_paste_strands(design)
            }
            ControllerState::PositioningHelicesPastingPoint { .. }
            | ControllerState::PositioningHelicesDuplicationPoint { .. } => {
                self.apply_paste_helices(design)
            }
            _ => Err(ErrOperation::IncompatibleState(format!(
                "Duplication impossible in state {}",
                self.state.state_name()
            ))),
        }
    }

    fn apply_paste_xovers(&mut self, design: Design) -> Result<Design, ErrOperation> {
        self.state = ControllerState::Normal;
        Ok(design)
    }

    fn apply_paste_helices(&mut self, design: Design) -> Result<Design, ErrOperation> {
        self.state = ControllerState::Normal;
        Ok(design)
    }

    fn apply_paste_strands(&mut self, mut design: Design) -> Result<Design, ErrOperation> {
        let pasted_strands = match &self.state {
            ControllerState::PositioningStrandPastingPoint { pasted_strands, .. } => {
                Ok(pasted_strands)
            }
            ControllerState::PositioningStrandDuplicationPoint { pasted_strands, .. } => {
                Ok(pasted_strands)
            }
            _ => Err(ErrOperation::IncompatibleState(format!(
                "Pasting strand impossible in state {}",
                self.state.state_name()
            ))),
        }?;
        Self::add_pasted_strands_to_design(&mut self.color_idx, &mut design, pasted_strands)?;
        self.state = ControllerState::Normal;
        Ok(design)
    }

    fn add_pasted_strands_to_design(
        color_idx: &mut usize,
        design: &mut Design,
        pasted_strands: &[PastedStrand],
    ) -> Result<(), ErrOperation> {
        if pasted_strands.get(0).map(|s| s.pastable) == Some(false) {
            return Err(ErrOperation::CannotPasteHere);
        }
        for pasted_strand in pasted_strands.iter() {
            let color = Self::new_color(color_idx);
            if pasted_strand.pastable {
                let junctions =
                    ensnano_design::read_junctions(pasted_strand.domains.as_slice(), false);
                let strand = Strand {
                    domains: pasted_strand.domains.clone(),
                    color,
                    junctions,
                    sequence: None,
                    cyclic: false,
                    name: None,
                };
                let strand_id = if let Some(n) = design.strands.keys().max() {
                    n + 1
                } else {
                    0
                };
                design.strands.insert(strand_id, strand.clone());
            }
        }
        Ok(())
    }

    pub(super) fn apply_duplication(&mut self, mut design: Design) -> Result<Design, ErrOperation> {
        let state = &mut self.state;
        match state.clone() {
            ControllerState::PositioningStrandDuplicationPoint {
                pasted_strands,
                pasting_point,
                duplication_edge,
                clipboard,
            } => {
                // first duplication
                if let Some((nucl, duplication_edge)) = pasting_point.zip(duplication_edge) {
                    Self::add_pasted_strands_to_design(
                        &mut self.color_idx,
                        &mut design,
                        &pasted_strands,
                    )?;
                    *state = ControllerState::WithPendingStrandDuplication {
                        last_pasting_point: nucl,
                        duplication_edge,
                        clipboard,
                    };
                } else {
                    // If it is not possible to ducplicate here, cancel the duplication
                    self.state = ControllerState::Normal
                }
                Ok(design)
            }
            ControllerState::WithPendingStrandDuplication {
                last_pasting_point,
                duplication_edge,
                clipboard,
                ..
            } => {
                let mut data = design.mut_strand_and_data();
                let new_duplication_point = self
                    .translate_nucl_by_edge(
                        &last_pasting_point,
                        &duplication_edge.0,
                        duplication_edge.1,
                        data.helices,
                        data.grid_data,
                    )
                    .ok_or(ErrOperation::CannotPasteHere)?;
                let (pasted_strands, _) =
                    self.paste_clipboard(&clipboard, new_duplication_point, &mut data)?;
                Self::add_pasted_strands_to_design(
                    &mut self.color_idx,
                    &mut design,
                    &pasted_strands,
                )?;
                self.state = ControllerState::WithPendingStrandDuplication {
                    last_pasting_point: new_duplication_point,
                    duplication_edge,
                    clipboard,
                };
                Ok(design)
            }
            ControllerState::DoingFirstXoversDuplication {
                xovers,
                pasting_point,
                duplication_edge,
                ..
            } => {
                if let Some((nucl, duplication_edge)) = pasting_point.zip(duplication_edge) {
                    self.state = ControllerState::WithPendingXoverDuplication {
                        last_pasting_point: nucl,
                        duplication_edge,
                        xovers,
                    };
                } else {
                    self.state = ControllerState::Normal
                }
                Ok(design)
            }
            ControllerState::WithPendingXoverDuplication {
                last_pasting_point,
                duplication_edge,
                xovers,
            } => {
                let data = design.mut_strand_and_data();
                let new_duplication_point = self
                    .translate_nucl_by_edge(
                        &last_pasting_point,
                        &duplication_edge.0,
                        duplication_edge.1,
                        data.helices,
                        data.grid_data,
                    )
                    .ok_or(ErrOperation::CannotPasteHere)?;
                let n1 = xovers
                    .get(0)
                    .map(|n| n.0)
                    .ok_or(ErrOperation::EmptyClipboard)?;
                let edge = Self::edge_beteen_nucls(
                    data.helices,
                    data.grid_data,
                    &n1,
                    &new_duplication_point,
                )
                .ok_or(ErrOperation::CannotPasteHere)?;
                self.put_xovers_on_design(
                    data.grid_data,
                    data.strands,
                    data.helices,
                    &xovers,
                    edge,
                )?;
                self.state = ControllerState::WithPendingXoverDuplication {
                    last_pasting_point: new_duplication_point,
                    duplication_edge,
                    xovers,
                };
                Ok(design)
            }
            ControllerState::WithPendingHelicesDuplication {
                last_pasting_point,
                duplication_edge,
                helices,
            } => {
                let data = design.get_updated_grid_data().clone();
                let new_duplication_point = data
                    .translate_by_edge(&last_pasting_point.to_helix_pos(), &duplication_edge)
                    .ok_or(ErrOperation::CannotPasteHere)?;
                let h_id0 = helices.get(0).ok_or(ErrOperation::EmptyClipboard)?;
                let edge = data
                    .get_helix_grid_position(*h_id0)
                    .as_ref()
                    .zip(Some(new_duplication_point))
                    .and_then(|(source, dest)| {
                        log::info!("source {:?}, dest {:?}", source, dest);
                        data.get_edge(source, &dest)
                    })
                    .ok_or(ErrOperation::CannotPasteHere)?;
                let mut helices_mut = design.helices.make_mut();
                for h_id in helices.iter() {
                    let h = helices_mut
                        .get(h_id)
                        .ok_or(ErrOperation::HelixDoesNotExists(*h_id))?;
                    if let Some(copy) = h.translated_by(edge, &data) {
                        log::info!("adding helix");
                        helices_mut.push_helix(copy);
                    }
                }
                self.state = ControllerState::WithPendingHelicesDuplication {
                    last_pasting_point: new_duplication_point.light(),
                    duplication_edge,
                    helices,
                };
                drop(helices_mut);
                Ok(design)
            }
            ControllerState::PositioningHelicesDuplicationPoint {
                pasting_point,
                duplication_edge,
                helices,
                ..
            } => {
                if let Some((grid_pos, duplication_edge)) = pasting_point.zip(duplication_edge) {
                    self.state = ControllerState::WithPendingHelicesDuplication {
                        last_pasting_point: grid_pos,
                        duplication_edge,
                        helices,
                    };
                } else {
                    self.state = ControllerState::Normal
                }
                Ok(design)
            }
            _ => Err(ErrOperation::IncompatibleState(format!(
                "Pasting helices impossible in state {}",
                self.state.state_name()
            ))),
        }
    }

    pub fn get_pasted_position(&self) -> Vec<(Vec<Vec3>, bool)> {
        match self.state {
            ControllerState::PositioningStrandPastingPoint {
                ref pasted_strands, ..
            } => pasted_strands
                .iter()
                .map(|s| (s.nucl_position.clone(), s.pastable))
                .collect(),
            ControllerState::PositioningStrandDuplicationPoint {
                ref pasted_strands, ..
            } => pasted_strands
                .iter()
                .map(|s| (s.nucl_position.clone(), s.pastable))
                .collect(),
            _ => vec![],
        }
    }

    pub fn get_copy_points(&self) -> Vec<Vec<Nucl>> {
        let pasted_strands = match self.state {
            ControllerState::PositioningStrandPastingPoint {
                ref pasted_strands, ..
            } => pasted_strands,
            ControllerState::PositioningStrandDuplicationPoint {
                ref pasted_strands, ..
            } => pasted_strands,
            _ => return vec![],
        };

        let mut ret = Vec::new();
        for strand in pasted_strands.iter() {
            let mut points = Vec::new();
            for domain in strand.domains.iter() {
                if let Domain::HelixDomain(domain) = domain {
                    if domain.forward {
                        points.push(Nucl::new(domain.helix, domain.start, domain.forward));
                        points.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                    } else {
                        points.push(Nucl::new(domain.helix, domain.end - 1, domain.forward));
                        points.push(Nucl::new(domain.helix, domain.start, domain.forward));
                    }
                }
            }
            ret.push(points)
        }
        ret
    }

    pub(super) fn get_pasting_point(&self) -> Option<Option<PastePosition>> {
        match self.state {
            ControllerState::PositioningStrandPastingPoint { pasting_point, .. } => {
                Some(pasting_point.map(|p| PastePosition::Nucl(p.clone())))
            }
            ControllerState::PositioningStrandDuplicationPoint { pasting_point, .. } => {
                Some(pasting_point.map(|p| PastePosition::Nucl(p.clone())))
            }
            ControllerState::DoingFirstXoversDuplication { pasting_point, .. } => {
                Some(pasting_point.map(|p| PastePosition::Nucl(p.clone())))
            }
            ControllerState::PastingXovers { pasting_point, .. } => {
                Some(pasting_point.map(|p| PastePosition::Nucl(p.clone())))
            }
            ControllerState::PositioningHelicesPastingPoint { pasting_point, .. } => {
                Some(pasting_point.map(|p| PastePosition::GridPosition(p.clone())))
            }
            _ => None,
        }
    }

    pub(super) fn copy_xovers(&mut self, xovers: Vec<(Nucl, Nucl)>) -> Result<(), ErrOperation> {
        if xovers.len() > 0 {
            self.clipboard = AddressPointer::new(Clipboard::Xovers(xovers))
        } else {
            self.clipboard = Default::default()
        }
        Ok(())
    }

    pub(super) fn copy_helices(&mut self, helices: Vec<usize>) -> Result<(), ErrOperation> {
        if helices.len() > 0 {
            self.clipboard = AddressPointer::new(Clipboard::Helices(helices))
        } else {
            self.clipboard = Default::default()
        }
        Ok(())
    }

    pub(super) fn get_design_beign_pasted_on(&self) -> Option<&AddressPointer<Design>> {
        match &self.state {
            ControllerState::PastingXovers { initial_design, .. } => Some(initial_design),
            ControllerState::DoingFirstXoversDuplication { initial_design, .. } => {
                Some(initial_design)
            }
            ControllerState::PositioningHelicesPastingPoint { initial_design, .. } => {
                Some(initial_design)
            }
            ControllerState::PositioningHelicesDuplicationPoint { initial_design, .. } => {
                Some(initial_design)
            }
            _ => None,
        }
    }

    fn position_helices_copy(
        &mut self,
        design: &mut Design,
        helices: Vec<usize>,
        position: Option<PastePosition>,
    ) -> Result<(), ErrOperation> {
        let data = design.get_updated_grid_data();
        let h_id0 = helices.get(0).ok_or(ErrOperation::EmptyClipboard)?;
        log::info!("position = {:?}", position);
        log::info!(
            "source position = {:?}",
            data.get_helix_grid_position(*h_id0)
        );
        let edge = data
            .get_helix_grid_position(*h_id0)
            .as_ref()
            .zip(position.and_then(PastePosition::to_grid_position).as_ref())
            .and_then(|(source, dest)| {
                log::info!("source {:?}, dest {:?}", source, dest);
                data.get_edge(source, &dest.to_helix_pos())
            });
        log::info!("edge = {:?}", edge);
        let grid_data = data.clone();
        self.state
            .update_helices_pasting_position(position, edge, design)?;
        if let Some(edge) = edge {
            log::info!("edge is some");
            let mut helices_mut = design.helices.make_mut();
            for h_id in helices {
                let h = helices_mut
                    .get(&h_id)
                    .ok_or(ErrOperation::HelixDoesNotExists(h_id))?;
                if let Some(copy) = h.translated_by(edge, &grid_data) {
                    log::info!("adding helix");
                    helices_mut.push_helix(copy);
                }
            }
        }
        Ok(())
    }

    fn position_xover_copies(
        &mut self,
        design: &mut Design,
        nucl: Option<Nucl>,
    ) -> Result<(), ErrOperation> {
        let data = design.mut_strand_and_data();
        let n1 = self
            .clipboard
            .get_leading_xover_nucl()
            .ok_or(ErrOperation::WrongClipboard)?;
        let edge = nucl
            .as_ref()
            .and_then(|n2| Self::edge_beteen_nucls(data.helices, data.grid_data, &n1, n2));
        self.state
            .update_xover_pasting_position(nucl, edge, design)?;
        let data = design.mut_strand_and_data();
        if cfg!(test) {
            if edge.is_none() {
                println!("EDGE IS NONE");
            }
        }
        if let Some(edge) = edge {
            let clipboard = self.clipboard.clone();
            let xovers = match clipboard.as_ref() {
                Clipboard::Xovers(xovers) => Ok(xovers),
                _ => Err(ErrOperation::WrongClipboard),
            }?;
            self.put_xovers_on_design(data.grid_data, data.strands, data.helices, xovers, edge)?;
        }
        Ok(())
    }

    fn put_xovers_on_design(
        &mut self,
        grid_manager: &GridData,
        strands: &mut Strands,
        helices: &Helices,
        xovers: &[(Nucl, Nucl)],
        copy_edge: (Edge, isize),
    ) -> Result<(), ErrOperation> {
        let (edge, shift) = copy_edge;
        for (n1, n2) in xovers.iter() {
            let copy_1 = self.translate_nucl_by_edge(n1, &edge, shift, helices, grid_manager);
            log::debug!("copy 1 {:?}", copy_1);
            let copy_2 = self.translate_nucl_by_edge(n2, &edge, shift, helices, grid_manager);
            log::debug!("copy 2 {:?}", copy_2);
            if let Some((copy_1, copy_2)) = copy_1.zip(copy_2) {
                if !strands.is_true_xover_end(&copy_1) && !strands.is_true_xover_end(&copy_2) {
                    // If general_cross_over returns an error we simply ignore this cross_over
                    self.general_cross_over(strands, copy_1, copy_2)
                        .unwrap_or_default();
                }
            }
        }
        Ok(())
    }
}

pub enum CopyOperation {
    CopyGrids(Vec<GridId>),
    CopyStrands(Vec<usize>),
    CopyXovers(Vec<(Nucl, Nucl)>),
    CopyHelices(Vec<usize>),
    InitStrandsDuplication(Vec<usize>),
    InitXoverDuplication(Vec<(Nucl, Nucl)>),
    InitHelicesDuplication(Vec<usize>),
    PositionPastingPoint(Option<PastePosition>),
    Paste,
    Duplicate,
}
