use super::grid::Edge;
use super::GridPosition;
use super::{icednano::Domain, icednano::HelixInterval, Data, Nucl, Strand};
use ultraviolet::Vec3;

/// A template describing the relation between the domains of a strand. Can be used for copy-paste
/// of strands.
#[derive(Debug, Clone)]
pub struct StrandTemplate {
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
pub enum DomainTemplate {
    Insertion(usize),
    HelixInterval {
        start: isize,
        end: isize,
        forward: bool,
    },
}

#[derive(Debug)]
pub struct PastedStrand {
    pub domains: Vec<Domain>,
    pub nucl_position: Vec<Vec3>,
    pub pastable: bool,
}

#[derive(Default, Debug)]
pub struct TemplateManager {
    pub templates: Vec<StrandTemplate>,
    pub template_edges: Vec<(Edge, isize)>,
    pub pasted_strands: Vec<PastedStrand>,
    pub duplication_edge: Option<(Edge, isize)>,
    pub starting_nucl: Option<Nucl>,
}

impl TemplateManager {
    pub fn update_templates(&mut self, templates: Vec<StrandTemplate>, edges: Vec<(Edge, isize)>) {
        println!("edges {:?}", edges);
        self.templates = templates;
        self.template_edges = edges;
    }

    pub fn update_chief_template(&mut self, template: StrandTemplate) {
        if let Some(t) = self.templates.get_mut(0) {
            *t = template
        }
    }
}

impl Data {
    pub fn strand_to_template(&self, strand: &Strand) -> Option<StrandTemplate> {
        let mut origin: Option<TemplateOrigin> = None;
        let mut domains = Vec::with_capacity(strand.domains.len());
        let mut edges = Vec::with_capacity(strand.domains.len());
        let mut previous_position = None;
        for domain in strand.domains.iter() {
            match domain {
                Domain::Insertion(n) => domains.push(DomainTemplate::Insertion(*n)),
                Domain::HelixDomain(dom) => {
                    if let Some(ref pos1) = previous_position {
                        let helix = self.design.helices.get(&dom.helix)?;
                        let pos2 = helix.grid_position?;
                        let edge = self.grid_manager.get_edge(pos1, &pos2)?;
                        edges.push(edge);
                        previous_position = Some(pos2);
                        domains.push(DomainTemplate::HelixInterval {
                            start: dom.start,
                            end: dom.end,
                            forward: dom.forward,
                        });
                    } else {
                        let helix = self.design.helices.get(&dom.helix)?;
                        let grid_position = helix.grid_position?;
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
        origin.map(|origin| StrandTemplate {
            origin,
            domains,
            edges,
        })
    }

    pub fn template_to_domains(
        &self,
        template: &StrandTemplate,
        start_nucl: Nucl,
        duplication_info: &mut Option<(Edge, isize)>,
    ) -> Option<Vec<Domain>> {
        let mut ret = Vec::with_capacity(template.domains.len());
        let mut edge_iter = template.edges.iter();
        let mut previous_position: Option<GridPosition> = None;
        let mut edge_opt = None;
        let shift = if start_nucl.forward {
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
                        let position = template.origin.helix;
                        let pos2 = self
                            .design
                            .helices
                            .get(&start_nucl.helix)
                            .and_then(|h| h.grid_position)?;

                        edge_opt = self.grid_manager.get_edge(&position, &pos2);
                        if self.grid_manager.get_edge(&position, &pos2).is_none() {
                            return None;
                        }
                        let helix = self.grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y)?;

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

    pub fn duplicate_template(
        &self,
        template: &StrandTemplate,
        first_edge: Edge,
        shift: isize,
        additional_edge: Option<(Edge, isize)>,
    ) -> Option<Vec<Domain>> {
        let mut ret = Vec::with_capacity(template.domains.len());
        let mut edge_iter = template.edges.iter();
        let mut previous_position: Option<GridPosition> = None;
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
                        let position = template.origin.helix;
                        let mut pos2 = self
                            .grid_manager
                            .translate_by_edge(&position, &first_edge)?;

                        println!("pos2 {:?}", pos2);
                        let start = if let Some((edge2, shift2)) = additional_edge {
                            println!("additional edge");
                            pos2 = self.grid_manager.translate_by_edge(&pos2, &edge2)?;
                            start + shift + shift2
                        } else {
                            start + shift
                        };
                        println!("pos2 => {:?}", pos2);

                        let helix = self.grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y)?;

                        ret.push(Domain::HelixDomain(HelixInterval {
                            helix,
                            start,
                            end: end + shift,
                            forward: template.origin.forward,
                            sequence: None,
                        }));
                        previous_position = Some(pos2);
                    }
                }
            }
        }
        Some(ret)
    }

