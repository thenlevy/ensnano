use super::grid::Edge;
use super::GridPosition;
use super::{icednano::Domain, icednano::HelixInterval, Data, Nucl, Strand};
use ultraviolet::Vec3;

#[derive(Debug, Clone)]
pub struct StrandPatron {
    origin: PatronOrigin,
    domains: Vec<DomainPatron>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
struct PatronOrigin {
    helix: GridPosition,
    start: isize,
    forward: bool,
}

#[derive(Debug, Clone)]
pub enum DomainPatron {
    Insertion(usize),
    HelixInterval {
        start: isize,
        end: isize,
        forward: bool,
    },
}

pub struct PastedStrand {
    pub domains: Vec<Domain>,
    pub nucl_position: Vec<Vec3>,
    pub pastable: bool,
}

impl Data {
    pub fn strand_to_patron(&self, strand: &Strand) -> Option<StrandPatron> {
        let mut origin: Option<PatronOrigin> = None;
        let mut domains = Vec::with_capacity(strand.domains.len());
        let mut edges = Vec::with_capacity(strand.domains.len());
        let mut previous_position = None;
        for domain in strand.domains.iter() {
            match domain {
                Domain::Insertion(n) => domains.push(DomainPatron::Insertion(*n)),
                Domain::HelixDomain(dom) => {
                    if let Some(ref pos1) = previous_position {
                        let helix = self.design.helices.get(&dom.helix)?;
                        let pos2 = helix.grid_position?;
                        let edge = self.grid_manager.get_edge(pos1, &pos2)?;
                        edges.push(edge);
                        previous_position = Some(pos2);
                        domains.push(DomainPatron::HelixInterval {
                            start: dom.start,
                            end: dom.end,
                            forward: dom.forward,
                        });
                    } else {
                        let helix = self.design.helices.get(&dom.helix)?;
                        let grid_position = helix.grid_position?;
                        let start = if dom.forward { dom.start } else { dom.end };
                        origin = Some(PatronOrigin {
                            helix: grid_position,
                            start: start,
                            forward: dom.forward,
                        });
                        previous_position = Some(grid_position);
                        domains.push(DomainPatron::HelixInterval {
                            start: dom.start,
                            end: dom.end,
                            forward: dom.forward,
                        });
                    }
                }
            }
        }
        origin.map(|origin| StrandPatron {
            origin,
            domains,
            edges,
        })
    }

    pub fn patron_to_domains(
        &self,
        patron: &StrandPatron,
        start_nucl: Nucl,
    ) -> Option<Vec<Domain>> {
        let mut ret = Vec::with_capacity(patron.domains.len());
        let mut edge_iter = patron.edges.iter();
        let mut previous_position: Option<GridPosition> = None;
        let shift = if start_nucl.forward {
            start_nucl.position - patron.origin.start
        } else {
            start_nucl.position - patron.origin.start + 1
        };
        for domain in patron.domains.iter() {
            match domain {
                DomainPatron::Insertion(n) => ret.push(Domain::Insertion(*n)),
                DomainPatron::HelixInterval {
                    start,
                    end,
                    forward,
                } => {
                    if let Some(ref pos1) = previous_position {
                        let edge = edge_iter.next()?;
                        let pos2 = self.grid_manager.translate_by_edge(pos1, edge)?;
                        let helix = self.grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y)?;
                        ret.push(Domain::HelixDomain(HelixInterval {
                            helix,
                            start: start + shift,
                            end: end + shift,
                            forward: *forward,
                            sequence: None,
                        }));
                        previous_position = Some(pos2);
                    } else {
                        let position = patron.origin.helix;
                        let pos2 = self
                            .design
                            .helices
                            .get(&start_nucl.helix)
                            .and_then(|h| h.grid_position)?;

                        if self.grid_manager.get_edge(&position, &pos2).is_none() {
                            return None;
                        }
                        let helix = self.grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y)?;

                        ret.push(Domain::HelixDomain(HelixInterval {
                            helix,
                            start: start + shift,
                            end: end + shift,
                            forward: patron.origin.forward,
                            sequence: None,
                        }));
                        previous_position = Some(pos2);
                    }
                }
            }
        }
        Some(ret)
    }

    pub(super) fn update_pasted_strand(&mut self, domains: Option<Vec<Domain>>) {
        if let Some(domains) = domains {
            let mut nucl_position = Vec::with_capacity(domains.len() * 15);
            for dom in domains.iter() {
                if let Domain::HelixDomain(dom) = dom {
                    let helix = self.design.helices.get(&dom.helix).unwrap();
                    let parameters = self.design.parameters.unwrap_or_default();
                    for position in dom.iter() {
                        nucl_position.push(helix.space_pos(&parameters, position, dom.forward));
                    }
                }
            }
            let pastable = self.can_add_domains(&domains);
            self.pasted_strand = Some(PastedStrand {
                domains,
                nucl_position,
                pastable,
            });
        } else {
            self.pasted_strand = None
        }
    }
}
