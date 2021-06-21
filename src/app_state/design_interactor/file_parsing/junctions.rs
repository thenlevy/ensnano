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

use super::IdGenerator;
use ensnano_design::*;
pub(super) trait StrandJunction {
    /// Read the junctions for self when loading the design.
    /// If `identified` is true (i.e. during the first pass), read the IdentifiedXover
    /// and insert them in the xover_ids.
    /// If `identified` is false (i.e. during the second pass), read the unidentified Xover and
    /// provide them with identifier
    ///
    /// Assumes that self.junctions is either empty or a Vec with the following properties.
    /// * Its length is equal to self.domains.length
    /// * All the junctions are appropriate.
    fn read_junctions(&mut self, xover_ids: &mut IdGenerator<(Nucl, Nucl)>, identified: bool);
}

impl StrandJunction for Strand {
    fn read_junctions(&mut self, xover_ids: &mut IdGenerator<(Nucl, Nucl)>, identified: bool) {
        //TODO check validity of self.junctions
        if self.junctions.is_empty() {
            let sane_domains = sanitize_domains(&self.domains, self.cyclic);
            self.domains = sane_domains;
            let junctions = read_junctions(&self.domains, self.cyclic);
            self.junctions = junctions;
        }
        if self.domains.is_empty() {
            return;
        }
        let mut previous_domain = self.domains.last().unwrap();
        for i in 0..(self.domains.len()) {
            let current = &self.domains[i];
            let next = if i == self.domains.len() - 1 {
                if self.cyclic {
                    &self.domains[0]
                } else {
                    break;
                }
            } else {
                &self.domains[i + 1]
            };
            match &mut self.junctions[i] {
                s @ DomainJunction::UnindentifiedXover => {
                    if !identified {
                        if let (Domain::HelixDomain(d1), Domain::HelixDomain(d2)) = (current, next)
                        {
                            let prime5 = d1.prime3();
                            let prime3 = d2.prime5();
                            let id = xover_ids.insert((prime5, prime3));
                            *s = DomainJunction::IdentifiedXover(id);
                        } else if let (Domain::HelixDomain(d1), Domain::HelixDomain(d2)) =
                            (previous_domain, next)
                        {
                            let prime5 = d1.prime3();
                            let prime3 = d2.prime5();
                            let id = xover_ids.insert((prime5, prime3));
                            *s = DomainJunction::IdentifiedXover(id);
                        } else if let Domain::Insertion(_) = next {
                            panic!("UnindentifiedXover before an insertion");
                        } else if let Domain::Insertion(_) = previous_domain {
                            panic!("Invariant violated: [SaneDomains]");
                        } else {
                            unreachable!("Unexhastive match");
                        }
                    }
                }
                DomainJunction::IdentifiedXover(id) => {
                    if identified {
                        if let (Domain::HelixDomain(d1), Domain::HelixDomain(d2)) = (current, next)
                        {
                            let prime5 = d1.prime3();
                            let prime3 = d2.prime5();
                            xover_ids.insert_at((prime5, prime3), *id);
                        } else if let (Domain::HelixDomain(d1), Domain::HelixDomain(d2)) =
                            (previous_domain, next)
                        {
                            let prime5 = d1.prime3();
                            let prime3 = d2.prime5();
                            xover_ids.insert_at((prime5, prime3), *id);
                        } else if let Domain::Insertion(_) = next {
                            panic!("UnindentifiedXover before an insertion");
                        } else if let Domain::Insertion(_) = previous_domain {
                            panic!("Invariant violated: [SaneDomains]");
                        } else {
                            unreachable!("Unexhastive match");
                        }
                    }
                }
                _ => (),
            }
            if let Domain::HelixDomain(_) = current {
                previous_domain = current;
            }
        }
    }
}

/// Return the appropriate junction between two HelixInterval
pub(super) fn junction(prime5: &HelixInterval, prime3: &HelixInterval) -> DomainJunction {
    let prime5_nucl = prime5.prime3();
    let prime3_nucl = prime3.prime5();

    if prime3_nucl == prime5_nucl.prime3() {
        DomainJunction::Adjacent
    } else {
        DomainJunction::UnindentifiedXover
    }
}

/// Add the correct juction between current and next to junctions.
/// Assumes and preseve the following invariant
/// Invariant [read_junctions::PrevDomain]: One of the following is true
/// * the strand is not cyclic
/// * the strand is cyclic and its first domain is NOT and insertion.
/// * previous domain points to some Domain::HelixDomain.
///
/// Moreover at the end of each iteration of the loop, previous_domain points to some
/// Domain::HelixDomain. The loop is responsible for preserving the invariant. The invariant is
/// true at initilasation if [SaneDomains] is true.
fn add_juction<'b, 'a: 'b>(
    junctions: &'b mut Vec<DomainJunction>,
    current: &'a Domain,
    next: &'a Domain,
    previous_domain: &'b mut &'a Domain,
    cyclic: bool,
    i: usize,
) {
    match next {
        Domain::Insertion(_) => {
            junctions.push(DomainJunction::Adjacent);
            if let Domain::HelixDomain(_) = current {
                *previous_domain = current;
            } else {
                panic!("Invariant violated: SaneDomains");
            }
        }
        Domain::HelixDomain(prime3) => {
            match current {
                Domain::Insertion(_) => {
                    if i == 0 && !cyclic {
                        // The first domain IS an insertion
                        junctions.push(DomainJunction::Adjacent);
                    } else {
                        // previous domain MUST point to some Domain::HelixDomain.
                        if let Domain::HelixDomain(prime5) = *previous_domain {
                            junctions.push(junction(prime5, prime3))
                        } else {
                            if i == 0 {
                                panic!("Invariant violated: SaneDomains");
                            } else {
                                panic!("Invariant violated: read_junctions::PrevDomain");
                            }
                        }
                    }
                }
                Domain::HelixDomain(prime5) => {
                    junctions.push(junction(prime5, prime3));
                    *previous_domain = current;
                }
            }
        }
    }
}

/// Infer juctions from a succession of domains.
pub(super) fn read_junctions(domains: &[Domain], cyclic: bool) -> Vec<DomainJunction> {
    if domains.len() == 0 {
        return vec![];
    }

    let mut ret = Vec::with_capacity(domains.len());
    let mut previous_domain = &domains[domains.len() - 1];

    for i in 0..(domains.len() - 1) {
        let current = &domains[i];
        let next = &domains[i + 1];
        add_juction(&mut ret, current, next, &mut previous_domain, cyclic, i);
    }

    if cyclic {
        let last = &domains[domains.len() - 1];
        let first = &domains[0];
        add_juction(
            &mut ret,
            last,
            first,
            &mut previous_domain,
            cyclic,
            domains.len() - 1,
        );
    } else {
        ret.push(DomainJunction::Prime3)
    }

    ret
}
