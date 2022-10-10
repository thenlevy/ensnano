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

use ensnano_interactor::InsertionPoint;

use super::*;

impl Controller {
    pub(super) fn update_insertion_length(
        &mut self,
        mut design: Design,
        insertion_point: InsertionPoint,
        length: usize,
    ) -> Result<Design, ErrOperation> {
        let s_id = design
            .strands
            .get_strand_nucl(&insertion_point.nucl)
            .ok_or(ErrOperation::NuclDoesNotExist(insertion_point.nucl))?;
        let strand_mut = design
            .strands
            .get_mut(&s_id)
            .ok_or(ErrOperation::StrandDoesNotExist(s_id))?;

        let cyclic = strand_mut.cyclic;
        if cyclic {
            let prime3 = strand_mut
                .get_3prime()
                .ok_or(ErrOperation::CouldNotGetPrime3of(s_id))?;
            Self::split_strand(&mut design.strands, &prime3, None, &mut self.color_idx)?;
        }

        let strand_mut = design
            .strands
            .get_mut(&s_id)
            .ok_or(ErrOperation::StrandDoesNotExist(s_id))?;

        if let Some(insertion_mut) = get_insertion_length_mut(strand_mut, insertion_point) {
            if length > 0 {
                *insertion_mut.length = length;
                Ok(design)
            } else {
                let d_id = insertion_mut.domain_id;
                strand_mut.domains.remove(d_id);
                strand_mut.junctions.remove(d_id);
                strand_mut.merge_consecutive_domains();
                Ok(design)
            }
        } else if length > 0 {
            // if the nucl is the 5' end of the insertion we want it to be the 3' end of the
            // resulting strand, and therefore be on the 5' end of the split
            let forced_end = Some(!insertion_point.nucl_is_prime5_of_insertion);

            let s_2 = Self::split_strand(
                &mut design.strands,
                &insertion_point.nucl,
                forced_end,
                &mut self.color_idx,
            )?;
            let strand_mut = design
                .strands
                .get_mut(&s_id)
                .ok_or(ErrOperation::StrandDoesNotExist(s_id))?;
            if cfg!(test) {
                println!(
                    "junction after split {}",
                    strand_mut.formated_anonymous_junctions()
                )
            }
            if insertion_point.nucl_is_prime5_of_insertion {
                // The nucl is the 3' end of the splited strand
                let insertion_junction_id = strand_mut.domains.len() - 1;
                strand_mut.domains.push(Domain::new_insertion(length));
                strand_mut
                    .junctions
                    .insert(insertion_junction_id, DomainJunction::Adjacent);
                if let Some(strand) = design.strands.get(&s_2) {
                    if strand.length() > 0 {
                        if s_2 != s_id {
                            Self::merge_strands(&mut design.strands, s_id, s_2)?;
                        } else {
                            Self::make_cycle(&mut design.strands, s_id, true)?;
                        }
                    } else {
                        design.strands.remove(&s_2);
                    }
                }
            } else {
                // the nucl is the 5' end of the splited strand
                strand_mut
                    .domains
                    .insert(0, Domain::new_prime5_insertion(length));
                strand_mut.junctions.insert(0, DomainJunction::Adjacent);
                if cfg!(test) {
                    println!(
                        "After adding junction, merging {}",
                        strand_mut.formated_anonymous_junctions()
                    );
                }
                if let Some(strand) = design.strands.get(&s_2) {
                    if strand.length() > 0 {
                        if s_2 != s_id {
                            if cfg!(test) {
                                println!("with {}", strand.formated_anonymous_junctions());
                            }
                            Self::merge_strands(&mut design.strands, s_2, s_id)?;
                            // The merged strand has id `s_2`, set it back to `s_id`
                            if let Some(merged_strand) = design.strands.remove(&s_2) {
                                design.strands.insert(s_id, merged_strand);
                            }
                        } else {
                            println!("with itself");
                            Self::make_cycle(&mut design.strands, s_id, true)?;
                        }
                    } else {
                        design.strands.remove(&s_2);
                    }
                }
            }
            if cyclic {
                Self::make_cycle(&mut design.strands, s_id, true)?;
            }

            Ok(design)
        } else {
            // Nothing to do
            Err(ErrOperation::NotImplemented)
        }
    }
}

/// If there already is an insertion at insertion point, return a mutable reference to its
/// length. Otherwise return None
fn get_insertion_length_mut<'a>(
    strand: &'a mut Strand,
    insertion_point: InsertionPoint,
) -> Option<InsertionMut<'a>> {
    let mut insertion_id: Option<usize> = None;
    let domains_iterator: Box<dyn Iterator<Item = ((usize, &Domain), (usize, &Domain))>> =
        if strand.cyclic {
            Box::new(
                strand
                    .domains
                    .iter()
                    .enumerate()
                    .zip(strand.domains.iter().cycle().enumerate().skip(1)),
            )
        } else {
            Box::new(
                strand
                    .domains
                    .iter()
                    .enumerate()
                    .zip(strand.domains.iter().enumerate().skip(1)),
            )
        };
    if insertion_point.nucl_is_prime5_of_insertion {
        // look for an insertion after the domain ending with the desired nucl
        for ((_, d_nucl), (d_id, d_insertion)) in domains_iterator {
            if d_nucl.prime3_end() == Some(insertion_point.nucl) {
                if let Domain::Insertion { .. } = d_insertion {
                    insertion_id = Some(d_id);
                } else {
                    insertion_id = None;
                }
                break;
            }
        }
    } else {
        // look for an insertion before the domain ending with the desired nucl
        for ((d_id, d_insertion), (_, d_nucl)) in domains_iterator {
            if d_nucl.prime5_end() == Some(insertion_point.nucl) {
                if let Domain::Insertion { .. } = d_insertion {
                    insertion_id = Some(d_id);
                } else {
                    insertion_id = None;
                }
                break;
            }
        }
    }

    if let Some(Domain::Insertion { nb_nucl, .. }) =
        insertion_id.and_then(move |id| strand.domains.get_mut(id))
    {
        Some(InsertionMut {
            domain_id: insertion_id.unwrap(),
            length: nb_nucl,
        })
    } else {
        None
    }
}

struct InsertionMut<'a> {
    domain_id: usize,
    length: &'a mut usize,
}