    pub(super) fn update_pasted_strand(&mut self, domains_vec: Vec<Vec<Domain>>) {
        let mut pasted_strands = vec![];
        for domains in domains_vec.into_iter() {
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
            pasted_strands.push(PastedStrand {
                domains,
                nucl_position,
                pastable,
            });
        }
        self.template_manager.pasted_strands = pasted_strands;
    }

    pub fn set_templates(&mut self, strand_ids: Vec<usize>) {
        let templates: Option<Vec<StrandTemplate>> = strand_ids
            .iter()
            .map(|id| {
                self.design
                    .strands
                    .get(&id)
                    .and_then(|s| self.strand_to_template(s))
            })
            .collect();
        let templates = templates.unwrap_or(vec![]);
        let mut edges = vec![];
        if templates.len() == 0 {
            self.template_manager.update_templates(vec![], vec![])
        } else {
            if let Some(s_id1) = strand_ids.get(0) {
                for s_id2 in strand_ids.iter().skip(1) {
                    edges.push(self.edge_between_strands(*s_id1, *s_id2));
                }
            }
            let edges = edges.into_iter().collect::<Option<Vec<(Edge, isize)>>>();
            if let Some(edges) = edges {
                self.template_manager.update_templates(templates, edges);
            } else {
                self.template_manager.update_templates(vec![], vec![]);
            }
        }
    }

    pub fn set_copy(&mut self, nucl: Option<Nucl>) {
        let mut duplication_edge = None;
        let domains_0 = nucl.and_then(|n| {
            self.template_manager
                .templates
                .get(0)
                .and_then(|t| self.template_to_domains(t, n, &mut duplication_edge))
        });
        if let Some(domains) = domains_0 {
            self.template_manager.duplication_edge = duplication_edge;
            self.template_manager.starting_nucl = nucl;
            let mut domains_vec = vec![domains];
            for n in 1..self.template_manager.templates.len() {
                let t = self.template_manager.templates.get(n);
                println!("updated template {:?}", t);
                let domains = t.as_ref().and_then(|t| {
                    nucl.as_ref().and_then(|nucl| {
                        self.template_manager.template_edges.get(n-1)
                            .and_then(|(e, s)| self.translate_nucl_by_edge(nucl, *e, *s))
                            .and_then(|n2| self.template_to_domains(t, n2, &mut None))
                    })
                });
                if let Some(domains) = domains {
                    domains_vec.push(domains);
                }
            }
            self.update_pasted_strand(domains_vec);
        } else {
            self.update_pasted_strand(vec![]);
        }
        self.hash_maps_update = true;
        self.update_status = true;
    }

    pub fn apply_copy(&mut self) -> Vec<(Strand, usize)> {
        let mut ret = Vec::with_capacity(self.template_manager.pasted_strands.len());
        let mut first = true;
        let mut chief_id = None;
        for pasted_strand in self.template_manager.pasted_strands.iter() {
            let color = super::new_color(&mut self.color_idx);
            if self.can_add_domains(&pasted_strand.domains) {
                let strand = super::icednano::Strand {
                    domains: pasted_strand.domains.clone(),
                    color,
                    sequence: None,
                    cyclic: false,
                };
                let strand_id = if let Some(n) = self.design.strands.keys().max() {
                    n + 1
                } else {
                    0
                };
                self.design.strands.insert(strand_id, strand.clone());
                if first {
                    chief_id = Some(strand_id);
                    first = false;
                }
                ret.push((strand, strand_id))
            }
        }
        if let Some(s_id) = chief_id {
            self.update_chief_template(s_id)
        }
        ret
    }

