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
    AddressPointer, Controller, ControllerState, Design, Domain, ErrOperation, GridManager,
    GridPosition, HelixInterval, Nucl, Strand,
};
use ensnano_design::grid::Edge;
use ultraviolet::Vec3;

pub(super) enum Clipboard {
    Empty,
    Strands(StrandClipboard),
}

impl Clipboard {
    pub fn size(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Strands(strand_clipboard) => strand_clipboard.templates.len(),
        }
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::Empty
    }
}

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
    helix: GridPosition,
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
    pub fn set_templates(
        &mut self,
        design: &Design,
        strand_ids: Vec<usize>,
    ) -> Result<(), ErrOperation> {
        let grid_manager = GridManager::new_from_design(design);
        let mut templates = Vec::with_capacity(strand_ids.len());
        for id in strand_ids.iter() {
            let strand = design
                .strands
                .get(&id)
                .ok_or(ErrOperation::StrandDoesNotExist(*id))?;
            let template = self.strand_to_template(strand, design, &grid_manager)?;
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
                        design,
                        &grid_manager,
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
        design: &Design,
        grid_manager: &GridManager,
    ) -> Result<StrandTemplate, ErrOperation> {
        let mut origin: Option<TemplateOrigin> = None;
        let mut domains = Vec::with_capacity(strand.domains.len());
        let mut edges = Vec::with_capacity(strand.domains.len());
        let mut previous_position = None;
        for domain in strand.domains.iter() {
            match domain {
                Domain::Insertion(n) => domains.push(DomainTemplate::Insertion(*n)),
                Domain::HelixDomain(dom) => {
                    if let Some(ref pos1) = previous_position {
                        let helix = design
                            .helices
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
                        let helix = design
                            .helices
                            .get(&dom.helix)
                            .ok_or(ErrOperation::HelixDoesNotExists(dom.helix))?;
                        let grid_position = helix
                            .grid_position
                            .ok_or(ErrOperation::HelixHasNoGridPosition(dom.helix))?;
                        let start = if dom.forward { dom.start } else { dom.end };
                        origin = Some(TemplateOrigin {
                            helix: grid_position,
                            start: start,
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
        design: &Design,
        grid_manager: &GridManager,
    ) -> Option<(Edge, isize)> {
        let strand1 = design.strands.get(&s_id1)?;
        let strand2 = design.strands.get(&s_id2)?;
        let nucl1 = strand1.get_5prime()?;
        let nucl2 = strand2.get_5prime()?;
        let pos1 = design
            .helices
            .get(&nucl1.helix)
            .and_then(|h| h.grid_position)?;
        let pos2 = design
            .helices
            .get(&nucl2.helix)
            .and_then(|h| h.grid_position)?;
        grid_manager
            .get_edge(&pos1, &pos2)
            .zip(Some(nucl2.position - nucl1.position))
    }

    pub(super) fn position_strand_copies(
        &mut self,
        design: &Design,
        nucl: Option<Nucl>,
    ) -> Result<(), ErrOperation> {
        let grid_manager = GridManager::new_from_design(design);
        let strand_clipboard = if let Clipboard::Strands(clipboard) = self.clipboard.as_ref() {
            Ok(clipboard)
        } else {
            Err(ErrOperation::EmptyClipboard)
        }?;
        let mut duplication_edge = None;
        let template_0 = strand_clipboard
            .templates
            .get(0)
            .ok_or(ErrOperation::EmptyClipboard)?;
        let domains_0 = nucl.and_then(|n| {
            self.template_to_domains(template_0, n, &mut duplication_edge, design, &grid_manager)
        });
        if let Some(domains) = domains_0 {
            let mut domains_vec = vec![domains];
            for n in 1..strand_clipboard.templates.len() {
                let t = strand_clipboard.templates.get(n);
                println!("updated template {:?}", t);
                let domains = t.as_ref().and_then(|t| {
                    nucl.as_ref().and_then(|nucl| {
                        strand_clipboard
                            .template_edges
                            .get(n - 1)
                            .and_then(|(e, s)| {
                                self.translate_nucl_by_edge(nucl, e, *s, design, &grid_manager)
                            })
                            .and_then(|n2| {
                                self.template_to_domains(t, n2, &mut None, design, &grid_manager)
                            })
                    })
                });
                if let Some(domains) = domains {
                    domains_vec.push(domains);
                }
            }
            let pasted_strands = self.domains_vec_to_pasted_strands(domains_vec, design);
            self.state
                .update_pasting_position(nucl, pasted_strands, duplication_edge)
        } else {
            self.state.update_pasting_position(nucl, vec![], None)
        }
    }

    fn template_to_domains(
        &self,
        template: &StrandTemplate,
        start_nucl: Nucl,
        duplication_info: &mut Option<(Edge, isize)>,
        design: &Design,
        grid_manager: &GridManager,
    ) -> Option<Vec<Domain>> {
        let mut ret = Vec::with_capacity(template.domains.len());
        let mut edge_iter = template.edges.iter();
        let mut previous_position: Option<GridPosition> = None;
        let mut edge_opt = None;
        let shift = if template.origin.forward {
            start_nucl.position - template.origin.start
        } else {
            start_nucl.position - template.origin.start + 1
        };
        for domain in template.domains.iter() {
            match domain {
                DomainTemplate::Insertion(n) => ret.push(Domain::Insertion(*n)),
                DomainTemplate::HelixInterval {
                    start,
                    end,
                    forward,
                } => {
                    if let Some(ref pos1) = previous_position {
                        let edge = edge_iter.next()?;
                        let pos2 = grid_manager.translate_by_edge(pos1, edge)?;
                        let helix = grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y)?;
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
                        let pos2 = design
                            .helices
                            .get(&start_nucl.helix)
                            .and_then(|h| h.grid_position)?;

                        edge_opt = grid_manager.get_edge(&position, &pos2);
                        if grid_manager.get_edge(&position, &pos2).is_none() {
                            return None;
                        }
                        let helix = grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y)?;

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
        Some(ret)
    }

    fn translate_nucl_by_edge(
        &self,
        nucl1: &Nucl,
        edge: &Edge,
        shift: isize,
        design: &Design,
        grid_manager: &GridManager,
    ) -> Option<Nucl> {
        let pos1 = design
            .helices
            .get(&nucl1.helix)
            .and_then(|h| h.grid_position)?;
        let h2 = grid_manager
            .translate_by_edge(&pos1, edge)
            .and_then(|pos2| grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y))?;
        Some(Nucl {
            helix: h2,
            position: nucl1.position + shift,
            forward: nucl1.forward,
        })
    }

    fn domains_vec_to_pasted_strands(
        &mut self,
        domains_vec: Vec<Vec<Domain>>,
        design: &Design,
    ) -> Vec<PastedStrand> {
        let mut pasted_strands = vec![];
        for domains in domains_vec.into_iter() {
            let mut nucl_position = Vec::with_capacity(domains.len() * 15);
            for dom in domains.iter() {
                if let Domain::HelixDomain(dom) = dom {
                    let helix = design.helices.get(&dom.helix).unwrap();
                    let parameters = design.parameters.unwrap_or_default();
                    for position in dom.iter() {
                        nucl_position.push(helix.space_pos(&parameters, position, dom.forward));
                    }
                }
            }
            let pastable = Self::can_add_domains(design, &domains);
            pasted_strands.push(PastedStrand {
                domains,
                nucl_position,
                pastable,
            });
        }
        pasted_strands
    }

    fn can_add_domains(design: &Design, domains: &[Domain]) -> bool {
        for s in design.strands.values() {
            if s.intersect_domains(domains) {
                return false;
            }
        }
        true
    }

    pub(super) fn apply_paste(&mut self, mut design: Design) -> Result<Design, ErrOperation> {
        let pasted_strands = match &mut self.state {
            ControllerState::PositioningPastingPoint { pasted_strands, .. } => Ok(pasted_strands),
            ControllerState::PositioningDuplicationPoint { pasted_strands, .. } => {
                Ok(pasted_strands)
            }
            _ => Err(ErrOperation::IncompatibleState),
        }?;
        if pasted_strands.get(0).map(|s| s.pastable) == Some(false) {
            return Err(ErrOperation::CannotPasteHere);
        }
        for pasted_strand in pasted_strands.iter() {
            let color = Self::new_color(&mut self.color_idx);
            if pasted_strand.pastable {
                let junctions =
                    ensnano_design::read_junctions(pasted_strand.domains.as_slice(), false);
                let strand = Strand {
                    domains: pasted_strand.domains.clone(),
                    color,
                    junctions,
                    sequence: None,
                    cyclic: false,
                };
                let strand_id = if let Some(n) = design.strands.keys().max() {
                    n + 1
                } else {
                    0
                };
                design.strands.insert(strand_id, strand.clone());
            }
        }
        self.state = ControllerState::Normal;
        Ok(design)
    }

    pub fn get_pasted_position(&self) -> Vec<(Vec<Vec3>, bool)> {
        match self.state {
            ControllerState::PositioningPastingPoint {
                ref pasted_strands, ..
            } => pasted_strands
                .iter()
                .map(|s| (s.nucl_position.clone(), s.pastable))
                .collect(),
            ControllerState::PositioningDuplicationPoint {
                ref pasted_strands, ..
            } => pasted_strands
                .iter()
                .map(|s| (s.nucl_position.clone(), s.pastable))
                .collect(),
            _ => vec![],
        }
    }

    pub(super) fn get_pasting_point(&self) -> Option<Option<Nucl>> {
        match self.state {
            ControllerState::PositioningPastingPoint { pasting_point, .. } => {
                Some(pasting_point.clone())
            }
            ControllerState::PositioningDuplicationPoint { pasting_point, .. } => {
                Some(pasting_point.clone())
            }
            _ => None,
        }
    }
}

pub enum CopyOperation {
    CopyStrands(Vec<usize>),
    CopyXovers(Vec<usize>),
    InitStrandsDuplication(Vec<usize>),
    IntiXoverDuplication(Vec<usize>),
    PositionPastingPoint(Option<Nucl>),
    Paste,
    Duplicate,
}