    pub fn apply_duplication(&mut self) -> Vec<(Strand, usize)> {
        let mut domains_vec = Vec::with_capacity(self.template_manager.templates.len());
        let starting_nucl = self.template_manager.starting_nucl.and_then(|n| self.template_manager.duplication_edge.and_then(|d| self.translate_nucl_by_edge(&n, d.0, d.1)));
        println!("starting nucl {:?}", starting_nucl);
        self.template_manager.starting_nucl = starting_nucl;
        for n in 0..self.template_manager.templates.len() {
            let template = self.template_manager.templates.get(n);
            let domains = if n > 0 {
                template.as_ref().and_then(|t| {
                    starting_nucl.as_ref().and_then(|nucl| {
                        self.template_manager.template_edges.get(n-1)
                            .and_then(|(e, s)| self.translate_nucl_by_edge(nucl, *e, *s))
                            .and_then(|n2| {
                                println!("n2 {:?}", n2);
                                self.template_to_domains(t, n2, &mut None)})
                    })
                })
            } else {
                template.as_ref().and_then(|t| {
                starting_nucl.as_ref().and_then(|n2| {
                            println!("n2 {:?}", n2);
                            self.template_to_domains(t, *n2, &mut None)})
                })
            };
            if let Some(domains) = domains {
                domains_vec.push(domains);
            } else if n == 0 {
                return vec![]
            }
        }
        self.update_pasted_strand(domains_vec);
        self.hash_maps_update = true;
        self.update_status = true;
        self.apply_copy()
    }

    fn update_chief_template(&mut self, s_id: usize) {
        let template = self
            .design
            .strands
            .get(&s_id)
            .and_then(|s| self.strand_to_template(s))
            .expect("update chief template");
        self.template_manager.update_chief_template(template);
    }

    fn updated_template(&self, n: usize) -> Option<StrandTemplate> {
        let chief = self.template_manager.templates.get(0).cloned()?;
        let chief_origin = chief.origin.clone();
        if n == 0 {
            Some(chief)
        } else {
            let (edge, shift) = self.template_manager.template_edges.get(n - 1)?;
            let mut ret = self.template_manager.templates.get(n).cloned()?;
            println!("chief origin {:?}", chief_origin);
            println!("edge {:?}, shift {:?}", edge, shift);
            let pos2 = self
                .grid_manager
                .translate_by_edge(&chief_origin.helix, edge)?;
            let new_origin = TemplateOrigin {
                helix: pos2,
                start: chief_origin.start + shift,
                forward: ret.origin.forward,
            };
            ret.origin = new_origin;
            println!("ret {:?}", ret);
            Some(ret)
        }
    }

    fn edge_between_strands(&self, s_id1: usize, s_id2: usize) -> Option<(Edge, isize)> {
        let strand1 = self.design.strands.get(&s_id1)?;
        let strand2 = self.design.strands.get(&s_id2)?;
        let nucl1 = strand1.get_5prime()?;
        let nucl2 = strand2.get_5prime()?;
        let pos1 = self
            .design
            .helices
            .get(&nucl1.helix)
            .and_then(|h| h.grid_position)?;
        let pos2 = self
            .design
            .helices
            .get(&nucl2.helix)
            .and_then(|h| h.grid_position)?;
        self.grid_manager
            .get_edge(&pos1, &pos2)
            .zip(Some(nucl2.position - nucl1.position))
    }

    fn translate_nucl_by_edge(&self, nucl1: &Nucl, edge: Edge, shift: isize) -> Option<Nucl> {
        let pos1 = self
            .design
            .helices
            .get(&nucl1.helix)
            .and_then(|h| h.grid_position)?;
        let h2 = self
            .grid_manager
            .translate_by_edge(&pos1, &edge)
            .and_then(|pos2| self.grid_manager.pos_to_helix(pos2.grid, pos2.x, pos2.y))?;
        Some(Nucl {
            helix: h2,
            position: nucl1.position + shift,
            forward: nucl1.forward,
        })
    }
}
